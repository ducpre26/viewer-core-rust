use super::*;
use crate::{auth::AuthConfig, persistence::session};
use base64::Engine;
use http_body_util::BodyExt;
use sea_orm::{PaginatorTrait, TransactionTrait};
use summer_web::axum::{Router as HttpRouter, http::Request as HttpRequest};
use tower::ServiceExt;

fn metadata(study: &str, sop: &str) -> Value {
    json!({"0020000D":{"vr":"UI","Value":[study]},"0020000E":{"vr":"UI","Value":["1.2.3.4"]},"00080018":{"vr":"UI","Value":[sop]},"00080016":{"vr":"UI","Value":["1.2.840.10008.5.1.4.1.1.7"]}})
}
async fn call(
    app: &HttpRouter,
    method: &str,
    path: &str,
    content_type: &str,
    body: impl Into<Body>,
    key: Option<&str>,
) -> (u16, Value) {
    let mut req = HttpRequest::builder()
        .method(method)
        .uri(format!("/ws/rest{path}"))
        .header("content-type", content_type)
        .header("host", "localhost");
    if let Some(key) = key {
        req = req.header("session-key", key)
    }
    let res = app
        .clone()
        .oneshot(req.body(body.into()).unwrap())
        .await
        .unwrap();
    let status = res.status().as_u16();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}
fn app(runtime: KeyImageRuntime) -> HttpRouter {
    HttpRouter::new().nest(
        "/ws/rest",
        crate::api::router(
            runtime.database.clone(),
            runtime.jvm.clone(),
            runtime.auth.clone(),
            super::super::SessionConfig::default(),
        )
        .layer(Extension(runtime))
        .into(),
    )
}
#[test]
fn key_image_settings_and_containment() {
    let mut settings = KeyImageSettings::default();
    assert!(settings.validate().is_ok());
    settings.interval_ms = 0;
    assert!(settings.validate().is_err());
    for name in ["../x", "a/b", "a\\b", "C:x", ".", "..", ""] {
        assert!(series_path(FsPath::new("files"), name).is_err());
    }
    assert_eq!(
        series_path(FsPath::new("files"), "1.2.3").unwrap(),
        FsPath::new("files/1.2.3")
    );
}
#[tokio::test]
async fn multipart_skips_unreadable_file_keeps_metadata() {
    let root = tempfile::tempdir().unwrap();
    let meta = root.path().join("metadata.json");
    fs::write(&meta, b"[]").unwrap();
    let parts = stow_stream(vec![root.path().join("missing.dcm"), meta])
        .collect::<Vec<_>>()
        .await;
    let bytes: Vec<_> = parts.into_iter().flat_map(Result::unwrap).collect();
    assert_eq!(
        bytes,
        b"\r\n--myboundary\r\nContent-Type: application/dicom\r\n\r\n[]\r\n--myboundary--\r\n"
    );
}
#[tokio::test]
#[ignore = "requires PostgreSQL, packaged JVM, and local Postman simulation; tools/smoke_key_image.py"]
async fn postgres_key_image_contract() {
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    assert_eq!(
        entity::Entity::find().count(db.connection()).await.unwrap(),
        0,
        "requires empty dedicated STUDY_KEY_IMAGE table; never dispatch pre-existing batches"
    );
    let transaction = db.connection().begin().await.unwrap();
    let constraint_id = format!("step13-constraint-{}", uuid::Uuid::new_v4());
    let row = entity::ActiveModel {
        id: Set(constraint_id.clone()),
        series_iuid: Set("1.2.3.4".into()),
        store_url: Set("http://fixture.invalid".into()),
        created_time: Set(chrono::Utc::now().into()),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .unwrap();
    assert_eq!(
        (
            row.num_of_image,
            row.num_of_retry,
            row.succeed,
            row.creator_id
        ),
        (0, 0, false, 1)
    );
    transaction.rollback().await.unwrap();
    assert!(
        entity::Entity::find_by_id(&constraint_id)
            .one(db.connection())
            .await
            .unwrap()
            .is_none()
    );
    let transaction = db.connection().begin().await.unwrap();
    assert!(
        entity::ActiveModel {
            id: Set(constraint_id),
            created_time: Set(chrono::Utc::now().into()),
            ..Default::default()
        }
        .insert(&transaction)
        .await
        .is_err(),
        "DDL must enforce required series/store columns"
    );
    transaction.rollback().await.unwrap();
    let jvm = Jvm::start(crate::config::jni_for_tests().validate().unwrap().unwrap())
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let store = std::env::var("KEY_IMAGE_TEST_STORE").expect("local simulation URL");
    let id = format!("step13-{}", uuid::Uuid::new_v4());
    let study = format!("2.25.{}", uuid::Uuid::new_v4().as_u128());
    auth.cache_session(session::Model {
        id: id.clone(),
        store_url: Some(store.clone()),
        store_auth: Some("Basic fixture-secret".into()),
        studies: study.clone(),
        user_id: None,
        anonymized: false,
        shared_session: false,
        share_key: Some("test-key".into()),
        expired_time: (chrono::Utc::now() + chrono::Duration::hours(1)).into(),
        permissions: None,
    })
    .await;
    let settings = KeyImageSettings {
        dicom_path: root.path().join("files"),
        temp_directory: root.path().join("spool"),
        ..Default::default()
    };
    // Original Java calls File.setWritable on newly created folders, which can fail
    // under Windows ACLs. Existing directory fixture isolates the DICOM behavior.
    fs::create_dir_all(settings.dicom_path.join("1.2.3.4")).unwrap();
    let runtime = KeyImageRuntime::new(
        settings.clone(),
        db.clone(),
        auth.clone(),
        Some(jvm.clone()),
    )
    .unwrap();
    let baseline: Value = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-13-java.json"
    ))
    .unwrap();
    for row in baseline.as_array().unwrap() {
        let kind = row["case"].as_str().unwrap();
        let mut runtime = runtime.clone();
        if kind == "maximum" {
            runtime.settings.max_file = 0;
        }
        let app = app(runtime.clone());
        let content_type = if kind == "missing-boundary" {
            "multipart/related"
        } else {
            "multipart/related; boundary=b"
        };
        let raw = base64::engine::general_purpose::STANDARD
            .decode(row["requestBase64"].as_str().unwrap())
            .unwrap();
        let needle = b"\"1.2.3\"";
        let replacement = format!("\"{study}\"");
        let mut request = Vec::new();
        let mut index = 0;
        while index < raw.len() {
            if raw[index..].starts_with(needle) {
                request.extend_from_slice(replacement.as_bytes());
                index += needle.len();
            } else {
                request.push(raw[index]);
                index += 1;
            }
        }
        let path = format!("/v1/session/{id}/studies");
        let (status, body) =
            call(&app, "POST", &path, content_type, request, Some("test-key")).await;
        assert_eq!(
            status,
            row["status"].as_u64().unwrap() as u16,
            "{kind}: {body}"
        );
        if status == 200 {
            if let Some(artifacts) = std::env::var_os("KEY_IMAGE_TEST_ARTIFACTS") {
                let target = PathBuf::from(artifacts).join(kind);
                fs::create_dir_all(&target).unwrap();
                for entry in fs::read_dir(settings.dicom_path.join("1.2.3.4")).unwrap() {
                    let path = entry.unwrap().path();
                    if path.extension().is_some_and(|e| e == "dcm") {
                        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
                    }
                }
            }

            if kind == "jpeg" {
                let response=app.clone().oneshot(HttpRequest::builder().uri(format!("/ws/rest/v1/session/{id}/wado-rs/studies/{study}/series/1.2.3.4/instances/1.2.3.4.5/frames/999")).header("session-key","test-key").body(Body::empty()).unwrap()).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(
                    response.headers()[header::CONTENT_TYPE],
                    "multipart/related; boundary=myboundary"
                );
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                assert!(bytes.starts_with(b"--myboundary\r\n"));
                assert!(bytes.windows(2).any(|w| w == [255, 216]));
                if let Some(artifacts) = std::env::var_os("KEY_IMAGE_TEST_ARTIFACTS") {
                    fs::write(
                        PathBuf::from(artifacts).join(kind).join("frame.bin"),
                        &bytes,
                    )
                    .unwrap();
                }
            }
            let mut expected: Value = serde_json::from_str(row["body"].as_str().unwrap()).unwrap();
            // Replace only synthetic source study/session identities; no output fields omitted.
            expected = serde_json::from_str(
                &expected
                    .to_string()
                    .replace(
                        "http://localhost/ws/rest/v1/session/s/",
                        &format!("http://localhost/ws/rest/v1/session/{id}/"),
                    )
                    .replace("studies/1.2.3/", &format!("studies/{study}/"))
                    .replace("\"1.2.3\"", &format!("\"{study}\"")),
            )
            .unwrap();
            assert_eq!(body, expected, "{kind}");
        } else {
            assert_eq!(body["errorCode"], row["errorCode"], "{kind}: {body}");
        }
        let pending = entity::Entity::find_by_id(&study)
            .one(db.connection())
            .await
            .unwrap();
        let expected = row["pending"].as_array().unwrap();
        assert_eq!(pending.is_some(), !expected.is_empty(), "{kind}");
        if let Some(p) = pending {
            assert_eq!(
                p.num_of_image,
                expected[0]["count"].as_i64().unwrap() as i32
            );
            assert_eq!(p.num_of_retry, 1);
            assert_eq!(p.series_iuid, "1.2.3.4");
        }
        let mut files = Vec::new();
        regular_files(root.path(), &mut files).unwrap();
        let mut actual: Vec<_> = files
            .iter()
            .map(|p| {
                p.strip_prefix(root.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        actual.sort();
        let expected: Vec<_> = row["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(actual, expected, "{kind}");
        entity::Entity::delete_by_id(&study)
            .exec(db.connection())
            .await
            .unwrap();
        let _ = fs::remove_dir_all(&settings.dicom_path);
        fs::create_dir_all(settings.dicom_path.join("1.2.3.4")).unwrap();
    }
    let app = app(runtime.clone());
    let path = format!("/v1/session/{id}/studies");
    assert_eq!(
        call(&app, "POST", &path, "multipart/related", "", None)
            .await
            .0,
        401
    );
    let body = format!(
        "--b\r\nContent-Type: application/dicom+json\r\n\r\n[{}]\r\n--b--\r\n",
        metadata(&study, "1.2.3.4.5")
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &path,
            "multipart/related; boundary=b",
            body.clone(),
            Some("test-key")
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &path,
            "multipart/related; boundary=b",
            body.clone(),
            Some("test-key")
        )
        .await
        .0,
        200
    );
    let row = entity::Entity::find_by_id(&study)
        .one(db.connection())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.num_of_image, 2);
    assert_eq!(row.description.as_deref(), Some("Send with new key image"));
    // Local series and metadata are visible before dispatch (upstream returns []).
    for suffix in ["/series", "/series/1.2.3.4/metadata"] {
        let (status, body) = call(
            &app,
            "GET",
            &format!("/v1/session/{id}/wado-rs/studies/{study}{suffix}"),
            "application/json",
            String::new(),
            Some("test-key"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(!body.as_array().unwrap().is_empty(), "{suffix}: {body}");
    }
    runtime.dispatch_once().await.unwrap();
    assert!(
        entity::Entity::find_by_id(&study)
            .one(db.connection())
            .await
            .unwrap()
            .is_some(),
        "disabled export must retain row"
    );
    let mut enabled = runtime.clone();
    enabled.settings.send_to_store = true;
    let client = reqwest::Client::new();
    client
        .get(format!("{store}/__control?status=503"))
        .send()
        .await
        .unwrap();
    for retry in [2, 3, 4] {
        enabled.dispatch_once().await.unwrap();
        assert_eq!(
            entity::Entity::find_by_id(&study)
                .one(db.connection())
                .await
                .unwrap()
                .unwrap()
                .num_of_retry,
            retry
        );
    }
    enabled.dispatch_once().await.unwrap();
    assert_eq!(
        entity::Entity::find_by_id(&study)
            .one(db.connection())
            .await
            .unwrap()
            .unwrap()
            .num_of_retry,
        4
    );
    // New ingest resets retry, retaining destination and series.
    assert_eq!(
        call(
            &app,
            "POST",
            &path,
            "multipart/related; boundary=b",
            body,
            Some("test-key")
        )
        .await
        .0,
        200
    );
    client
        .get(format!("{store}/__control?status=202"))
        .send()
        .await
        .unwrap();
    enabled.dispatch_once().await.unwrap();
    assert!(
        entity::Entity::find_by_id(&study)
            .one(db.connection())
            .await
            .unwrap()
            .is_none()
    );
    assert!(!settings.dicom_path.join("1.2.3.4").exists());
    let observed: Value = client
        .get(format!("{store}/__control"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .map(|v| serde_json::from_str(&v).unwrap())
        .unwrap();
    assert!(
        observed["requests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["metadata"] == true
                && r["dicom"] == true
                && r["authorization"] == "Basic fixture-secret")
    );
    // Query limit and retry predicate tested against real PostgreSQL, independent of HTTP.
    let mut ids = Vec::new();
    for n in 0..102 {
        let folder = settings.dicom_path.join(format!("1.2.3.4.{n}"));
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("metadata.json"), b"[]").unwrap();
        let id = format!("{study}.{n}");
        ids.push(id.clone());
        entity::ActiveModel {
            id: Set(id),
            series_iuid: Set(format!("1.2.3.4.{n}")),
            num_of_image: Set(0),
            store_url: Set(store.clone()),
            store_auth: Set(None),
            succeed: Set(false),
            num_of_retry: Set(if n == 101 { 4 } else { 3 }),
            description: Set(None),
            created_time: Set(chrono::Utc::now().into()),
            creator_id: Set(1),
            last_updated_time: Set(None),
            last_updated_id: Set(None),
        }
        .insert(db.connection())
        .await
        .unwrap();
    }
    let rows = entity::pending(db.connection()).await.unwrap();
    assert_eq!(rows.len(), 100);
    assert!(rows.iter().all(|r| r.num_of_retry <= 3));
    let before: Value = client
        .get(format!("{store}/__control"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .map(|v| serde_json::from_str(&v).unwrap())
        .unwrap();
    enabled.dispatch_once().await.unwrap();
    let after: Value = client
        .get(format!("{store}/__control"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .map(|v| serde_json::from_str(&v).unwrap())
        .unwrap();
    assert_eq!(
        after["requests"].as_array().unwrap().len() - before["requests"].as_array().unwrap().len(),
        100
    );
    let mut remaining = 0;
    for id in &ids {
        if entity::Entity::find_by_id(id)
            .one(db.connection())
            .await
            .unwrap()
            .is_some()
        {
            remaining += 1;
        }
    }
    assert_eq!(remaining, 2);
    for id in ids {
        entity::Entity::delete_by_id(id)
            .exec(db.connection())
            .await
            .unwrap();
    }
    jvm.shutdown().await.unwrap();
    db.close().await.unwrap();
}
