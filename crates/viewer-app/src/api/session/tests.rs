use super::*;
use crate::auth::AuthConfig;
use http_body_util::BodyExt;
use sea_orm::EntityTrait;
use summer_web::axum::{Router as HttpRouter, body::Body, http::Request};
use tower::ServiceExt;

#[test]
fn expiry_key_and_midnight_boundaries() {
    let config = SessionConfig::default();
    let now = DateTime::parse_from_rfc3339("2026-10-27T16:59:59.500Z")
        .unwrap()
        .with_timezone(&Utc);
    for (minutes, seconds) in [
        (0, 3600),
        (-1, 3600),
        (1, 60),
        (129600, 7776000),
        (129601, 3600),
    ] {
        assert_eq!(
            (share_expiry(&config, minutes, now) - now).num_seconds(),
            seconds
        );
    }
    assert_eq!(until_midnight(now), std::time::Duration::from_millis(500));
    assert_eq!(
        until_midnight(now + Duration::milliseconds(500)),
        std::time::Duration::from_secs(86400)
    );
    for _ in 0..100 {
        let key = share_key();
        assert_eq!(key.len(), 6);
        assert!(key.bytes().all(|b| b.is_ascii_digit()));
    }
}
async fn call(
    app: &HttpRouter,
    method: &str,
    path: &str,
    input: Value,
    key: Option<&str>,
) -> (u16, Vec<u8>) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/ws/rest{path}"))
        .header("content-type", "application/json");
    if let Some(key) = key {
        request = request.header("session-key", key);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(input.to_string())).unwrap())
        .await
        .unwrap();
    (
        response.status().as_u16(),
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL configured in TOML"]
async fn postgres_migrations_transaction_readiness_and_close_session_lifecycle() {
    let _url_guard = crate::api::URL_CONFIG_TEST_LOCK.lock().await;
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let router = crate::api::router(db.clone(), None, auth.clone(), SessionConfig::default());
    let app = HttpRouter::new().nest("/ws/rest", router.into());
    let source = session::Model {
        id: format!("step9-{}", uuid::Uuid::new_v4()),
        user_id: Some("fixture".into()),
        permissions: Some(vec!["ADMIN".into()]),
        store_url: Some("https://fixture.invalid".into()),
        store_auth: Some("fixture-secret".into()),
        studies: "study-1".into(),
        anonymized: false,
        shared_session: false,
        share_key: None,
        expired_time: (Utc::now() + Duration::hours(1)).fixed_offset(),
    };
    auth.cache_session(source.clone()).await;
    let path = format!("/v1/session/{}", source.id);
    let (status, body) = call(&app, "GET", &path, Value::Null, None).await;
    assert_eq!(status, 200);
    let value: Value = serde_json::from_slice(&body).unwrap();
    for field in ["storeAuth", "storeUrl", "shareKey", "permissions"] {
        assert!(value.get(field).is_none());
    }
    assert_eq!(
        call(&app, "GET", "/v1/session/step9-missing", Value::Null, None)
            .await
            .0,
        400
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            "/v1/session",
            json!({"id":"step9-missing"}),
            None
        )
        .await,
        (200, vec![])
    );
    let share_path = format!("{path}/share");
    assert_eq!(
        call(&app, "POST", &share_path, json!({}), None).await.0,
        400
    );
    let (status, body) = call(
        &app,
        "POST",
        &share_path,
        json!({"expireIn":0,"anonymized":true,"requiresShareKey":true}),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let shared: Value = serde_json::from_slice(&body).unwrap();
    let id = shared["link"]
        .as_str()
        .unwrap()
        .split("session=")
        .nth(1)
        .unwrap();
    let key = shared["shareKey"].as_str().unwrap();
    let saved = session::Entity::find_by_id(id)
        .one(db.connection())
        .await
        .unwrap()
        .unwrap();
    assert!(saved.shared_session && saved.anonymized);
    assert!(saved.user_id.is_none() && saved.permissions.is_none());
    assert_eq!(saved.store_auth, source.store_auth);
    assert!((saved.expired_time.with_timezone(&Utc) - Utc::now()).num_seconds() > 3590);
    let shared_path = format!("/v1/session/{id}");
    // Terminal GET bypasses share-key check; nested operations require it.
    assert_eq!(
        call(&app, "GET", &shared_path, Value::Null, None).await.0,
        200
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("{shared_path}/share"),
            json!({"expireIn":1}),
            None
        )
        .await
        .0,
        401
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("{shared_path}/share"),
            json!({"expireIn":1}),
            Some(key)
        )
        .await
        .0,
        400
    );
    // Warmed DB session is editable only in cache.
    assert_eq!(
        call(
            &app,
            "PUT",
            "/v1/session",
            json!({"id":id,"studyInstanceUIDs":["changed"]}),
            None
        )
        .await
        .0,
        200
    );
    assert!(
        auth.session(id)
            .await
            .unwrap()
            .unwrap()
            .studies
            .contains(&Some("changed".into()))
    );
    assert_eq!(
        session::Entity::find_by_id(id)
            .one(db.connection())
            .await
            .unwrap()
            .unwrap()
            .studies,
        "study-1"
    );
    assert_eq!(
        call(&app, "DELETE", &shared_path, Value::Null, None)
            .await
            .0,
        200
    );
    assert_eq!(
        call(&app, "DELETE", &shared_path, Value::Null, None)
            .await
            .0,
        200
    );
    assert!(auth.session(id).await.unwrap().is_none());
    // Cleanup is strict (< now), DB only; live cache entries survive.
    let cutoff = Utc::now();
    for (suffix, expiry) in [
        ("expired", cutoff - Duration::seconds(1)),
        ("equal", cutoff),
        ("future", cutoff + Duration::hours(1)),
    ] {
        let mut model = source.clone();
        model.id = format!("{}-{suffix}", source.id);
        model.expired_time = expiry.fixed_offset();
        model
            .into_active_model()
            .insert(db.connection())
            .await
            .unwrap();
    }
    cleanup(&db, cutoff).await.unwrap();
    assert!(
        session::Entity::find_by_id(format!("{}-expired", source.id))
            .one(db.connection())
            .await
            .unwrap()
            .is_none()
    );
    for suffix in ["equal", "future"] {
        assert!(
            session::Entity::find_by_id(format!("{}-{suffix}", source.id))
                .one(db.connection())
                .await
                .unwrap()
                .is_some()
        );
        session::Entity::delete_by_id(format!("{}-{suffix}", source.id))
            .exec(db.connection())
            .await
            .unwrap();
    }
    call(&app, "DELETE", &path, Value::Null, None).await;
    // Public URL config overrides share the same store/cache as Step 8.
    let configs = ConfigStore::new(db.clone());
    let service = Lifecycle {
        database: db.clone(),
        auth: auth.clone(),
        configs: configs.clone(),
        config: SessionConfig {
            public_domain: "https://fallback.invalid".into(),
            ..Default::default()
        },
    };
    auth.cache_session(source.clone()).await;
    for (name, value) in [
        ("PATH_PREFIX", "/shared?session="),
        ("PUBLIC_DOMAIN", "https://public.invalid"),
    ] {
        configs
            .create(crate::persistence::config::Model {
                id: name.into(),
                value: value.into(),
                description: None,
                datatype: None,
            })
            .await
            .unwrap();
    }
    let result = service
        .share(
            &source.id,
            Share {
                public: Some(true),
                anonymized: None,
                requires_share_key: None,
                expire_in: Some(1),
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        result["link"]
            .as_str()
            .unwrap()
            .starts_with("https://public.invalid/shared?session=")
    );
    assert!(result.get("shareKey").is_none());
    let public_id = result["link"]
        .as_str()
        .unwrap()
        .split("session=")
        .nth(1)
        .unwrap();
    session::Entity::delete_by_id(public_id)
        .exec(db.connection())
        .await
        .unwrap();
    for name in ["PATH_PREFIX", "PUBLIC_DOMAIN"] {
        configs.delete(name).await.unwrap();
    }
    let result = service
        .share(
            &source.id,
            Share {
                public: Some(true),
                anonymized: None,
                requires_share_key: None,
                expire_in: Some(1),
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        result["link"]
            .as_str()
            .unwrap()
            .starts_with("https://fallback.invalid/viewer?session=")
    );
    session::Entity::delete_by_id(
        result["link"]
            .as_str()
            .unwrap()
            .split("session=")
            .nth(1)
            .unwrap(),
    )
    .exec(db.connection())
    .await
    .unwrap();
    // Long-term cache miss, expiry eviction, and cache-only expired update.
    let mut expired = source.clone();
    expired.id = format!("{}-stale", source.id);
    expired.expired_time = (Utc::now() - Duration::seconds(1)).fixed_offset();
    expired
        .clone()
        .into_active_model()
        .insert(db.connection())
        .await
        .unwrap();
    assert!(auth.session(&expired.id).await.unwrap().is_none());
    assert!(
        session::Entity::find_by_id(&expired.id)
            .one(db.connection())
            .await
            .unwrap()
            .is_none()
    );
    auth.cache_session(expired.clone()).await;
    assert_eq!(
        call(
            &app,
            "PUT",
            "/v1/session",
            json!({"id":expired.id,"studyInstanceUIDs":[]}),
            None
        )
        .await
        .0,
        200
    );
    assert!(auth.session(&expired.id).await.unwrap().is_none());
    // With source cached, persistence failure occurs inside the controller.
    db.close().await.unwrap();
    assert_eq!(
        call(&app, "POST", &share_path, json!({"expireIn":1}), None).await,
        (500, vec![])
    );
    assert_eq!(
        call(&app, "GET", "/v1/session/step9-db-error", Value::Null, None).await,
        (500, vec![])
    );
    assert_eq!(
        call(&app, "DELETE", &path, Value::Null, None).await,
        (500, vec![])
    );
    assert_eq!(
        call(&app, "POST", &share_path, json!({"expireIn":1}), None).await,
        (500, b"Authentication service unavailable".to_vec())
    );
}

/// Export the same HTTP fixtures as Step0809BaselineTest in the Java checkout.
/// Dedicated DB required; this test runs separately from the regression suites.
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL; exports Java/Rust comparison fixtures"]
async fn step_08_09_baseline_export() {
    let _guard = crate::api::URL_CONFIG_TEST_LOCK.lock().await;
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    // Refuse to run against a DB containing sessions; cleanup only fixture writes.
    assert!(
        session::Entity::find()
            .all(db.connection())
            .await
            .unwrap()
            .is_empty(),
        "baseline export requires an empty dedicated SESSION table"
    );
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../../../docs/fixtures/step-08-09-http.json"
    ))
    .unwrap();
    let mut results = Vec::new();
    for fixture in fixtures.as_array().unwrap() {
        let auth = Auth::new(
            db.clone(),
            AuthConfig {
                client_security_disabled: true,
                ..Default::default()
            },
        );
        for id in ["baseline-source", "baseline-shared", "baseline-key"] {
            let model = session::Model {
                id: id.into(),
                user_id: None,
                permissions: None,
                store_url: Some("http://store.invalid".into()),
                store_auth: Some("fixture-token".into()),
                studies: "a".into(),
                anonymized: false,
                shared_session: id != "baseline-source",
                share_key: (id == "baseline-key").then(|| "123456".into()),
                expired_time: DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z").unwrap(),
            };
            auth.cache_created_session(CachedSession {
                model,
                studies: BTreeSet::from([Some("a".into())]),
            })
            .await;
        }
        let app = HttpRouter::new().nest(
            "/ws/rest",
            crate::api::router(
                db.clone(),
                None,
                auth,
                SessionConfig {
                    public_domain: "https://public.invalid".into(),
                    ..Default::default()
                },
            )
            .into(),
        );
        let mut request = Request::builder()
            .method(fixture["method"].as_str().unwrap())
            .uri(fixture["path"].as_str().unwrap())
            .header("content-type", "application/json");
        if let Some(key) = fixture.get("key") {
            request = request.header("session-key", key.as_str().unwrap());
        }
        let response = app
            .oneshot(
                request
                    .body(Body::from(fixture["body"].to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let records = session::Entity::find().all(db.connection()).await.unwrap();
        results.push(json!({"name":fixture["name"],"step":fixture["step"],"status":status,"body":String::from_utf8(body.to_vec()).unwrap(),"dbRows":records.len(),"dbStudies":records.iter().map(|r| &r.studies).collect::<Vec<_>>(),"dbSessions":records.iter().map(|r|json!({"anonymized":r.anonymized,"sharedSession":r.shared_session,"hasUser":r.user_id.is_some(),"hasPermissions":r.permissions.is_some(),"storeUrl":r.store_url,"storeAuth":r.store_auth,"expiredTime":r.expired_time.with_timezone(&Utc).to_rfc3339()})).collect::<Vec<_>>(),"capturedAt":Utc::now().to_rfc3339()}));
        for record in records {
            session::Entity::delete_by_id(record.id)
                .exec(db.connection())
                .await
                .unwrap();
        }
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/step-09-logs/baseline-rust.json");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(out, serde_json::to_string_pretty(&results).unwrap()).unwrap();
    db.close().await.unwrap();
}

#[test]
fn shared_expiry_accepts_integer_strings() {
    for (input, expected) in [
        (json!("1"), Some(1)),
        (json!(" 0 "), Some(0)),
        (json!(""), None),
        (Value::Null, None),
    ] {
        let share: Share = serde_json::from_value(json!({"expireIn":input})).unwrap();
        assert_eq!(share.expire_in, expected);
    }
}
