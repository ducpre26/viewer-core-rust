use super::*;
use crate::{auth::AuthConfig, domain::MODIFY_OVERLAY_CONFIG, persistence::client};
use base64::{Engine, engine::general_purpose::STANDARD};
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, ConnectionTrait, PaginatorTrait, Set};
use summer_web::axum::{Router as HttpRouter, body::Body, extract::ConnectInfo, http::Request};
use tower::ServiceExt;

const ADMIN: &str = "step8-admin:fixture-admin-password";
const CLIENT: &str = "step8-client:fixture-password";

fn app(db: Persistence, disabled: bool) -> (HttpRouter, Auth) {
    let auth = Auth::new(
        db.clone(),
        AuthConfig {
            client_security_disabled: disabled,
            admin_username: "step8-admin".into(),
            admin_password: "fixture-admin-password".into(),
        },
    );
    let router = crate::api::router(
        db,
        None,
        auth.clone(),
        SessionConfig {
            default_expire_minutes: 12,
            path_prefix: "/fixture?session=".into(),
        },
    );
    (HttpRouter::new().nest("/ws/rest", router.into()), auth)
}

async fn call(
    app: &HttpRouter,
    method: &str,
    path: &str,
    body: Value,
    credential: Option<&str>,
    xff: Option<&str>,
) -> (u16, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/ws/rest/{path}"))
        .header("content-type", "application/json");
    if let Some(credential) = credential {
        request = request.header(
            "authorization",
            format!("Basic {}", STANDARD.encode(credential)),
        );
    }
    if let Some(xff) = xff {
        request = request.header("x-forwarded-for", xff);
    }
    let mut request = request.body(Body::from(body.to_string())).unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:44008".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn input() -> Value {
    json!({"storeUrl":"http://store.invalid/wado", "storeAuth":"fixture-store-token",
        "studies":["1.2.3", "1.2.4", "1.2.3"], "user":"fixture-user"})
}

fn assert_expiry(value: &Value, before: DateTime<Utc>, minutes: i64) {
    let actual = DateTime::parse_from_rfc3339(value.as_str().unwrap())
        .unwrap()
        .with_timezone(&Utc);
    assert!(actual >= before + Duration::minutes(minutes) - Duration::milliseconds(1));
    assert!(actual <= Utc::now() + Duration::minutes(minutes));
    assert_eq!(actual.timestamp_subsec_nanos() % 1_000_000, 0);
}

#[test]
fn expiry_normalization_and_db_study_conversion() {
    let config = SessionConfig::default();
    let now = Utc::now();
    for value in [None, Some(0), Some(-1), Some(61), Some(i32::MAX)] {
        assert_eq!(
            config.expiry(value, now).unwrap(),
            now + Duration::minutes(60)
        );
    }
    for value in [1, 59, 60] {
        assert_eq!(
            config.expiry(Some(value), now).unwrap(),
            now + Duration::minutes(value.into())
        );
    }
    let mut config = config;
    config.default_expire_minutes = i64::MAX;
    assert!(config.expiry(None, now).is_err());
    config.default_expire_minutes = 0;
    assert_eq!(config.expiry(Some(1), now).unwrap(), now);
    let cached = CachedSession::from(session::Model {
        id: "fixture".into(),
        store_url: None,
        store_auth: None,
        studies: " 1.2.3 , ,1.2.4,1.2.3".into(),
        user_id: None,
        anonymized: false,
        shared_session: false,
        share_key: None,
        expired_time: now.fixed_offset(),
        permissions: None,
    });
    assert_eq!(
        cached.studies,
        BTreeSet::from([Some("1.2.3".into()), Some("1.2.4".into())])
    );
    assert!(!format!("{cached:?}").contains("1.2.3"));
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL; included by documented PostgreSQL regression filter"]
async fn postgres_migrations_transaction_readiness_and_close_create_session() {
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let conn = db.connection();
    conn.execute_unprepared(
        "DELETE FROM client WHERE id LIKE 'step8-%'; DELETE FROM config WHERE id='PATH_PREFIX';",
    )
    .await
    .unwrap();
    client::ActiveModel {
        id: Set("step8-client".into()),
        password: Set(bcrypt::hash("fixture-password", 8).unwrap()),
        ip_allowed: Set(Some("127.0.0.1,203.0.113.10".into())),
    }
    .insert(conn)
    .await
    .unwrap();
    client_permission::ActiveModel {
        client_id: Set("step8-client".into()),
        permission_id: Set(MODIFY_OVERLAY_CONFIG.into()),
    }
    .insert(conn)
    .await
    .unwrap();
    let (router, auth) = app(db.clone(), false);
    let before = Utc::now();
    let (status, value) = call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
    assert_eq!(status, 200);
    let s = &value["session"];
    let id = s["id"].as_str().unwrap();
    assert_eq!(uuid::Uuid::parse_str(id).unwrap().get_version_num(), 4);
    assert_eq!(value["link"], format!("/fixture?session={id}"));
    assert_eq!(s["storeUrl"], input()["storeUrl"]);
    assert_eq!(s["storeAuth"], input()["storeAuth"]);
    assert_eq!(s["user"], "fixture-user");
    assert_eq!(s["studies"], json!(["1.2.3", "1.2.4"]));
    assert_eq!(s["permissions"], json!([MODIFY_OVERLAY_CONFIG]));
    assert_expiry(&s["expiredTime"], before, 12);
    for field in ["anonymized", "sharedSession", "requiresShareKey"] {
        assert_eq!(s[field], false);
    }
    assert!(s["shareKey"].is_null());
    assert!(
        session::Entity::find_by_id(id)
            .one(conn)
            .await
            .unwrap()
            .is_none()
    );
    let cached = auth.session(id).await.unwrap().unwrap();
    assert_eq!(
        cached.permissions.as_deref(),
        Some(&[MODIFY_OVERLAY_CONFIG.to_owned()][..])
    );
    assert_eq!(cached.studies.len(), 2);
    assert_eq!(cached.store_auth.as_deref(), Some("fixture-store-token"));

    // A real permission-protected endpoint accepts the just-created cache entry.
    let write = format!("v1/session/{id}/config");
    assert_eq!(
        call(
            &router,
            "POST",
            &write,
            json!({"id":"step8-overlay","value":"fixture"}),
            None,
            None
        )
        .await
        .0,
        201
    );
    client_permission::Entity::delete_many()
        .filter(client_permission::Column::ClientId.eq("step8-client"))
        .exec(conn)
        .await
        .unwrap();
    assert!(
        auth.has_permission(Some(id), MODIFY_OVERLAY_CONFIG)
            .await
            .unwrap()
    );
    let (status, next) = call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
    assert_eq!(status, 200);
    assert_ne!(next["session"]["id"], id);
    assert_eq!(next["session"]["permissions"], json!([]));
    let next_id = next["session"]["id"].as_str().unwrap();
    assert!(
        !auth
            .has_permission(Some(next_id), MODIFY_OVERLAY_CONFIG)
            .await
            .unwrap()
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("v1/session/{next_id}/config"),
            json!({"id":"step8-denied","value":"x"}),
            None,
            None
        )
        .await
        .0,
        401
    );
    // Ordinary client may carry ADMIN permission without becoming an admin principal.
    client_permission::ActiveModel {
        client_id: Set("step8-client".into()),
        permission_id: Set("ADMIN".into()),
    }
    .insert(conn)
    .await
    .unwrap();
    let (status, all) = call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
    assert_eq!(status, 200);
    assert_eq!(all["session"]["permissions"], json!(["ADMIN"]));
    assert!(
        auth.has_permission(all["session"]["id"].as_str(), MODIFY_OVERLAY_CONFIG)
            .await
            .unwrap()
    );
    assert_eq!(
        call(
            &router,
            "GET",
            "v1/admin/config",
            Value::Null,
            Some(CLIENT),
            None
        )
        .await
        .0,
        403
    );

    // Basic/IP failures occur before validation and create no DB session.
    for credential in [None, Some("unknown:fixture"), Some("step8-client:wrong")] {
        for path in ["client/link", "client/getlink"] {
            assert_eq!(
                call(&router, "POST", path, input(), credential, None)
                    .await
                    .0,
                401
            );
        }
    }
    for (xff, expected) in [
        ("203.0.113.10", 200),
        ("203.0.113.1", 200),
        ("203.0.113.10, 127.0.0.1", 401),
        ("198.51.100.1", 401),
    ] {
        assert_eq!(
            call(
                &router,
                "POST",
                "client/link",
                input(),
                Some(CLIENT),
                Some(xff)
            )
            .await
            .0,
            expected
        );
    }
    let (status, admin) = call(&router, "POST", "client/link", input(), Some(ADMIN), None).await;
    assert_eq!(status, 404);
    assert_eq!(admin["errorCode"], "NOT_FOUND");
    // Sysadmin must have a CLIENT row, then receives only explicit mappings.
    client::ActiveModel {
        id: Set("step8-admin".into()),
        password: Set(bcrypt::hash("unused", 8).unwrap()),
        ip_allowed: Set(None),
    }
    .insert(conn)
    .await
    .unwrap();
    client_permission::ActiveModel {
        client_id: Set("step8-admin".into()),
        permission_id: Set("ADMIN".into()),
    }
    .insert(conn)
    .await
    .unwrap();
    let (_, admin) = call(&router, "POST", "client/link", input(), Some(ADMIN), None).await;
    assert_eq!(admin["session"]["permissions"], json!(["ADMIN"]));

    // Required fields, empty set and legacy nested validation exactly follow Java.
    for field in ["storeUrl", "studies"] {
        for missing in [true, false] {
            let mut invalid = input();
            if missing {
                invalid.as_object_mut().unwrap().remove(field);
            } else {
                invalid[field] = Value::Null;
            }
            let (status, error) =
                call(&router, "POST", "client/link", invalid, Some(CLIENT), None).await;
            assert_eq!(status, 400);
            assert_eq!(error["errorCode"], "VALIDATION_ERROR");
        }
    }
    let mut empty = input();
    empty["studies"] = json!([]);
    assert_eq!(
        call(&router, "POST", "client/link", empty, Some(CLIENT), None).await,
        (400, Value::Null)
    );
    let (_, unusual) = call(
        &router,
        "POST",
        "client/link",
        json!({"storeUrl":"", "studies":[null,""," a,b ",null]}),
        Some(CLIENT),
        None,
    )
    .await;
    assert_eq!(unusual["session"]["storeUrl"], "");
    assert_eq!(unusual["session"]["studies"], json!([null, "", " a,b "]));
    let unusual_cached = auth
        .session(unusual["session"]["id"].as_str().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        unusual_cached.studies,
        BTreeSet::from([None, Some("".into()), Some(" a,b ".into())])
    );
    for body in [
        json!({}),
        json!({"studyUIDs":null}),
        json!({"studyUIDs":{}}),
    ] {
        assert_eq!(
            call(&router, "POST", "client/getlink", body, Some(CLIENT), None)
                .await
                .0,
            400
        );
    }
    for store in [Value::Null, json!({}), json!({"url":null})] {
        let (status, legacy) = call(
            &router,
            "POST",
            "client/getlink",
            json!({"studyUIDs":{"1.2.3":store},"userID":"fixture-user"}),
            Some(CLIENT),
            None,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(legacy["header"]["code"], 200);
        let id = legacy["body"]
            .as_str()
            .unwrap()
            .strip_prefix("/fixture?session=")
            .unwrap();
        assert_eq!(
            legacy["header"]["message"],
            format!("Viewer session created: {id}")
        );
        let cached = auth.session(id).await.unwrap().unwrap();
        assert!(cached.store_url.is_none());
        assert_eq!(cached.user_id.as_deref(), Some("fixture-user"));
    }
    let before = Utc::now();
    let (status, legacy) = call(&router,"POST","client/getlink",json!({"studyUIDs":{"1.2.3":{"url":"http://legacy.invalid","authentication":"fixture"}},"expireIn":2}),Some(CLIENT),None).await;
    assert_eq!(status, 200);
    let legacy_id = legacy["body"]
        .as_str()
        .unwrap()
        .strip_prefix("/fixture?session=")
        .unwrap();
    let legacy_session = auth.session(legacy_id).await.unwrap().unwrap();
    assert_eq!(
        legacy_session.store_url.as_deref(),
        Some("http://legacy.invalid")
    );
    assert_eq!(legacy_session.store_auth.as_deref(), Some("fixture"));
    assert!(
        legacy_session.expired_time >= before + Duration::minutes(2) - Duration::milliseconds(1)
    );

    for (requested, minutes) in [
        (Value::Null, 12),
        (json!(0), 12),
        (json!(-1), 12),
        (json!(13), 12),
        (json!(1), 1),
        (json!(12), 12),
    ] {
        let mut body = input();
        body["expireIn"] = requested;
        let before = Utc::now();
        let (status, result) = call(&router, "POST", "client/link", body, Some(CLIENT), None).await;
        assert_eq!(status, 200);
        assert_expiry(&result["session"]["expiredTime"], before, minutes);
    }

    // Shared Step 5 cache: create, update and delete PATH_PREFIX through admin API.
    for (method, value, prefix) in [
        ("POST", "/configured?session=", "/configured?session="),
        ("PUT", "/changed/", "/changed/"),
        ("PUT", "", ""),
    ] {
        assert!(
            call(
                &router,
                method,
                "v1/admin/config",
                json!({"id":"PATH_PREFIX","value":value}),
                Some(ADMIN),
                None
            )
            .await
            .0 < 300
        );
        let (status, result) =
            call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
        assert_eq!(status, 200);
        assert_eq!(
            result["link"],
            format!("{prefix}{}", result["session"]["id"].as_str().unwrap())
        );
    }
    assert_eq!(
        call(
            &router,
            "DELETE",
            "v1/admin/config/PATH_PREFIX",
            Value::Null,
            Some(ADMIN),
            None
        )
        .await
        .0,
        200
    );
    let (_, result) = call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
    assert!(
        result["link"]
            .as_str()
            .unwrap()
            .starts_with("/fixture?session=")
    );

    // Disabled security ignores even supplied credentials and snapshots no permissions.
    let (disabled, disabled_auth) = app(db.clone(), true);
    for credential in [None, Some(CLIENT), Some("wrong:wrong")] {
        let (status, result) =
            call(&disabled, "POST", "client/link", input(), credential, None).await;
        assert_eq!(status, 200);
        assert!(result["session"]["permissions"].is_null());
        assert!(
            disabled_auth
                .session(result["session"]["id"].as_str().unwrap())
                .await
                .unwrap()
                .is_some()
        );
    }
    let (fresh, fresh_auth) = app(db.clone(), false);
    assert!(fresh_auth.session(id).await.unwrap().is_none());
    assert_eq!(
        call(
            &fresh,
            "GET",
            &format!("v1/session/{id}/config"),
            Value::Null,
            None,
            None
        )
        .await
        .0,
        401
    );
    assert_eq!(
        session::Entity::find_by_id(id).count(conn).await.unwrap(),
        0
    );
    auth.invalidate_session(id).await;
    assert_eq!(
        call(
            &router,
            "GET",
            &format!("v1/session/{id}/config"),
            Value::Null,
            None,
            None
        )
        .await
        .0,
        401
    );
    conn.execute_unprepared(
        "DELETE FROM client WHERE id LIKE 'step8-%'; DELETE FROM config WHERE id LIKE 'step8-%';",
    )
    .await
    .unwrap();
    db.close().await.unwrap();
    // DB errors are sanitized and cannot produce a link or a persisted session.
    let (status, error) = call(&router, "POST", "client/link", input(), Some(CLIENT), None).await;
    assert_eq!(status, 500);
    assert_eq!(error["message"], "Database operation failed");
    assert!(!error.to_string().contains("fixture-store-token"));
}
