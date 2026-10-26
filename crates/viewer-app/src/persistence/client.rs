use sea_orm::entity::prelude::*;

// Never serialize credentials. Debug intentionally omits the model contents.
#[derive(Clone, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "client")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub password: String,
    pub ip_allowed: Option<String>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client { .. }")
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
