use super::*;
use crate::{
    auth::AuthConfig,
    persistence::{config as entity, session},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use http_body_util::BodyExt;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait, Set, Statement,
    TransactionTrait,
};
use serde_json::Value;
use summer_web::axum::{Router as HttpRouter, body::Body, http::Request};
use tower::ServiceExt;

const ADMIN: &str = "step5-admin:fixture-password";

fn test_app(db: Persistence, disabled: bool) -> HttpRouter {
    let auth = Auth::new(
        db.clone(),
        AuthConfig {
            client_security_disabled: disabled,
            admin_username: "step5-admin".into(),
            admin_password: "fixture-password".into(),
        },
    );
    HttpRouter::new().nest("/ws/rest", crate::api::router(db, None, auth).into())
}

async fn call(
    app: &HttpRouter,
    method: &str,
    path: &str,
    body: Value,
    credential: Option<&str>,
    key: Option<&str>,
) -> (u16, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/ws/rest/v1/{path}"))
        .header("content-type", "application/json");
    if let Some(credential) = credential {
        request = request.header(
            "authorization",
            format!("Basic {}", STANDARD.encode(credential)),
        );
    }
    if let Some(key) = key {
        request = request.header("session-key", key);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn expect(
    app: &HttpRouter,
    method: &str,
    path: &str,
    body: Value,
    credential: Option<&str>,
    status: u16,
) -> Value {
    let (actual, value) = call(app, method, path, body, credential, None).await;
    assert_eq!(actual, status, "{method} {path}: {value}");
    value
}

async fn fixture_session(
    db: &Persistence,
    id: &str,
    permissions: Option<Vec<String>>,
    key: Option<String>,
    expired: bool,
) {
    session::ActiveModel {
        id: Set(id.into()),
        studies: Set("[]".into()),
        anonymized: Set(false),
        shared_session: Set(key.is_some()),
        share_key: Set(key),
        expired_time: Set((chrono::Utc::now()
            + chrono::Duration::minutes(if expired { -5 } else { 60 }))
        .into()),
        permissions: Set(permissions),
        ..Default::default()
    }
    .insert(db.connection())
    .await
    .unwrap();
}

async fn stored(db: &Persistence, id: &str) -> entity::Model {
    entity::Entity::find_by_id(id)
        .one(db.connection())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL; included by documented PostgreSQL regression filter"]
async fn postgres_migrations_transaction_readiness_and_close_config() {
    let db = Persistence::connect_from_env().await.unwrap();
    let conn = db.connection();
    conn.execute_unprepared(
        "DELETE FROM config WHERE id LIKE 'step5-%'; DELETE FROM session WHERE id LIKE 'step5-%';",
    )
    .await
    .unwrap();
    for (id, permissions, key, expired) in [
        (
            "step5-edit",
            Some(vec![MODIFY_OVERLAY_CONFIG.into()]),
            None,
            false,
        ),
        ("step5-bypass", Some(vec!["ADMIN".into()]), None, false),
        ("step5-read", Some(vec!["OTHER".into()]), None, false),
        ("step5-null", None, None, false),
        ("step5-empty", Some(vec![]), None, false),
        (
            "step5-shared",
            Some(vec![MODIFY_OVERLAY_CONFIG.into()]),
            Some("fixture-key".into()),
            false,
        ),
        (
            "step5-expired",
            Some(vec![MODIFY_OVERLAY_CONFIG.into()]),
            None,
            true,
        ),
    ] {
        fixture_session(&db, id, permissions, key, expired).await;
    }
    let app = test_app(db.clone(), false);

    // Both route families share CRUD state and the Java response contract.
    for (prefix, credential, id) in [
        ("admin/config", Some(ADMIN), "step5-admin-config"),
        ("session/step5-edit/config", None, "step5-session-config"),
    ] {
        let input = json!({"id":id,"value":" \t fixture \n", "description":"  note  "});
        let value = expect(&app, "POST", prefix, input, credential, 201).await;
        assert_eq!(value["header"]["code"], 201);
        assert_eq!(value["header"]["message"], "Created");
        assert_eq!(value["body"], id);
        assert!(value["header"]["datetime"].is_string());
        let path = format!("{prefix}/{id}");
        let value = expect(&app, "GET", &path, Value::Null, credential, 200).await;
        assert_eq!(value["header"]["message"], "Success");
        assert_eq!(
            value["body"],
            json!({"id":id,"value":"fixture","description":"note","datatype":"TEXT"})
        );
        assert_eq!(stored(&db, id).await.value, "fixture");

        let merged = json!({"id":id,"value":"replacement", "datatype":"INTEGER"});
        expect(&app, "POST", prefix, merged, credential, 201).await;
        let value = expect(&app, "GET", &path, Value::Null, credential, 200).await;
        assert_eq!(value["body"]["value"], "replacement");
        assert_eq!(value["body"]["description"], Value::Null);
        assert_eq!(value["body"]["datatype"], "INTEGER");
        assert_eq!(stored(&db, id).await.description, None);

        let value = expect(
            &app,
            "PUT",
            prefix,
            json!({"id":id,"value":"  raw  ","description":" ","datatype":"custom"}),
            credential,
            200,
        )
        .await;
        assert_eq!(value["header"]["message"], "Updated");
        assert_eq!(value["body"], id);
        expect(
            &app,
            "PUT",
            prefix,
            json!({"id":id,"value":null,"description":null,"datatype":null}),
            credential,
            200,
        )
        .await;
        let value = expect(&app, "GET", &path, Value::Null, credential, 200).await;
        assert_eq!(value["body"]["value"], "  raw  ");
        assert_eq!(value["body"]["description"], " ");
        assert_eq!(value["body"]["datatype"], "custom");
        let before = stored(&db, id).await;
        let failure = expect(
            &app,
            "PUT",
            prefix,
            json!({"id":id,"value":"must rollback","datatype":"x".repeat(33)}),
            credential,
            500,
        )
        .await;
        assert_eq!(failure["errorCode"], "INTERNAL_ERROR");
        assert_eq!(stored(&db, id).await, before);
        assert_eq!(
            expect(&app, "GET", &path, Value::Null, credential, 200).await["body"]["value"],
            "  raw  "
        );

        let value = expect(&app, "DELETE", &path, Value::Null, credential, 200).await;
        assert_eq!(value["header"]["message"], "Deleted");
        assert_eq!(value["body"], id);
        assert!(
            entity::Entity::find_by_id(id)
                .one(conn)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            expect(&app, "GET", &path, Value::Null, credential, 404).await,
            Value::Null
        );
        let value = expect(&app, "DELETE", &path, Value::Null, credential, 404).await;
        assert_eq!(value["errorCode"], "NOT_FOUND");
        assert_eq!(value["message"], format!("Entity not found with id: {id}"));
        assert_eq!(
            expect(
                &app,
                "PUT",
                prefix,
                json!({"id":id,"value":"absent"}),
                credential,
                404
            )
            .await,
            Value::Null
        );
    }

    // Validation is create-only; update accepts blank strings and arbitrary datatype.
    for prefix in ["admin/config", "session/step5-edit/config"] {
        let credential = prefix.starts_with("admin").then_some(ADMIN);
        for body in [
            json!({}),
            json!({"id":"step5-invalid"}),
            json!({"value":"v"}),
            json!({"id":null,"value":"v"}),
            json!({"id":"step5-invalid","value":null}),
            json!({"id":"step5-invalid","value":" \t "}),
        ] {
            let value = expect(&app, "POST", prefix, body, credential, 400).await;
            assert_eq!(value["errorCode"], "VALIDATION_ERROR");
            assert!(value["timestamp"].is_string());
        }
        for body in [json!({}), json!({"id":null,"value":"v"})] {
            assert_eq!(
                expect(&app, "PUT", prefix, body, credential, 400).await["errorCode"],
                "BAD_REQUEST"
            );
        }
        expect(
            &app,
            "POST",
            prefix,
            json!({"id":"step5-invalid","value":"v","datatype":"text"}),
            credential,
            500,
        )
        .await;
        expect(
            &app,
            "POST",
            prefix,
            json!({"id":"x".repeat(65),"value":"v"}),
            credential,
            500,
        )
        .await;
        assert!(
            entity::Entity::find_by_id("step5-invalid")
                .one(conn)
                .await
                .unwrap()
                .is_none()
        );
    }
    for datatype in ["TEXT", "INTEGER", "LIST"] {
        expect(&app, "POST", "admin/config", json!({"id":"step5-types","value":"not type checked","datatype":datatype,"description":"\u{a0}"}), Some(ADMIN), 201).await;
        assert_eq!(
            stored(&db, "step5-types").await.datatype.as_deref(),
            Some(datatype)
        );
        assert_eq!(
            stored(&db, "step5-types").await.description.as_deref(),
            Some("\u{a0}")
        );
    }
    expect(
        &app,
        "POST",
        "admin/config",
        json!({"id":"step5-types","value":"v","datatype":null,"description":" \t "}),
        Some(ADMIN),
        201,
    )
    .await;
    assert_eq!(stored(&db, "step5-types").await.description, None);
    assert_eq!(
        stored(&db, "step5-types").await.datatype.as_deref(),
        Some("TEXT")
    );
    expect(
        &app,
        "PUT",
        "admin/config",
        json!({"id":"step5-types","value":"","description":"","datatype":""}),
        Some(ADMIN),
        200,
    )
    .await;
    assert_eq!(stored(&db, "step5-types").await.value, "");
    for id in ["", " "] {
        expect(
            &app,
            "POST",
            "admin/config",
            json!({"id":id,"value":"v"}),
            Some(ADMIN),
            201,
        )
        .await;
        assert_eq!(stored(&db, id).await.id, id);
        entity::Entity::delete_by_id(id).exec(conn).await.unwrap();
    }

    // Every admin operation is protected; ordinary clients cannot use ADMIN permission to enter.
    conn.execute_unprepared("INSERT INTO client (id,password) VALUES ('step5-client','fixture-unused'); INSERT INTO client_permission (client_id,permission_id) VALUES ('step5-client','ADMIN');").await.unwrap();
    let hash = bcrypt::hash("fixture-client-password", 8).unwrap();
    conn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE client SET password=$1 WHERE id='step5-client'",
        [hash.into()],
    ))
    .await
    .unwrap();
    for (method, path) in [
        ("GET", "admin/config"),
        ("GET", "admin/config/step5-types"),
        ("POST", "admin/config"),
        ("PUT", "admin/config"),
        ("DELETE", "admin/config/step5-types"),
    ] {
        expect(
            &app,
            method,
            path,
            json!({"id":"step5-denied","value":"v"}),
            None,
            401,
        )
        .await;
        expect(
            &app,
            method,
            path,
            json!({"id":"step5-denied","value":"v"}),
            Some("step5-client:fixture-client-password"),
            403,
        )
        .await;
    }
    for session_id in ["step5-read", "step5-null", "step5-empty"] {
        let path = format!("session/{session_id}/config");
        expect(&app, "GET", &path, Value::Null, None, 200).await;
        for method in ["POST", "PUT"] {
            expect(
                &app,
                method,
                &format!("{path}?sessionId=step5-bypass"),
                json!({"id":"step5-denied","value":"denied"}),
                None,
                401,
            )
            .await;
            expect(
                &app,
                method,
                &path,
                json!({"id":"step5-denied","value":"denied"}),
                Some(ADMIN),
                401,
            )
            .await;
        }
    }
    assert!(
        entity::Entity::find_by_id("step5-denied")
            .one(conn)
            .await
            .unwrap()
            .is_none()
    );
    for method in ["POST", "PUT"] {
        expect(
            &app,
            method,
            "session/step5-bypass/config",
            json!({"id":"step5-admin-permission","value":"v"}),
            None,
            if method == "POST" { 201 } else { 200 },
        )
        .await;
    }
    // DELETE deliberately follows Java: a valid read-only session may delete.
    expect(
        &app,
        "DELETE",
        "session/step5-read/config/step5-admin-permission",
        Value::Null,
        None,
        200,
    )
    .await;
    expect(
        &app,
        "GET",
        "admin/config/step5-admin-permission",
        Value::Null,
        Some(ADMIN),
        404,
    )
    .await;
    expect(
        &app,
        "GET",
        "session/step5-read/config/step5-types",
        Value::Null,
        None,
        200,
    )
    .await;
    for id in ["step5-absent", "step5-expired"] {
        for (method, suffix) in [
            ("GET", ""),
            ("GET", "/step5-types"),
            ("POST", ""),
            ("PUT", ""),
            ("DELETE", "/step5-types"),
        ] {
            expect(
                &app,
                method,
                &format!("session/{id}/config{suffix}"),
                json!({"id":"step5-types","value":"denied"}),
                None,
                401,
            )
            .await;
        }
    }
    for (key, status) in [
        (None, 401),
        (Some("wrong"), 401),
        (Some("fixture-key"), 201),
    ] {
        assert_eq!(
            call(
                &app,
                "POST",
                "session/step5-shared/config",
                json!({"id":"step5-shared-config","value":"v"}),
                None,
                key
            )
            .await
            .0,
            status
        );
    }
    let disabled = test_app(db.clone(), true);
    expect(
        &disabled,
        "POST",
        "session/step5-read/config",
        json!({"id":"step5-disabled","value":"v"}),
        None,
        201,
    )
    .await;
    expect(
        &disabled,
        "POST",
        "session/step5-absent/config",
        json!({"id":"step5-disabled","value":"denied"}),
        None,
        401,
    )
    .await;
    expect(&disabled, "GET", "admin/config", Value::Null, None, 401).await;

    // Cross-route invalidation: both families must use the same cache.
    expect(
        &app,
        "POST",
        "admin/config",
        json!({"id":"step5-cross-cache","value":"admin"}),
        Some(ADMIN),
        201,
    )
    .await;
    expect(
        &app,
        "GET",
        "admin/config/step5-cross-cache",
        Value::Null,
        Some(ADMIN),
        200,
    )
    .await;
    expect(
        &app,
        "PUT",
        "session/step5-edit/config",
        json!({"id":"step5-cross-cache","value":"session"}),
        None,
        200,
    )
    .await;
    assert_eq!(
        expect(
            &app,
            "GET",
            "admin/config/step5-cross-cache",
            Value::Null,
            Some(ADMIN),
            200
        )
        .await["body"]["value"],
        "session"
    );
    expect(
        &app,
        "GET",
        "session/step5-read/config/step5-cross-cache",
        Value::Null,
        None,
        200,
    )
    .await;
    expect(
        &app,
        "POST",
        "admin/config",
        json!({"id":"step5-cross-cache","value":"merged"}),
        Some(ADMIN),
        201,
    )
    .await;
    assert_eq!(
        expect(
            &app,
            "GET",
            "session/step5-read/config/step5-cross-cache",
            Value::Null,
            None,
            200
        )
        .await["body"]["value"],
        "merged"
    );
    expect(
        &app,
        "DELETE",
        "session/step5-read/config/step5-cross-cache",
        Value::Null,
        None,
        200,
    )
    .await;
    expect(
        &app,
        "GET",
        "admin/config/step5-cross-cache",
        Value::Null,
        Some(ADMIN),
        404,
    )
    .await;

    // Pagination uses Java page rounding; list never returns cached entities.
    let total = entity::Entity::find().count(conn).await.unwrap();
    for prefix in ["admin/config", "session/step5-read/config"] {
        let credential = prefix.starts_with("admin").then_some(ADMIN);
        for query in [
            "",
            "?offset=3&limit=2&orderBy=ignored",
            "?offset=-4&limit=2",
            "?offset=99999&limit=2",
        ] {
            let value = expect(
                &app,
                "GET",
                &format!("{prefix}{query}"),
                Value::Null,
                credential,
                200,
            )
            .await;
            assert_eq!(value["header"]["code"], 302);
            assert_eq!(value["header"]["totalRecords"], total);
            let n = value["body"].as_array().unwrap().len();
            assert_eq!(value["header"]["message"], format!("Found {n} configs"));
            if query.contains("99999") {
                assert_eq!(n, 0);
            }
            if query.contains("offset=3") {
                assert_eq!(n as u64, total.saturating_sub(2).min(2));
                assert_eq!(value["header"]["offset"], 3);
                assert_eq!(value["header"]["limit"], 2);
            }
            if query.contains("offset=-4") {
                assert_eq!(n as u64, total.min(2));
                assert_eq!(value["header"]["offset"], -4);
            }
            if query.is_empty() {
                assert_eq!(value["header"]["offset"], 0);
                assert_eq!(value["header"]["limit"], 100);
            }
        }
        for limit in [0, -1] {
            expect(
                &app,
                "GET",
                &format!("{prefix}?limit={limit}"),
                Value::Null,
                credential,
                400,
            )
            .await;
        }
    }

    // Real cache hit, no negative caching, invalidation and write failure postconditions.
    let store = ConfigStore::new(db.clone());
    let row = entity::Model {
        id: "step5-cache".into(),
        value: "first".into(),
        description: None,
        datatype: None,
    };
    store.create(row.clone()).await.unwrap();
    conn.execute_unprepared("UPDATE config SET value='external' WHERE id='step5-cache'")
        .await
        .unwrap();
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "first");
    assert_eq!(
        store
            .list(0, 100)
            .await
            .unwrap()
            .1
            .into_iter()
            .find(|m| m.id == row.id)
            .unwrap()
            .value,
        "external"
    );
    let fresh = ConfigStore::new(db.clone());
    assert_eq!(fresh.get(&row.id).await.unwrap().unwrap().value, "external");
    conn.execute_unprepared("UPDATE config SET value='external2' WHERE id='step5-cache'")
        .await
        .unwrap();
    assert_eq!(fresh.get(&row.id).await.unwrap().unwrap().value, "external");
    store
        .update(&row.id, Some("updated".into()), None, None)
        .await
        .unwrap();
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "updated");
    assert!(
        store
            .update(&row.id, Some("bad".into()), None, Some("x".repeat(33)))
            .await
            .is_err()
    );
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "updated");
    let mut invalid_row = row.clone();
    invalid_row.datatype = Some("x".repeat(33));
    assert!(store.create(invalid_row).await.is_err());
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "updated");
    store.create(row.clone()).await.unwrap();
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "first");
    assert!(store.delete(&row.id).await.unwrap());
    assert!(store.get(&row.id).await.unwrap().is_none());
    // Null misses are not cached: another repository's create becomes visible.
    fresh.create(row.clone()).await.unwrap();
    assert_eq!(store.get(&row.id).await.unwrap().unwrap(), row);
    assert_eq!(
        store
            .list(0, 100)
            .await
            .unwrap()
            .1
            .iter()
            .find(|m| m.id == row.id)
            .unwrap(),
        &row
    );

    let (a, b) = tokio::join!(
        store.update(&row.id, Some("parallel".into()), None, None),
        store.update(&row.id, None, Some("parallel note".into()), None)
    );
    assert!(a.unwrap() && b.unwrap());
    let final_row = stored(&db, &row.id).await;
    assert_eq!(final_row.value, "parallel");
    assert_eq!(final_row.description.as_deref(), Some("parallel note"));
    assert_eq!(store.get(&row.id).await.unwrap().unwrap(), final_row);
    let readers = (0..16)
        .map(|_| {
            let store = store.clone();
            tokio::spawn(async move { store.get("step5-cache").await.unwrap() })
        })
        .collect::<Vec<_>>();
    store
        .update(&row.id, Some("final".into()), None, None)
        .await
        .unwrap();
    for task in readers {
        task.await.unwrap();
    }
    assert_eq!(store.get(&row.id).await.unwrap().unwrap().value, "final");
    assert_eq!(stored(&db, &row.id).await.value, "final");

    // Migration enforces constraints and rollback on the real database.
    for sql in [
        "INSERT INTO config (id,value) VALUES ('step5-cache','duplicate')",
        "INSERT INTO config (id,value) VALUES ('step5-null-value',NULL)",
    ] {
        let tx = conn.begin().await.unwrap();
        assert!(tx.execute_unprepared(sql).await.is_err());
        tx.rollback().await.unwrap();
    }
    let tx = conn.begin().await.unwrap();
    tx.execute_unprepared("UPDATE config SET value='rolled back' WHERE id='step5-cache'")
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(stored(&db, &row.id).await.value, "final");
    conn.execute_unprepared("DELETE FROM config WHERE id LIKE 'step5-%'; DELETE FROM session WHERE id LIKE 'step5-%'; DELETE FROM client WHERE id='step5-client';").await.unwrap();
    db.close().await.unwrap();
}
