use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    time::Instant,
};

/// The parent launches this same test in isolated processes, never two VMs per process.
#[test]
#[ignore = "requires explicit Java 25 and packaged facade; run tools/smoke_jni.py"]
fn jvm_subprocess_suite() {
    let settings = test_settings();
    if let Ok(case) = std::env::var("JNI_TEST_CASE") {
        child(&case, &settings);
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_dir = root.join("target/step-03-logs");
    fs::create_dir_all(&log_dir).unwrap();
    let fixtures = tempfile::tempdir_in(root.join("target")).unwrap();
    let real = settings
        .facade_dir
        .clone()
        .expect("packaged facade required");
    let java = settings
        .java_home
        .clone()
        .expect("explicit test JDK required");
    let fixture = make_fixture(fixtures.path(), &real, &java, "fixture", None);
    let missing_class = make_fixture(
        fixtures.path(),
        &real,
        &java,
        "missing-class",
        Some(("DicomFacade", "AbsentFacade")),
    );
    let missing_method = make_fixture(
        fixtures.path(),
        &real,
        &java,
        "missing-method",
        Some(("int apiVersion()", "int absentVersion()")),
    );
    let wrong_signature = make_fixture(
        fixtures.path(),
        &real,
        &java,
        "wrong-signature",
        Some(("int apiVersion()", "long apiVersion()")),
    );
    let bad_java = fixtures.path().join("java24");
    let relative_lib = if cfg!(windows) {
        "bin/server/jvm.dll"
    } else {
        "lib/server/libjvm.so"
    };
    fs::create_dir_all(bad_java.join(relative_lib).parent().unwrap()).unwrap();
    fs::write(bad_java.join(relative_lib), b"test").unwrap();
    fs::write(bad_java.join("release"), "JAVA_VERSION=\"24.0.1\"\n").unwrap();
    for case in [
        "normal",
        "lib-only",
        "exceptions",
        "slow",
        "version",
        "json",
        "capabilities",
        "null",
        "large",
        "missing-class",
        "missing-method",
        "wrong-signature",
        "missing-runtime",
        "bad-lib",
        "conflicting-lib",
        "bad-java",
        "bad-checksum",
        "bad-manifest",
        "missing-jar",
        "throw-probe",
        "hang-startup",
        "hang-probe",
        "java-thread",
    ] {
        let log = log_dir.join(format!("jni-{case}.log"));
        let output = fs::File::create(&log).unwrap();
        let mut command = Process::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "tests::jvm_subprocess_suite",
                "--ignored",
                "--nocapture",
            ])
            .env("JNI_TEST_CASE", case)
            .stdout(Stdio::from(output.try_clone().unwrap()))
            .stderr(Stdio::from(output));
        let distribution = match case {
            "normal" | "lib-only" => &real,
            "missing-class" => &missing_class,
            "missing-method" => &missing_method,
            "wrong-signature" => &wrong_signature,
            _ => &fixture,
        };
        let mut child_settings = settings.clone();
        child_settings.facade_dir = Some(distribution.to_owned());
        match case {
            "lib-only" => {
                child_settings.java_home = None;
                child_settings.libjvm_path = Some(java.join(relative_lib));
            }
            "missing-runtime" => {
                child_settings.java_home = None;
            }
            "bad-lib" => {
                child_settings.libjvm_path = Some(fixtures.path().join("absent"));
            }
            "conflicting-lib" => {
                child_settings.libjvm_path = Some(java.join("release"));
            }
            "bad-java" => {
                child_settings.java_home = Some(bad_java.clone());
            }
            "bad-checksum" | "bad-manifest" | "missing-jar" => {
                let bad = fixtures.path().join(case);
                fs::create_dir_all(bad.join("dependencies")).unwrap();
                fs::copy(
                    real.join("pacs-dicom-facade.jar"),
                    bad.join("pacs-dicom-facade.jar"),
                )
                .unwrap();
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(real.join("manifest.json")).unwrap()).unwrap();
                if case == "bad-checksum" {
                    fs::write(bad.join("pacs-dicom-facade.jar"), b"changed").unwrap();
                }
                if case == "missing-jar" {
                    fs::remove_file(bad.join("pacs-dicom-facade.jar")).unwrap();
                }
                if case == "bad-manifest" {
                    manifest["apiVersion"] = 2.into();
                }
                fs::write(
                    bad.join("manifest.json"),
                    serde_json::to_vec(&manifest).unwrap(),
                )
                .unwrap();
                child_settings.facade_dir = Some(bad);
            }
            _ => {}
        }
        let settings_file = fixtures.path().join(format!("{case}.toml"));
        fs::write(&settings_file, toml::to_string(&child_settings).unwrap()).unwrap();
        // libtest has no application config argument; this test-only variable
        // transports a file path, never runtime settings/credentials.
        command.env("JNI_TEST_CONFIG", settings_file);
        let mut process = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                process.kill().unwrap();
                let _ = process.wait();
                panic!(
                    "{case} exceeded external subprocess deadline; see {}",
                    log.display()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let text = fs::read_to_string(&log).unwrap();
        assert!(
            !text.contains("PRIVATE_PATIENT_TOKEN") && !text.contains("password=secret"),
            "sensitive exception leaked: {case}"
        );
        for warning in [
            "WARNING in native method",
            "JNI local refs",
            "FATAL ERROR in native method",
        ] {
            assert!(!text.contains(warning), "JNI diagnostic in {case}: {text}");
        }
        let expected = if matches!(case, "hang-startup" | "hang-probe" | "java-thread") {
            42
        } else {
            0
        };
        assert_eq!(status.code(), Some(expected), "{case}: {text}");
        println!("PASS {case}");
    }
}

fn test_settings() -> JvmSettings {
    let path = std::env::var_os("JNI_TEST_CONFIG").expect("test-only TOML fixture path");
    toml::from_str(&fs::read_to_string(path).unwrap()).expect("JNI TOML fixture")
}

fn make_fixture(
    base: &Path,
    real: &Path,
    java: &Path,
    name: &str,
    replacement: Option<(&str, &str)>,
) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(dir.join("classes")).unwrap();
    fs::create_dir_all(dir.join("dependencies")).unwrap();
    let mut source = include_str!("../tests/fixtures/DicomFacade.java").to_owned();
    if let Some((from, to)) = replacement {
        source = source.replace(from, to);
    }
    let filename = if name == "missing-class" {
        "AbsentFacade.java"
    } else {
        "DicomFacade.java"
    };
    fs::write(dir.join(filename), source).unwrap();
    for (tool, args) in [
        (
            "javac",
            vec![
                "--release".into(),
                "25".into(),
                "-d".into(),
                dir.join("classes").into_os_string(),
                dir.join(filename).into_os_string(),
            ],
        ),
        (
            "jar",
            vec![
                "--create".into(),
                "--file".into(),
                dir.join("pacs-dicom-facade.jar").into_os_string(),
                "-C".into(),
                dir.join("classes").into_os_string(),
                ".".into(),
            ],
        ),
    ] {
        let executable = java.join("bin").join(if cfg!(windows) {
            format!("{tool}.exe")
        } else {
            tool.to_owned()
        });
        let output = Process::new(executable).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "fixture compilation: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(real.join("manifest.json")).unwrap()).unwrap();
    manifest["artifacts"] = serde_json::json!({"pacs-dicom-facade.jar": format!("{:x}", Sha256::digest(fs::read(dir.join("pacs-dicom-facade.jar")).unwrap()))});
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

fn child(case: &str, settings: &JvmSettings) {
    let result = settings.validate();
    match case {
        "missing-runtime" | "bad-lib" | "conflicting-lib" => {
            assert!(matches!(result, Err(Error::RuntimeConfig)));
            return;
        }
        "bad-java" => {
            assert!(matches!(result, Err(Error::JavaVersion)));
            return;
        }
        "bad-checksum" | "missing-jar" => {
            assert!(matches!(result, Err(Error::Artifact)));
            return;
        }
        "bad-manifest" => {
            assert!(matches!(result, Err(Error::Contract)));
            return;
        }
        _ => {}
    }
    let mut config = result.unwrap().unwrap();
    config.options.push(format!("-Dfixture.mode={case}"));
    if matches!(case, "hang-startup" | "hang-probe") {
        config.timeout = Duration::from_millis(1500);
    }
    if case == "exceptions" {
        exception_checks(config);
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let started = Jvm::start(config).await;
        match case {
            "version" | "json" | "capabilities" | "large" => {
                assert!(matches!(started, Err(Error::Contract)));
                return;
            }
            "missing-class" | "missing-method" | "wrong-signature" | "null" => {
                assert!(started.is_err());
                return;
            }
            "hang-startup" => {
                assert!(matches!(started, Err(Error::Timeout)));
                std::process::exit(42);
            }
            _ => {}
        }
        let jvm = started.unwrap();
        if case == "hang-probe" {
            assert_eq!(jvm.check().await, Err(Error::Timeout));
            tokio::time::timeout(Duration::from_secs(1), jvm.wait_failed())
                .await
                .unwrap();
            std::process::exit(42);
        }
        if case == "throw-probe" {
            assert_eq!(jvm.check().await, Err(Error::JavaException));
            tokio::time::timeout(Duration::from_secs(1), jvm.wait_failed())
                .await
                .unwrap();
            assert_eq!(jvm.check().await, Err(Error::WorkerStopped));
            return;
        }
        if case == "slow" {
            let start = Instant::now();
            let (result, tick) = tokio::join!(jvm.check(), async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                start.elapsed()
            });
            result.unwrap();
            assert!(
                tick < Duration::from_millis(350),
                "JNI blocked single-thread Tokio"
            );
            assert!(start.elapsed() >= Duration::from_millis(450));
        } else {
            for _ in 0..2000 {
                assert!(jvm.check().await.unwrap().is_foundation());
            }
        }
        if case == "java-thread" {
            assert_eq!(jvm.shutdown().await, Err(Error::Shutdown));
            std::process::exit(42);
        }
        jvm.shutdown().await.unwrap();
        assert_eq!(jvm.check().await, Err(Error::WorkerStopped));
        assert!(matches!(
            Jvm::start(settings.validate().unwrap().unwrap()).await,
            Err(Error::AlreadyStarted)
        ));
    });
}

fn exception_checks(config: JvmConfig) {
    // All test-only calls and the VM stay on this dedicated test thread.
    std::thread::spawn(move || {
        let vm = create_vm(&config).unwrap();
        {
            let mut env = vm.attach_current_thread().unwrap();
            let class = frame(&mut env, |env| {
                let class = invoke(env, |env| env.find_class(FACADE))?;
                invoke(env, |env| env.new_global_ref(class))
            })
            .unwrap();
            for _ in 0..128 {
                for broken in [false, true] {
                    frame(&mut env, |env| {
                        let result = invoke(env, |env| {
                            env.call_static_method(
                                FACADE,
                                "syntheticError",
                                "(Z)V",
                                &[broken.into()],
                            )
                        });
                        assert!(matches!(
                            (result, broken),
                            (Err(Error::JavaException), false)
                                | (Err(Error::ExceptionDetailsUnavailable), true)
                        ));
                        assert!(!env.exception_check().unwrap());
                        Ok(())
                    })
                    .unwrap();
                    assert!(probe(&mut env, &class).unwrap().is_foundation());
                }
            }
        }
        assert_eq!(vm.threads_attached(), 0);
        // SAFETY: all references and the sole attachment were dropped above.
        unsafe { vm.destroy() }.unwrap();
        println!("PASS exception clear/fallback/recovery; local frames; detach");
    })
    .join()
    .unwrap();
}
