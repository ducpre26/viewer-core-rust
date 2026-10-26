use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE client (
                id varchar(64) PRIMARY KEY,
                password varchar(64) NOT NULL,
                ip_allowed text
            );
            CREATE TABLE session (
                id varchar(64) PRIMARY KEY,
                store_url text,
                store_auth text,
                studies text NOT NULL,
                user_id varchar(32),
                anonymized boolean NOT NULL DEFAULT false,
                shared_session boolean NOT NULL DEFAULT true,
                share_key varchar(16),
                expired_time timestamptz NOT NULL,
                permissions text[]
            );
            CREATE INDEX idx_session_expired_time ON session(expired_time);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE session; DROP TABLE client;")
            .await?;
        Ok(())
    }
}
