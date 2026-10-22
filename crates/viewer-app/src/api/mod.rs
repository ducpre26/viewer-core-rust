//! HTTP infrastructure; business routes belong to their migration steps.
use schemars::JsonSchema;
use serde::Serialize;
use summer_web::{Router, aide::axum::routing::get, axum::Json};

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

async fn readiness() -> Json<Readiness> {
    // Step 1 has no external dependencies. Add checks when each adapter is enabled.
    Json(Readiness {
        status: "ready",
        dependencies: vec![],
    })
}

pub fn router() -> Router {
    Router::new()
        .api_route("/health", get(health))
        .api_route("/ready", get(readiness))
}
