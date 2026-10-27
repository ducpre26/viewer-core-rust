//! UC-003: Java-compatible study search over QIDO-RS; no DICOM JVM required.
use super::response::{self, Error, Paging};
use crate::auth::Auth;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, de};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use summer_web::{
    Router,
    aide::axum::routing::post,
    axum::{
        Extension, Json,
        extract::{
            Query,
            rejection::{JsonRejection, QueryRejection},
        },
        http::StatusCode,
        response::{IntoResponse, Response},
    },
};

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Filter {
    #[serde(rename = "sessionID")]
    #[serde(default, deserialize_with = "optional_text")]
    session_id: Option<String>,
    #[serde(rename = "patientID")]
    #[serde(default, deserialize_with = "optional_text")]
    patient_id: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    patient_name: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    accession_number: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    modality_types: Option<String>,
    #[serde(rename = "studyInstanceUID")]
    #[serde(default, deserialize_with = "optional_text")]
    study_instance_uid: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    start_date: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    end_date: Option<String>,
}

// Jackson String fields accept scalar numbers/booleans, but not arrays/objects.
fn optional_text<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    match Value::deserialize(d)? {
        Value::Null => Ok(None),
        Value::String(value) => Ok(Some(value)),
        Value::Number(value) => Ok(Some(value.to_string())),
        Value::Bool(value) => Ok(Some(value.to_string())),
        _ => Err(de::Error::custom("expected a string or scalar value")),
    }
}

#[derive(Deserialize, JsonSchema)]
struct SearchPaging {
    #[serde(default, deserialize_with = "query_integer")]
    offset: Option<i32>,
    #[serde(default, deserialize_with = "query_integer")]
    limit: Option<i32>,
    #[serde(rename = "orderBy")]
    _order_by: Option<String>,
}
fn query_integer<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i32>, D::Error> {
    let value = String::deserialize(d)?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| de::Error::custom("expected a 32-bit integer"))
}
impl SearchPaging {
    fn page(self) -> Paging {
        Paging {
            offset: self.offset.unwrap_or(0),
            limit: self.limit.unwrap_or(100),
        }
    }
}

impl Filter {
    fn validate(&self) -> Result<&str, Error> {
        let id = self.session_id.as_deref().filter(|id| !id.is_empty());
        if id.is_none() || self.patient_id.is_none() {
            return Err(response::invalid());
        }
        Ok(id.unwrap())
    }

    fn query(&self, paging: &Paging) -> BTreeMap<&'static str, String> {
        let mut query = BTreeMap::new();
        for (key, value) in [
            ("PatientID", &self.patient_id),
            ("PatientName", &self.patient_name),
            ("AccessionNumber", &self.accession_number),
            ("ModalitiesInStudy", &self.modality_types),
            ("StudyInstanceUID", &self.study_instance_uid),
        ] {
            if let Some(value) = value {
                query.insert(key, value.clone());
            }
        }
        if self.start_date.is_some() || self.end_date.is_some() {
            query.insert(
                "StudyDate",
                format!(
                    "{}-{}",
                    self.start_date.as_deref().unwrap_or(""),
                    self.end_date.as_deref().unwrap_or("")
                ),
            );
        }
        query.insert("offset", paging.offset.max(0).to_string());
        query.insert(
            "limit",
            if paging.limit <= 0 { 20 } else { paging.limit }.to_string(),
        );
        query
    }
}

#[derive(Clone)]
struct Qido {
    client: reqwest::Client,
}

fn upstream_error() -> Error {
    // Never include reqwest's URL, upstream body, headers, or patient criteria.
    Error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to search studies in DICOM store".into(),
        "INTERNAL_ERROR",
    )
}

impl Qido {
    async fn search(
        &self,
        store: &str,
        authorization: Option<&str>,
        query: &BTreeMap<&str, String>,
    ) -> Result<Vec<Value>, Error> {
        let mut url = url::Url::parse(store).map_err(|_| upstream_error())?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(upstream_error());
        }
        let path = format!("{}/studies", url.path().trim_end_matches('/'));
        url.set_path(&path);
        let mut request = self.client.get(url).query(query);
        if let Some(authorization) = authorization {
            request = request.header(reqwest::header::AUTHORIZATION, authorization);
        }
        let response = request.send().await.map_err(|_| upstream_error())?;
        if !response.status().is_success() {
            return Err(upstream_error());
        }
        let body = response.bytes().await.map_err(|_| upstream_error())?;
        Ok(datasets(&body))
    }
}

fn datasets(bytes: &[u8]) -> Vec<Value> {
    // DicomWebUtil.parseJSON catches invalid JSON and returns an empty list.
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return vec![];
    };
    let Some(rows) = value.as_array() else {
        return vec![];
    };
    let mut result = Vec::new();
    for row in rows {
        let Some(attributes) = row.as_object() else {
            return vec![];
        };
        let mut dto = serde_json::Map::new();
        for (tag, field) in [
            ("00100020", "pid"),
            ("0020000D", "studyInstanceUID"),
            ("00100010", "patientName"),
            ("00180015", "bodyPart"),
            ("00081030", "description"),
            ("00080020", "studyDate"),
            ("00080030", "studyTime"),
            ("00080061", "modalityType"),
            ("00080050", "accessionNumber"),
        ] {
            if let Some(value) = attributes
                .get(tag)
                .and_then(|value| dicom_string(value, tag))
            {
                dto.insert(field.into(), json!(value));
            }
        }
        result.push(Value::Object(dto));
    }
    result
}

fn dicom_string(attribute: &Value, tag: &str) -> Option<String> {
    let value = attribute.get("Value")?.as_array()?.first()?;
    let vr = attribute
        .get("vr")
        .and_then(Value::as_str)
        .unwrap_or(if tag == "00100010" { "PN" } else { "" });
    if vr == "PN" {
        let groups: Vec<_> = ["Alphabetic", "Ideographic", "Phonetic"]
            .iter()
            .map(|key| value.get(key).and_then(Value::as_str).unwrap_or(""))
            .collect();
        let name = groups
            .join("=")
            .trim_end_matches('=')
            .trim_matches([' ', '\0'])
            .to_owned();
        return (!name.is_empty()).then_some(name);
    }
    value
        .as_str()
        .map(|s| s.trim_matches([' ', '\0']))
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

async fn search(
    Extension(auth): Extension<Auth>,
    Extension(qido): Extension<Qido>,
    paging: Result<Query<SearchPaging>, QueryRejection>,
    input: Result<Json<Filter>, JsonRejection>,
) -> Result<Response, Error> {
    let input = super::request::body(input)?;
    let id = input.validate()?;
    let Query(paging) = paging.map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "offset and limit must be 32-bit integers".into(),
            "INTERNAL_ERROR",
        )
    })?;
    let paging = paging.page();
    let Some(session) = auth.session(id).await? else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    let store = session.store_url.as_deref().ok_or_else(upstream_error)?;
    let rows = qido
        .search(store, session.store_auth.as_deref(), &input.query(&paging))
        .await?;
    Ok(paging.response("studies", rows.len() as u64, rows))
}

pub(super) fn router(auth: Auth) -> Router {
    // Feign Request.Options defaults: connect 10s, read 60s, follow redirects.
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("QIDO HTTP client configuration");
    Router::new()
        .api_route("/v1/study/search", post(search))
        .layer(Extension(auth))
        .layer(Extension(Qido { client }))
}

#[cfg(test)]
mod tests;
