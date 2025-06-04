//! Database initialization and seeding module
//!
//! Provides comprehensive database setup and test data seeding for local validation
//! of the FO3 Wallet Core system. Supports both SQLite (development) and PostgreSQL
//! (production-like testing) environments.

pub mod seed_data;
pub mod initializer;
pub mod connection;
pub mod repositories;
pub mod performance;

pub use seed_data::{SeedDataManager, SeedDataConfig};
pub use initializer::{DatabaseInitializer, DatabaseConfig, DatabaseType};
pub use connection::{DatabasePool, DatabaseConfig as ConnectionConfig, initialize_database};

use crate::error::ServiceError;
use std::collections::HashMap;
use uuid::Uuid;

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_type: DatabaseType,
    pub connection_url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub enable_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::SQLite,
            connection_url: "sqlite://./fo3_wallet_dev.db".to_string(),
            max_connections: 10,
            connection_timeout_seconds: 30,
            enable_logging: true,
        }
    }
}

/// Supported database types
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
}

/// Database initialization result
#[derive(Debug)]
pub struct InitializationResult {
    pub success: bool,
    pub tables_created: Vec<String>,
    pub seed_data_loaded: bool,
    pub total_records_seeded: HashMap<String, usize>,
    pub initialization_time_ms: u64,
}

/// Database health check result
#[derive(Debug)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub connection_count: u32,
    pub last_query_time_ms: Option<u64>,
    pub error_message: Option<String>,
}
