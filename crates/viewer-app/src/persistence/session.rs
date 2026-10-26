use sea_orm::entity::prelude::*;

#[derive(Clone, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub store_url: Option<String>,
    pub store_auth: Option<String>,
    pub studies: String,
    pub user_id: Option<String>,
    pub anonymized: bool,
    pub shared_session: bool,
    pub share_key: Option<String>,
    pub expired_time: DateTimeWithTimeZone,
    pub permissions: Option<Vec<String>>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session { .. }")
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
