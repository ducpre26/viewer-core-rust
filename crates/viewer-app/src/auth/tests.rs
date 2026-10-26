use super::*;
use crate::domain::{ADMIN, MODIFY_OVERLAY_CONFIG};
use base64::engine::general_purpose::STANDARD;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, Set, Statement, TransactionTrait};
use summer_web::axum::{Extension, body::Body, http::Request as HttpRequest};
use summer_web::{Router, aide::axum::routing::get};
use tower::ServiceExt;

fn config(disabled: bool) -> AuthConfig {
    AuthConfig {
        client_security_disabled: disabled,
        admin_username: "fixture-admin".into(),
        admin_password: "fixture-admin-pass".into(),
    }
}
fn basic(raw: &str) -> String {
    format!("Basic {}", STANDARD.encode(raw))
}

#[test]
fn basic_and_ip_java_branches() {
    for (raw, expected) in [
        ("user:pass:tail", ("user", "pass:tail")),
        ("user:", ("user", "")),
        (":pass", ("", "pass")),
    ] {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", basic(raw).parse().unwrap());
        assert_eq!(
            basic_credentials(&headers),
            Some((expected.0.into(), expected.1.into()))
        );
    }
    for value in [
        "Basic @@@",
        "Basic YWJj",
        "basic dTpw",
        "Bearer x",
        "Basic ",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", value.parse().unwrap());
        assert!(basic_credentials(&headers).is_none());
    }
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Basic dTpwYQ".parse().unwrap());
    assert_eq!(basic_credentials(&headers), Some(("u".into(), "pa".into())));
    let peer = Some("127.0.0.1:4321".parse().unwrap());
    assert!(ip_allowed(None, None, None));
    assert!(ip_allowed(Some(""), None, None));
    assert!(ip_allowed(Some("127.0.0.1"), None, peer));
    assert!(!ip_allowed(Some("127.0.0.1"), None, None));
    assert!(ip_allowed(Some("203.0.113.10"), Some("203.0.113.1"), peer));
    assert!(ip_allowed(Some("203.0.113.10"), Some(""), peer));
    assert!(!ip_allowed(
        Some("203.0.113.10"),
        Some("203.0.113.10, 127.0.0.1"),
        peer
    ));
    assert!(!ip_allowed(Some("127.0.0.1"), Some("198.51.100.1"), peer));
}

#[test]
fn extraction_and_snapshot_rules() {
    assert!(filtered("/ws/rest/v1/session/a/config"));
    assert!(filtered("/ws/rest/client/link"));
    assert!(!filtered("/ws/rest/health"));
    assert!(!filtered("/outside/session/a/config"));
    assert_eq!(session_id("/ws/rest/v1/session/a/config"), Some("a"));
    assert_eq!(session_id("/ws/rest/v1/session/a"), Some("a"));
    assert_eq!(session_id("/ws/rest/v1/session//config"), Some(""));
    assert_eq!(session_id("/ws/rest/v1/query"), None);
    assert!(!permits(None, MODIFY_OVERLAY_CONFIG));
    assert!(!permits(Some(&[]), MODIFY_OVERLAY_CONFIG));
    assert!(!permits(Some(&["other".into()]), MODIFY_OVERLAY_CONFIG));
    assert!(permits(Some(&[ADMIN.into()]), MODIFY_OVERLAY_CONFIG));
    assert!(permits(
        Some(&[MODIFY_OVERLAY_CONFIG.into()]),
        MODIFY_OVERLAY_CONFIG
    ));
}

#[test]
fn bcrypt_spring_prefixes_and_truncation() {
    let hash = bcrypt::hash("fixture:password", 8).unwrap();
    for prefix in ["$2a$", "$2b$", "$2y$"] {
        let java_hash = format!("{prefix}{}", &hash[4..]);
        assert!(bcrypt::verify("fixture:password", &java_hash).unwrap());
        assert!(!bcrypt::verify("wrong", &java_hash).unwrap());
    }
    let long = "x".repeat(72);
    let hash = bcrypt::hash(&long, 8).unwrap();
    assert!(bcrypt::verify(format!("{long}tail"), &hash).unwrap());
}

async fn context(Extension(context): Extension<SecurityContext>) -> String {
    context
        .principal
        .map(|p| format!("{}:{:?}", p.id, p.role))
        .unwrap_or_else(|| "anonymous".into())
}

fn test_router(auth: Auth) -> summer_web::axum::Router {
    let admin = Router::new()
        .api_route("/v1/admin/probe", get(context))
        .route_layer(middleware::from_fn(require_admin));
    let permission = Router::new()
        .api_route(
            "/v1/session/{sessionId}/permission",
            get(|| async { "allowed" }),
        )
        .api_route(
            "/v1/session/{sessionID}/uppercase",
            get(|| async { "allowed" }),
        )
        .api_route("/v1/query-permission", get(|| async { "allowed" }))
        .api_route("/v1/admin/permission", get(|| async { "allowed" }))
        .route_layer(middleware::from_fn_with_state(
            (auth.clone(), MODIFY_OVERLAY_CONFIG),
            require_permission,
        ));
    let router = Router::new()
        .merge(admin)
        .merge(permission)
        .api_route("/client/probe", get(context))
        .api_route("/v1/session/{sessionId}/probe", get(context))
        .api_route("/v1/session/{sessionId}", get(context))
        .api_route("/v1/session/{sessionId}/", get(context))
        .api_route("/v1/session/{sessionId}/tail/{tail}", get(context))
        .api_route("/v1/query", get(context))
        .api_route("/health", get(context));
    // Same production secure() + nesting as Summer's global_prefix, no auth bypass.
    summer_web::axum::Router::new().nest("/ws/rest", secure(router, auth).into())
}

async fn request(
    router: &summer_web::axum::Router,
    path: &str,
    auth: Option<&str>,
    extra: &[(&str, &str)],
    method: &str,
) -> Response {
    let mut builder = HttpRequest::builder().uri(path).method(method);
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    for (key, value) in extra {
        builder = builder.header(*key, *value);
    }
    let mut req = builder.body(Body::empty()).unwrap();
    req.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:44000".parse::<SocketAddr>().unwrap(),
    ));
    router.clone().oneshot(req).await.unwrap()
}
async fn status(router: &summer_web::axum::Router, path: &str, auth: Option<&str>, expected: u16) {
    let response = request(router, path, auth, &[], "GET").await;
    assert_eq!(response.status().as_u16(), expected, "{path}");
    assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");
    if expected >= 400 {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!String::from_utf8_lossy(&body).contains("postgres"));
    }
}

fn snapshot(id: &str, expiry: DateTime<Utc>) -> session::Model {
    session::Model {
        id: id.into(),
        store_url: None,
        store_auth: None,
        studies: "1.2.3".into(),
        user_id: None,
        anonymized: false,
        shared_session: false,
        share_key: None,
        expired_time: expiry.fixed_offset(),
        permissions: Some(vec![MODIFY_OVERLAY_CONFIG.into()]),
    }
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL; included by the documented PostgreSQL test filter"]
async fn postgres_migrations_transaction_readiness_and_close_auth() {
    let db = Persistence::connect_from_env().await.unwrap();
    let conn = db.connection();
    for sql in [
        "SELECT to_regclass('public.idx_session_expired_time') IS NOT NULL AS ok",
        "SELECT data_type = 'timestamp with time zone' AND is_nullable = 'NO' AS ok FROM information_schema.columns WHERE table_name='session' AND column_name='expired_time' AND table_schema='public'",
        "SELECT udt_name = '_text' AS ok FROM information_schema.columns WHERE table_name='session' AND column_name='permissions' AND table_schema='public'",
        "SELECT character_maximum_length = 64 AND is_nullable = 'NO' AS ok FROM information_schema.columns WHERE table_name='client' AND column_name='password' AND table_schema='public'",
    ] {
        let row = conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql.to_owned()))
            .await
            .unwrap()
            .unwrap();
        assert!(row.try_get::<bool>("", "ok").unwrap());
    }
    conn.execute_unprepared("DELETE FROM session WHERE id='auth-default'; INSERT INTO session(id,studies,expired_time) VALUES ('auth-default','1.2.3',now())").await.unwrap();
    let defaults = session::Entity::find_by_id("auth-default")
        .one(conn)
        .await
        .unwrap()
        .unwrap();
    assert!(defaults.shared_session);
    assert!(!defaults.anonymized);
    assert!(defaults.permissions.is_none());
    session::Entity::delete_by_id("auth-default")
        .exec(conn)
        .await
        .unwrap();
    let now = Utc::now();
    let future = now + chrono::Duration::days(1);
    let hash = bcrypt::hash("fixture:password", 8).unwrap();
    let tx = conn.begin().await.unwrap();
    client::ActiveModel {
        id: Set("auth-rollback".into()),
        password: Set(hash.clone()),
        ip_allowed: Set(None),
    }
    .insert(&tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert!(
        client::Entity::find_by_id("auth-rollback")
            .one(conn)
            .await
            .unwrap()
            .is_none()
    );
    // Invalid writes prove schema constraints; never run against the Java database.
    assert!(
        conn.execute_unprepared("INSERT INTO client(id,password) VALUES ('auth-invalid',NULL)")
            .await
            .is_err()
    );
    assert!(
        conn.execute_unprepared(
            "INSERT INTO session(id,studies,expired_time) VALUES ('auth-invalid',NULL,now())"
        )
        .await
        .is_err()
    );
    for disabled in [false, true] {
        conn.execute_unprepared("DELETE FROM client WHERE id LIKE 'auth-%'; DELETE FROM session WHERE id LIKE 'auth-%';").await.unwrap();
        for (id, allowed) in [
            ("auth-client", Some("127.0.0.1 203.0.113.10")),
            ("auth-open", None),
            ("auth-empty-ip", Some("")),
        ] {
            client::ActiveModel {
                id: Set(id.into()),
                password: Set(hash.clone()),
                ip_allowed: Set(allowed.map(str::to_owned)),
            }
            .insert(conn)
            .await
            .unwrap();
        }
        for id in [
            "auth-live",
            "auth-empty",
            "auth-null",
            "auth-all",
            "auth-shared",
            "auth-expired",
        ] {
            let mut value = snapshot(
                id,
                if id == "auth-expired" {
                    now - chrono::Duration::days(1)
                } else {
                    future
                },
            );
            value.permissions = match id {
                "auth-empty" => Some(vec![]),
                "auth-null" => None,
                "auth-all" => Some(vec![ADMIN.into()]),
                _ => value.permissions,
            };
            if id == "auth-shared" {
                value.share_key = Some("fixture-key".into());
            }
            let active: session::ActiveModel = value.into();
            active.insert(conn).await.unwrap();
        }
        let auth = Auth::new(db.clone(), config(disabled));
        let router = test_router(auth.clone());
        let basic = basic("auth-client:fixture:password");
        let admin = super::tests::basic("fixture-admin:fixture-admin-pass");
        let deny = if disabled { 200 } else { 401 };
        for value in [
            None,
            Some("Basic @@@"),
            Some("basic YTpi"),
            Some("Basic YWJj"),
        ] {
            status(&router, "/ws/rest/client/probe", value, deny).await;
        }
        status(&router, "/ws/rest/client/probe", Some(&basic), 200).await;
        status(
            &router,
            "/ws/rest/client/probe",
            Some(&super::tests::basic("auth-client:wrong")),
            deny,
        )
        .await;
        status(
            &router,
            "/ws/rest/client/probe",
            Some(&super::tests::basic("unknown:wrong")),
            deny,
        )
        .await;
        status(&router, "/ws/rest/v1/admin/probe", None, 401).await;
        let (admin_response, client_response) = tokio::join!(
            request(&router, "/ws/rest/v1/admin/probe", Some(&admin), &[], "GET"),
            request(&router, "/ws/rest/client/probe", Some(&basic), &[], "GET")
        );
        assert_eq!(
            admin_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
            "fixture-admin:Admin"
        );
        let client_body = client_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(
            client_body,
            if disabled {
                "anonymous"
            } else {
                "auth-client:Client"
            }
        );
        status(&router, "/ws/rest/v1/admin/probe", Some(&admin), 200).await;
        status(&router, "/ws/rest/v1/admin/probe", Some(&basic), 403).await;
        status(&router, "/ws/rest/v1/admin/probe", None, 401).await;
        let response = request(&router, "/ws/rest/health", Some(&admin), &[], "GET").await;
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            "anonymous"
        );
        status(
            &router,
            "/ws/rest/v1/query?session=missing&sessionId=missing",
            None,
            200,
        )
        .await;
        status(&router, "/ws/rest/v1/session/missing", None, 200).await; // forwarded to later feature controller
        status(&router, "/ws/rest/v1/session/missing/", None, 401).await;
        status(&router, "/ws/rest/v1/session/missing/probe", None, 401).await;
        status(
            &router,
            "/ws/rest/v1/session/missing/tail/missing",
            None,
            200,
        )
        .await;
        status(
            &router,
            "/ws/rest/v1/session/auth-live/probe?sessionId=missing",
            None,
            200,
        )
        .await;
        assert!(auth.sessions.get("auth-live").await.is_some());
        status(
            &router,
            "/ws/rest/v1/session/auth-live/permission",
            None,
            200,
        )
        .await;
        for path in [
            "/ws/rest/v1/session/auth-live/uppercase",
            "/ws/rest/v1/query-permission?sessionId=auth-live",
            "/ws/rest/v1/session/auth-empty/permission",
            "/ws/rest/v1/session/auth-null/permission",
        ] {
            status(&router, path, None, deny).await;
        }
        status(
            &router,
            "/ws/rest/v1/session/auth-all/permission",
            None,
            200,
        )
        .await;
        status(
            &router,
            "/ws/rest/v1/admin/permission",
            Some(&basic),
            if disabled { 200 } else { 403 },
        )
        .await;
        status(&router, "/ws/rest/v1/session/auth-shared/probe", None, 401).await;
        status(&router, "/ws/rest/v1/session/auth-shared", None, 200).await;
        for (key, expected) in [("wrong", 401), ("fixture-key", 200)] {
            let response = request(
                &router,
                "/ws/rest/v1/session/auth-shared/probe",
                None,
                &[("session-key", key)],
                "GET",
            )
            .await;
            assert_eq!(response.status().as_u16(), expected);
        }
        status(&router, "/ws/rest/v1/session/auth-expired/probe", None, 401).await;
        assert!(
            session::Entity::find_by_id("auth-expired")
                .one(conn)
                .await
                .unwrap()
                .is_none()
        );
        auth.cache_session(snapshot("auth-live", now)).await;
        assert!(auth.session_at("auth-live", now).await.unwrap().is_none()); // exact boundary
        assert!(auth.sessions.get("auth-live").await.is_none());
        assert!(
            session::Entity::find_by_id("auth-live")
                .one(conn)
                .await
                .unwrap()
                .is_some()
        );
        status(&router, "/ws/rest/v1/session/auth-live/probe", None, 200).await;
        conn.execute_unprepared(
            "UPDATE session SET permissions = ARRAY[]::text[] WHERE id='auth-live'",
        )
        .await
        .unwrap();
        status(
            &router,
            "/ws/rest/v1/session/auth-live/permission",
            None,
            200,
        )
        .await;
        auth.invalidate_session("auth-live").await;
        status(
            &router,
            "/ws/rest/v1/session/auth-live/permission",
            None,
            deny,
        )
        .await;
        for (forwarded, expected) in [
            ("203.0.113.10", 200),
            ("203.0.113.1", 200),
            ("", 200),
            ("198.51.100.1", deny),
            ("203.0.113.10, 127.0.0.1", deny),
        ] {
            assert_eq!(
                request(
                    &router,
                    "/ws/rest/client/probe",
                    Some(&basic),
                    &[("x-forwarded-for", forwarded)],
                    "GET"
                )
                .await
                .status()
                .as_u16(),
                expected
            );
        }
        for id in ["auth-open", "auth-empty-ip"] {
            status(
                &router,
                "/ws/rest/client/probe",
                Some(&super::tests::basic(&format!("{id}:fixture:password"))),
                200,
            )
            .await;
        }
        let cors = request(
            &router,
            "/ws/rest/v1/admin/probe",
            None,
            &[
                ("origin", "https://fixture.invalid"),
                ("access-control-request-method", "GET"),
                (
                    "access-control-request-headers",
                    "authorization,session-key",
                ),
            ],
            "OPTIONS",
        )
        .await;
        assert_eq!(cors.status(), StatusCode::OK);
        assert_eq!(cors.headers()["access-control-allow-origin"], "*");
        for (method, header) in [("TRACE", "authorization"), ("GET", "x-unknown")] {
            let response = request(
                &router,
                "/ws/rest/v1/admin/probe",
                None,
                &[
                    ("origin", "https://fixture.invalid"),
                    ("access-control-request-method", method),
                    ("access-control-request-headers", header),
                ],
                "OPTIONS",
            )
            .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        assert!(
            cors.headers()["access-control-allow-headers"]
                .to_str()
                .unwrap()
                .contains("session-key")
        );
        assert_eq!(
            auth.sessions.policy().time_to_live(),
            Some(Duration::from_secs(7200))
        );
        assert_eq!(auth.sessions.policy().max_capacity(), Some(1000));
        assert_eq!(
            auth.clients.policy().time_to_live(),
            Some(Duration::from_secs(36000))
        );
        assert_eq!(auth.clients.policy().max_capacity(), Some(10));
        if !disabled {
            client::ActiveModel {
                id: Set("auth-client".into()),
                password: Set("invalid".into()),
                ..Default::default()
            }
            .update(conn)
            .await
            .unwrap();
            status(&router, "/ws/rest/client/probe", Some(&basic), 200).await;
            auth.invalidate_client("auth-client").await;
            status(&router, "/ws/rest/client/probe", Some(&basic), 401).await;
            client::ActiveModel {
                id: Set("auth-client".into()),
                password: Set(hash.clone()),
                ..Default::default()
            }
            .update(conn)
            .await
            .unwrap();
            auth.invalidate_client("auth-client").await;
        }
        // Cache hit remains usable even when the backing row disappears.
        auth.cache_session(snapshot("auth-memory", future)).await;
        status(&router, "/ws/rest/v1/session/auth-memory/probe", None, 200).await;
        assert!(
            !auth
                .has_permission(Some(" "), MODIFY_OVERLAY_CONFIG)
                .await
                .unwrap()
                || disabled
        );
        // Verify real TCP peer extraction, with the production router/security layer.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            summer_web::axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
        });
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(format!("GET /ws/rest/client/probe HTTP/1.1\r\nHost: localhost\r\nAuthorization: {basic}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert!(bytes.starts_with(b"HTTP/1.1 200"));
        stop.send(()).unwrap();
        task.await.unwrap();
    }
    conn.execute_unprepared(
        "DELETE FROM client WHERE id LIKE 'auth-%'; DELETE FROM session WHERE id LIKE 'auth-%';",
    )
    .await
    .unwrap();
    let auth = Auth::new(db.clone(), config(false));
    db.clone().close().await.unwrap();
    assert!(auth.session("missing").await.is_err()); // DB failures are not a missing session.
    let router = test_router(auth);
    status(&router, "/ws/rest/v1/session/missing/probe", None, 500).await;
}
