use super::*;
use crate::auth::AuthConfig;
use base64::{Engine, engine::general_purpose::STANDARD};
use http_body_util::BodyExt;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use summer_web::axum::{Router as HttpRouter, body::Body, http::Request};
use tower::ServiceExt;

fn app(db: Persistence, disabled: bool) -> HttpRouter {
    let auth = Auth::new(
        db.clone(),
        AuthConfig {
            client_security_disabled: disabled,
            admin_username: "step4-admin".into(),
            admin_password: "fixture-admin-password".into(),
        },
    );
    HttpRouter::new().nest(
        "/ws/rest",
        crate::api::router(db, None, auth, crate::api::SessionConfig::default()).into(),
    )
}
async fn call(
    app: &HttpRouter,
    method: &str,
    path: &str,
    body: Value,
    credentials: Option<&str>,
) -> (u16, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(format!("/ws/rest/v1/admin/{path}"))
        .header("content-type", "application/json");
    if let Some(credentials) = credentials {
        req = req.header(
            "authorization",
            format!("Basic {}", STANDARD.encode(credentials)),
        );
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
async fn admin(app: &HttpRouter, method: &str, path: &str, body: Value, expected: u16) -> Value {
    let (status, value) = call(
        app,
        method,
        path,
        body,
        Some("step4-admin:fixture-admin-password"),
    )
    .await;
    assert_eq!(status, expected, "{method} {path}: {value}");
    value
}
async fn count(db: &Persistence, sql: &str) -> i64 {
    db.connection()
        .query_one(Statement::from_string(DbBackend::Postgres, sql.to_owned()))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

#[test]
fn java_clean_text_and_page_boundaries() {
    assert_eq!(clean(Some(" \t value \n".into())), Some("value".into()));
    assert_eq!(clean(Some(" \t".into())), None);
    assert_eq!(clean(Some("\u{a0}".into())), Some("\u{a0}".into()));
    assert_eq!(
        Paging {
            offset: 3,
            limit: 2
        }
        .start()
        .unwrap(),
        2
    );
    assert_eq!(
        Paging {
            offset: -3,
            limit: 2
        }
        .start()
        .unwrap(),
        0
    );
    assert!(
        Paging {
            offset: 0,
            limit: 0
        }
        .start()
        .is_err()
    );
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL; included by documented PostgreSQL regression filter"]
async fn postgres_migrations_transaction_readiness_and_close_admin() {
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let conn = db.connection();
    // Fixture IDs are isolated from the parallel auth tests and existing data.
    conn.execute_unprepared("DELETE FROM client WHERE id LIKE 'step4-%'; DELETE FROM permission WHERE id LIKE 'step4-%';").await.unwrap();
    let app = app(db.clone(), false);
    for (method, path, body) in [
        ("GET", "client", Value::Null),
        (
            "POST",
            "client",
            json!({"id":"step4-denied","password":"fixture"}),
        ),
        ("PUT", "client", json!({"id":"step4-denied"})),
        ("DELETE", "client/step4-denied", Value::Null),
        ("GET", "permission", Value::Null),
        (
            "POST",
            "permission",
            json!({"id":"step4-denied","name":"denied"}),
        ),
        ("PUT", "permission", json!({"id":"step4-denied"})),
        ("DELETE", "permission/step4-denied", Value::Null),
        ("GET", "client/step4-denied/permission", Value::Null),
        (
            "POST",
            "client/step4-denied/permission",
            json!({"permissionId":"ADMIN"}),
        ),
        (
            "DELETE",
            "client/step4-denied/permission/ADMIN",
            Value::Null,
        ),
    ] {
        assert_eq!(call(&app, method, path, body, None).await.0, 401);
    }
    for body in [
        json!({}),
        json!({"id":"step4-bad"}),
        json!({"password":"fixture"}),
        json!({"id":"step4-bad","password":"  "}),
    ] {
        let v = admin(&app, "POST", "client", body, 400).await;
        assert_eq!(v["errorCode"], "VALIDATION_ERROR");
    }
    for body in [
        json!({}),
        json!({"id":"step4-bad"}),
        json!({"name":"fixture"}),
    ] {
        admin(&app, "POST", "permission", body, 400).await;
    }
    assert_eq!(
        count(&db, "SELECT count(*) AS n FROM client WHERE id='step4-bad'").await,
        0
    );
    let created = admin(
        &app,
        "POST",
        "client",
        json!({"id":"step4-client","password":" fixture-secret ","ipAllowed":" 127.0.0.1 "}),
        201,
    )
    .await;
    assert_eq!(created["header"]["code"], 201);
    assert_eq!(created["body"], "step4-client");
    assert!(created["header"]["datetime"].is_string());
    let c = client::Entity::find_by_id("step4-client")
        .one(conn)
        .await
        .unwrap()
        .unwrap();
    assert!(c.password.starts_with("$2a$08$"));
    assert!(bcrypt::verify("fixture-secret", &c.password).unwrap());
    assert_eq!(c.ip_allowed.as_deref(), Some("127.0.0.1"));
    // Prime auth cache: a real client is forbidden even with the ADMIN permission.
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-secret")
        )
        .await
        .0,
        401
    ); // no matching remote IP
    admin(
        &app,
        "POST",
        "client",
        json!({"id":"step4-client","password":"fixture-secret"}),
        201,
    )
    .await;
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-secret")
        )
        .await
        .0,
        403
    );
    admin(
        &app,
        "POST",
        "client/step4-client/permission",
        json!({"permissionId":"ADMIN"}),
        201,
    )
    .await;
    for (method, path, body) in [
        (
            "POST",
            "client",
            json!({"id":"step4-denied","password":"fixture"}),
        ),
        (
            "PUT",
            "client",
            json!({"id":"step4-client","password":"denied"}),
        ),
        ("DELETE", "client/step4-client", Value::Null),
        ("GET", "client/step4-client", Value::Null),
        ("GET", "permission", Value::Null),
        ("GET", "permission/ADMIN", Value::Null),
        (
            "POST",
            "permission",
            json!({"id":"step4-denied","name":"Denied"}),
        ),
        ("PUT", "permission", json!({"id":"ADMIN","name":"Denied"})),
        ("DELETE", "permission/ADMIN", Value::Null),
        ("GET", "client/step4-client/permission", Value::Null),
        (
            "POST",
            "client/step4-client/permission",
            json!({"permissionId":"MODIFY_OVERLAY_CONFIG"}),
        ),
        (
            "DELETE",
            "client/step4-client/permission/ADMIN",
            Value::Null,
        ),
    ] {
        assert_eq!(
            call(
                &app,
                method,
                path,
                body,
                Some("step4-client:fixture-secret")
            )
            .await
            .0,
            403
        );
    }
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-secret")
        )
        .await
        .0,
        403
    );
    admin(
        &app,
        "PUT",
        "client",
        json!({"id":"step4-client","password":" fixture-new "}),
        200,
    )
    .await;
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-secret")
        )
        .await
        .0,
        401
    );
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-new")
        )
        .await
        .0,
        403
    );
    let before = client::Entity::find_by_id("step4-client")
        .one(conn)
        .await
        .unwrap()
        .unwrap();
    admin(
        &app,
        "PUT",
        "client",
        json!({"id":"step4-client","password":"  ","ipAllowed":null}),
        200,
    )
    .await;
    assert_eq!(
        client::Entity::find_by_id("step4-client")
            .one(conn)
            .await
            .unwrap()
            .unwrap()
            .password,
        before.password
    );
    let got = admin(&app, "GET", "client/step4-client", Value::Null, 200).await;
    assert!(got["body"].get("password").is_none());
    assert_eq!(got["body"]["permissionIds"], json!(["ADMIN"]));
    let list = admin(
        &app,
        "GET",
        "client?offset=0&limit=1&orderBy=ignored",
        Value::Null,
        200,
    )
    .await;
    assert_eq!(list["header"]["code"], 302);
    assert_eq!(list["header"]["limit"], 1);
    assert!(list["body"][0].get("password").is_none());
    assert!(list["body"][0]["permissionIds"].is_null());
    admin(&app, "GET", "client?limit=0", Value::Null, 400).await;
    admin(&app, "GET", "client?offset=no", Value::Null, 400).await;

    for (id, name) in [("step4-p1", "Step4 first"), ("step4-p2", "Step4 second")] {
        admin(
            &app,
            "POST",
            "permission",
            json!({"id":id,"name":name,"description":"fixture"}),
            201,
        )
        .await;
    }
    admin(
        &app,
        "POST",
        "permission",
        json!({"id":"step4-duplicate","name":"Step4 first"}),
        500,
    )
    .await;
    assert!(
        permission::Entity::find_by_id("step4-duplicate")
            .one(conn)
            .await
            .unwrap()
            .is_none()
    );
    admin(
        &app,
        "PUT",
        "permission",
        json!({"id":"step4-p2","name":"Step4 first","description":"must rollback"}),
        500,
    )
    .await;
    let p = permission::Entity::find_by_id("step4-p2")
        .one(conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.name, "Step4 second");
    assert_eq!(p.description.as_deref(), Some("fixture"));
    admin(
        &app,
        "PUT",
        "permission",
        json!({"id":"step4-p1","name":"Step4 renamed","description":"new"}),
        200,
    )
    .await;
    let p = admin(&app, "GET", "permission/step4-p1", Value::Null, 200).await;
    assert_eq!(
        p["body"],
        json!({"id":"step4-p1","name":"Step4 renamed","description":"new"})
    );
    let list = admin(&app, "GET", "permission?limit=1&offset=1", Value::Null, 200).await;
    assert_eq!(list["header"]["offset"], 1);
    assert_eq!(list["body"].as_array().unwrap().len(), 1);
    for resource in ["client", "permission"] {
        assert!(
            admin(
                &app,
                "GET",
                &format!("{resource}/step4-missing"),
                Value::Null,
                404
            )
            .await
            .is_null()
        );
        admin(&app, "PUT", resource, json!({"id":"step4-missing"}), 404).await;
        assert_eq!(
            admin(
                &app,
                "DELETE",
                &format!("{resource}/step4-missing"),
                Value::Null,
                404
            )
            .await["errorCode"],
            "NOT_FOUND"
        );
    }
    for (path, body, msg) in [
        (
            "client/step4-client/permission",
            json!({}),
            "permissionId không được rỗng",
        ),
        (
            "client/step4-client/permission",
            json!({"permissionId":" "}),
            "permissionId không được rỗng",
        ),
        (
            "client/step4-missing/permission",
            json!({"permissionId":"ADMIN"}),
            "Client không tồn tại: step4-missing",
        ),
        (
            "client/step4-client/permission",
            json!({"permissionId":"step4-missing"}),
            "Permission không tồn tại: step4-missing",
        ),
        (
            "client/step4-client/permission",
            json!({"permissionId":"ADMIN"}),
            "Client đã có quyền này rồi",
        ),
    ] {
        let v = admin(&app, "POST", path, body, 500).await;
        assert_eq!(v["header"]["message"], msg);
        assert_eq!(v["header"]["code"], 500);
        assert!(v["body"].is_null());
    }
    admin(
        &app,
        "GET",
        "client/step4-missing/permission",
        Value::Null,
        500,
    )
    .await;
    let mapped = admin(
        &app,
        "POST",
        "client/step4-client/permission",
        json!({"permissionId":"step4-p1"}),
        201,
    )
    .await;
    assert_eq!(mapped["header"]["code"], 200);
    let mappings = admin(
        &app,
        "GET",
        "client/step4-client/permission",
        Value::Null,
        200,
    )
    .await;
    assert!(mappings["body"].as_array().unwrap().iter().any(|p|p==&json!({"clientId":"step4-client","permissionId":"step4-p1","permissionName":"Step4 renamed","permissionDescription":"new"})));
    // Concurrent duplicate requests cannot bypass the composite primary key.
    let (a, b) = tokio::join!(
        call(
            &app,
            "POST",
            "client/step4-client/permission",
            json!({"permissionId":"step4-p2"}),
            Some("step4-admin:fixture-admin-password")
        ),
        call(
            &app,
            "POST",
            "client/step4-client/permission",
            json!({"permissionId":"step4-p2"}),
            Some("step4-admin:fixture-admin-password")
        )
    );
    assert!((a.0 == 201 && b.0 == 500) || (a.0 == 500 && b.0 == 201));
    assert_eq!(count(&db,"SELECT count(*) AS n FROM client_permission WHERE client_id='step4-client' AND permission_id='step4-p2'").await,1);
    admin(
        &app,
        "DELETE",
        "client/step4-client/permission/step4-p2",
        Value::Null,
        200,
    )
    .await;
    admin(
        &app,
        "DELETE",
        "client/step4-missing/permission/step4-missing",
        Value::Null,
        200,
    )
    .await;
    admin(&app, "DELETE", "permission/step4-p1", Value::Null, 200).await;
    assert_eq!(
        count(
            &db,
            "SELECT count(*) AS n FROM client_permission WHERE permission_id='step4-p1'"
        )
        .await,
        0
    );
    // Actual FK rejection rolls back earlier work in the same transaction.
    let tx = conn.begin().await.unwrap();
    tx.execute_unprepared(
        "INSERT INTO permission(id,name) VALUES ('step4-rollback','Step4 rollback')",
    )
    .await
    .unwrap();
    assert!(
        tx.execute_unprepared(
            "INSERT INTO client_permission VALUES ('step4-missing','step4-rollback')"
        )
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
    assert!(
        permission::Entity::find_by_id("step4-rollback")
            .one(conn)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        conn.execute_unprepared(
            "INSERT INTO client_permission VALUES ('step4-client','step4-missing')"
        )
        .await
        .is_err()
    );
    assert!(
        conn.execute_unprepared("INSERT INTO client_permission VALUES ('step4-client','ADMIN')")
            .await
            .is_err()
    );
    admin(&app, "DELETE", "client/step4-client", Value::Null, 200).await;
    assert_eq!(
        count(
            &db,
            "SELECT count(*) AS n FROM client_permission WHERE client_id='step4-client'"
        )
        .await,
        0
    );
    assert_eq!(
        call(
            &app,
            "GET",
            "client",
            Value::Null,
            Some("step4-client:fixture-new")
        )
        .await
        .0,
        401
    );
    let disabled = super::tests::app(db.clone(), true);
    assert_eq!(
        call(&disabled, "GET", "client", Value::Null, None).await.0,
        401
    );
    assert_eq!(count(&db,"SELECT count(*) AS n FROM client_permission cp LEFT JOIN client c ON cp.client_id=c.id LEFT JOIN permission p ON cp.permission_id=p.id WHERE c.id IS NULL OR p.id IS NULL").await,0);
    conn.execute_unprepared("DELETE FROM permission WHERE id LIKE 'step4-%';")
        .await
        .unwrap();
    db.close().await.unwrap();
}
