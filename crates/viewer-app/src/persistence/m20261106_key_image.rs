use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE study_key_image (
    id varchar PRIMARY KEY NOT NULL, series_iuid varchar(64) NOT NULL,
    num_of_image integer NOT NULL DEFAULT 0, store_url text NOT NULL, store_auth text,
    succeed boolean NOT NULL DEFAULT false, num_of_retry integer NOT NULL DEFAULT 0,
    description text, created_time timestamptz NOT NULL, creator_id integer NOT NULL DEFAULT 1,
    last_updated_time timestamptz, last_updated_id integer);",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE study_key_image")
            .await?;
        Ok(())
    }
}
