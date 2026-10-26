//! PostgreSQL infrastructure. Business entities and migrations belong to later steps.
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
    TransactionTrait,
};
use sea_orm_migration::prelude::*;
use std::{env, time::Duration};

pub mod client;
pub mod client_permission;
mod m20261026_auth;
mod m20261026_permissions;
pub mod permission;
pub mod session;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261026_auth::Migration),
            Box::new(m20261026_permissions::Migration),
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
    /// Read credentials only from the environment. Never format connection errors/URLs.
    pub async fn connect_from_env() -> Result<Self, &'static str> {
        let raw = env::var("VIEWER_DATABASE_URL").map_err(|_| "VIEWER_DATABASE_URL is required")?;
        let mut url = url::Url::parse(&raw).map_err(|_| "Invalid VIEWER_DATABASE_URL")?;
        if !matches!(url.scheme(), "postgres" | "postgresql") || url.host_str().is_none() {
            return Err("VIEWER_DATABASE_URL must be a PostgreSQL URL with a host");
        }
        if let Ok(user) = env::var("VIEWER_DATABASE_USER") {
            url.set_username(&user)
                .map_err(|_| "Invalid database user")?;
        }
        if let Ok(password) = env::var("VIEWER_DATABASE_PASSWORD") {
            url.set_password(Some(&password))
                .map_err(|_| "Invalid database password")?;
        }
        let max = number("VIEWER_DATABASE_MAX_CONNECTIONS", 10)?;
        let timeout = number("VIEWER_DATABASE_TIMEOUT_SECONDS", 5)?;
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

fn number(name: &str, default: u32) -> Result<u32, &'static str> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("Database pool size and timeout must be positive integers"),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(_) => Err("Invalid database pool configuration"),
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
