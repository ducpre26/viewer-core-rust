//! UC-005: Rust transport/ZIP with owned, file-based Java codec jobs.
use crate::{
    auth::{Auth, CachedSession},
    persistence::config::ConfigStore,
};
use dicom_jvm::{FileOperation, FileRequest, Jvm};
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use summer::config::Configurable;
use summer_web::{
    Router,
    aide::axum::routing::get,
    axum::{
        Extension,
        body::{Body, Bytes},
        extract::{Path, Request},
        http::{StatusCode, header},
        response::{IntoResponse, Response},
    },
};
mod storage;
use storage::{Cleanup, Lease};

#[derive(Clone, Deserialize, Configurable)]
#[config_prefix = "video"]
#[serde(default, deny_unknown_fields)]
pub struct VideoSettings {
    pub fps: i32,
}
impl Default for VideoSettings {
    fn default() -> Self {
        Self { fps: 1 }
    }
}
impl VideoSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.fps <= 0 {
            Err("video.fps must be positive")
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct DownloadRuntime {
    video: VideoSettings,
    temp_dir: PathBuf,
    jvm: Option<Jvm>,
    client: reqwest::Client,
    _cleanup: Arc<Cleanup>,
}
impl DownloadRuntime {
    fn jvm(&self) -> Result<&Jvm, Failure> {
        self.jvm
            .as_ref()
            .filter(|jvm| jvm.supports_download())
            .ok_or_else(|| {
                error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "SERVICE_UNAVAILABLE",
                    "This download operation requires configured JNI",
                )
            })
    }
    pub fn new(video: VideoSettings, jvm: Option<Jvm>) -> Result<Self, &'static str> {
        video.validate()?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Download HTTP client configuration")?;
        let temp_dir = storage::root().map_err(|_| "Download temporary directory unavailable")?;
        let cleanup = Arc::new(Cleanup::start(temp_dir.clone()));
        Ok(Self {
            video,
            temp_dir,
            jvm,
            client,
            _cleanup: cleanup,
        })
    }
}
#[derive(Clone)]
struct Download {
    auth: Auth,
    configs: ConfigStore,
}
struct Job {
    runtime: DownloadRuntime,
    lease: Arc<Lease>,
    sequence: u64,
}
#[derive(Deserialize)]
struct Instance {
    sop: String,
    frames: i32,
    path: String,
}
#[derive(Deserialize)]
struct Series {
    sop: String,
    index: String,
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dicom,
    Jpeg,
    Mp4,
}
use super::response::Error as Failure;
fn error(status: StatusCode, code: &'static str, message: &str) -> Failure {
    Failure(status, message.into(), code)
}
fn failed() -> Failure {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        "Download failed",
    )
}
fn jni_error(e: dicom_jvm::Error) -> Failure {
    match e {
        dicom_jvm::Error::Busy => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "Download queue full",
        ),
        dicom_jvm::Error::Timeout => error(
            StatusCode::GATEWAY_TIMEOUT,
            "DOWNLOAD_TIMEOUT",
            "Download deadline exceeded",
        ),
        _ => failed(),
    }
}
fn kind(value: &str, series: bool) -> Result<Kind, Failure> {
    if value.eq_ignore_ascii_case("dicom") {
        Ok(Kind::Dicom)
    } else if value.eq_ignore_ascii_case("jpeg") {
        Ok(Kind::Jpeg)
    } else if series && value.eq_ignore_ascii_case("mp4") {
        Ok(Kind::Mp4)
    } else {
        Err(error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_MEDIA_TYPE",
            "Unsupported media type",
        ))
    }
}
fn safe_zip_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 4096
        && !name.contains(['\\', ':', '\0', '\r', '\n'])
        && !name.starts_with('/')
        && name.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
fn safe_id(value: &str) -> bool {
    // Java accepts opaque String identifiers. Keep that behavior while preventing
    // path traversal and response-header injection; do not impose UID grammar.
    !value.is_empty()
        && value.len() <= 4096
        && !matches!(value, "." | "..")
        && !value.contains(['/', '\\', '\0', '\r', '\n'])
}
impl Job {
    fn file(&mut self) -> PathBuf {
        self.sequence += 1;
        self.lease
            .directory
            .path()
            .join(format!("{}.bin", self.sequence))
    }
    async fn fetch(
        &mut self,
        url: url::Url,
        session: &CachedSession,
        media: Option<&'static str>,
    ) -> Result<PathBuf, Failure> {
        let mut request = self.runtime.client.get(url);
        if let Some(auth) = &session.store_auth {
            request = request.header(header::AUTHORIZATION, auth);
        }
        if let Some(media) = media {
            request = request.header(header::ACCEPT, media);
        }
        let response = request.send().await.map_err(|_| failed())?;
        if response.status() != StatusCode::OK {
            return Err(failed());
        }
        if let Some(media) = media
            && !response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .is_some_and(|h| {
                    h.split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case(media)
                })
        {
            return Err(failed());
        }
        let path = self.file();
        let target = path.clone();
        owned_io(self.lease.clone(), move || {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map(drop)
        })
        .await?;
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(|_| failed())?;
            let target = path.clone();
            owned_io(self.lease.clone(), move || {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(target)?
                    .write_all(&chunk)
            })
            .await?;
        }
        Ok(path)
    }
    async fn metadata(
        &mut self,
        url: url::Url,
        session: &CachedSession,
    ) -> Result<String, Failure> {
        let file = self.fetch(url, session, None).await?;
        let text = owned_io(self.lease.clone(), move || std::fs::read_to_string(file)).await?;
        Ok(text)
    }
    async fn jpeg(&mut self, url: url::Url, session: &CachedSession) -> Result<PathBuf, Failure> {
        let output = self.fetch(url, session, Some("image/jpeg")).await?;
        let path = output.clone();
        owned_io(self.lease.clone(), move || verify_jpeg(&path)).await?;
        Ok(output)
    }
    async fn codec(
        &mut self,
        input: PathBuf,
        operation: FileOperation,
    ) -> Result<PathBuf, Failure> {
        let output = self.file();
        self.runtime
            .jvm()?
            .download_file(FileRequest {
                operation,
                input,
                output: output.clone(),
                owner: self.lease.clone(),
            })
            .await
            .map_err(jni_error)?;
        Ok(output)
    }
}
fn upstream(
    store: &str,
    study: &str,
    series: Option<&str>,
    instance: Option<&str>,
    metadata: bool,
) -> Result<url::Url, Failure> {
    let mut url = url::Url::parse(store).map_err(|_| failed())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(failed());
    }
    url.set_query(None);
    url.set_fragment(None);
    {
        let mut path = url.path_segments_mut().map_err(|_| failed())?;
        path.pop_if_empty();
        path.push("studies").push(study);
        if let Some(series) = series {
            path.push("series").push(series);
        } else {
            path.push("series");
        }
        if let Some(instance) = instance {
            path.push("instances").push(instance);
        }
        if metadata {
            path.push("metadata");
        }
    }
    Ok(url)
}
fn wado_uri(store: &str, study: &str, series: &str, instance: &str) -> Result<url::Url, Failure> {
    let base = store.rsplit_once('/').map_or(store, |(base, _)| base);
    let mut url = url::Url::parse(base).map_err(|_| failed())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(failed());
    }
    url.set_query(None);
    url.set_fragment(None);
    url.path_segments_mut()
        .map_err(|_| failed())?
        .pop_if_empty()
        .push("wado");
    url.query_pairs_mut().extend_pairs([
        ("studyUID", study),
        ("seriesUID", series),
        ("objectUID", instance),
        ("requestType", "WADO"),
        ("contentType", "application/dicom"),
    ]);
    Ok(url)
}

fn rendered_frame(
    store: &str,
    study: &str,
    series: &str,
    sop: &str,
    frame: i32,
) -> Result<url::Url, Failure> {
    let mut url = upstream(store, study, Some(series), Some(sop), false)?;
    url.path_segments_mut()
        .map_err(|_| failed())?
        .push("frames")
        .push(&(frame + 1).to_string())
        .push("rendered");
    url.query_pairs_mut().append_pair("quality", "90");
    Ok(url)
}

fn instance_frames(json: &str) -> Result<i32, Failure> {
    let metadata: serde_json::Value = serde_json::from_str(json).map_err(|_| failed())?;
    let first = metadata
        .as_array()
        .and_then(|v| v.first())
        .filter(|v| v.is_object())
        .ok_or_else(failed)?;
    let value = &first["00280008"]["Value"][0];
    Ok(value
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .or_else(|| value.as_str().and_then(|n| n.trim().parse().ok()))
        .unwrap_or(1))
}

// Inspect the JPEG envelope/dimensions without decoding or loading a native codec.
// The live verification additionally decodes the response with Pillow.
fn verify_jpeg(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let invalid = || std::io::Error::other("Invalid rendered JPEG");
    let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut pair = [0u8; 2];
    file.read_exact(&mut pair)?;
    if pair != [0xff, 0xd8] {
        return Err(invalid());
    }
    file.seek(SeekFrom::End(-2))?;
    file.read_exact(&mut pair)?;
    if pair != [0xff, 0xd9] {
        return Err(invalid());
    }
    file.seek(SeekFrom::Start(2))?;
    loop {
        file.read_exact(&mut pair)?;
        if pair[0] != 0xff {
            return Err(invalid());
        }
        while pair[1] == 0xff {
            file.read_exact(&mut pair[1..])?;
        }
        let marker = pair[1];
        if marker == 0xd9 || marker == 0xda || marker == 0 {
            return Err(invalid());
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        file.read_exact(&mut pair)?;
        let length = u16::from_be_bytes(pair) as i64;
        if length < 2 {
            return Err(invalid());
        }
        if (0xc0..=0xcf).contains(&marker) && ![0xc4, 0xc8, 0xcc].contains(&marker) {
            if length < 8 {
                return Err(invalid());
            }
            let mut dimensions = [0u8; 5];
            file.read_exact(&mut dimensions)?;
            let height = u16::from_be_bytes([dimensions[1], dimensions[2]]) as i64;
            let width = u16::from_be_bytes([dimensions[3], dimensions[4]]) as i64;
            return if width > 0 && height > 0 {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        file.seek(SeekFrom::Current(length - 2))?;
    }
}

async fn retrieve(
    Extension(download): Extension<Download>,
    Extension(runtime): Extension<DownloadRuntime>,
    Path(params): Path<BTreeMap<String, String>>,
    request: Request,
) -> Result<Response, Failure> {
    let series = params.get("seriesUID");
    let instance = params.get("sopUID");
    let study = &params["studyUID"];
    let mut query = BTreeMap::new();
    for (k, v) in url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
        query.entry(k.into_owned()).or_insert(v.into_owned());
    }
    let kind = kind(
        query
            .get("downloadType")
            .map_or("OCTET_STREAM", String::as_str),
        series.is_some() && instance.is_none(),
    )?;
    let session = download
        .auth
        .session(&params["sessionID"])
        .await
        .map_err(|_| failed())?;
    // Java returns a null StreamingResponseBody for an out-of-scope study: 200/empty.
    let Some(session) = session.filter(|s| s.studies.contains(&Some(study.clone()))) else {
        return Ok(StatusCode::OK.into_response());
    };
    if !safe_id(study)
        || series.is_some_and(|s| !safe_id(s))
        || instance.is_some_and(|s| !safe_id(s))
    {
        return Err(failed());
    }
    let lease = Arc::new(Lease::new(&runtime.temp_dir).map_err(|_| failed())?);
    let mut job = Job {
        runtime,
        lease,
        sequence: 0,
    };
    let pattern = if instance.is_none() && kind != Kind::Mp4 {
        download
            .configs
            .get("DOWNLOAD_PATH_FORMAT")
            .await
            .ok()
            .flatten()
            .map_or_else(String::new, |c| c.value)
    } else {
        String::new()
    };
    let result = prepare(
        &mut job,
        &session,
        study,
        series.map(String::as_str),
        instance.map(String::as_str),
        kind,
        pattern,
    )
    .await?;
    file_response(result.0, result.1, &result.2, job.lease).await
}

async fn prepare(
    job: &mut Job,
    session: &CachedSession,
    study: &str,
    series: Option<&str>,
    instance: Option<&str>,
    kind: Kind,
    pattern: String,
) -> Result<(PathBuf, &'static str, String), Failure> {
    let store = session.store_url.as_deref().ok_or_else(failed)?;
    let mut series_list = Vec::new();
    if let Some(series) = series {
        series_list.push(Series {
            sop: series.into(),
            index: "1".into(),
        });
    } else {
        let json = job
            .metadata(upstream(store, study, None, None, false)?, session)
            .await?;
        let parsed = job
            .runtime
            .jvm()?
            .download_series(json)
            .await
            .map_err(jni_error)?;
        series_list = serde_json::from_str(&parsed).map_err(|_| failed())?;
    }
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let mut names = BTreeSet::new();
    let mut video_inputs = Vec::new();
    for series in series_list {
        if !safe_id(&series.sop) {
            return Err(failed());
        }
        // DICOM instance download does not fetch metadata in Java.
        let entries: Vec<Instance> = if let Some(sop) = instance.filter(|_| kind == Kind::Dicom) {
            vec![Instance {
                sop: sop.into(),
                frames: 1,
                path: String::new(),
            }]
        } else if let Some(sop) = instance.filter(|_| kind == Kind::Jpeg) {
            let json = job
                .metadata(
                    upstream(store, study, Some(&series.sop), Some(sop), true)?,
                    session,
                )
                .await?;
            vec![Instance {
                sop: sop.into(),
                frames: instance_frames(&json)?,
                path: String::new(),
            }]
        } else {
            let json = job
                .metadata(
                    upstream(store, study, Some(&series.sop), instance, true)?,
                    session,
                )
                .await?;
            let json = job
                .runtime
                .jvm()?
                .download_metadata(json, pattern.clone(), series.index.clone())
                .await
                .map_err(jni_error)?;
            let mut entries: Vec<Instance> = serde_json::from_str(&json).map_err(|_| failed())?;
            if let Some(sop) = instance {
                let mut first = entries.drain(..).next().ok_or_else(failed)?;
                first.sop = sop.into();
                entries = vec![first];
            }
            entries
        };
        for entry in entries {
            if !safe_id(&entry.sop) || (kind != Kind::Dicom && entry.frames <= 0) {
                return Err(failed());
            }
            if kind == Kind::Jpeg {
                for frame in 0..entry.frames {
                    let output = job
                        .jpeg(
                            rendered_frame(store, study, &series.sop, &entry.sop, frame)?,
                            session,
                        )
                        .await?;
                    if let Some(sop) = instance.filter(|_| entry.frames == 1) {
                        return Ok((output, "application/jpeg", format!("{sop}.jpeg")));
                    }
                    let name = if instance.is_some() {
                        format!("1/{frame}.jpeg")
                    } else if entry.frames > 1 {
                        format!("{}_{frame}.jpeg", entry.path)
                    } else {
                        format!("{}.jpeg", entry.path)
                    };
                    add_file(&mut files, &mut names, name, output)?;
                }
                continue;
            }
            let input = job
                .fetch(
                    wado_uri(store, study, &series.sop, &entry.sop)?,
                    session,
                    None,
                )
                .await?;
            if kind == Kind::Mp4 {
                video_inputs.push(input);
                continue;
            }
            // Preserve the upstream DICOM bytes/transfer syntax, without a codec call.
            if let Some(sop) = instance {
                return Ok((input, "application/dicom", format!("{sop}.dcm")));
            }
            add_file(&mut files, &mut names, format!("{}.dcm", entry.path), input)?;
        }
    }
    if kind == Kind::Mp4 {
        let manifest = job.file();
        let mut bytes = Vec::new();
        for input in video_inputs {
            bytes.extend_from_slice(input.to_str().ok_or_else(failed)?.as_bytes());
            bytes.push(b'\n');
        }
        let target = manifest.clone();
        owned_io(job.lease.clone(), move || std::fs::write(target, bytes)).await?;
        let result = job
            .codec(
                manifest,
                FileOperation::Video {
                    fps: job.runtime.video.fps,
                },
            )
            .await?;
        return Ok((result, "video/mp4", format!("{study}.mp4")));
    }
    let zip_path = job.file();
    let output = zip_path.clone();
    let owner = job.lease.clone();
    tokio::task::spawn_blocking(move || -> Result<(), Failure> {
        let _owner = owner;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|_| failed())?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        if instance_is_multiframe(&files) {
            zip.add_directory("1/", options).map_err(|_| failed())?;
        }
        for (name, path) in files {
            let mut input = std::fs::File::open(path).map_err(|_| failed())?;
            let options = zip_options(input.metadata().map_err(|_| failed())?.len());
            zip.start_file(name, options).map_err(|_| failed())?;
            let mut buf = [0; 65536];
            loop {
                let n = input.read(&mut buf).map_err(|_| failed())?;
                if n == 0 {
                    break;
                }
                zip.write_all(&buf[..n]).map_err(|_| failed())?;
            }
        }
        zip.finish().map_err(|_| failed())?;
        Ok(())
    })
    .await
    .map_err(|_| failed())??;
    Ok((
        zip_path,
        "application/zip",
        format!("{}.zip", instance.unwrap_or(study)),
    ))
}
fn instance_is_multiframe(files: &[(String, PathBuf)]) -> bool {
    files.first().is_some_and(|(n, _)| n == "1/0.jpeg")
}
fn add_file(
    files: &mut Vec<(String, PathBuf)>,
    names: &mut BTreeSet<String>,
    name: String,
    path: PathBuf,
) -> Result<(), Failure> {
    if !safe_zip_name(&name) || !names.insert(name.clone()) {
        return Err(failed());
    }
    files.push((name, path));
    Ok(())
}
fn zip_options(length: u64) -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(length >= u32::MAX as u64)
}
async fn file_response(
    path: PathBuf,
    media: &'static str,
    name: &str,
    lease: Arc<Lease>,
) -> Result<Response, Failure> {
    let (file, length) = owned_io(lease.clone(), move || {
        let file = std::fs::File::open(path)?;
        let length = file.metadata()?.len();
        Ok((file, length))
    })
    .await?;
    let state = Arc::new(std::sync::Mutex::new((file, lease)));
    let body = stream::try_unfold(state, |state| async move {
        let owned = state.clone();
        let buf = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<u8>> {
            let mut guard = owned
                .lock()
                .map_err(|_| std::io::Error::other("Download stream unavailable"))?;
            let mut buf = vec![0; 65536];
            let n = guard.0.read(&mut buf)?;
            buf.truncate(n);
            Ok(buf)
        })
        .await
        .map_err(|_| std::io::Error::other("Download stream unavailable"))??;
        let n = buf.len();
        if n == 0 {
            Ok::<_, std::io::Error>(None)
        } else {
            Ok(Some((Bytes::from(buf), state)))
        }
    });
    let mut response = Response::new(Body::from_stream(body));
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, header::HeaderValue::from(length));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(media),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment;filename={name}"))
            .map_err(|_| failed())?,
    );
    Ok(response)
}
async fn owned_io<T: Send + 'static>(
    lease: Arc<Lease>,
    operation: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> Result<T, Failure> {
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        operation()
    })
    .await
    .map_err(|_| failed())?
    .map_err(|_| failed())
}
pub(super) fn router(auth: Auth, configs: ConfigStore) -> Router {
    let root = "/v1/session/{sessionID}/download/studies/{studyUID}";
    Router::new()
        .api_route(root, get(retrieve))
        .api_route(&format!("{root}/series/{{seriesUID}}"), get(retrieve))
        .api_route(
            &format!("{root}/series/{{seriesUID}}/instances/{{sopUID}}"),
            get(retrieve),
        )
        .layer(Extension(Download { auth, configs }))
}

#[cfg(test)]
mod tests;
