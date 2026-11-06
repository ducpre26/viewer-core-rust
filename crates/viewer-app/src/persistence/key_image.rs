//! Java STUDY_KEY_IMAGE: one pending row per study, with no SESSION foreign key.
use sea_orm::entity::prelude::*;
use sea_orm::{QuerySelect, Set};
#[derive(Clone, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "study_key_image")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub series_iuid: String,
    pub num_of_image: i32,
    pub store_url: String,
    pub store_auth: Option<String>,
    pub succeed: bool,
    pub num_of_retry: i32,
    pub description: Option<String>,
    pub created_time: DateTimeWithTimeZone,
    pub creator_id: i32,
    pub last_updated_time: Option<DateTimeWithTimeZone>,
    pub last_updated_id: Option<i32>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
pub async fn pending(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
    Entity::find()
        .filter(Column::Succeed.eq(false))
        .filter(Column::NumOfRetry.lte(3))
        .limit(100)
        .all(db)
        .await
}
pub async fn failed(db: &DatabaseConnection, row: Model, reason: String) -> Result<(), DbErr> {
    let retry = row.num_of_retry + 1;
    let mut active: ActiveModel = row.into();
    active.num_of_retry = Set(retry);
    active.description = Set(Some(reason));
    active.last_updated_time = Set(Some(chrono::Utc::now().into()));
    active.last_updated_id = Set(Some(1));
    active.update(db).await?;
    Ok(())
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StudyKeyImage { .. }")
    }
}
