//! Authentication types and session permission rules; no framework or I/O.
pub const ADMIN: &str = "ADMIN";
pub const MODIFY_OVERLAY_CONFIG: &str = "MODIFY_OVERLAY_CONFIG";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    Client,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub id: String,
    pub role: Role,
}

pub fn permits(permissions: Option<&[String]>, required: &str) -> bool {
    permissions.is_some_and(|values| values.iter().any(|p| p == required || p == ADMIN))
}
