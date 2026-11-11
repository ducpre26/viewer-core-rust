//! Java printer management contract under the authenticated session routes.
use super::response::*;
use crate::persistence::{
    Persistence,
    printer::{self, Entity, Model},
};
use schemars::JsonSchema;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QuerySelect, Set};
use serde::Deserialize;
use serde_json::json;
use summer_web::{
    Router,
    aide::axum::routing::get,
    axum::{
        Extension, Json,
        extract::{Path, Query},
        http::StatusCode,
        response::IntoResponse,
    },
};

#[derive(Deserialize, JsonSchema)]
struct PrinterPath {
    id: i32,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Input {
    id: Option<i32>,
    printer_name: Option<String>,
    host: Option<String>,
    port: Option<i32>,
    ae_title: Option<String>,
    called_ae_title: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
}
fn value(row: Model) -> serde_json::Value {
    json!({"id":row.id,"printerName":row.printer_name,"host":row.host,"port":row.port,
        "aeTitle":row.aetitle,"calledAeTitle":row.called_aetitle,"description":row.description,"enabled":row.enabled})
}
fn validate(input: &Input, create: bool) -> std::result::Result<(), Error> {
    let mut errors = Vec::new();
    for field in [
        &input.printer_name,
        &input.host,
        &input.ae_title,
        &input.called_ae_title,
    ] {
        if create && field.is_none() {
            errors.push("must not be null");
        }
        if let Some(field) = field {
            // Bean Validation measures Java string length in UTF-16 code units.
            if !(1..=255).contains(&field.encode_utf16().count()) {
                errors.push("size must be between 1 and 255");
            }
        }
    }
    let port = input.port.or(create.then_some(0));
    if port.is_some_and(|port| port < 1) {
        errors.push("must be greater than or equal to 1");
    }
    if port.is_some_and(|port| port > 65535) {
        errors.push("must be less than or equal to 65535");
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error(
            StatusCode::BAD_REQUEST,
            errors.join("; "),
            "VALIDATION_ERROR",
        ))
    }
}

async fn list(Extension(db): Extension<Persistence>, Query(page): Query<Paging>) -> Result {
    let start = page.start()?;
    let total = Entity::find().count(db.connection()).await?;
    let rows = Entity::find()
        .offset(start)
        .limit(page.limit as u64)
        .all(db.connection())
        .await?;
    Ok(page.response("printers", total, rows.into_iter().map(value).collect()))
}
async fn get_one(Extension(db): Extension<Persistence>, Path(path): Path<PrinterPath>) -> Result {
    match Entity::find_by_id(path.id).one(db.connection()).await? {
        Some(row) => Ok(reply(200, 200, "Success", value(row))),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
async fn create(Extension(db): Extension<Persistence>, Json(input): Json<Input>) -> Result {
    validate(&input, true)?;
    let row = printer::ActiveModel {
        printer_name: Set(input.printer_name.unwrap()),
        host: Set(input.host.unwrap()),
        port: Set(input.port.unwrap()),
        aetitle: Set(input.ae_title.unwrap()),
        called_aetitle: Set(input.called_ae_title.unwrap()),
        description: Set(input.description),
        enabled: Set(input.enabled.unwrap_or(false)),
        created_time: Set(chrono::Utc::now().into()),
        creator_id: Set(1),
        ..Default::default()
    }
    .insert(db.connection())
    .await?;
    Ok(reply(201, 201, "Created", json!(row.id)))
}
async fn update(Extension(db): Extension<Persistence>, Json(input): Json<Input>) -> Result {
    validate(&input, false)?;
    let id = input.id.ok_or_else(|| {
        Error(
            StatusCode::BAD_REQUEST,
            "The given id must not be null".into(),
            "BAD_REQUEST",
        )
    })?;
    let Some(row) = Entity::find_by_id(id).one(db.connection()).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let mut active = row.into_active_model();
    if let Some(value) = input.printer_name {
        active.printer_name = Set(value);
    }
    if let Some(value) = input.host {
        active.host = Set(value);
    }
    if let Some(value) = input.port {
        active.port = Set(value);
    }
    if let Some(value) = input.ae_title {
        active.aetitle = Set(value);
    }
    if let Some(value) = input.called_ae_title {
        active.called_aetitle = Set(value);
    }
    if let Some(value) = input.description {
        active.description = Set(Some(value));
    }
    if let Some(value) = input.enabled {
        active.enabled = Set(value);
    }
    active.last_updated_time = Set(Some(chrono::Utc::now().into()));
    active.last_updated_id = Set(Some(1));
    active.update(db.connection()).await?;
    Ok(reply(200, 200, "Updated", json!(id)))
}
async fn delete(Extension(db): Extension<Persistence>, Path(path): Path<PrinterPath>) -> Result {
    let result = Entity::delete_by_id(path.id).exec(db.connection()).await?;
    if result.rows_affected == 0 {
        return Err(missing(&path.id.to_string()));
    }
    Ok(reply(200, 200, "Deleted", json!(path.id)))
}
// Java repository configuration and Spring Boot 4.1.0 defaults.
const MAX_FILE_BYTES: usize = 100 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 10 * 1024 * 1024;
fn upload_too_large() -> Error {
    Error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Maximum upload size exceeded".into(),
        "INTERNAL_ERROR",
    )
}
fn multipart_error(error: axum::extract::multipart::MultipartError) -> Error {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        upload_too_large()
    } else {
        invalid()
    }
}
fn print_result(succeed: bool, message: &str) -> summer_web::axum::response::Response {
    reply(
        200,
        200,
        "Requested print",
        json!({"succeed":succeed,"message":message}),
    )
}
async fn print(
    Extension(db): Extension<Persistence>,
    Extension(jvm): Extension<Option<crate::dicom::Jvm>>,
    Query(mut fields): Query<std::collections::HashMap<String, String>>,
    mut multipart: axum::extract::Multipart,
) -> Result {
    let directory = tempfile::Builder::new()
        .prefix("viewer-print-")
        .tempdir()
        .map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Print temporary directory unavailable".into(),
                "INTERNAL_ERROR",
            )
        })?;
    let owner = std::sync::Arc::new(directory);
    let mut options = serde_json::Map::new();
    let mut printer_id = None;
    let mut image = false;
    while let Some(mut field) = multipart.next_field().await.map_err(multipart_error)? {
        let name = field.name().unwrap_or("").to_owned();
        if name == "image" && image {
            // Spring binds a scalar MultipartFile to the first matching file.
            // Drain later parts so request-size and parser errors still apply.
            while field.chunk().await.map_err(multipart_error)?.is_some() {}
            continue;
        }
        if name == "image" {
            let owned = owner.clone();
            tokio::task::spawn_blocking(move || std::fs::write(owned.path().join("image"), []))
                .await
                .map_err(|_| invalid())?
                .map_err(|_| {
                    Error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Print image write failed".into(),
                        "INTERNAL_ERROR",
                    )
                })?;
            let mut file_bytes = 0usize;
            while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                file_bytes += chunk.len();
                if file_bytes > MAX_FILE_BYTES {
                    return Err(upload_too_large());
                }
                let owned = owner.clone();
                // The blocking write retains the directory even if this
                // HTTP future is dropped while the write is in progress.
                tokio::task::spawn_blocking(move || {
                    use std::io::Write;
                    let mut output = std::fs::OpenOptions::new()
                        .append(true)
                        .open(owned.path().join("image"))?;
                    output.write_all(&chunk)
                })
                .await
                .map_err(|_| invalid())?
                .map_err(|_| {
                    Error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Print image write failed".into(),
                        "INTERNAL_ERROR",
                    )
                })?;
            }
            image = true;
        } else {
            let text = field.text().await.map_err(multipart_error)?;
            fields.entry(name).or_insert(text);
        }
    }
    for (name, text) in fields {
        match name.as_str() {
            "printerId" => printer_id = Some(text.trim().parse::<i64>().map_err(|_| invalid())?),
            "numOfCopies" | "minDensity" | "maxDensity" => {
                options.insert(
                    name,
                    json!(text.trim().parse::<i32>().map_err(|_| invalid())?),
                );
            }
            "colorPrint" | "printOnlySelectedView" => {
                options.insert(
                    name,
                    json!(match text.trim().to_ascii_lowercase().as_str() {
                        "true" | "on" | "yes" | "1" => true,
                        "false" | "off" | "no" | "0" => false,
                        _ => return Err(invalid()),
                    }),
                );
            }
            "priority" | "mediumType" | "filmDestination" | "filmOrientation" | "filmSizeId"
            | "magnificationType" | "smoothingType" | "trim" | "borderDensity"
            | "imageDisplayFormat" => {
                options.insert(name, json!(text));
            }
            _ => {}
        }
    }
    let id = printer_id.ok_or_else(invalid)?;
    if !image {
        return Err(invalid());
    }
    // Java narrows the submitted Long to Integer before the repository lookup.
    let row = Entity::find_by_id(id as i32)
        .one(db.connection())
        .await
        .unwrap_or(None);
    let Some(row) = row.filter(|printer| printer.enabled) else {
        return Ok(print_result(false, "Printer not found or not enabled"));
    };
    let Some(jvm) = jvm else {
        return Ok(print_result(
            false,
            "Print DICOM image error: JVM unavailable",
        ));
    };
    let manifest = owner.path().join("request.json");
    let input = json!({"image":owner.path().join("image"),"host":row.host,"port":row.port,
        "aeTitle":row.aetitle,"calledAeTitle":row.called_aetitle,"options":options,
        "timeoutMillis":jvm.print_timeout().as_millis().min(i32::MAX as u128)});
    std::fs::write(&manifest, input.to_string()).map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Print manifest write failed".into(),
            "INTERNAL_ERROR",
        )
    })?;
    match jvm
        .print(manifest.to_string_lossy().into_owned(), owner)
        .await
    {
        Ok(result) => {
            let result: serde_json::Value = serde_json::from_str(&result).map_err(|_| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Invalid print result".into(),
                    "INTERNAL_ERROR",
                )
            })?;
            if result["outcomeUnknown"].as_bool() == Some(true) {
                return Ok(print_result(
                    false,
                    "Print DICOM image error: outcome_unknown; do not automatically retry",
                ));
            }
            Ok(print_result(
                result["succeed"].as_bool().unwrap_or(false),
                result["message"].as_str().unwrap_or("Invalid print result"),
            ))
        }
        Err(dicom_jvm::Error::Timeout) => Ok(print_result(
            false,
            "Print DICOM image error: outcome_unknown; do not automatically retry",
        )),
        Err(error) => Ok(print_result(
            false,
            &format!("Print DICOM image error: {error}"),
        )),
    }
}
pub fn router() -> Router {
    Router::new()
        .api_route(
            "/v1/session/{sessionId}/print",
            summer_web::aide::axum::routing::post(print)
                .layer(axum::extract::DefaultBodyLimit::max(MAX_REQUEST_BYTES)),
        )
        .api_route(
            "/v1/session/{sessionId}/printer",
            get(list).post(create).put(update),
        )
        .api_route(
            "/v1/session/{sessionId}/printer/{id}",
            get(get_one).delete(delete),
        )
}
