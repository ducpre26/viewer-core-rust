//! Java-compatible authentication. Feature routes explicitly attach their guards.
mod config;
pub use config::AuthConfig;

use crate::{
    domain::{Principal, Role, permits},
    persistence::{Persistence, client, session},
};
use base64::{
    Engine, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use chrono::{DateTime, Utc};
use moka::future::Cache;
use sea_orm::{DbErr, EntityTrait};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use summer_web::axum::{
    extract::{ConnectInfo, FromRequestParts, OriginalUri, RawPathParams, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::Instrument;

#[derive(Clone)]
pub struct Auth {
    config: Arc<AuthConfig>,
    database: Persistence,
    sessions: Cache<String, Arc<session::Model>>,
    clients: Cache<String, client::Model>,
}

/// Owned by one request; never a thread-local principal.
#[derive(Clone, Default)]
pub struct SecurityContext {
    pub principal: Option<Principal>,
    pub session_id: Option<String>,
    pub session: Option<Arc<session::Model>>,
}

#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    Forbidden,
    Unavailable,
}
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Access is denied"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            Self::Unavailable => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authentication service unavailable",
            ),
        }
        .into_response()
    }
}

impl Auth {
    pub fn new(database: Persistence, config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
            database,
            sessions: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(120 * 60))
                .build(),
            clients: Cache::builder()
                .max_capacity(10)
                .time_to_live(Duration::from_secs(600 * 60))
                .build(),
        }
    }

    pub async fn session(&self, id: &str) -> Result<Option<Arc<session::Model>>, DbErr> {
        self.session_at(id, Utc::now()).await
    }
    async fn session_at(
        &self,
        id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<Arc<session::Model>>, DbErr> {
        if let Some(value) = self.sessions.get(id).await {
            if value.expired_time > now {
                return Ok(Some(value));
            }
            self.sessions.invalidate(id).await;
            return Ok(None);
        }
        if let Some(value) = session::Entity::find_by_id(id)
            .one(self.database.connection())
            .await?
        {
            if value.expired_time > now {
                let value = Arc::new(value);
                self.sessions.insert(id.to_owned(), value.clone()).await;
                return Ok(Some(value));
            }
            session::Entity::delete_by_id(id)
                .exec(self.database.connection())
                .await?;
            self.sessions.invalidate(id).await;
        }
        Ok(None)
    }
    /// Shared with the later session lifecycle implementation.
    pub async fn cache_session(&self, value: session::Model) {
        self.sessions
            .insert(value.id.clone(), Arc::new(value))
            .await;
    }
    pub async fn invalidate_session(&self, id: &str) {
        self.sessions.invalidate(id).await;
    }
    /// Step 4 invalidates credentials/IP policy when updating a client.
    pub async fn invalidate_client(&self, id: &str) {
        self.clients.invalidate(id).await;
    }

    async fn authenticate(
        &self,
        headers: &HeaderMap,
        remote: Option<SocketAddr>,
    ) -> Result<Principal, AuthError> {
        let (username, password) = basic_credentials(headers).ok_or(AuthError::Unauthorized)?;
        if username == self.config.admin_username && password == self.config.admin_password {
            return Ok(Principal {
                id: username,
                role: Role::Admin,
            });
        }
        let value = match self.clients.get(&username).await {
            Some(value) => value,
            None => {
                // Java catches client lookup exceptions and rejects credentials.
                let value = client::Entity::find_by_id(&username)
                    .one(self.database.connection())
                    .await
                    .map_err(|_| AuthError::Unauthorized)?
                    .ok_or(AuthError::Unauthorized)?;
                self.clients.insert(username.clone(), value.clone()).await;
                value
            }
        };
        if !ip_allowed(
            value.ip_allowed.as_deref(),
            request_header(headers, "x-forwarded-for"),
            remote,
        ) {
            return Err(AuthError::Unauthorized);
        }
        let verified = tokio::task::spawn_blocking(move || {
            bcrypt::verify(password, &value.password).unwrap_or(false)
        })
        .await
        .map_err(|_| AuthError::Unavailable)?;
        if !verified {
            return Err(AuthError::Unauthorized);
        }
        Ok(Principal {
            id: username,
            role: Role::Client,
        })
    }

    pub async fn has_permission(&self, id: Option<&str>, permission: &str) -> Result<bool, DbErr> {
        if self.config.client_security_disabled {
            return Ok(true);
        }
        let Some(id) = id.filter(|s| !s.trim().is_empty()) else {
            return Ok(false);
        };
        Ok(self
            .session(id)
            .await?
            .is_some_and(|s| permits(s.permissions.as_deref(), permission)))
    }
}

fn request_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|h| h.to_str().ok())
}
fn basic_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let encoded = request_header(headers, "authorization")?
        .strip_prefix("Basic ")?
        .trim();
    let engine = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new()
            .with_decode_padding_mode(DecodePaddingMode::Indifferent)
            .with_decode_allow_trailing_bits(true),
    );
    let decoded = engine.decode(encoded).ok()?;
    let decoded = String::from_utf8_lossy(&decoded);
    let (user, password) = decoded.split_once(':')?;
    Some((user.to_owned(), password.to_owned()))
}
fn ip_allowed(allowed: Option<&str>, forwarded: Option<&str>, remote: Option<SocketAddr>) -> bool {
    match allowed.filter(|value| !value.is_empty()) {
        None => true,
        Some(allowed) => match forwarded {
            Some(address) => allowed.contains(address),
            None => remote.is_some_and(|address| allowed.contains(&address.ip().to_string())),
        },
    }
}
fn session_id(path: &str) -> Option<&str> {
    path.split_once("/session/")
        .map(|(_, value)| value.split('/').next().unwrap_or(""))
}
fn filtered(path: &str) -> bool {
    path.starts_with("/ws/rest/v1/") || path.starts_with("/ws/rest/client/")
}

pub async fn authenticate_request(
    State(auth): State<Auth>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request
        .extensions()
        .get::<OriginalUri>()
        .map(|uri| uri.0.path())
        .unwrap_or(request.uri().path())
        .to_owned();
    let mut context = SecurityContext::default();
    if filtered(&path) {
        context.session_id = session_id(&path).map(str::to_owned);
        if let Some(id) = context.session_id.as_deref().filter(|id| !id.is_empty())
            && !path.ends_with(id)
        {
            let value = match auth.session(id).await {
                Ok(Some(value)) => value,
                Ok(None) => return AuthError::Unauthorized.into_response(),
                Err(_) => return AuthError::Unavailable.into_response(),
            };
            if let Some(key) = &value.share_key
                && request_header(request.headers(), "session-key") != Some(key.as_str())
            {
                return AuthError::Unauthorized.into_response();
            }
            context.session = Some(value);
        }
        if (!auth.config.client_security_disabled && path.contains("/client/"))
            || path.contains("/admin/")
        {
            let remote = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0);
            match auth.authenticate(request.headers(), remote).await {
                Ok(principal) => context.principal = Some(principal),
                Err(error) => return error.into_response(),
            }
        }
    }
    let span = tracing::info_span!(
        "viewer_request",
        session_id = context.session_id.as_deref().unwrap_or("")
    );
    request.extensions_mut().insert(context);
    next.run(request).instrument(span).await
}

pub async fn require_admin(request: Request, next: Next) -> Response {
    match request
        .extensions()
        .get::<SecurityContext>()
        .and_then(|c| c.principal.as_ref())
    {
        Some(Principal {
            role: Role::Admin, ..
        }) => next.run(request).await,
        Some(_) => AuthError::Forbidden.into_response(),
        None => AuthError::Unauthorized.into_response(),
    }
}

/// Apply as a route layer with state `(auth.clone(), permission_key)`.
pub async fn require_permission(
    State((auth, permission)): State<(Auth, &'static str)>,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let params = RawPathParams::from_request_parts(&mut parts, &())
        .await
        .ok();
    let id = params.as_ref().and_then(|params| {
        params
            .iter()
            .find(|(key, value)| *key == "sessionId" && !value.trim().is_empty())
            .map(|(_, value)| value)
    });
    let allowed = auth.has_permission(id, permission).await;
    let request = Request::from_parts(parts, body);
    match allowed {
        Ok(true) => next.run(request).await,
        Ok(false) => match request
            .extensions()
            .get::<SecurityContext>()
            .and_then(|c| c.principal.as_ref())
        {
            Some(_) => AuthError::Forbidden.into_response(),
            None => AuthError::Unauthorized.into_response(),
        },
        Err(_) => AuthError::Unavailable.into_response(),
    }
}

async fn response_headers(request: Request, next: Next) -> Response {
    let allowed_headers = [
        "authorization",
        "content-type",
        "x-auth-token",
        "x-total-count",
        "compression-type",
        "accept",
        "session-key",
        "request-time",
        "x-rt-s",
    ];
    let invalid_cors = request.headers().contains_key("origin")
        && (request_header(request.headers(), "access-control-request-method").is_some_and(
            |method| !["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"].contains(&method),
        ) || (request.method() == Method::OPTIONS
            && request
                .headers()
                .contains_key("access-control-request-method")
            && request_header(request.headers(), "access-control-request-headers").is_some_and(
                |headers| {
                    headers
                        .split(',')
                        .any(|h| !allowed_headers.contains(&h.trim().to_ascii_lowercase().as_str()))
                },
            )));
    let mut response = if invalid_cors {
        (StatusCode::FORBIDDEN, "Invalid CORS request").into_response()
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "cache-control",
        HeaderValue::from_static("no-cache, no-store, max-age=0, must-revalidate"),
    );
    response
}

/// Attach after feature routes/guards; CORS runs before auth including preflight.
pub fn secure(router: summer_web::Router, auth: Auth) -> summer_web::Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::AllowHeaders::mirror_request())
        .expose_headers([
            "x-auth-token".parse().unwrap(),
            "x-total-count".parse().unwrap(),
        ]);
    router
        .layer(middleware::from_fn_with_state(auth, authenticate_request))
        .layer(cors)
        .layer(middleware::from_fn(response_headers))
}

#[cfg(test)]
mod tests;
