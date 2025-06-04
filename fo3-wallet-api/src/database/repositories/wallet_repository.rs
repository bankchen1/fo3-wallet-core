//! SQLx-based Wallet repository implementation
//! 
//! Replaces the in-memory HashMap storage with persistent database operations

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error};

use crate::database::connection::DatabasePool;
use crate::error::ServiceError;
use fo3_wallet::account::Wallet;

/// Wallet entity for database storage
#[derive(Debug, Clone)]
pub struct WalletEntity {
    pub id: Uuid,
    pub name: String,
    pub encrypted_mnemonic: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Wallet repository trait
#[async_trait]
pub trait WalletRepository {
    type Error;

    async fn create_wallet(&self, wallet: &WalletEntity) -> Result<(), Self::Error>;
    async fn get_wallet_by_id(&self, id: Uuid) -> Result<Option<WalletEntity>, Self::Error>;
    async fn get_wallet_by_name(&self, name: &str) -> Result<Option<WalletEntity>, Self::Error>;
    async fn update_wallet(&self, wallet: &WalletEntity) -> Result<(), Self::Error>;
    async fn list_wallets(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<WalletEntity>, Self::Error>;
    async fn delete_wallet(&self, id: Uuid) -> Result<(), Self::Error>;
}

/// SQLx-based Wallet repository implementation
pub struct SqlxWalletRepository {
    pool: DatabasePool,
}

impl SqlxWalletRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for SqlxWalletRepository {
    type Error = ServiceError;

    async fn create_wallet(&self, wallet: &WalletEntity) -> Result<(), Self::Error> {
        info!("Creating wallet: {}", wallet.name);

        let query = r#"
            INSERT INTO wallets (id, name, encrypted_mnemonic, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(wallet.id)
                    .bind(&wallet.name)
                    .bind(&wallet.encrypted_mnemonic)
                    .bind(wallet.created_at)
                    .bind(wallet.updated_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to create wallet: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(wallet.id.to_string())
                    .bind(&wallet.name)
                    .bind(&wallet.encrypted_mnemonic)
                    .bind(wallet.created_at.to_rfc3339())
                    .bind(wallet.updated_at.to_rfc3339())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to create wallet: {}", e)))?;
            }
        }

        info!("Wallet created successfully: {}", wallet.id);
        Ok(())
    }

    async fn get_wallet_by_id(&self, id: Uuid) -> Result<Option<WalletEntity>, Self::Error> {
        info!("Fetching wallet by ID: {}", id);

        let query = "SELECT id, name, encrypted_mnemonic, created_at, updated_at FROM wallets WHERE id = $1";

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch wallet: {}", e)))?;

                if let Some(row) = row {
                    let wallet = self.row_to_wallet_postgres(&row)?;
                    Ok(Some(wallet))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(query)
                    .bind(id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch wallet: {}", e)))?;

                if let Some(row) = row {
                    let wallet = self.row_to_wallet_sqlite(&row)?;
                    Ok(Some(wallet))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn get_wallet_by_name(&self, name: &str) -> Result<Option<WalletEntity>, Self::Error> {
        info!("Fetching wallet by name: {}", name);

        let query = "SELECT id, name, encrypted_mnemonic, created_at, updated_at FROM wallets WHERE name = $1";

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch wallet: {}", e)))?;

                if let Some(row) = row {
                    let wallet = self.row_to_wallet_postgres(&row)?;
                    Ok(Some(wallet))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(query)
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch wallet: {}", e)))?;

                if let Some(row) = row {
                    let wallet = self.row_to_wallet_sqlite(&row)?;
                    Ok(Some(wallet))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn update_wallet(&self, wallet: &WalletEntity) -> Result<(), Self::Error> {
        info!("Updating wallet: {}", wallet.id);

        let query = r#"
            UPDATE wallets SET name = $2, encrypted_mnemonic = $3, updated_at = $4
            WHERE id = $1
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(wallet.id)
                    .bind(&wallet.name)
                    .bind(&wallet.encrypted_mnemonic)
                    .bind(wallet.updated_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to update wallet: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(wallet.id.to_string())
                    .bind(&wallet.name)
                    .bind(&wallet.encrypted_mnemonic)
                    .bind(wallet.updated_at.to_rfc3339())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to update wallet: {}", e)))?;
            }
        }

        info!("Wallet updated successfully: {}", wallet.id);
        Ok(())
    }

    async fn list_wallets(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<WalletEntity>, Self::Error> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        info!("Listing wallets with limit: {}, offset: {}", limit, offset);

        let query = r#"
            SELECT id, name, encrypted_mnemonic, created_at, updated_at
            FROM wallets
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to list wallets: {}", e)))?;

                let wallets = rows.iter()
                    .map(|row| self.row_to_wallet_postgres(row))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(wallets)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to list wallets: {}", e)))?;

                let wallets = rows.iter()
                    .map(|row| self.row_to_wallet_sqlite(row))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(wallets)
            }
        }
    }

    async fn delete_wallet(&self, id: Uuid) -> Result<(), Self::Error> {
        info!("Deleting wallet: {}", id);

        let query = "DELETE FROM wallets WHERE id = $1";

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to delete wallet: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to delete wallet: {}", e)))?;
            }
        }

        info!("Wallet deleted successfully: {}", id);
        Ok(())
    }
}

impl SqlxWalletRepository {
    /// Convert PostgreSQL row to WalletEntity
    fn row_to_wallet_postgres(&self, row: &sqlx::postgres::PgRow) -> Result<WalletEntity, ServiceError> {
        Ok(WalletEntity {
            id: row.try_get("id")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get id: {}", e)))?,
            name: row.try_get("name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get name: {}", e)))?,
            encrypted_mnemonic: row.try_get("encrypted_mnemonic")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get encrypted_mnemonic: {}", e)))?,
            created_at: row.try_get("created_at")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get created_at: {}", e)))?,
            updated_at: row.try_get("updated_at")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get updated_at: {}", e)))?,
        })
    }

    /// Convert SQLite row to WalletEntity
    fn row_to_wallet_sqlite(&self, row: &sqlx::sqlite::SqliteRow) -> Result<WalletEntity, ServiceError> {
        let id_str: String = row.try_get("id")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get id: {}", e)))?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse id UUID: {}", e)))?;

        let created_at_str: String = row.try_get("created_at")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get created_at: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse created_at: {}", e)))?
            .with_timezone(&Utc);

        let updated_at_str: String = row.try_get("updated_at")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get updated_at: {}", e)))?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse updated_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(WalletEntity {
            id,
            name: row.try_get("name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get name: {}", e)))?,
            encrypted_mnemonic: row.try_get("encrypted_mnemonic")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get encrypted_mnemonic: {}", e)))?,
            created_at,
            updated_at,
        })
    }
}
