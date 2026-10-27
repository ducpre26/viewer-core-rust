use super::*;
use crate::{
    auth::AuthConfig,
    persistence::{Persistence, session},
};
use base64::Engine;
use chrono::{Duration as ChronoDuration, Utc};
use http_body_util::BodyExt;
use serde_json::Value;
use summer_web::axum::{Router as HttpRouter, http::Request as HttpRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

const MEDIA: &str = "multipart/related; type=\"application/dicom\"; transfer-syntax=1.2.840.10008.1.2.1; boundary=fixture";

async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    loop {
        let mut buf = [0; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0);
        request.extend_from_slice(&buf[..n]);
        if request.windows(4).any(|s| s == b"\r\n\r\n") {
            break;
        }
        assert!(request.len() < 65536);
    }
    String::from_utf8(request).unwrap()
}

async fn store(
    status: u16,
    payload: Vec<u8>,
    media: Option<&str>,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dicom/wado-rs", listener.local_addr().unwrap());
    let media = media.map(str::to_owned);
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_request(&mut socket).await;
        let mut headers = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\nSet-Cookie: secret=fixture\r\nContent-Disposition: attachment; filename=upstream.exe\r\nAccess-Control-Allow-Origin: https://untrusted.invalid\r\nX-Upstream-Secret: private\r\n",
            payload.len()
        );
        if let Some(media) = media {
            headers.push_str(&format!("cOnTeNt-TyPe: {media}\r\n"));
        }
        headers.push_str("\r\n");
        socket.write_all(headers.as_bytes()).await.unwrap();
        socket.write_all(&payload).await.unwrap();
        request
    });
    (url, task)
}

fn model(id: &str, store: &str) -> session::Model {
    session::Model {
        id: id.into(),
        store_url: Some(store.into()),
        store_auth: Some("Basic fixture-secret".into()),
        studies: "1.2.3".into(),
        user_id: None,
        anonymized: false,
        shared_session: false,
        share_key: None,
        expired_time: (Utc::now() + ChronoDuration::hours(1)).fixed_offset(),
        permissions: None,
    }
}

fn app(db: &Persistence, auth: &Auth) -> HttpRouter {
    HttpRouter::new().nest(
        "/ws/rest",
        crate::api::router(
            db.clone(),
            None,
            auth.clone(),
            super::super::SessionConfig::default(),
        )
        .into(),
    )
}

async fn call(app: &HttpRouter, path: &str, key: Option<&str>, accept: Option<&str>) -> Response {
    let mut request =
        HttpRequest::get(path).header("authorization", "Basic caller-must-not-be-forwarded");
    if let Some(key) = key {
        request = request.header("session-key", key);
    }
    if let Some(accept) = accept {
        request = request.header("accept", accept);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[test]
fn url_scope_and_query_cannot_replace_the_store_origin() {
    let mut params = BTreeMap::from([
        ("studyUID".into(), "1.2.3".into()),
        ("seriesUID".into(), "https://evil.invalid/?x=#".into()),
    ]);
    let op = Operation {
        suffix: "/series/{seriesUID}",
        accept: false,
        thumbnail: false,
        uri: false,
    };
    let url = upstream_url("http://fixture.invalid/dicom/wado-rs", op, &params).unwrap();
    assert_eq!(url.host_str(), Some("fixture.invalid"));
    assert!(
        url.path()
            .starts_with("/dicom/wado-rs/studies/1.2.3/series/https:")
    );
    assert!(url.query().is_none());
    assert!(url.fragment().is_none());
    for store in ["file:///tmp", "not-url", "ftp://fixture.invalid"] {
        assert!(upstream_url(store, op, &params).is_none());
    }
    params.insert("seriesUID".into(), "..".into());
    // Url's segment API omits dot segments instead of allowing traversal.
    let url = upstream_url("http://fixture.invalid/dicom", op, &params).unwrap();
    assert!(url.path().starts_with("/dicom/studies/1.2.3/series"));
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL TOML fixture"]
async fn postgres_migrations_transaction_readiness_and_close_wado_contract() {
    use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let app = app(&db, &auth);
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-11-java.json"
    ))
    .unwrap();
    for row in fixtures {
        let name = row["name"].as_str().unwrap();
        if name == "scope-26" {
            continue;
        } // Controller-only Java case; filter tested below.
        let payload = base64::engine::general_purpose::STANDARD
            .decode(row["bodyBase64"].as_str().unwrap_or(""))
            .unwrap();
        let status = name
            .strip_prefix("upstream-")
            .map(|s| s.parse().unwrap())
            .unwrap_or(200);
        let (url, task) = store(
            status,
            payload.clone(),
            if row["upstreamMedia"] == false {
                None
            } else {
                Some(MEDIA)
            },
        )
        .await;
        auth.cache_session(model("s", &url)).await;
        let path = format!(
            "{}?{}",
            row["path"].as_str().unwrap(),
            row["query"].as_str().unwrap()
        );
        let response = call(
            &app,
            &path,
            None,
            Some(
                row["accept"]
                    .as_str()
                    .unwrap_or("multipart/related; transfer-syntax=*"),
            ),
        )
        .await;
        assert_eq!(
            response.status().as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{name}"
        );
        for (key, values) in row["headers"].as_object().unwrap() {
            assert_eq!(
                response.headers().get(key).unwrap().to_str().unwrap(),
                values[0].as_str().unwrap(),
                "{name}: {key}"
            );
        }
        for header in ["set-cookie", "x-upstream-secret"] {
            assert!(!response.headers().contains_key(header), "{name}: {header}");
        }
        assert_ne!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|h| h.to_str().ok()),
            Some("https://untrusted.invalid")
        );
        if row.get("upstreamUrl").is_some() {
            assert!(!response.headers().contains_key("content-length"), "{name}");
        }
        let body = response.into_body().collect().await.unwrap().to_bytes();
        if let Some(expected) = row.get("bodyJson") {
            let mut actual: Value = serde_json::from_slice(&body).unwrap();
            actual.as_object_mut().unwrap().remove("timestamp");
            assert_eq!(&actual, expected, "{name}");
        } else {
            assert_eq!(body.as_ref(), payload, "{name}");
        }
        if let Some(expected) = row.get("upstreamUrl") {
            let captured = tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .unwrap()
                .unwrap();
            let target = captured
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            let actual = url::Url::parse(&format!("http://fixture.invalid{target}")).unwrap();
            let expected = url::Url::parse(expected.as_str().unwrap()).unwrap();
            assert_eq!(
                actual.path().replace("%2C", ","),
                expected.path().replace("%2C", ","),
                "{name}"
            );
            let pairs = |u: &url::Url| {
                u.query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect::<BTreeMap<_, _>>()
            };
            assert_eq!(pairs(&actual), pairs(&expected), "{name}");
            assert!(
                captured
                    .to_lowercase()
                    .contains("authorization: basic fixture-secret")
            );
            assert!(!captured.contains("caller-must-not"));
            if let Some(expected) = row["upstreamHeaders"].get("accept") {
                assert!(
                    captured
                        .to_lowercase()
                        .contains(&format!("accept: {}", expected[0].as_str().unwrap())),
                    "{name}"
                );
            } else {
                assert!(
                    !captured
                        .to_lowercase()
                        .contains("accept: multipart/related; transfer-syntax=*"),
                    "{name}"
                );
            }
        } else {
            task.abort();
        }
    }
    // Real session filter: missing/expired IDs and bad shared-session key are 401.
    assert_eq!(
        call(
            &app,
            "/ws/rest/v1/session/step11-missing/wado-rs/studies/1.2.3",
            None,
            None
        )
        .await
        .status(),
        401
    );
    let (url, task) = store(200, vec![1, 2, 3], Some("application/dicom")).await;
    let mut shared = model("step11-shared", &url);
    shared.share_key = Some("fixture-key".into());
    auth.cache_session(shared.clone()).await;
    let path = "/ws/rest/v1/session/step11-shared/wado-rs/studies/1.2.3";
    for key in [None, Some("wrong")] {
        assert_eq!(call(&app, path, key, None).await.status(), 401);
    }
    assert_eq!(
        call(&app, path, Some("fixture-key"), None).await.status(),
        200
    );
    task.await.unwrap();
    shared.expired_time = (Utc::now() - ChronoDuration::seconds(1)).fixed_offset();
    auth.cache_session(shared).await;
    assert_eq!(
        call(&app, path, Some("fixture-key"), None).await.status(),
        401
    );
    // DB fallback uses existing SESSION and never changes its business fields.
    let id = format!("step11-{}", uuid::Uuid::new_v4());
    let (url, task) = store(200, b"[]".to_vec(), Some("application/dicom+json")).await;
    let original = model(&id, &url)
        .into_active_model()
        .insert(db.connection())
        .await
        .unwrap();
    let response = call(
        &app,
        &format!("/ws/rest/v1/session/{id}/wado-rs/studies/1.2.3/metadata"),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        b"[]"
    );
    task.await.unwrap();
    assert_eq!(
        session::Entity::find_by_id(&id)
            .one(db.connection())
            .await
            .unwrap()
            .unwrap(),
        original
    );
    session::Entity::delete_by_id(&id)
        .exec(db.connection())
        .await
        .unwrap();
    auth.invalidate_session(&id).await;
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL TOML fixture"]
async fn postgres_migrations_transaction_readiness_and_close_wado_stream() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let app = app(&db, &auth);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dicom", listener.local_addr().unwrap());
    auth.cache_session(model("stream", &url)).await;
    let written = Arc::new(AtomicUsize::new(0));
    let count = written.clone();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nhead\r\n").await.unwrap();
        release_rx.await.unwrap();
        let block = vec![42; 65536];
        // A 512 MiB producer must block at the network window with a slow client.
        for _ in 0..8192 {
            if socket.write_all(b"10000\r\n").await.is_err()
                || socket.write_all(&block).await.is_err()
                || socket.write_all(b"\r\n").await.is_err()
            {
                return;
            }
            count.fetch_add(block.len(), Ordering::SeqCst);
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    let response = tokio::time::timeout(
        Duration::from_secs(3),
        call(
            &app,
            "/ws/rest/v1/session/stream/wado-rs/studies/1.2.3",
            None,
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), 200);
    let mut body = response.into_body();
    let first = tokio::time::timeout(Duration::from_secs(3), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    assert_eq!(first.as_ref(), b"head"); // Returned while the rest is still gated.
    release_tx.send(()).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(written.load(Ordering::SeqCst) < 512 * 1024 * 1024);
    drop(body);
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("disconnect releases upstream writer")
        .unwrap();
    // Many chunks consumed slowly: payload integrity without a whole-body buffer.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    auth.cache_session(model(
        "large",
        &format!("http://{}/dicom", listener.local_addr().unwrap()),
    ))
    .await;
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 16777216\r\nConnection: close\r\n\r\n").await.unwrap();
        for _ in 0..256 {
            socket.write_all(&vec![0xA5; 65536]).await.unwrap();
        }
    });
    let mut body = call(
        &app,
        "/ws/rest/v1/session/large/wado-rs/studies/1.2.3",
        None,
        None,
    )
    .await
    .into_body();
    let mut total = 0;
    while let Some(frame) = body.frame().await {
        if let Ok(bytes) = frame.unwrap().into_data() {
            assert!(bytes.iter().all(|b| *b == 0xA5));
            total += bytes.len();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    assert_eq!(total, 16777216);
    task.await.unwrap();
    // An upstream body truncated after headers terminates the stream with an error.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    auth.cache_session(model(
        "truncated",
        &format!("http://{}/dicom", listener.local_addr().unwrap()),
    ))
    .await;
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nshort")
            .await
            .unwrap();
    });
    let response = call(
        &app,
        "/ws/rest/v1/session/truncated/wado-rs/studies/1.2.3",
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), 200);
    assert!(response.into_body().collect().await.is_err());
    task.await.unwrap();
    // Exercise the real reqwest client with shorter test deadlines. No HTTP
    // client stub: both header and mid-body timeouts cross a TCP connection.
    let client = reqwest::Client::builder()
        .no_proxy()
        .read_timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let wado = Wado {
        auth: auth.clone(),
        client,
    };
    let operation = Operation {
        suffix: "",
        accept: false,
        thumbnail: false,
        uri: false,
    };
    let params = BTreeMap::from([
        ("sessionID".into(), "timeout".into()),
        ("studyUID".into(), "1.2.3".into()),
    ]);
    for send_headers in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        auth.cache_session(model(
            "timeout",
            &format!("http://{}/dicom", listener.local_addr().unwrap()),
        ))
        .await;
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_request(&mut socket).await;
            if send_headers {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nstart")
                    .await
                    .unwrap();
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let response = retrieve(
            Extension(wado.clone()),
            Extension(operation),
            Path(params.clone()),
            HttpRequest::get("/").body(Body::empty()).unwrap(),
        )
        .await;
        if send_headers {
            assert_eq!(response.status(), 200);
            assert!(response.into_body().collect().await.is_err());
        } else {
            assert_eq!(response.status(), 500);
            assert!(
                response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .is_empty()
            );
        }
        task.abort();
    }
    // Connection refusal before headers returns an empty 500.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    auth.cache_session(model(
        "timeout",
        &format!("http://{}/dicom", listener.local_addr().unwrap()),
    ))
    .await;
    drop(listener);
    let response = retrieve(
        Extension(wado),
        Extension(operation),
        Path(params),
        HttpRequest::get("/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), 500);
    assert!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    db.close().await.unwrap();
}
