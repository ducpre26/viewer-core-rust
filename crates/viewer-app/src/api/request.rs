//! Step 8/9 JSON input compatibility and safe, actionable parser errors.
use super::response::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, de};
use std::error::Error as StdError;
use summer_web::axum::{Json, extract::rejection::JsonRejection, http::StatusCode};

/// Jackson accepts integer JSON numbers and trimmed integer strings for Integer.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(super) enum IntegerInput {
    Integer(i32),
    Text(String),
}

pub(super) fn optional_integer<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<IntegerInput>::deserialize(deserializer)? {
        None => Ok(None),
        Some(IntegerInput::Integer(value)) => Ok(Some(value)),
        Some(IntegerInput::Text(text)) => {
            let text = text.trim_matches(|c: char| c <= '\u{20}');
            if text.is_empty() {
                return Ok(None);
            }
            text.parse::<i32>()
                .map(Some)
                .map_err(|_| de::Error::custom("expected a 32-bit integer or integer string"))
        }
    }
}

fn find_source<'a, T: StdError + 'static>(error: &'a (dyn StdError + 'static)) -> Option<&'a T> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(found) = error.downcast_ref::<T>() {
            return Some(found);
        }
        current = error.source();
    }
    None
}
fn field_message(path: &str) -> String {
    // Only expose known DTO field names. Dynamic map keys may contain study IDs.
    let root = path.split(['.', '[']).next().unwrap_or("");
    let expected = match root {
        "expireIn" => "a 32-bit integer or integer string, or null",
        "studies" | "studyInstanceUIDs" => "an array of strings (entries may be null), or null",
        "studyUIDs" => "an object mapping study IDs to store objects, or null",
        "usePublicURL" | "anonymized" | "requiresShareKey" => "a boolean or null",
        "user" | "userID" | "storeUrl" | "storeAuth" | "id" => "a string or null",
        _ => return "Request body must be a JSON object with valid field types".into(),
    };
    format!("Field '{root}' must be {expected}")
}
fn json_error(rejection: JsonRejection) -> Error {
    let message = match &rejection {
        JsonRejection::JsonDataError(error) => find_source::<
            serde_path_to_error::Error<serde_json::Error>,
        >(error)
        .map(|error| field_message(&error.path().to_string()))
        .unwrap_or_else(|| "Request body must be a JSON object with valid field types".into()),
        JsonRejection::JsonSyntaxError(error) => find_source::<serde_json::Error>(error)
            .or_else(|| {
                find_source::<serde_path_to_error::Error<serde_json::Error>>(error)
                    .map(|error| error.inner())
            })
            .map(|error| {
                format!(
                    "Malformed JSON at line {}, column {}",
                    error.line(),
                    error.column()
                )
            })
            .unwrap_or_else(|| "Malformed JSON in request body".into()),
        JsonRejection::MissingJsonContentType(_) => "Content-Type must be application/json".into(),
        JsonRejection::BytesRejection(_) => "Request body could not be read".into(),
        _ => "Request body could not be decoded as JSON".into(),
    };
    // Preserve the Step 8/9 status and envelope; messages are Rust-specific.
    Error(StatusCode::INTERNAL_SERVER_ERROR, message, "INTERNAL_ERROR")
}
pub(super) fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    input.map(|Json(value)| value).map_err(json_error)
}
