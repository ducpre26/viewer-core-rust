use super::*;
use crate::{
    auth::AuthConfig,
    persistence::{Persistence, session},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use summer_web::axum::{Router as HttpRouter, body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn recorded_java_contract() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-10-java.json"
    ))
    .unwrap();
    let filter: Filter =
        serde_json::from_value(json!({"sessionID":"s","patientID":"","startDate":"20260101"}))
            .unwrap();
    let paging = Paging {
        offset: -7,
        limit: 0,
    };
    for fixture in fixtures {
        assert_eq!(
            json!(filter.query(&paging)),
            fixture["query"],
            "{}",
            fixture["name"]
        );
        let rows = datasets(fixture["input"].as_str().unwrap().as_bytes());
        let response = paging.response("studies", rows.len() as u64, rows);
        assert_eq!(
            response.status().as_u16(),
            fixture["status"].as_u64().unwrap() as u16
        );
        let mut body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        body["header"].as_object_mut().unwrap().remove("datetime");
        assert_eq!(body, fixture["body"], "{}", fixture["name"]);
    }
}

#[test]
fn filters_validation_and_pagination() {
    for body in [
        json!({}),
        json!({"sessionID":"","patientID":"p"}),
        json!({"sessionID":"s"}),
    ] {
        let filter: Filter = serde_json::from_value(body).unwrap();
        assert!(filter.validate().is_err());
    }
    let filter: Filter = serde_json::from_value(json!({
        "sessionID":"s", "patientID":"", "patientName":"Fixture^A & B",
        "accessionNumber":"a", "modalityTypes":"CT,MR", "studyInstanceUID":"1.2.3",
        "startDate":"20260101", "endDate":"20261231"
    }))
    .unwrap();
    assert_eq!(filter.validate().unwrap(), "s");
    for (offset, limit, expected_offset, expected_limit) in [
        (0, 100, "0", "100"),
        (-3, 0, "0", "20"),
        (7, -2, "7", "20"),
        (7, 3, "7", "3"),
    ] {
        let query = filter.query(&Paging { offset, limit });
        assert_eq!(query["offset"], expected_offset);
        assert_eq!(query["limit"], expected_limit);
        assert_eq!(query["PatientID"], "");
        assert_eq!(query["PatientName"], "Fixture^A & B");
        assert_eq!(query["AccessionNumber"], "a");
        assert_eq!(query["ModalitiesInStudy"], "CT,MR");
        assert_eq!(query["StudyInstanceUID"], "1.2.3");
        assert_eq!(query["StudyDate"], "20260101-20261231");
        assert!(!query.contains_key("orderBy"));
    }
    for (start, end, expected) in [
        (Some("20260101"), None, "20260101-"),
        (None, Some("20261231"), "-20261231"),
    ] {
        let filter: Filter = serde_json::from_value(
            json!({"sessionID":"s","patientID":"p","startDate":start,"endDate":end}),
        )
        .unwrap();
        assert_eq!(
            filter.query(&Paging {
                offset: 0,
                limit: 100
            })["StudyDate"],
            expected
        );
    }
}

#[test]
fn dicom_mapping_empty_invalid_and_multi_value() {
    for bytes in [b"".as_slice(), b"[]", b"not-json", b"[null]"] {
        assert!(datasets(bytes).is_empty());
    }
    let value = json!([{
        "00100020":{"vr":"LO","Value":["P-FIXTURE"]},
        "0020000D":{"vr":"UI","Value":["1.2.3"]},
        "00100010":{"vr":"PN","Value":[{"Alphabetic":"Fixture^A","Ideographic":"Fixture^B","Phonetic":"Fixture^C"}]},
        "00180015":{"vr":"CS","Value":["HEAD"]},
        "00081030":{"vr":"LO","Value":["Example"]},
        "00080020":{"vr":"DA","Value":["20260101"]},
        "00080030":{"vr":"TM","Value":["120000"]},
        "00080061":{"vr":"CS","Value":["CT","MR"]},
        "00080050":{"vr":"SH","Value":["A1"]}
    }, {"00100020":{"vr":"LO"},"00080061":{"vr":"CS","Value":[]}}]);
    assert_eq!(
        datasets(value.to_string().as_bytes()),
        vec![
            json!({
                "pid":"P-FIXTURE","studyInstanceUID":"1.2.3","patientName":"Fixture^A=Fixture^B=Fixture^C",
                "bodyPart":"HEAD","description":"Example","studyDate":"20260101","studyTime":"120000","modalityType":"CT","accessionNumber":"A1"
            }),
            json!({})
        ]
    );
}

// A real local HTTP store captures URL encoding/auth and serves error fixtures.
async fn store(
    status: u16,
    body: &'static str,
    delay: std::time::Duration,
) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dicom", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..n]);
            if request.windows(4).any(|s| s == b"\r\n\r\n") {
                break;
            }
        }
        tokio::time::sleep(delay).await;
        let reply = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/dicom+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes()).await;
        String::from_utf8(request).unwrap()
    });
    (url, task)
}

#[tokio::test]
async fn qido_http_contract_errors_and_timeout() {
    let qido = Qido {
        client: reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_millis(200))
            .build()
            .unwrap(),
    };
    let filter: Filter = serde_json::from_value(
        json!({"sessionID":"s","patientID":"P &+?","patientName":"Fixture^A"}),
    )
    .unwrap();
    let query = filter.query(&Paging {
        offset: -2,
        limit: 0,
    });
    let (url, task) = store(
        200,
        r#"[{"00100020":{"vr":"LO","Value":["P-FIXTURE"]}}]"#,
        std::time::Duration::ZERO,
    )
    .await;
    assert_eq!(
        qido.search(&url, Some("Basic fixture-secret"), &query)
            .await
            .unwrap(),
        vec![json!({"pid":"P-FIXTURE"})]
    );
    let request = task.await.unwrap();
    let path = request
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    let url = url::Url::parse(&format!("http://fixture{path}")).unwrap();
    assert_eq!(url.path(), "/dicom/studies");
    let captured: BTreeMap<_, _> = url
        .query_pairs()
        .map(|(a, b)| (a.into_owned(), b.into_owned()))
        .collect();
    assert_eq!(captured["PatientID"], "P &+?");
    assert_eq!(captured["offset"], "0");
    assert_eq!(captured["limit"], "20");
    assert!(
        request
            .to_lowercase()
            .contains("authorization: basic fixture-secret")
    );
    for (status, body) in [(200, "[]"), (204, ""), (200, "broken")] {
        let (url, task) = store(status, body, std::time::Duration::ZERO).await;
        assert!(qido.search(&url, None, &query).await.unwrap().is_empty());
        assert!(
            !task
                .await
                .unwrap()
                .to_lowercase()
                .contains("authorization:")
        );
    }
    for status in [401, 403, 500] {
        let (url, task) = store(status, "private upstream body", std::time::Duration::ZERO).await;
        let error = qido.search(&url, None, &query).await.unwrap_err();
        assert_eq!(error.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!error.1.contains("private"));
        task.await.unwrap();
    }
    let (url, task) = store(200, "[]", std::time::Duration::from_secs(1)).await;
    assert!(qido.search(&url, None, &query).await.is_err());
    task.abort();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    assert!(qido.search(&closed, None, &query).await.is_err());
    assert!(qido.search("file:///fixture", None, &query).await.is_err());
}

async fn call(app: &HttpRouter, query: &str, body: Value) -> (u16, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/ws/rest/v1/study/search{query}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL TOML fixture"]
async fn postgres_migrations_transaction_readiness_and_close_study_search() {
    use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let router = crate::api::router(
        db.clone(),
        None,
        auth.clone(),
        super::super::SessionConfig::default(),
    );
    let app = HttpRouter::new().nest("/ws/rest", router.into());
    for body in [
        json!({}),
        json!({"sessionID":"","patientID":"p"}),
        json!({"sessionID":"s","patientID":null}),
    ] {
        let (status, body) = call(&app, "", body).await;
        assert_eq!(status, 400);
        assert_eq!(body["errorCode"], "VALIDATION_ERROR");
    }
    assert_eq!(
        call(
            &app,
            "",
            json!({"sessionID":"step10-missing","patientID":"p"})
        )
        .await,
        (204, Value::Null)
    );
    for query in ["?offset=bad", "?limit=2147483648"] {
        let (status, body) =
            call(&app, query, json!({"sessionID":"missing","patientID":"p"})).await;
        assert_eq!(status, 500);
        assert_eq!(body["errorCode"], "INTERNAL_ERROR");
    }
    let id = format!("step10-{}", uuid::Uuid::new_v4());
    for persist in [false, true] {
        let (url, task) = store(
            200,
            r#"[{"00100020":{"vr":"LO","Value":["P-FIXTURE"]}}]"#,
            std::time::Duration::ZERO,
        )
        .await;
        let model = session::Model {
            id: id.clone(),
            store_url: Some(url),
            store_auth: Some("Basic fixture-secret".into()),
            studies: "unrelated-study".into(),
            user_id: None,
            anonymized: false,
            shared_session: false,
            share_key: None,
            expired_time: (Utc::now() + Duration::hours(1)).fixed_offset(),
            permissions: None,
        };
        if persist {
            model
                .into_active_model()
                .insert(db.connection())
                .await
                .unwrap();
            auth.invalidate_session(&id).await;
        } else {
            auth.cache_session(model).await;
        }
        let (status, body) = call(
            &app,
            "?offset=-7&limit=0&orderBy=ignored",
            json!({"sessionID":id,"patientID":""}),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(body["header"]["code"], 302);
        assert_eq!(body["header"]["offset"], -7);
        assert_eq!(body["header"]["limit"], 0);
        assert_eq!(body["header"]["totalRecords"], 1);
        assert_eq!(body["body"], json!([{"pid":"P-FIXTURE"}]));
        let request = task.await.unwrap();
        assert!(!request.contains("orderBy"));
        assert!(request.contains("offset=0"));
        assert!(request.contains("limit=20"));
    }
    let original = session::Entity::find_by_id(id.clone())
        .one(db.connection())
        .await
        .unwrap()
        .unwrap();
    for (status, document, expected) in [
        (200, "[]", 200),
        (204, "", 200),
        (200, "broken", 200),
        (401, "private upstream body", 500),
        (403, "private upstream body", 500),
        (500, "private upstream body", 500),
    ] {
        let (url, task) = store(status, document, std::time::Duration::ZERO).await;
        let mut cached = original.clone();
        cached.store_url = Some(url);
        auth.cache_session(cached).await;
        let (actual, body) = call(
            &app,
            "?offset=&limit=",
            json!({"sessionID":id,"patientID":"p"}),
        )
        .await;
        assert_eq!(actual, expected);
        if expected == 200 {
            assert_eq!(body["body"], json!([]));
            assert_eq!(body["header"]["offset"], 0);
            assert_eq!(body["header"]["limit"], 100);
        } else {
            assert_eq!(body["errorCode"], "INTERNAL_ERROR");
            assert!(!body.to_string().contains("private"));
            assert!(!body.to_string().contains("fixture-secret"));
        }
        task.await.unwrap();
        assert_eq!(
            session::Entity::find_by_id(id.clone())
                .one(db.connection())
                .await
                .unwrap()
                .unwrap(),
            original
        );
    }
    session::Entity::delete_by_id(id.clone())
        .exec(db.connection())
        .await
        .unwrap();
    auth.invalidate_session(&id).await;
    assert!(
        session::Entity::find_by_id(id)
            .one(db.connection())
            .await
            .unwrap()
            .is_none()
    );
    db.close().await.unwrap();
}
