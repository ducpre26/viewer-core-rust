//! UC-009 contract from AdminAPI; database constraints arbitrate concurrent writes.
use super::response::*;
use crate::{
    auth::Auth,
    persistence::{Persistence, client, client_permission as mapping, permission},
};
use schemars::JsonSchema;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait, sea_query::OnConflict,
};
use serde::Deserialize;
use serde_json::{Value, json};
use summer_web::{
    Router,
    aide::axum::routing::get,
    axum::{
        Extension, Json,
        extract::{Path, Query},
        http::StatusCode,
        middleware,
        response::{IntoResponse, Response},
    },
};

async fn password(raw: String) -> std::result::Result<String, Error> {
    // Spring BCryptPasswordEncoder(8) defaults to version 2a.
    tokio::task::spawn_blocking(move || bcrypt::hash(raw, 8).map(|s| s.replacen("$2b$", "$2a$", 1)))
        .await
        .map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password encoding failed".into(),
                "INTERNAL_ERROR",
            )
        })?
        .map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password encoding failed".into(),
                "INTERNAL_ERROR",
            )
        })
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ClientInput {
    id: Option<String>,
    password: Option<String>,
    ip_allowed: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
struct PermissionInput {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MappingInput {
    permission_id: Option<String>,
}
fn client_dto(c: client::Model, ids: Option<Vec<String>>) -> Value {
    json!({"id":c.id,"ipAllowed":c.ip_allowed,"permissionIds":ids})
}
fn permission_dto(p: permission::Model) -> Value {
    json!({"id":p.id,"name":p.name,"description":p.description})
}

async fn clients(Extension(db): Extension<Persistence>, Query(page): Query<Paging>) -> Result {
    let start = page.start()?;
    let total = client::Entity::find().count(db.connection()).await?;
    let rows = client::Entity::find()
        .offset(start)
        .limit(page.limit as u64)
        .all(db.connection())
        .await?;
    Ok(page.response(
        "clients",
        total,
        rows.into_iter().map(|c| client_dto(c, None)).collect(),
    ))
}
async fn client_get(Extension(db): Extension<Persistence>, Path(id): Path<String>) -> Result {
    let Some(c) = client::Entity::find_by_id(&id).one(db.connection()).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let ids = mapping::Entity::find()
        .filter(mapping::Column::ClientId.eq(id))
        .all(db.connection())
        .await?
        .into_iter()
        .map(|m| m.permission_id)
        .collect();
    Ok(reply(200, 200, "Success", client_dto(c, Some(ids))))
}
async fn client_create(
    Extension(db): Extension<Persistence>,
    Extension(auth): Extension<Auth>,
    Json(input): Json<ClientInput>,
) -> Result {
    let id = input.id.ok_or_else(invalid)?;
    let hash = password(clean(input.password).ok_or_else(invalid)?).await?;
    client::Entity::insert(client::ActiveModel {
        id: Set(id.clone()),
        password: Set(hash),
        ip_allowed: Set(clean(input.ip_allowed)),
    })
    .on_conflict(
        OnConflict::column(client::Column::Id)
            .update_columns([client::Column::Password, client::Column::IpAllowed])
            .to_owned(),
    )
    .exec(db.connection())
    .await?;
    auth.invalidate_client(&id).await;
    Ok(reply(201, 201, "Created", json!(id)))
}
async fn client_update(
    Extension(db): Extension<Persistence>,
    Extension(auth): Extension<Auth>,
    Json(input): Json<ClientInput>,
) -> Result {
    let id = input.id.ok_or_else(invalid)?;
    let Some(c) = client::Entity::find_by_id(&id).one(db.connection()).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let mut c: client::ActiveModel = c.into();
    if let Some(raw) = clean(input.password) {
        c.password = Set(password(raw).await?);
    }
    if let Some(ip) = clean(input.ip_allowed) {
        c.ip_allowed = Set(Some(ip));
    }
    c.update(db.connection()).await?;
    auth.invalidate_client(&id).await;
    Ok(reply(200, 200, "Updated", json!(id)))
}
async fn client_delete(
    Extension(db): Extension<Persistence>,
    Extension(auth): Extension<Auth>,
    Path(id): Path<String>,
) -> Result {
    if client::Entity::delete_by_id(&id)
        .exec(db.connection())
        .await?
        .rows_affected
        == 0
    {
        return Err(missing(&id));
    }
    auth.invalidate_client(&id).await;
    Ok(reply(200, 200, "Deleted", json!(id)))
}

async fn permissions(Extension(db): Extension<Persistence>, Query(page): Query<Paging>) -> Result {
    let start = page.start()?;
    let total = permission::Entity::find().count(db.connection()).await?;
    let rows = permission::Entity::find()
        .offset(start)
        .limit(page.limit as u64)
        .all(db.connection())
        .await?;
    Ok(page.response(
        "permissions",
        total,
        rows.into_iter().map(permission_dto).collect(),
    ))
}
async fn permission_get(Extension(db): Extension<Persistence>, Path(id): Path<String>) -> Result {
    Ok(
        match permission::Entity::find_by_id(id)
            .one(db.connection())
            .await?
        {
            Some(p) => reply(200, 200, "Success", permission_dto(p)),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    )
}
async fn permission_create(
    Extension(db): Extension<Persistence>,
    Json(input): Json<PermissionInput>,
) -> Result {
    let id = input.id.ok_or_else(invalid)?;
    let name = input.name.ok_or_else(invalid)?;
    permission::Entity::insert(permission::ActiveModel {
        id: Set(id.clone()),
        name: Set(name),
        description: Set(input.description),
    })
    .on_conflict(
        OnConflict::column(permission::Column::Id)
            .update_columns([permission::Column::Name, permission::Column::Description])
            .to_owned(),
    )
    .exec(db.connection())
    .await?;
    Ok(reply(201, 201, "Created", json!(id)))
}
async fn permission_update(
    Extension(db): Extension<Persistence>,
    Json(input): Json<PermissionInput>,
) -> Result {
    let id = input.id.ok_or_else(invalid)?;
    let Some(p) = permission::Entity::find_by_id(&id)
        .one(db.connection())
        .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let mut p: permission::ActiveModel = p.into();
    if let Some(name) = input.name {
        p.name = Set(name);
    }
    if let Some(description) = input.description {
        p.description = Set(Some(description));
    }
    p.update(db.connection()).await?;
    Ok(reply(200, 200, "Updated", json!(id)))
}
async fn permission_delete(
    Extension(db): Extension<Persistence>,
    Path(id): Path<String>,
) -> Result {
    if permission::Entity::delete_by_id(&id)
        .exec(db.connection())
        .await?
        .rows_affected
        == 0
    {
        return Err(missing(&id));
    }
    Ok(reply(200, 200, "Deleted", json!(id)))
}
fn mapping_db(_: sea_orm::DbErr) -> Error {
    Error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Database operation failed".into(),
        "MAPPING_ERROR",
    )
}
fn mapping_error(message: &str) -> Response {
    reply(500, 500, message, Value::Null)
}
async fn mapping_get(Extension(db): Extension<Persistence>, Path(id): Path<String>) -> Result {
    if client::Entity::find_by_id(&id)
        .one(db.connection())
        .await
        .map_err(mapping_db)?
        .is_none()
    {
        return Ok(mapping_error(&format!("Client không tồn tại: {id}")));
    }
    let rows = mapping::Entity::find()
        .filter(mapping::Column::ClientId.eq(&id))
        .all(db.connection())
        .await
        .map_err(mapping_db)?;
    let mut body = Vec::new();
    for row in rows {
        let p = permission::Entity::find_by_id(&row.permission_id)
            .one(db.connection())
            .await
            .map_err(mapping_db)?;
        body.push(json!({"clientId":id,"permissionId":row.permission_id,"permissionName":p.as_ref().map(|p|&p.name),"permissionDescription":p.and_then(|p|p.description)}));
    }
    Ok(reply(
        200,
        200,
        "Lấy danh sách quyền thành công",
        json!(body),
    ))
}
async fn mapping_create(
    Extension(db): Extension<Persistence>,
    Path(id): Path<String>,
    Json(input): Json<MappingInput>,
) -> Result {
    let Some(pid) = input.permission_id.filter(|s| !s.trim().is_empty()) else {
        return Ok(mapping_error("permissionId không được rỗng"));
    };
    let tx = db.connection().begin().await.map_err(mapping_db)?;
    let error = if id.trim().is_empty() {
        Some("clientId không được rỗng".into())
    } else if client::Entity::find_by_id(&id)
        .one(&tx)
        .await
        .map_err(mapping_db)?
        .is_none()
    {
        Some(format!("Client không tồn tại: {id}"))
    } else if permission::Entity::find_by_id(&pid)
        .one(&tx)
        .await
        .map_err(mapping_db)?
        .is_none()
    {
        Some(format!("Permission không tồn tại: {pid}"))
    } else if mapping::Entity::find_by_id((id.clone(), pid.clone()))
        .one(&tx)
        .await
        .map_err(mapping_db)?
        .is_some()
    {
        Some("Client đã có quyền này rồi".into())
    } else {
        None
    };
    if let Some(error) = error {
        tx.rollback().await.map_err(mapping_db)?;
        return Ok(mapping_error(&error));
    }
    if (mapping::ActiveModel {
        client_id: Set(id),
        permission_id: Set(pid.clone()),
    })
    .insert(&tx)
    .await
    .is_err()
    {
        tx.rollback().await.map_err(mapping_db)?;
        return Ok(mapping_error("Database operation failed"));
    }
    tx.commit().await.map_err(mapping_db)?;
    Ok(reply(201, 200, "Thêm quyền thành công", json!(pid)))
}
async fn mapping_delete(
    Extension(db): Extension<Persistence>,
    Path((id, pid)): Path<(String, String)>,
) -> Result {
    mapping::Entity::delete_by_id((id.clone(), pid))
        .exec(db.connection())
        .await
        .map_err(mapping_db)?;
    Ok(reply(200, 200, "Xóa quyền thành công", json!(id)))
}
pub fn router(auth: Auth) -> Router {
    Router::new()
        .api_route(
            "/v1/admin/client",
            get(clients).post(client_create).put(client_update),
        )
        .api_route(
            "/v1/admin/client/{id}",
            get(client_get).delete(client_delete),
        )
        .api_route(
            "/v1/admin/permission",
            get(permissions)
                .post(permission_create)
                .put(permission_update),
        )
        .api_route(
            "/v1/admin/permission/{id}",
            get(permission_get).delete(permission_delete),
        )
        .api_route(
            "/v1/admin/client/{id}/permission",
            get(mapping_get).post(mapping_create),
        )
        .api_route(
            "/v1/admin/client/{id}/permission/{permissionId}",
            summer_web::aide::axum::routing::delete(mapping_delete),
        )
        .route_layer(middleware::from_fn(crate::auth::require_admin))
        .layer(Extension(auth))
}

#[cfg(test)]
mod tests;
