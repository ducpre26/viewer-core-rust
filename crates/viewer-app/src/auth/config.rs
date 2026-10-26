use std::env;

/// Deliberately no Debug/Serialize: this contains credentials.
#[derive(Clone)]
pub struct AuthConfig {
    pub client_security_disabled: bool,
    pub admin_username: String,
    pub admin_password: String,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, &'static str> {
        let disabled = match env::var("VIEWER_CLIENT_SECURITY_DISABLED") {
            Err(env::VarError::NotPresent) => false,
            Ok(v) if v.eq_ignore_ascii_case("true") => true,
            Ok(v) if v.eq_ignore_ascii_case("false") => false,
            _ => return Err("VIEWER_CLIENT_SECURITY_DISABLED must be true or false"),
        };
        Ok(Self {
            client_security_disabled: disabled,
            admin_username: credential("VIEWER_ADMIN_USERNAME")?,
            admin_password: credential("VIEWER_ADMIN_PASSWORD")?,
        })
    }
}

fn credential(name: &str) -> Result<String, &'static str> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok("sysadmin".into()),
        Err(_) => Err("Invalid admin credential configuration"),
    }
}
