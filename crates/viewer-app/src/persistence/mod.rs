//! PostgreSQL infrastructure. Business entities and migrations belong to later steps.
use crate::config::DatabaseConfig;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
    TransactionTrait,
};
use sea_orm_migration::prelude::*;
use std::time::Duration;

pub mod client;
pub mod client_permission;
pub mod config;
pub mod key_image;
mod m20261026_auth;
mod m20261026_config;
mod m20261026_permissions;
mod m20261106_key_image;
pub mod permission;
pub mod session;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261026_auth::Migration),
            Box::new(m20261026_permissions::Migration),
            Box::new(m20261026_config::Migration),
            Box::new(m20261106_key_image::Migration),
        ]
    }
}

/// One shared pool per process; repositories borrow this connection or a transaction.
#[derive(Clone)]
pub struct Persistence {
    connection: DatabaseConnection,
    query_timeout: Duration,
}

impl Persistence {
    /// Connect using settings already bound from the application's TOML registry.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self, &'static str> {
        let url = config.connection_url()?;
        let max = config.max_connections;
        let timeout = config.timeout_seconds;
        let mut options = ConnectOptions::new(url.to_string());
        options
            .max_connections(max)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(timeout.into()))
            .acquire_timeout(Duration::from_secs(timeout.into()))
            .sqlx_logging(false);
        let connection = Database::connect(options).await.map_err(connection_error)?;
        let persistence = Self {
            connection,
            query_timeout: Duration::from_secs(timeout.into()),
        };
        if persistence.check().await.is_err() {
            let _ = persistence.close().await;
            return Err("PostgreSQL startup SELECT 1 failed");
        }
        if persistence.migrate().await.is_err() {
            let _ = persistence.close().await;
            return Err(
                "PostgreSQL migration failed; check database DDL permissions and migration state",
            );
        }
        Ok(persistence)
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    async fn migrate(&self) -> Result<(), DbErr> {
        // Serialize startup DDL, including creation of SeaORM's own tracking table.
        let migration = async {
            let transaction = self.connection.begin().await?;
            transaction
                .execute(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT pg_advisory_xact_lock(1635018794)".to_owned(),
                ))
                .await?;
            Migrator::up(&transaction, None).await?;
            transaction.commit().await
        };
        tokio::time::timeout(self.query_timeout, migration)
            .await
            .map_err(|_| DbErr::Custom("PostgreSQL migration timed out".into()))?
    }

    pub async fn check(&self) -> Result<(), DbErr> {
        tokio::time::timeout(
            self.query_timeout,
            self.connection
                .execute(Statement::from_string(DbBackend::Postgres, "SELECT 1")),
        )
        .await
        .map_err(|_| DbErr::Custom("PostgreSQL readiness timed out".into()))??;
        Ok(())
    }

    pub async fn close(self) -> Result<(), DbErr> {
        self.connection.close().await
    }
}

fn connection_error(error: DbErr) -> &'static str {
    // Inspect internally, but never expose the driver's potentially sensitive text.
    let text = error.to_string().to_lowercase();
    if text.contains("password authentication failed") || text.contains("28p01") {
        "PostgreSQL authentication failed; check database credentials"
    } else if text.contains("does not exist") || text.contains("3d000") {
        "PostgreSQL database does not exist; create the configured database first"
    } else {
        "PostgreSQL connection failed; check host, port, credentials, TLS and server availability"
    }
}
