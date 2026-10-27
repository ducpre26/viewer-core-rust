//! UC-002 lifecycle, sharing and daily PostgreSQL expiry cleanup.
use super::{SessionConfig, response};
use crate::{
    auth::{Auth, CachedSession},
    persistence::{Persistence, config::ConfigStore, session},
};
use chrono::{DateTime, Duration, Utc};
use schemars::JsonSchema;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use summer_web::{
    Router,
    aide::axum::routing::{get, post, put},
    axum::{
        Extension, Json,
        extract::{Path, rejection::JsonRejection},
        http::StatusCode,
        response::{IntoResponse, Response},
    },
};

#[derive(Clone)]
struct Lifecycle {
    database: Persistence,
    auth: Auth,
    configs: ConfigStore,
    config: SessionConfig,
}

fn dto(s: &CachedSession, base: bool) -> Value {
    let mut v = json!({"id":s.id,"user":s.user_id,"studies":s.studies,"anonymized":s.anonymized,"sharedSession":s.shared_session,"requiresShareKey":s.share_key.is_some(),"expiredTime":s.expired_time.with_timezone(&Utc).to_rfc3339_opts(chrono::SecondsFormat::AutoSi,true)});
    if !base {
        v["permissions"] = json!(s.permissions);
        v["storeUrl"] = json!(s.store_url);
        v["storeAuth"] = json!(s.store_auth);
        v["shareKey"] = json!(s.share_key);
    }
    v.as_object_mut().unwrap().retain(|_, v| !v.is_null());
    v
}

async fn get_session(Extension(s): Extension<Lifecycle>, Path(id): Path<String>) -> Response {
    match s.auth.session(&id).await {
        Ok(Some(value)) => Json(dto(&value, true)).into_response(),
        Ok(None) => StatusCode::BAD_REQUEST.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn end_session(Extension(s): Extension<Lifecycle>, Path(id): Path<String>) -> Response {
    s.auth.invalidate_session(&id).await;
    match session::Entity::delete_by_id(id.clone())
        .exec(s.database.connection())
        .await
    {
        Ok(_) => id.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Update {
    id: Option<String>,
    #[serde(rename = "studyInstanceUIDs")]
    studies: Option<BTreeSet<Option<String>>>,
}
async fn update_session(
    Extension(s): Extension<Lifecycle>,
    input: Result<Json<Update>, JsonRejection>,
) -> Response {
    let input = match super::request::body(input) {
        Ok(input) => input,
        Err(error) => return error.into_response(),
    };
    let Some(id) = input.id else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Some(cached) = s.auth.cached_session(&id).await else {
        return StatusCode::OK.into_response();
    };
    let mut value = (*cached).clone();
    if let Some(studies) = input.studies {
        value.studies = studies;
        s.auth.cache_created_session(value.clone()).await;
    }
    Json(dto(&value, false)).into_response()
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Share {
    #[serde(rename = "usePublicURL")]
    public: Option<bool>,
    anonymized: Option<bool>,
    requires_share_key: Option<bool>,
    #[serde(default, deserialize_with = "super::request::optional_integer")]
    #[schemars(with = "Option<super::request::IntegerInput>")]
    expire_in: Option<i32>,
}
fn share_expiry(config: &SessionConfig, requested: i32, now: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = if requested <= 0 || i64::from(requested) > config.maximum_share_minutes {
        3600
    } else {
        i64::from(requested) * 60
    };
    now + Duration::seconds(seconds)
}
fn share_key() -> String {
    // Rejection sampling avoids modulo bias; UUID v4 supplies OS randomness.
    let mut key = String::new();
    while key.len() < 6 {
        for byte in uuid::Uuid::new_v4().as_bytes() {
            if *byte < 250 && key.len() < 6 {
                key.push(char::from(b'0' + byte % 10));
            }
        }
    }
    key
}
impl Lifecycle {
    async fn share(&self, id: &str, input: Share) -> Result<Option<Value>, sea_orm::DbErr> {
        let Some(source) = self.auth.session(id).await? else {
            return Ok(None);
        };
        if source.shared_session || source.expired_time < Utc::now() {
            return Ok(None);
        }
        let model = session::Model {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            permissions: None,
            store_url: source.store_url.clone(),
            store_auth: source.store_auth.clone(),
            studies: source
                .studies
                .iter()
                .map(|s| s.as_deref().unwrap_or("null"))
                .collect::<Vec<_>>()
                .join(","),
            anonymized: input.anonymized.unwrap_or(false),
            shared_session: true,
            share_key: input.requires_share_key.unwrap_or(false).then(share_key),
            expired_time: share_expiry(&self.config, input.expire_in.unwrap(), Utc::now())
                .fixed_offset(),
        }
        .into_active_model()
        .insert(self.database.connection())
        .await?;
        let prefix = self
            .configs
            .get("PATH_PREFIX")
            .await?
            .map(|c| c.value)
            .unwrap_or_else(|| self.config.path_prefix.clone());
        let domain = if input.public.unwrap_or(false) {
            self.configs
                .get("PUBLIC_DOMAIN")
                .await?
                .map(|c| c.value)
                .unwrap_or_else(|| self.config.public_domain.clone())
        } else {
            String::new()
        };
        let mut result = json!({"link":format!("{domain}{prefix}{}",model.id)});
        if let Some(key) = model.share_key {
            result["shareKey"] = json!(key);
        }
        Ok(Some(result))
    }
}
async fn share_session(
    Extension(s): Extension<Lifecycle>,
    Path(id): Path<String>,
    input: Result<Json<Share>, JsonRejection>,
) -> Response {
    let input = match super::request::body(input) {
        Ok(input) => input,
        Err(error) => return error.into_response(),
    };
    if input.expire_in.is_none() {
        return response::invalid().into_response();
    }
    match s.share(&id, input).await {
        Ok(Some(value)) => Json(value).into_response(),
        Ok(None) => StatusCode::BAD_REQUEST.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
pub fn router(
    database: Persistence,
    auth: Auth,
    configs: ConfigStore,
    config: SessionConfig,
) -> Router {
    Router::new()
        .api_route(
            "/v1/session/{sessionID}",
            get(get_session).delete(end_session),
        )
        .api_route("/v1/session", put(update_session))
        .api_route("/v1/session/{sessionID}/share", post(share_session))
        .layer(Extension(Lifecycle {
            database,
            auth,
            configs,
            config,
        }))
}
// Vietnam has a fixed UTC+07 offset and no daylight-saving transitions.
fn until_midnight(now: DateTime<Utc>) -> std::time::Duration {
    let local_seconds = (now.timestamp() + 7 * 3600).rem_euclid(86400);
    std::time::Duration::from_secs((86400 - local_seconds) as u64)
        - std::time::Duration::from_nanos(u64::from(now.timestamp_subsec_nanos()))
}
async fn cleanup(database: &Persistence, now: DateTime<Utc>) -> Result<u64, sea_orm::DbErr> {
    session::Entity::delete_many()
        .filter(session::Column::ExpiredTime.lt(now.fixed_offset()))
        .exec(database.connection())
        .await
        .map(|r| r.rows_affected)
}
pub async fn run_cleanup(database: Persistence) {
    loop {
        tokio::time::sleep(until_midnight(Utc::now())).await;
        match cleanup(&database, Utc::now()).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "Removed expired sessions from database")
            }
            Ok(_) => {}
            Err(_) => tracing::error!("Session cleanup failed"),
        }
    }
}
#[cfg(test)]
mod tests;
