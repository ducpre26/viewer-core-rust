use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE printer (
    id                serial PRIMARY KEY,
    printer_name      varchar(255) NOT NULL,
    host              varchar(255) NOT NULL,
    port              integer NOT NULL,
    aetitle           varchar(255) NOT NULL,
    called_aetitle    varchar(255) NOT NULL,
    description       text,
    enabled           boolean NOT NULL,

    created_time      timestamp with time zone NOT NULL DEFAULT now(),
    creator_id        integer NOT NULL DEFAULT 1,
    last_updated_time timestamp with time zone,
    last_updated_id   integer
);",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE printer")
            .await?;
        Ok(())
    }
}
