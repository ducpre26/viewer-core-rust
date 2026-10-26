//! UC-008: Java ConfigPublicAPI and the config portion of AdminAPI.
use super::response::*;
use crate::{
    auth::Auth,
    domain::MODIFY_OVERLAY_CONFIG,
    persistence::config::{ConfigStore, Model},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use summer_web::{
    Router,
    aide::axum::routing::{get, post},
    axum::{
        Extension, Json,
        extract::{Path, Query},
        http::StatusCode,
        middleware,
        response::IntoResponse,
    },
};

#[derive(Deserialize, JsonSchema)]
struct ConfigInput {
    id: Option<String>,
    value: Option<String>,
    description: Option<String>,
    datatype: Option<String>,
}

async fn list(Extension(store): Extension<ConfigStore>, Query(page): Query<Paging>) -> Result {
    let (total, rows) = store.list(page.start()?, page.limit as u64).await?;
    Ok(page.response(
        "configs",
        total,
        rows.into_iter().map(|row| json!(row)).collect(),
    ))
}

// Named params support both the admin route and the route with a sessionId.
#[derive(Deserialize, JsonSchema)]
struct ConfigPath {
    id: String,
}

async fn get_one(Extension(store): Extension<ConfigStore>, Path(path): Path<ConfigPath>) -> Result {
    let Some(row) = store.get(&path.id).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(reply(200, 200, "Success", json!(row)))
}

async fn create(
    Extension(store): Extension<ConfigStore>,
    Json(input): Json<ConfigInput>,
) -> Result {
    let datatype = input.datatype.unwrap_or_else(|| "TEXT".into());
    if !["TEXT", "INTEGER", "LIST"].contains(&datatype.as_str()) {
        // Enum deserialization precedes Bean Validation in Spring.
        return Err(Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid config datatype".into(),
            "INTERNAL_ERROR",
        ));
    }
    let id_missing = input.id.is_none();
    let value = clean(input.value);
    if id_missing || value.is_none() {
        let mut error = invalid();
        if id_missing && value.is_none() {
            error.1 = "must not be null; must not be null".into();
        }
        return Err(error);
    }
    let id = input.id.unwrap();
    store
        .create(Model {
            id: id.clone(),
            value: value.unwrap(),
            description: clean(input.description),
            datatype: Some(datatype),
        })
        .await?;
    Ok(reply(201, 201, "Created", json!(id)))
}

async fn update(
    Extension(store): Extension<ConfigStore>,
    Json(input): Json<ConfigInput>,
) -> Result {
    let id = input.id.ok_or_else(|| {
        Error(
            StatusCode::BAD_REQUEST,
            "The given id must not be null".into(),
            "BAD_REQUEST",
        )
    })?;
    if !store
        .update(&id, input.value, input.description, input.datatype)
        .await?
    {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(reply(200, 200, "Updated", json!(id)))
}

async fn delete(Extension(store): Extension<ConfigStore>, Path(path): Path<ConfigPath>) -> Result {
    if !store.delete(&path.id).await? {
        return Err(missing(&path.id));
    }
    Ok(reply(200, 200, "Deleted", json!(path.id)))
}

pub fn router(store: ConfigStore, auth: Auth) -> Router {
    let admin = Router::new()
        .api_route("/v1/admin/config", get(list).post(create).put(update))
        .api_route("/v1/admin/config/{id}", get(get_one).delete(delete))
        .route_layer(middleware::from_fn(crate::auth::require_admin));
    let writes = Router::new()
        .api_route("/v1/session/{sessionId}/config", post(create).put(update))
        .route_layer(middleware::from_fn_with_state(
            (auth, MODIFY_OVERLAY_CONFIG),
            crate::auth::require_permission,
        ));
    let session = Router::new()
        .api_route("/v1/session/{sessionId}/config", get(list))
        .api_route(
            "/v1/session/{sessionId}/config/{id}",
            get(get_one).delete(delete),
        )
        .merge(writes);
    Router::new()
        .merge(admin)
        .merge(session)
        .layer(Extension(store))
}

#[cfg(test)]
mod tests;
