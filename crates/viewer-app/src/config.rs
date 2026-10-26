//! Application settings bound through Summer's existing TOML/profile registry.
//! No independent dotenv, profile, logging or environment-variable loader.
use crate::{api::SessionConfig, auth::AuthConfig};
use dicom_jvm::JvmSettings;
use serde::Deserialize;
use summer::config::{ConfigRegistry, Configurable};

/// Deliberately no Debug/Serialize: contains database credentials.
#[derive(Clone, Deserialize, Configurable)]
#[config_prefix = "database"]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_connections")]
    pub max_connections: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}
fn default_connections() -> u32 {
    10
}
fn default_timeout() -> u32 {
    5
}

impl DatabaseConfig {
    pub fn connection_url(&self) -> Result<url::Url, &'static str> {
        let mut url = url::Url::parse(&self.url).map_err(|_| "Invalid database.url")?;
        if !matches!(url.scheme(), "postgres" | "postgresql")
            || url.host_str().is_none()
            || url.path().trim_matches('/').is_empty()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("database.url must specify PostgreSQL host/database without credentials");
        }
        if self.max_connections == 0 || self.timeout_seconds == 0 {
            return Err("Database pool size and timeout must be positive integers");
        }
        url.set_username(&self.username)
            .map_err(|_| "Invalid database.username")?;
        url.set_password(Some(&self.password))
            .map_err(|_| "Invalid database.password")?;
        Ok(url)
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct JniSection(JvmSettings);
impl Configurable for JniSection {
    fn config_prefix() -> &'static str {
        "jni"
    }
}

/// One snapshot from the same registry used by WebPlugin/JobPlugin/logger.
/// Never log this value or the underlying deserialize errors.
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub session: SessionConfig,
    pub jni: JvmSettings,
}

impl AppConfig {
    pub fn from_registry(registry: &impl ConfigRegistry) -> Result<Self, &'static str> {
        let database = registry
            .get_config::<DatabaseConfig>()
            .map_err(|_| "Invalid or missing [database] configuration")?;
        database.connection_url()?;
        let auth = registry
            .get_config::<AuthConfig>()
            .map_err(|_| "Invalid [auth] configuration")?;
        let session = registry
            .get_config::<SessionConfig>()
            .map_err(|_| "Invalid [session] configuration")?;
        session.validate()?;
        let jni = registry
            .get_config::<JniSection>()
            .map_err(|_| "Invalid [jni] configuration")?
            .0;
        Ok(Self {
            database,
            auth,
            session,
            jni,
        })
    }
}

#[cfg(test)]
pub(crate) fn for_tests() -> AppConfig {
    use std::path::Path;
    use summer::config::{env::Env, toml::TomlConfigRegistry};
    // libtest changes cwd to the package directory. Use the same framework
    // loader against the repository config; do not seed a second config path.
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/app.toml");
    let registry = TomlConfigRegistry::new(&file, Env::init()).expect("test TOML registry");
    AppConfig::from_registry(&registry).expect("test application settings")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn jni_for_tests() -> JvmSettings {
    use std::str::FromStr;
    use summer::config::toml::TomlConfigRegistry;
    // Test-only libtest transport. Actual JNI settings are read from TOML.
    let file = std::env::var_os("JNI_TEST_CONFIG").expect("JNI TOML fixture path");
    let source = std::fs::read_to_string(file).expect("JNI TOML fixture");
    let registry = TomlConfigRegistry::from_str(&format!("[jni]\n{source}")).unwrap();
    registry.get_config::<JniSection>().unwrap().0
}
