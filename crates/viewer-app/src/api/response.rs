//! Shared Java API envelopes, errors, cleaning and pagination.
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use summer_web::axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub(super) type Result = std::result::Result<Response, Error>;
#[derive(Debug)]
pub(super) struct Error(
    pub(super) StatusCode,
    pub(super) String,
    pub(super) &'static str,
);
impl summer_web::aide::OperationOutput for Error {
    type Inner = Self;
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        if self.2 == "MAPPING_ERROR" {
            return reply(500, 500, &self.1, Value::Null);
        }
        (self.0, Json(json!({"status":self.0.as_u16(),"errorCode":self.2,"message":self.1,"timestamp":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true)}))).into_response()
    }
}
impl From<sea_orm::DbErr> for Error {
    fn from(_: sea_orm::DbErr) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database operation failed".into(),
            "INTERNAL_ERROR",
        )
    }
}
pub(super) fn invalid() -> Error {
    Error(
        StatusCode::BAD_REQUEST,
        "must not be null".into(),
        "VALIDATION_ERROR",
    )
}
pub(super) fn envelope(code: u16, message: &str, body: Value) -> Value {
    json!({"header":{"code":code,"message":message,"datetime":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true)},"body":body})
}
pub(super) fn reply(http: u16, code: u16, message: &str, body: Value) -> Response {
    (
        StatusCode::from_u16(http).unwrap(),
        Json(envelope(code, message, body)),
    )
        .into_response()
}
pub(super) fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim_matches(|c: char| c <= '\u{20}').to_owned())
        .filter(|s| !s.is_empty())
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct Paging {
    #[serde(default)]
    pub(super) offset: i32,
    #[serde(default = "default_limit")]
    pub(super) limit: i32,
}
fn default_limit() -> i32 {
    100
}
impl Paging {
    pub(super) fn start(&self) -> std::result::Result<u64, Error> {
        if self.limit <= 0 {
            return Err(Error(
                StatusCode::BAD_REQUEST,
                "Page size must not be less than one".into(),
                "BAD_REQUEST",
            ));
        }
        Ok((self.offset / self.limit).max(0) as u64 * self.limit as u64)
    }
    pub(super) fn response(&self, kind: &str, total: u64, body: Vec<Value>) -> Response {
        let mut value = envelope(302, &format!("Found {} {kind}", body.len()), json!(body));
        value["header"]["offset"] = json!(self.offset);
        value["header"]["limit"] = json!(self.limit);
        value["header"]["totalRecords"] = json!(total);
        Json(value).into_response()
    }
}
pub(super) fn missing(id: &str) -> Error {
    Error(
        StatusCode::NOT_FOUND,
        format!("Entity not found with id: {id}"),
        "NOT_FOUND",
    )
}
