use super::*;
use std::str::FromStr;
use summer::config::toml::TomlConfigRegistry;

fn registry(extra: &str) -> TomlConfigRegistry {
    TomlConfigRegistry::from_str(&format!(
        r#"
[database]
url = "postgresql://127.0.0.1/test"
username = "fixture-user"
password = 'space @:/#?% " ${{UNEXPANDED}}'
{extra}
"#
    ))
    .unwrap()
}

#[test]
fn typed_defaults_and_literal_credentials() {
    let config = AppConfig::from_registry(&registry("")).unwrap();
    assert_eq!(config.database.max_connections, 10);
    assert_eq!(config.database.timeout_seconds, 5);
    assert_eq!(config.database.password, "space @:/#?% \" ${UNEXPANDED}");
    let url = config.database.connection_url().unwrap();
    assert_eq!(url.username(), "fixture-user");
    assert!(!config.auth.client_security_disabled);
    assert_eq!(config.auth.admin_username, "sysadmin");
    assert_eq!(config.session.default_expire_minutes, 60);
    assert_eq!(config.video.fps, 1);
    assert!(config.jni.validate().unwrap().is_none());
}

#[test]
fn invalid_settings_return_only_safe_categories() {
    for extra in [
        "max_connections = 0",
        "timeout_seconds = 0",
        "typo = 'sensitive-marker'",
    ] {
        let error = AppConfig::from_registry(&registry(extra)).err().unwrap();
        assert!(!error.contains("sensitive-marker"));
    }
    for extra in [
        "[auth]\nclient_security_disabled = 'sensitive-marker'",
        "[session]\ndefault_expire_minutes = 9223372036854775807",
        "[jni]\nenabled = 'sensitive-marker'",
        "[jni]\ntypo = 'sensitive-marker'",
    ] {
        assert!(AppConfig::from_registry(&registry(extra)).is_err());
    }
    let bad = TomlConfigRegistry::from_str(
        "[database]\nurl='postgres://secret:secret@host/test'\nusername='u'\npassword='p'",
    )
    .unwrap();
    assert!(AppConfig::from_registry(&bad).is_err());
    let config = AppConfig::from_registry(&registry("[jni]\nenabled=true")).unwrap();
    assert!(config.jni.validate().is_err());
}

#[test]
fn video_fps_is_optional_and_typed() {
    let config = AppConfig::from_registry(&registry("[video]\nfps=24")).unwrap();
    assert_eq!(config.video.fps, 24);
    for extra in [
        "[video]\nfps=0",
        "[video]\nfps=-1",
        "[video]\nfps='bad'",
        "[video]\ntypo=2",
    ] {
        assert!(AppConfig::from_registry(&registry(extra)).is_err());
    }
}

#[test]
fn video_fps_uses_summer_profile_overlay() {
    use summer::config::env::Env;
    let root = tempfile::tempdir().unwrap();
    let base = root.path().join("app.toml");
    std::fs::write(&base, "[video]\nfps=1").unwrap();
    std::fs::write(root.path().join("app-prod.toml"), "[video]\nfps=24").unwrap();
    let dev = TomlConfigRegistry::new(&base, Env::Dev).unwrap();
    let prod = TomlConfigRegistry::new(&base, Env::Prod).unwrap();
    assert_eq!(
        dev.get_config::<crate::api::VideoSettings>().unwrap().fps,
        1
    );
    assert_eq!(
        prod.get_config::<crate::api::VideoSettings>().unwrap().fps,
        24
    );
}
