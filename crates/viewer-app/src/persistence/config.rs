//! UC-008 repository/cache shared by admin and session routes.
use super::Persistence;
use moka::future::Cache;
use sea_orm::{ActiveValue::Set, QuerySelect, entity::prelude::*, sea_query::OnConflict};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize)]
#[sea_orm(table_name = "config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub value: String,
    pub description: Option<String>,
    pub datatype: Option<String>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone)]
pub struct ConfigStore {
    database: Persistence,
    cache: Cache<String, Model>,
    // Cover the DB read + cache fill and committed write + cache change together.
    // Concurrent misses may read together, but cannot repopulate after a write.
    access: Arc<RwLock<()>>,
}

impl ConfigStore {
    pub fn new(database: Persistence) -> Self {
        Self {
            database,
            cache: Cache::builder()
                .max_capacity(10)
                .time_to_live(Duration::from_secs(600 * 60))
                .build(),
            access: Arc::new(RwLock::new(())),
        }
    }

    pub async fn list(&self, start: u64, limit: u64) -> Result<(u64, Vec<Model>), DbErr> {
        let db = self.database.connection();
        let total = Entity::find().count(db).await?;
        let rows = Entity::find().offset(start).limit(limit).all(db).await?;
        Ok((total, rows))
    }

    pub async fn get(&self, id: &str) -> Result<Option<Model>, DbErr> {
        let _guard = self.access.read().await;
        if let Some(value) = self.cache.get(id).await {
            return Ok(Some(value));
        }
        let value = Entity::find_by_id(id)
            .one(self.database.connection())
            .await?;
        if let Some(value) = &value {
            self.cache.insert(id.to_owned(), value.clone()).await;
        }
        Ok(value)
    }

    pub async fn create(&self, value: Model) -> Result<(), DbErr> {
        let _guard = self.access.write().await;
        Entity::insert(ActiveModel::from(value.clone()))
            .on_conflict(
                OnConflict::column(Column::Id)
                    .update_columns([Column::Value, Column::Description, Column::Datatype])
                    .to_owned(),
            )
            .exec(self.database.connection())
            .await?;
        self.cache.insert(value.id.clone(), value).await;
        Ok(())
    }

    pub async fn update(
        &self,
        id: &str,
        value: Option<String>,
        description: Option<String>,
        datatype: Option<String>,
    ) -> Result<bool, DbErr> {
        let _guard = self.access.write().await;
        // Read authoritative state so a cached value cannot overwrite another write.
        let Some(current) = Entity::find_by_id(id)
            .one(self.database.connection())
            .await?
        else {
            self.cache.invalidate(id).await;
            return Ok(false);
        };
        let mut model: ActiveModel = current.into();
        if let Some(value) = value {
            model.value = Set(value);
        }
        if let Some(description) = description {
            model.description = Set(Some(description));
        }
        if let Some(datatype) = datatype {
            model.datatype = Set(Some(datatype));
        }
        model.update(self.database.connection()).await?;
        self.cache.invalidate(id).await;
        Ok(true)
    }

    pub async fn delete(&self, id: &str) -> Result<bool, DbErr> {
        let _guard = self.access.write().await;
        let deleted = Entity::delete_by_id(id)
            .exec(self.database.connection())
            .await?
            .rows_affected
            != 0;
        self.cache.invalidate(id).await;
        Ok(deleted)
    }
}
