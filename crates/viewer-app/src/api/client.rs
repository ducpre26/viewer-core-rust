//! UC-001: ClientAPI link creation, using the Step 3a cache/authentication.
use super::response::{self, Error};
use crate::{
    auth::{Auth, CachedSession, SecurityContext},
    persistence::{Persistence, client_permission, config::ConfigStore, session},
};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use schemars::JsonSchema;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use summer_web::{
    Router,
    aide::axum::routing::post,
    axum::{
        Extension, Json, extract::rejection::JsonRejection, http::StatusCode,
        response::IntoResponse,
    },
};

#[derive(Clone, Deserialize, summer::config::Configurable)]
#[config_prefix = "session"]
#[serde(default, deny_unknown_fields)]
pub struct SessionConfig {
    pub default_expire_minutes: i64,
    pub path_prefix: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_expire_minutes: 60,
            path_prefix: "/viewer?session=".into(),
        }
    }
}

impl SessionConfig {
    pub fn validate(&self) -> std::result::Result<(), &'static str> {
        self.expiry(None, Utc::now())
            .map_err(|_| "Session expiry configuration out of range")?;
        Ok(())
    }

    fn expiry(
        &self,
        requested: Option<i32>,
        now: DateTime<Utc>,
    ) -> std::result::Result<DateTime<Utc>, Error> {
        let minutes = requested
            .filter(|v| *v > 0 && i64::from(*v) <= self.default_expire_minutes)
            .map(i64::from)
            .unwrap_or(self.default_expire_minutes);
        Duration::try_minutes(minutes)
            .and_then(|d| now.checked_add_signed(d))
            .ok_or_else(|| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Session expiry out of range".into(),
                    "INTERNAL_ERROR",
                )
            })
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct LinkInput {
    user: Option<String>,
    store_url: Option<String>,
    store_auth: Option<String>,
    studies: Option<BTreeSet<Option<String>>>,
    expire_in: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
struct LegacyStore {
    url: Option<String>,
    authentication: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct LegacyInput {
    #[serde(rename = "userID")]
    user: Option<String>,
    #[serde(rename = "expireIn")]
    expire_in: Option<i32>,
    #[serde(rename = "studyUIDs")]
    studies: Option<BTreeMap<String, Option<LegacyStore>>>,
}

#[derive(Clone)]
struct SessionCreator {
    database: Persistence,
    auth: Auth,
    config: SessionConfig,
    configs: ConfigStore,
}

impl SessionCreator {
    async fn create(
        &self,
        input: LinkInput,
        context: SecurityContext,
    ) -> std::result::Result<(String, Value), Error> {
        let mut permissions = None;
        if let Some(principal) = context.principal {
            // ClientService.findById throws ObjectNotFoundException when even
            // the Basic sysadmin principal has no corresponding CLIENT row.
            if !self.auth.client_exists(&principal.id).await? {
                return Err(response::missing(&principal.id));
            }
            permissions = Some(
                client_permission::Entity::find()
                    .filter(client_permission::Column::ClientId.eq(principal.id))
                    .all(self.database.connection())
                    .await?
                    .into_iter()
                    .map(|m| m.permission_id)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>(),
            );
        }
        // Match System.currentTimeMillis(), including millisecond precision.
        let now = DateTime::from_timestamp_millis(Utc::now().timestamp_millis()).unwrap();
        let expiry = self.config.expiry(input.expire_in, now)?;
        let id = uuid::Uuid::new_v4().to_string();
        let studies = input.studies.unwrap_or_default();
        let model = session::Model {
            id: id.clone(),
            store_url: input.store_url,
            store_auth: input.store_auth,
            studies: studies
                .iter()
                .map(|s| s.as_deref().unwrap_or("null"))
                .collect::<Vec<_>>()
                .join(","),
            user_id: input.user,
            anonymized: false,
            shared_session: false,
            share_key: None,
            expired_time: expiry.fixed_offset(),
            permissions,
        };
        let result = json!({
            "id":id, "user":model.user_id, "permissions":model.permissions,
            "storeUrl":model.store_url, "storeAuth":model.store_auth, "studies":studies,
            "anonymized":false, "sharedSession":false, "requiresShareKey":false,
            "shareKey":null, "expiredTime":expiry.to_rfc3339_opts(SecondsFormat::Millis, true),
        });
        // Java creates the cache entry before generating its URL. A CONFIG
        // failure therefore leaves that short-term entry, without a DB write.
        self.auth
            .cache_created_session(CachedSession { model, studies })
            .await;
        let prefix = self
            .configs
            .get("PATH_PREFIX")
            .await?
            .map(|c| c.value)
            .unwrap_or_else(|| self.config.path_prefix.clone());
        Ok((format!("{prefix}{id}"), result))
    }
}

fn body<T>(input: std::result::Result<Json<T>, JsonRejection>) -> std::result::Result<T, Error> {
    input.map(|Json(value)| value).map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid request body".into(),
            "INTERNAL_ERROR",
        )
    })
}

async fn link(
    Extension(creator): Extension<SessionCreator>,
    Extension(context): Extension<SecurityContext>,
    input: std::result::Result<Json<LinkInput>, JsonRejection>,
) -> response::Result {
    let input = body(input)?;
    if input.store_url.is_none() || input.studies.is_none() {
        let mut error = response::invalid();
        if input.store_url.is_none() && input.studies.is_none() {
            error.1 = "must not be null; must not be null".into();
        }
        return Err(error);
    }
    if input.studies.as_ref().is_some_and(|s| s.is_empty()) {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let (link, session) = creator.create(input, context).await?;
    Ok(Json(json!({"link":link,"session":session})).into_response())
}

async fn legacy_link(
    Extension(creator): Extension<SessionCreator>,
    Extension(context): Extension<SecurityContext>,
    input: std::result::Result<Json<LegacyInput>, JsonRejection>,
) -> response::Result {
    let input = body(input)?;
    let stores = input.studies.ok_or_else(|| {
        let mut error = response::invalid();
        error.1 = "must not be null; must not be empty".into();
        error
    })?;
    if stores.is_empty() {
        let mut error = response::invalid();
        error.1 = "must not be empty".into();
        return Err(error);
    }
    let mut request = LinkInput {
        user: input.user,
        expire_in: input.expire_in,
        studies: Some(stores.keys().cloned().map(Some).collect()),
        store_url: None,
        store_auth: None,
    };
    for store in stores.into_values().flatten() {
        if store.url.is_some() {
            request.store_url = store.url;
            request.store_auth = store.authentication;
        }
    }
    let (link, session) = creator.create(request, context).await?;
    Ok(response::reply(
        200,
        200,
        &format!(
            "Viewer session created: {}",
            session["id"].as_str().unwrap()
        ),
        json!(link),
    ))
}

pub fn router(
    database: Persistence,
    auth: Auth,
    configs: ConfigStore,
    config: SessionConfig,
) -> Router {
    Router::new()
        .api_route("/client/link", post(link))
        .api_route("/client/getlink", post(legacy_link))
        .layer(Extension(SessionCreator {
            database,
            auth,
            configs,
            config,
        }))
}

#[cfg(test)]
mod tests;
