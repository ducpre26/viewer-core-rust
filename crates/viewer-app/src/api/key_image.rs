//! UC-006 Java contract: pending precedes DICOM writes; failed ingest is not atomic.
use super::response::Error;
use crate::{
    auth::Auth,
    dicom::Jvm,
    persistence::{Persistence, key_image as entity},
};
use futures_util::{StreamExt, stream};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::Duration,
};
use summer::config::Configurable;
use summer_web::{
    Router,
    aide::axum::routing::post,
    axum::{
        Extension,
        body::Body,
        extract::{OriginalUri, Path, Request},
        http::{StatusCode, header},
        response::{IntoResponse, Response},
    },
};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Deserialize, Configurable)]
#[config_prefix = "key_image"]
#[serde(default, deny_unknown_fields)]
pub struct KeyImageSettings {
    pub dicom_path: PathBuf,
    pub temp_directory: PathBuf,
    pub max_file: i32,
    pub send_to_store: bool,
    pub interval_ms: u64,
}
impl Default for KeyImageSettings {
    fn default() -> Self {
        Self {
            dicom_path: PathBuf::from("keyImages"),
            temp_directory: std::env::temp_dir().join("keyImages"),
            max_file: 10,
            send_to_store: false,
            interval_ms: 30_000,
        }
    }
}
impl KeyImageSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.interval_ms == 0
            || self.dicom_path.as_os_str().is_empty()
            || self.temp_directory.as_os_str().is_empty()
        {
            Err("Invalid key_image paths or interval_ms")
        } else {
            Ok(())
        }
    }
}
#[derive(Clone)]
pub struct KeyImageRuntime {
    settings: KeyImageSettings,
    database: Persistence,
    auth: Auth,
    jvm: Option<Jvm>,
    gate: Arc<Mutex<()>>,
    client: reqwest::Client,
}
// A queued/running JNI job retains both spool files and the storage exclusion lock.
// HTTP cancellation cannot let the scheduler delete a file still used by Java.
struct Job {
    directory: tempfile::TempDir,
    _guard: OwnedMutexGuard<()>,
}
impl Job {
    fn path(&self) -> &FsPath {
        self.directory.path()
    }
}
fn failure(code: &'static str, message: impl Into<String>, status: u16) -> Error {
    Error(StatusCode::from_u16(status).unwrap(), message.into(), code)
}
fn io_error(_: std::io::Error) -> Error {
    failure(
        "KEY_IMG_GLOBAL_ERROR",
        "Key image file operation failed",
        400,
    )
}
fn generic() -> Error {
    failure("KEY_IMG_GLOBAL_ERROR", "Key image global error", 400)
}
fn java_error(value: &Value) -> Option<Error> {
    let code = match value.get("errorCode")?.as_str()? {
        "BAD_REQUEST" => "BAD_REQUEST",
        "KEY_IMG_MISSING_TAG" => "KEY_IMG_MISSING_TAG",
        "KEY_IMG_SUPPLEMENT_ERROR" => "KEY_IMG_SUPPLEMENT_ERROR",
        "KEY_IMG_SAVE_FILE_ERROR" => "KEY_IMG_SAVE_FILE_ERROR",
        _ => "KEY_IMG_GLOBAL_ERROR",
    };
    Some(failure(
        code,
        value["message"]
            .as_str()
            .unwrap_or("Key image global error"),
        value["status"].as_u64().unwrap_or(400) as u16,
    ))
}
impl KeyImageRuntime {
    pub fn new(
        settings: KeyImageSettings,
        database: Persistence,
        auth: Auth,
        jvm: Option<Jvm>,
    ) -> Result<Self, &'static str> {
        settings.validate()?;
        Ok(Self {
            settings,
            database,
            auth,
            jvm,
            gate: Arc::new(Mutex::new(())),
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .map_err(|_| "Key image HTTP client configuration")?,
        })
    }
    async fn job(&self) -> Result<Arc<Job>, Error> {
        let guard = self.gate.clone().lock_owned().await;
        fs::create_dir_all(&self.settings.temp_directory).map_err(io_error)?;
        let directory = tempfile::Builder::new()
            .prefix("ingest-")
            .tempdir_in(&self.settings.temp_directory)
            .map_err(io_error)?;
        Ok(Arc::new(Job {
            directory,
            _guard: guard,
        }))
    }
    async fn call(&self, op: &str, mut manifest: Value, job: Arc<Job>) -> Result<Value, Error> {
        manifest["job"] = json!(job.path());
        manifest["storage"] = json!(self.settings.dicom_path);
        let jvm = self.jvm.as_ref().ok_or_else(|| {
            failure(
                "SERVICE_UNAVAILABLE",
                "Key image requires configured JNI",
                503,
            )
        })?;
        let result = jvm
            .key_image(op.to_owned(), manifest.to_string(), job)
            .await
            .map_err(|e| {
                let status = match e {
                    dicom_jvm::Error::Busy
                    | dicom_jvm::Error::WorkerStopped
                    | dicom_jvm::Error::Contract => 503,
                    dicom_jvm::Error::Timeout => 504,
                    _ => 500,
                };
                failure("INTERNAL_ERROR", e.to_string(), status)
            })?;
        let value: Value = serde_json::from_str(&result).map_err(|_| generic())?;
        if let Some(e) = java_error(&value) {
            return Err(e);
        }
        Ok(value)
    }
    pub async fn run(self) {
        loop {
            if let Err(_error) = self.dispatch_once().await {
                tracing::warn!("Key image dispatch failed");
            }
            tokio::time::sleep(Duration::from_millis(self.settings.interval_ms)).await;
        }
    }
    pub(super) async fn dispatch_once(&self) -> Result<(), Error> {
        if !self.settings.send_to_store {
            return Ok(());
        }
        let _guard = self.gate.lock().await;
        for row in entity::pending(self.database.connection()).await? {
            let result = self.stow(&row).await;
            match result {
                Ok(()) => {
                    let series = row.series_iuid.clone();
                    match entity::Entity::delete_by_id(row.id.clone())
                        .exec(self.database.connection())
                        .await
                    {
                        Ok(_) => {
                            if let Ok(path) = series_path(&self.settings.dicom_path, &series) {
                                let _ = fs::remove_dir_all(path);
                            }
                        }
                        Err(e) => {
                            entity::failed(self.database.connection(), row, e.to_string()).await?;
                        }
                    }
                }
                Err(reason) => entity::failed(self.database.connection(), row, reason).await?,
            }
        }
        Ok(())
    }
    async fn stow(&self, row: &entity::Model) -> Result<(), String> {
        let path = series_path(&self.settings.dicom_path, &row.series_iuid)
            .map_err(|_| "Key image series path unavailable".to_owned())?;
        let mut paths = Vec::new();
        regular_files(&path, &mut paths)
            .map_err(|_| "Key image directory unavailable".to_owned())?;
        // Files.walk in Java includes metadata.json. File-read errors are logged and skipped.
        let stream = stow_stream(paths);
        let mut request = self
            .client
            .post(format!("{}/studies", row.store_url))
            .header(
                header::CONTENT_TYPE,
                "multipart/related; type=\"application/dicom\"; boundary=myboundary",
            )
            .body(reqwest::Body::wrap_stream(stream));
        if let Some(auth) = row.store_auth.as_ref().filter(|a| !a.is_empty()) {
            request = request.header(header::AUTHORIZATION, auth);
        }
        let response = request
            .send()
            .await
            .map_err(|e| e.without_url().to_string())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(response
                .text()
                .await
                .map_err(|e| e.without_url().to_string())?)
        }
    }
}
fn stow_stream(
    paths: Vec<PathBuf>,
) -> impl futures_util::Stream<Item = Result<Vec<u8>, std::io::Error>> {
    stream::unfold(
        (paths.into_iter(), None::<File>, false),
        |(mut paths, mut file, done)| async move {
            if done {
                return None;
            }
            loop {
                if let Some(mut input) = file.take() {
                    let mut buffer = vec![0_u8; 8192];
                    match input.read(&mut buffer) {
                        Ok(0) => {}
                        Ok(n) => {
                            buffer.truncate(n);
                            return Some((
                                Ok::<_, std::io::Error>(buffer),
                                (paths, Some(input), false),
                            ));
                        }
                        Err(_) => tracing::warn!("Error reading key image file"),
                    }
                }
                match paths.next() {
                    Some(path) => {
                        match File::open(path) {
                            Ok(input) => return Some((
                                Ok(
                                    b"\r\n--myboundary\r\nContent-Type: application/dicom\r\n\r\n"
                                        .to_vec(),
                                ),
                                (paths, Some(input), false),
                            )),
                            Err(_) => tracing::warn!("Error reading key image file"),
                        }
                    }
                    None => {
                        return Some((Ok(b"\r\n--myboundary--\r\n".to_vec()), (paths, None, true)));
                    }
                }
            }
        },
    )
}
fn regular_files(path: &FsPath, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            regular_files(&entry.path(), result)?;
        } else if kind.is_file() {
            result.push(entry.path());
        }
    }
    Ok(())
}
// Do not let a supplied UID become an arbitrary filesystem path. DICOM/business
// validation remains in Java; this boundary is limited to file containment.
fn series_path(root: &FsPath, series: &str) -> Result<PathBuf, Error> {
    if series.is_empty() || series.contains(['/', '\\', ':']) || series == "." || series == ".." {
        return Err(generic());
    }
    Ok(root.join(series))
}
async fn ingest(
    Extension(runtime): Extension<KeyImageRuntime>,
    Path(id): Path<String>,
    original: Option<Extension<OriginalUri>>,
    request: Request,
) -> Result<Response, Error> {
    let job = runtime.job().await?;
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost")
        .to_owned();
    let uri = original
        .map(|e| e.0.0)
        .unwrap_or_else(|| request.uri().clone());
    let retrieve = format!(
        "{}://{host}{}",
        uri.scheme_str().unwrap_or("http"),
        uri.path()
    );
    let mut output = File::create(job.path().join("request")).map_err(io_error)?;
    let mut body = request.into_body().into_data_stream();
    while let Some(chunk) = body.next().await {
        output
            .write_all(&chunk.map_err(|_| generic())?)
            .map_err(io_error)?;
    }
    drop(output);
    let parsed = runtime
        .call("parse", json!({"contentType":content_type}), job.clone())
        .await?;
    let count = parsed["count"].as_i64().unwrap_or(0) as i32;
    if count == 0 {
        return Err(failure("KEY_IMG_GLOBAL_ERROR", "No key image data", 400));
    }
    let study = parsed["study"]
        .as_str()
        .ok_or_else(|| failure("KEY_IMG_MISSING_TAG", "Missing tag: StudyInstanceUID", 400))?;
    let old = entity::Entity::find_by_id(study)
        .one(runtime.database.connection())
        .await?;
    if count > runtime.settings.max_file - old.as_ref().map_or(0, |r| r.num_of_image) {
        return Err(failure("KEY_IMG_MAXIMUM", "Maximum key image reached", 400));
    }
    let series = parsed["series"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| old.as_ref().map(|r| r.series_iuid.clone()))
        .unwrap_or_else(|| format!("2.25.{}", uuid::Uuid::new_v4().as_u128()));
    let session = runtime
        .auth
        .session(&id)
        .await?
        .ok_or_else(|| failure("SESSION_NOT_FOUND", "Session not found", 404))?;
    let now = chrono::Utc::now();
    if let Some(old) = old {
        let total = old.num_of_image + count;
        let mut active: entity::ActiveModel = old.into();
        active.num_of_image = Set(total);
        active.succeed = Set(false);
        active.num_of_retry = Set(1);
        active.description = Set(Some("Send with new key image".to_owned()));
        active.last_updated_time = Set(Some(now.into()));
        active.last_updated_id = Set(Some(1));
        active.update(runtime.database.connection()).await?;
    } else {
        entity::ActiveModel {
            id: Set(study.to_owned()),
            series_iuid: Set(series.clone()),
            num_of_image: Set(count),
            store_url: Set(session.model.store_url.clone().ok_or_else(generic)?),
            store_auth: Set(session.model.store_auth.clone()),
            succeed: Set(false),
            num_of_retry: Set(1),
            description: Set(None),
            created_time: Set(now.into()),
            creator_id: Set(1),
            last_updated_time: Set(None),
            last_updated_id: Set(None),
        }
        .insert(runtime.database.connection())
        .await?;
    }
    // Java writes pending before files; no rollback when this call fails.
    let value = runtime
        .call("write", json!({"series":series,"retrieve":retrieve}), job)
        .await?;
    Ok((
        [(header::CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
        .into_response())
}
pub(super) fn router() -> Router {
    Router::new().api_route("/v1/session/{sessionID}/studies", post(ingest))
}

pub(super) async fn frame(
    runtime: &KeyImageRuntime,
    study: &str,
    series: &str,
    sop: &str,
) -> Result<Option<Response>, Error> {
    let job = runtime.job().await?;
    let row = entity::Entity::find_by_id(study)
        .one(runtime.database.connection())
        .await?;
    if !row.is_some_and(|r| r.series_iuid == series) {
        return Ok(None);
    }
    let value = runtime
        .call("frame", json!({"series":series,"sop":sop}), job.clone())
        .await?;
    if value["missing"] == true {
        return Ok(None);
    }
    let file = File::open(job.path().join("frame")).map_err(io_error)?;
    let stream = stream::try_unfold((file, job), |(mut file, owner)| async move {
        let mut buffer = vec![0_u8; 64 * 1024];
        let n = file.read(&mut buffer)?;
        buffer.truncate(n);
        Ok::<_, std::io::Error>(if n == 0 {
            None
        } else {
            Some((buffer, (file, owner)))
        })
    });
    Ok(Some(
        (
            [(
                header::CONTENT_TYPE,
                "multipart/related; boundary=myboundary",
            )],
            Body::from_stream(stream),
        )
            .into_response(),
    ))
}
pub(super) async fn merge(
    runtime: &KeyImageRuntime,
    study: &str,
    series: Option<&str>,
    retrieve: &str,
    remote: &[u8],
) -> Result<Option<Value>, Error> {
    let job = runtime.job().await?;
    let Some(row) = entity::Entity::find_by_id(study)
        .one(runtime.database.connection())
        .await?
    else {
        return Ok(None);
    };
    if series.is_some_and(|s| s != row.series_iuid) {
        return Ok(None);
    }
    series_path(&runtime.settings.dicom_path, &row.series_iuid)?;
    fs::write(job.path().join("remote.json"), remote).map_err(io_error)?;
    let value=runtime.call("merge",json!({"study":study,"series":row.series_iuid,"listing":series.is_none(),"retrieve":retrieve}),job).await?;
    if value["missing"] == true {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(super) async fn has_pending(runtime: &KeyImageRuntime, study: &str) -> bool {
    entity::Entity::find_by_id(study)
        .one(runtime.database.connection())
        .await
        .is_ok_and(|r| r.is_some())
}

#[cfg(test)]
mod tests;
