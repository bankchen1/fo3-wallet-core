//! Database connection management with SQLx
//! 
//! Provides connection pooling and database operations for both PostgreSQL and SQLite

use sqlx::{Pool, Postgres, Sqlite, Row};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Duration;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::ServiceError;

/// Database connection pool enum supporting both PostgreSQL and SQLite
#[derive(Clone)]
pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
}

impl DatabasePool {
    /// Create a new PostgreSQL connection pool
    pub async fn new_postgres(database_url: &str, max_connections: u32) -> Result<Self, ServiceError> {
        info!("Creating PostgreSQL connection pool with {} max connections", max_connections);
        
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to create PostgreSQL pool: {}", e)))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to run migrations: {}", e)))?;

        info!("PostgreSQL connection pool created successfully");
        Ok(DatabasePool::Postgres(pool))
    }

    /// Create a new SQLite connection pool
    pub async fn new_sqlite(database_url: &str, max_connections: u32) -> Result<Self, ServiceError> {
        info!("Creating SQLite connection pool with {} max connections", max_connections);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to create SQLite pool: {}", e)))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to run migrations: {}", e)))?;

        info!("SQLite connection pool created successfully");
        Ok(DatabasePool::Sqlite(pool))
    }

    /// Health check for the database connection
    pub async fn health_check(&self) -> Result<(), ServiceError> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("PostgreSQL health check failed: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("SQLite health check failed: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Get connection pool statistics
    pub fn get_stats(&self) -> ConnectionStats {
        match self {
            DatabasePool::Postgres(pool) => ConnectionStats {
                size: pool.size(),
                idle: pool.num_idle(),
                is_closed: pool.is_closed(),
            },
            DatabasePool::Sqlite(pool) => ConnectionStats {
                size: pool.size(),
                idle: pool.num_idle(),
                is_closed: pool.is_closed(),
            },
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub size: u32,
    pub idle: usize,
    pub is_closed: bool,
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub enable_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 10,
            connection_timeout_seconds: 30,
            enable_logging: true,
        }
    }
}

impl DatabaseConfig {
    /// Create configuration for PostgreSQL
    pub fn postgres(database_url: String) -> Self {
        Self {
            database_url,
            max_connections: 20,
            connection_timeout_seconds: 30,
            enable_logging: true,
        }
    }

    /// Create configuration for SQLite
    pub fn sqlite(database_url: String) -> Self {
        Self {
            database_url,
            max_connections: 5,
            connection_timeout_seconds: 30,
            enable_logging: true,
        }
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:fo3_wallet.db".to_string());
        
        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10);

        Self {
            database_url,
            max_connections,
            connection_timeout_seconds: 30,
            enable_logging: true,
        }
    }
}

/// Initialize database connection pool based on configuration
pub async fn initialize_database(config: &DatabaseConfig) -> Result<DatabasePool, ServiceError> {
    info!("Initializing database with URL: {}", config.database_url);

    if config.database_url.starts_with("postgres://") || config.database_url.starts_with("postgresql://") {
        DatabasePool::new_postgres(&config.database_url, config.max_connections).await
    } else if config.database_url.starts_with("sqlite:") {
        DatabasePool::new_sqlite(&config.database_url, config.max_connections).await
    } else {
        Err(ServiceError::DatabaseError(
            "Unsupported database URL format. Use postgres:// or sqlite:".to_string()
        ))
    }
}

/// Database transaction helper
pub struct DatabaseTransaction<'a> {
    pool: &'a DatabasePool,
}

impl<'a> DatabaseTransaction<'a> {
    pub fn new(pool: &'a DatabasePool) -> Self {
        Self { pool }
    }

    /// Execute a query and return the number of affected rows
    pub async fn execute(&self, query: &str, params: &[&(dyn sqlx::Encode<'_, sqlx::Any> + sqlx::Type<sqlx::Any> + Sync)]) -> Result<u64, ServiceError> {
        match self.pool {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(query)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Query execution failed: {}", e)))?;
                Ok(result.rows_affected())
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(query)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Query execution failed: {}", e)))?;
                Ok(result.rows_affected())
            }
        }
    }
}
