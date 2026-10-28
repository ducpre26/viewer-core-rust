//! HTTP infrastructure; business routes belong to their migration steps.
mod admin;
mod download;
pub use download::{DownloadRuntime, DownloadSettings};
mod client;
pub use client::SessionConfig;
mod config;
mod request;
mod response;
mod session;
mod study;
mod wado;
use crate::dicom::Jvm;
use crate::persistence::Persistence;
use schemars::JsonSchema;
use serde::Serialize;
pub use session::run_cleanup;
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

async fn readiness(
    Extension(database): Extension<Persistence>,
    Extension(jvm): Extension<Option<Jvm>>,
) -> (StatusCode, Json<Readiness>) {
    let db_ready = database.check().await.is_ok();
    let mut dependencies = vec![if db_ready {
        "postgresql:up"
    } else {
        "postgresql:down"
    }];
    let jvm_ready = if let Some(jvm) = jvm {
        let ready = jvm.check().await.is_ok();
        dependencies.push(if ready { "jvm:up" } else { "jvm:down" });
        ready
    } else {
        true
    };
    let ready = db_ready && jvm_ready;
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(Readiness {
            status: if ready { "ready" } else { "not_ready" },
            dependencies,
        }),
    )
}

pub fn router(
    database: Persistence,
    jvm: Option<Jvm>,
    auth: crate::auth::Auth,
    session_config: SessionConfig,
) -> Router {
    let configs = crate::persistence::config::ConfigStore::new(database.clone());
    crate::auth::secure(
        Router::new()
            .merge(download::router(auth.clone(), configs.clone()))
            .merge(wado::router(auth.clone()))
            .merge(study::router(auth.clone()))
            .merge(admin::router(auth.clone()))
            .merge(config::router(configs.clone(), auth.clone()))
            .merge(session::router(
                database.clone(),
                auth.clone(),
                configs.clone(),
                session_config.clone(),
            ))
            .merge(client::router(
                database.clone(),
                auth.clone(),
                configs,
                session_config,
            ))
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
            .layer(Extension(jvm)),
        auth,
    )
}

// Suites that mutate shared URL configuration must not race each other.
#[cfg(test)]
pub(crate) static URL_CONFIG_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Migrator;
    use sea_orm::TransactionTrait;
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    #[ignore = "requires dedicated PostgreSQL and explicit JNI environment; run smoke_jni.py --app"]
    async fn postgres_and_jvm_readiness() {
        let database = Persistence::connect(&crate::config::for_tests().database)
            .await
            .expect("PostgreSQL startup");
        let config = crate::config::jni_for_tests()
            .validate()
            .unwrap()
            .expect("JNI must be enabled");
        let jvm = Jvm::start(config).await.expect("JVM startup");
        let (status, body) =
            readiness(Extension(database.clone()), Extension(Some(jvm.clone()))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.dependencies, vec!["postgresql:up", "jvm:up"]);
        jvm.shutdown().await.unwrap();
        let (status, body) = readiness(Extension(database.clone()), Extension(Some(jvm))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.dependencies, vec!["postgresql:up", "jvm:down"]);
        database.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires dedicated PostgreSQL configured in TOML"]
    async fn postgres_migrations_transaction_readiness_and_close() {
        let database = Persistence::connect(&crate::config::for_tests().database)
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
        let (status, body) = readiness(Extension(database.clone()), Extension(None)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "ready");
        database.clone().close().await.unwrap();
        let (status, body) = readiness(Extension(database), Extension(None)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.status, "not_ready");
    }
}
