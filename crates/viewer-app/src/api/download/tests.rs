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
use tower::ServiceExt;

fn settings(root: PathBuf) -> DownloadSettings {
    DownloadSettings {
        enabled: true,
        temp_dir: root,
        max_jobs: 1,
        timeout_ms: 10_000,
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 1024 * 1024,
        max_job_bytes: 16 * 1024 * 1024,
        max_metadata_bytes: 1024 * 1024,
        max_entries: 100,
        max_frames: 100,
        max_pixels: 1024 * 1024,
        video_fps: 1,
        cleanup_ttl_seconds: 300,
        cleanup_interval_seconds: 60,
    }
}
#[test]
fn admission_paths_limits_and_java_types() {
    for name in [
        "../x",
        "/x",
        "a/../../x",
        "C:/x",
        "a\\x",
        "a//x",
        "a/./x",
        "x\r\ny",
    ] {
        assert!(!safe_zip_name(name), "{name}");
    }
    assert!(safe_zip_name("患者/series/1.jpeg"));
    assert!(kind("Mp4", true).is_ok());
    assert!(kind("mp4", false).is_err());
    for value in ["", "OCTET_STREAM", "DICOM ", "pdf", "image/jpeg"] {
        assert!(kind(value, true).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let mut config = settings(temp.path().to_owned());
    assert!(config.validate().is_ok());
    config.max_jobs = 0;
    assert!(config.validate().is_err());
    config.max_jobs = 1;
    config.max_metadata_bytes = 17 * 1024 * 1024;
    assert!(config.validate().is_err());
    let file = std::fs::File::create(temp.path().join("quota")).unwrap();
    let mut limited = LimitedFile { file, limit: 4 };
    limited.write_all(&[1, 2, 3, 4]).unwrap();
    assert!(limited.write_all(&[5]).is_err());
    assert_eq!(
        std::fs::metadata(temp.path().join("quota")).unwrap().len(),
        4
    );
}
#[tokio::test]
async fn response_disconnect_releases_file_and_permit() {
    let slots = Arc::new(Semaphore::new(1));
    let directory = tempfile::tempdir().unwrap();
    let lease =
        Arc::new(Lease::new(directory.path(), slots.clone().try_acquire_owned().unwrap()).unwrap());
    let root = lease.directory.path().to_owned();
    let path = root.join("output");
    std::fs::write(&path, vec![1u8; 256 * 1024]).unwrap();
    let mut response = file_response(path, "application/dicom", "1.dcm", lease)
        .await
        .unwrap();
    assert!(root.exists());
    assert_eq!(slots.available_permits(), 0);
    let frame = response.body_mut().frame().await.unwrap().unwrap();
    assert_eq!(frame.data_ref().unwrap().len(), 65536);
    drop(response);
    tokio::time::timeout(Duration::from_secs(3), async {
        while root.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(slots.available_permits(), 1);
}
fn model(store: &str) -> session::Model {
    session::Model {
        id: "download-fixture".into(),
        store_url: Some(store.into()),
        store_auth: Some("Basic fixture".into()),
        studies: "1.2.3".into(),
        user_id: None,
        anonymized: false,
        shared_session: false,
        share_key: None,
        expired_time: (Utc::now() + ChronoDuration::hours(1)).fixed_offset(),
        permissions: None,
    }
}
async fn call(app: &HttpRouter, path: &str) -> Response {
    app.clone()
        .oneshot(HttpRequest::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires configured PostgreSQL and isolated codec fixture; run tools/smoke_download.py --app"]
async fn postgres_download_contract() {
    let db = Persistence::connect(&crate::config::for_tests().database)
        .await
        .unwrap();
    let jvm = Jvm::start(crate::config::jni_for_tests().validate().unwrap().unwrap())
        .await
        .unwrap();
    let auth = Auth::new(db.clone(), AuthConfig::default());
    let requests = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let captured = requests.clone();
    let frame_mode = Arc::new(std::sync::atomic::AtomicI32::new(2));
    let store_mode = frame_mode.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let store_url = format!("http://{}/dicom/wado-rs", listener.local_addr().unwrap());
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-12-media.json"
    ))
    .unwrap();
    let dicom = Arc::new(
        base64::engine::general_purpose::STANDARD
            .decode(corpus["inputDicomBase64"].as_str().unwrap())
            .unwrap(),
    );
    let jpeg_fixture: Value = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-12-rendered-jpeg.json"
    ))
    .unwrap();
    let jpegs = Arc::new(
        jpeg_fixture["framesBase64"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                base64::engine::general_purpose::STANDARD
                    .decode(value.as_str().unwrap())
                    .unwrap()
            })
            .collect::<Vec<_>>(),
    );
    let served_jpegs = jpegs.clone();
    let served_dicom = dicom.clone();
    let store=HttpRouter::new().fallback(move |request:Request| {
        let captured=captured.clone();let dicom=served_dicom.clone();let jpegs=served_jpegs.clone();let mode=store_mode.load(std::sync::atomic::Ordering::SeqCst);
        async move {
            assert_eq!(request.headers()[header::AUTHORIZATION],"Basic fixture");
            captured.lock().unwrap().push(request.uri().to_string());
            let path=request.uri().path();
            if path.ends_with("/rendered") {
                assert_eq!(request.headers()[header::ACCEPT],"image/jpeg");
                assert_eq!(request.uri().query(),Some("quality=90"));
                let frame=path.split("/frames/").nth(1).unwrap().split('/').next().unwrap().parse::<usize>().unwrap();
                let mut body=jpegs[frame-1].clone();
                let media=if mode==4 {"image/png"}else{"image/jpeg"};
                if mode==5 {body=b"not-a-jpeg".to_vec();} else if mode==6 {body.truncate(body.len()-2);}
                return (if mode==7 {StatusCode::NOT_FOUND}else{StatusCode::OK},[(header::CONTENT_TYPE,media)],Body::from(body)).into_response();
            }
            let body=if path.ends_with("/metadata") {
                if mode==0 {b"[]".to_vec()} else {
                    let value=r#"[{"00080018":{"vr":"UI","Value":["1.2.3.4.5"]},"00200013":{"vr":"IS","Value":[1]},"00280008":{"vr":"IS","Value":[2]}}]"#;
                    if mode==1 {value.replace("[2]","[1]").into_bytes()} else {value.as_bytes().to_vec()}
                }
            } else if path.ends_with("/series") {
                br#"[{"0020000E":{"vr":"UI","Value":["1.2.3.4"]}}]"#.to_vec()
            } else {
                let query:BTreeMap<_,_>=url::form_urlencoded::parse(request.uri().query().unwrap().as_bytes()).into_owned().collect();
                assert_eq!(query["requestType"],"WADO");assert_eq!(query["contentType"],"application/dicom");
                assert_eq!(query["studyUID"],"1.2.3");assert_eq!(query["seriesUID"],"1.2.3.4");assert_eq!(query["objectUID"],"1.2.3.4.5");
                if mode==3 {b"slow".to_vec()}else{(*dicom).clone()}
            };
            Body::from(body).into_response()
        }
    });
    let server =
        tokio::spawn(async move { summer_web::axum::serve(listener, store).await.unwrap() });
    auth.cache_session(model(&store_url)).await;
    let root = tempfile::tempdir().unwrap();
    let runtime = DownloadRuntime::new(settings(root.path().to_owned()), Some(jvm.clone()))
        .unwrap()
        .unwrap();
    let app: HttpRouter = crate::api::router(
        db.clone(),
        Some(jvm.clone()),
        auth.clone(),
        super::super::SessionConfig::default(),
    )
    .layer(Extension(runtime.clone()))
    .into();
    let app = HttpRouter::new().nest("/ws/rest", app);
    let base = "/ws/rest/v1/session/download-fixture/download/studies/1.2.3";
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-12-java.json"
    ))
    .unwrap();
    for row in rows {
        frame_mode.store(
            if row["emptySeries"] == true {
                0
            } else {
                row["metadataFrames"].as_i64().unwrap_or(2) as i32
            },
            std::sync::atomic::Ordering::SeqCst,
        );
        let suffix = match row["scope"].as_str().unwrap() {
            "study" => "",
            "series" => "/series/1.2.3.4",
            _ => "/series/1.2.3.4/instances/1.2.3.4.5",
        };
        let path = format!(
            "{base}{suffix}?downloadType={}",
            row["type"].as_str().unwrap()
        );
        let path = if row["outsideStudy"] == true {
            path.replace("studies/1.2.3", "studies/9.9")
        } else {
            path
        };
        let before = requests.lock().unwrap().len();
        let response = call(&app, &path).await;
        assert_eq!(
            response.status().as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{path}"
        );
        for (fixture, header) in [
            ("contentType", "content-type"),
            ("disposition", "content-disposition"),
        ] {
            if let Some(value) = row[fixture].as_str() {
                assert_eq!(response.headers()[header], value, "{path}");
            }
        }
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        if let Some(code) = row["errorCode"].as_str() {
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["errorCode"], code);
            assert_eq!(requests.lock().unwrap().len(), before);
        }
        if row["emptyBody"] == true {
            assert!(bytes.is_empty());
            assert_eq!(requests.lock().unwrap().len(), before);
        }
        if let Some(body) = row["body"].as_str() {
            assert_eq!(
                &bytes[..],
                if row["type"] == "DICOM" {
                    (*dicom).clone()
                } else if row["type"] == "JPEG" {
                    jpegs[0].clone()
                } else {
                    base64::engine::general_purpose::STANDARD
                        .decode(body)
                        .unwrap()
                }
            );
        }
        if let Some(expected) = row["entries"].as_object() {
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            let mut actual = BTreeMap::new();
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).unwrap();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                actual.insert(
                    entry.name().to_owned(),
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                );
            }
            let expected: BTreeMap<String, String> = expected
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        if row["type"] == "DICOM" {
                            base64::engine::general_purpose::STANDARD.encode(dicom.as_slice())
                        } else if row["type"] == "JPEG" && !k.ends_with('/') {
                            let frame =
                                usize::from(k.ends_with("_1.jpeg") || k.ends_with("/1.jpeg"));
                            base64::engine::general_purpose::STANDARD.encode(&jpegs[frame])
                        } else {
                            v.as_str().unwrap().into()
                        },
                    )
                })
                .collect();
            assert_eq!(actual, expected, "{path}");
        }
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            0,
            "No job files remain after body completion"
        );
    }
    frame_mode.store(2, std::sync::atomic::Ordering::SeqCst);
    let response = call(&app, &format!("{base}/series/1.2.3.4?downloadType=Mp4")).await;
    assert_eq!(response.headers()["content-type"], "video/mp4");
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment;filename=1.2.3.mp4"
    );
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "fixture-mp4"
    );
    // DICOM instance download must work with no JVM at all. Pixel/frame limits
    // apply to codecs, not to an unchanged DICOM object.
    let mut limits = settings(root.path().to_owned());
    limits.max_frames = 1;
    limits.max_pixels = 1;
    let no_jvm = DownloadRuntime::new(limits, None).unwrap().unwrap();
    let no_jvm: HttpRouter = crate::api::router(
        db.clone(),
        None,
        auth.clone(),
        super::super::SessionConfig::default(),
    )
    .layer(Extension(no_jvm))
    .into();
    let no_jvm = HttpRouter::new().nest("/ws/rest", no_jvm);
    let response = call(
        &no_jvm,
        &format!("{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType=dicom"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        dicom.as_slice()
    );
    let jpeg_runtime = DownloadRuntime::new(settings(root.path().to_owned()), None)
        .unwrap()
        .unwrap();
    let jpeg_app: HttpRouter = crate::api::router(
        db.clone(),
        None,
        auth.clone(),
        super::super::SessionConfig::default(),
    )
    .layer(Extension(jpeg_runtime))
    .into();
    let jpeg_app = HttpRouter::new().nest("/ws/rest", jpeg_app);
    let before = requests.lock().unwrap().len();
    let response = call(
        &jpeg_app,
        &format!("{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType=jpeg"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let mut first = Vec::new();
    archive
        .by_name("1/0.jpeg")
        .unwrap()
        .read_to_end(&mut first)
        .unwrap();
    assert_eq!(first, jpegs[0]);
    assert!(
        requests.lock().unwrap()[before..]
            .iter()
            .all(|url| !url.starts_with("/dicom/wado?")),
        "JPEG must not download DICOM or call a local renderer"
    );
    for mode in 4..=7 {
        frame_mode.store(mode, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            call(
                &jpeg_app,
                &format!("{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType=jpeg")
            )
            .await
            .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
    frame_mode.store(2, std::sync::atomic::Ordering::SeqCst);
    let missing = call(
        &app,
        "/ws/rest/v1/session/download-missing/download/studies/1.2.3?downloadType=dicom",
    )
    .await;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    let held = runtime.slots.clone().try_acquire_owned().unwrap();
    let full = call(&app, &format!("{base}?downloadType=dicom")).await;
    assert_eq!(full.status(), StatusCode::SERVICE_UNAVAILABLE);
    drop(held);
    frame_mode.store(3, std::sync::atomic::Ordering::SeqCst);
    let executing = app.clone();
    let path = format!("{base}/series/1.2.3.4?downloadType=mp4");
    let running = tokio::spawn(async move { call(&executing, &path).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let started = std::fs::read_dir(root.path()).unwrap().any(|entry| {
                std::fs::read_dir(entry.unwrap().path())
                    .unwrap()
                    .any(|file| {
                        file.unwrap()
                            .file_name()
                            .to_string_lossy()
                            .ends_with(".started")
                    })
            });
            if started {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    running.abort();
    let _ = running.await;
    assert_eq!(
        runtime.slots.available_permits(),
        0,
        "Aborted request still owns native worker/file lease"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    tokio::time::timeout(Duration::from_secs(3), async {
        while runtime.slots.available_permits() == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    frame_mode.store(2, std::sync::atomic::Ordering::SeqCst);
    let mut expired = model(&store_url);
    expired.expired_time = (Utc::now() - ChronoDuration::minutes(1)).fixed_offset();
    auth.cache_session(expired).await;
    assert_eq!(
        call(&app, &format!("{base}?downloadType=dicom"))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let mut shared = model(&store_url);
    shared.shared_session = true;
    shared.share_key = Some("fixture-key".into());
    auth.cache_session(shared).await;
    assert_eq!(
        call(&app, &format!("{base}?downloadType=dicom"))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get(format!(
                "{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType=dicom"
            ))
            .header("session-key", "fixture-key")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.into_body().collect().await.unwrap();
    auth.cache_session(model(&store_url)).await;
    for input_limit in [false, true] {
        let mut limits = settings(root.path().to_owned());
        if input_limit {
            limits.max_input_bytes = 4;
        } else {
            limits.max_metadata_bytes = 4;
        }
        let limited = DownloadRuntime::new(limits, Some(jvm.clone()))
            .unwrap()
            .unwrap();
        let limited: HttpRouter = crate::api::router(
            db.clone(),
            Some(jvm.clone()),
            auth.clone(),
            super::super::SessionConfig::default(),
        )
        .layer(Extension(limited))
        .into();
        let limited = HttpRouter::new().nest("/ws/rest", limited);
        let response = call(
            &limited,
            &format!(
                "{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType={}",
                if input_limit { "dicom" } else { "jpeg" }
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = unavailable.local_addr().unwrap();
    drop(unavailable);
    auth.cache_session(model(&format!("http://{address}/dicom/wado-rs")))
        .await;
    let failed = call(
        &app,
        &format!("{base}/series/1.2.3.4/instances/1.2.3.4.5?downloadType=dicom"),
    )
    .await;
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    server.abort();
    let _ = server.await;
    jvm.shutdown().await.unwrap();
    db.close().await.unwrap();
}

#[test]
fn ttl_cleanup_skips_active_lease_and_unowned_files() {
    let root = tempfile::tempdir().unwrap();
    let slots = Arc::new(Semaphore::new(1));
    let active = Lease::new(root.path(), slots.clone().try_acquire_owned().unwrap()).unwrap();
    let marker = std::fs::File::options()
        .write(true)
        .open(active.directory.path().join(".lease"))
        .unwrap();
    marker
        .set_modified(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    drop(marker);
    let orphan = root.path().join("download-orphan");
    std::fs::create_dir(&orphan).unwrap();
    let marker = std::fs::File::create_new(orphan.join(".lease")).unwrap();
    marker
        .set_modified(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    drop(marker);
    std::fs::write(orphan.join("input"), "synthetic").unwrap();
    let unowned = root.path().join("download-not-owned");
    std::fs::create_dir(&unowned).unwrap();
    storage::reap(root.path(), Duration::from_secs(1)).unwrap();
    assert!(active.directory.path().exists());
    assert!(!orphan.exists());
    assert!(unowned.exists());
    assert_eq!(slots.available_permits(), 0);
    drop(active);
    assert_eq!(slots.available_permits(), 1);
}

#[test]
fn rendered_jpeg_frames_and_resource_checks() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../docs/baseline/step-12-rendered-jpeg.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("frame.jpeg");
    for value in fixture["framesBase64"].as_array().unwrap() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value.as_str().unwrap())
            .unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(verify_jpeg(&path, 256).is_ok());
        assert!(verify_jpeg(&path, 255).is_err());
        std::fs::write(&path, &bytes[..bytes.len() - 2]).unwrap();
        assert!(verify_jpeg(&path, 256).is_err());
    }
    for bytes in [
        b"not-a-jpeg".as_slice(),
        &[0xff, 0xd8, 0xff, 0xe0, 0, 1, 0xff, 0xd9],
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(verify_jpeg(&path, 256).is_err());
    }
    assert_eq!(instance_frames(r#"[{}]"#).unwrap(), 1);
    assert_eq!(
        instance_frames(r#"[{"00280008":{"Value":["2"]}}]"#).unwrap(),
        2
    );
    assert!(instance_frames("[]").is_err());
    assert!(instance_frames("invalid").is_err());
    let url = rendered_frame(
        "http://store.test/dicom/wado-rs",
        "study",
        "series",
        "sop",
        1,
    )
    .unwrap();
    assert_eq!(
        url.as_str(),
        "http://store.test/dicom/wado-rs/studies/study/series/series/instances/sop/frames/2/rendered?quality=90"
    );
}
