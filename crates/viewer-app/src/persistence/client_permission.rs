use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "client_permission")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub client_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub permission_id: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
