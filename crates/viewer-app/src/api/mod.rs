//! HTTP infrastructure; business routes belong to their migration steps.
use crate::persistence::Persistence;
use schemars::JsonSchema;
use serde::Serialize;
use summer_web::axum::{Extension, http::StatusCode};
use summer_web::{
    Router,
    aide::axum::routing::{get, get_with},
    axum::Json,
};

#[derive(Serialize, JsonSchema)]
struct Health {
    status: &'static str,
}

#[derive(Serialize, JsonSchema)]
struct Readiness {
    status: &'static str,
    dependencies: Vec<&'static str>,
}

async fn health() -> Json<Health> {
    Json(Health { status: "up" })
}

async fn readiness(Extension(database): Extension<Persistence>) -> (StatusCode, Json<Readiness>) {
    let ready = database.check().await.is_ok();
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(Readiness {
            status: if ready { "ready" } else { "not_ready" },
            dependencies: vec![if ready {
                "postgresql:up"
            } else {
                "postgresql:down"
            }],
        }),
    )
}

pub fn router(database: Persistence) -> Router {
    Router::new()
        .api_route("/health", get(health))
        .api_route(
            "/ready",
            get_with(readiness, |operation| {
                operation
                    .response::<200, Json<Readiness>>()
                    .response::<503, Json<Readiness>>()
            }),
        )
        .layer(Extension(database))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Migrator;
    use sea_orm::TransactionTrait;
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    #[ignore = "requires dedicated PostgreSQL and VIEWER_DATABASE_URL"]
    async fn postgres_migrations_transaction_readiness_and_close() {
        let database = Persistence::connect_from_env()
            .await
            .expect("PostgreSQL startup");
        Migrator::up(database.connection(), None)
            .await
            .expect("idempotent empty migration runner");
        assert!(
            Migrator::get_pending_migrations(database.connection())
                .await
                .unwrap()
                .is_empty()
        );
        let tx = database.connection().begin().await.unwrap();
        tx.rollback().await.unwrap();
        let (status, body) = readiness(Extension(database.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "ready");
        database.clone().close().await.unwrap();
        let (status, body) = readiness(Extension(database)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.status, "not_ready");
    }
}
