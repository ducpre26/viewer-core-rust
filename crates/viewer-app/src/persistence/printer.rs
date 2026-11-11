use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "printer")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub printer_name: String,
    pub host: String,
    pub port: i32,
    pub aetitle: String,
    pub called_aetitle: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_time: DateTimeWithTimeZone,
    pub creator_id: i32,
    pub last_updated_time: Option<DateTimeWithTimeZone>,
    pub last_updated_id: Option<i32>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
