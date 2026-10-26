use serde::Deserialize;
use summer::config::Configurable;

/// Deliberately no Debug/Serialize: this contains credentials.
#[derive(Clone, Deserialize, Configurable)]
#[config_prefix = "auth"]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    pub client_security_disabled: bool,
    pub admin_username: String,
    pub admin_password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            client_security_disabled: false,
            admin_username: "sysadmin".into(),
            admin_password: "sysadmin".into(),
        }
    }
}
