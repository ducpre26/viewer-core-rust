use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE permission (
                id varchar(64) PRIMARY KEY,
                name varchar(64) NOT NULL UNIQUE,
                description text
            );
            INSERT INTO permission (id, name, description) VALUES
                ('MODIFY_OVERLAY_CONFIG', 'Modify Overlay Config', 'Cho phép sửa cấu hình overlay DICOM'),
                ('ADMIN', 'Admin', 'Có quyền thực hiện tất cả các thao tác');
            CREATE TABLE client_permission (
                client_id varchar(64) NOT NULL REFERENCES client(id) ON DELETE CASCADE,
                permission_id varchar(64) NOT NULL REFERENCES permission(id) ON DELETE CASCADE,
                PRIMARY KEY (client_id, permission_id)
            );"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE client_permission; DROP TABLE permission;")
            .await?;
        Ok(())
    }
}
