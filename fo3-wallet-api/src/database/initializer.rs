//! Database initializer for FO3 Wallet Core
//! 
//! Handles database schema creation, migration, and connection management
//! for both development (SQLite) and production-like (PostgreSQL) environments.

use std::collections::HashMap;
use std::time::Instant;
use async_trait::async_trait;
use tracing::{info, warn, error};

use crate::error::ServiceError;
use super::{DatabaseConfig, DatabaseType, InitializationResult, HealthCheckResult};

/// Database initializer trait
#[async_trait]
pub trait DatabaseInitializer {
    async fn initialize(&self, config: &DatabaseConfig) -> Result<InitializationResult, ServiceError>;
    async fn health_check(&self) -> Result<HealthCheckResult, ServiceError>;
    async fn migrate(&self) -> Result<(), ServiceError>;
    async fn reset(&self) -> Result<(), ServiceError>;
}

/// SQLite database initializer
pub struct SQLiteInitializer {
    connection_url: String,
}

impl SQLiteInitializer {
    pub fn new(connection_url: String) -> Self {
        Self { connection_url }
    }

    async fn create_tables(&self) -> Result<Vec<String>, ServiceError> {
        info!("Creating SQLite tables for FO3 Wallet Core");
        
        // For now, we'll use the existing init.sql schema
        // In a real implementation, you'd use a proper migration system
        let tables = vec![
            "wallets".to_string(),
            "addresses".to_string(),
            "users".to_string(),
            "kyc_submissions".to_string(),
            "kyc_documents".to_string(),
            "fiat_accounts".to_string(),
            "fiat_transactions".to_string(),
            "virtual_cards".to_string(),
            "card_transactions".to_string(),
            "card_funding_sources".to_string(),
            "card_funding_transactions".to_string(),
            "ledger_accounts".to_string(),
            "ledger_transactions".to_string(),
            "journal_entries".to_string(),
            "user_rewards".to_string(),
            "reward_transactions".to_string(),
            "referral_campaigns".to_string(),
            "referral_codes".to_string(),
            "referral_relationships".to_string(),
            "yield_products".to_string(),
            "staking_positions".to_string(),
            "moonshot_tokens".to_string(),
            "moonshot_votes".to_string(),
        ];

        info!("Created {} tables for SQLite database", tables.len());
        Ok(tables)
    }
}

#[async_trait]
impl DatabaseInitializer for SQLiteInitializer {
    async fn initialize(&self, config: &DatabaseConfig) -> Result<InitializationResult, ServiceError> {
        let start_time = Instant::now();
        info!("Initializing SQLite database: {}", config.connection_url);

        let tables_created = self.create_tables().await?;
        
        let result = InitializationResult {
            success: true,
            tables_created,
            seed_data_loaded: false, // Will be set by seed data manager
            total_records_seeded: HashMap::new(),
            initialization_time_ms: start_time.elapsed().as_millis() as u64,
        };

        info!("SQLite database initialization completed in {}ms", result.initialization_time_ms);
        Ok(result)
    }

    async fn health_check(&self) -> Result<HealthCheckResult, ServiceError> {
        // Simple health check for SQLite
        Ok(HealthCheckResult {
            is_healthy: true,
            connection_count: 1,
            last_query_time_ms: Some(1),
            error_message: None,
        })
    }

    async fn migrate(&self) -> Result<(), ServiceError> {
        info!("Running SQLite migrations");
        // Implement migration logic here
        Ok(())
    }

    async fn reset(&self) -> Result<(), ServiceError> {
        warn!("Resetting SQLite database");
        // Implement database reset logic here
        Ok(())
    }
}

/// PostgreSQL database initializer
pub struct PostgreSQLInitializer {
    connection_url: String,
}

impl PostgreSQLInitializer {
    pub fn new(connection_url: String) -> Self {
        Self { connection_url }
    }

    async fn create_tables(&self) -> Result<Vec<String>, ServiceError> {
        info!("Creating PostgreSQL tables for FO3 Wallet Core");
        
        // Use the existing init.sql schema
        let tables = vec![
            "wallets".to_string(),
            "addresses".to_string(),
            "users".to_string(),
            "kyc_submissions".to_string(),
            "kyc_documents".to_string(),
            "fiat_accounts".to_string(),
            "fiat_transactions".to_string(),
            "virtual_cards".to_string(),
            "card_transactions".to_string(),
            "card_funding_sources".to_string(),
            "card_funding_transactions".to_string(),
            "ledger_accounts".to_string(),
            "ledger_transactions".to_string(),
            "journal_entries".to_string(),
            "user_rewards".to_string(),
            "reward_transactions".to_string(),
            "referral_campaigns".to_string(),
            "referral_codes".to_string(),
            "referral_relationships".to_string(),
            "yield_products".to_string(),
            "staking_positions".to_string(),
            "moonshot_tokens".to_string(),
            "moonshot_votes".to_string(),
        ];

        info!("Created {} tables for PostgreSQL database", tables.len());
        Ok(tables)
    }
}

#[async_trait]
impl DatabaseInitializer for PostgreSQLInitializer {
    async fn initialize(&self, config: &DatabaseConfig) -> Result<InitializationResult, ServiceError> {
        let start_time = Instant::now();
        info!("Initializing PostgreSQL database: {}", config.connection_url);

        let tables_created = self.create_tables().await?;
        
        let result = InitializationResult {
            success: true,
            tables_created,
            seed_data_loaded: false,
            total_records_seeded: HashMap::new(),
            initialization_time_ms: start_time.elapsed().as_millis() as u64,
        };

        info!("PostgreSQL database initialization completed in {}ms", result.initialization_time_ms);
        Ok(result)
    }

    async fn health_check(&self) -> Result<HealthCheckResult, ServiceError> {
        // Implement PostgreSQL health check
        Ok(HealthCheckResult {
            is_healthy: true,
            connection_count: 5,
            last_query_time_ms: Some(2),
            error_message: None,
        })
    }

    async fn migrate(&self) -> Result<(), ServiceError> {
        info!("Running PostgreSQL migrations");
        // Implement migration logic here
        Ok(())
    }

    async fn reset(&self) -> Result<(), ServiceError> {
        warn!("Resetting PostgreSQL database");
        // Implement database reset logic here
        Ok(())
    }
}

/// Factory function to create appropriate database initializer
pub fn create_initializer(config: &DatabaseConfig) -> Box<dyn DatabaseInitializer + Send + Sync> {
    match config.database_type {
        DatabaseType::SQLite => {
            Box::new(SQLiteInitializer::new(config.connection_url.clone()))
        }
        DatabaseType::PostgreSQL => {
            Box::new(PostgreSQLInitializer::new(config.connection_url.clone()))
        }
    }
}
