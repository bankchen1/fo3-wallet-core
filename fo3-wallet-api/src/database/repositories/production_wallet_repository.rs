//! Production PostgreSQL Wallet Repository with User Isolation
//!
//! Implements real database operations with multi-user support, RBAC enforcement,
//! and comprehensive audit logging for production use.

use async_trait::async_trait;
use sqlx::{Row, FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error, warn, instrument};
use rust_decimal::Decimal;

use crate::database::connection::DatabasePool;
use crate::models::user_context::{UserContext, Permission};
use crate::error::ServiceError;

/// Production wallet entity with user isolation
#[derive(Debug, Clone, FromRow)]
pub struct ProductionWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub encrypted_mnemonic: String,
    pub balance_usd: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Production wallet repository with user isolation and RBAC
pub struct ProductionWalletRepository {
    pool: DatabasePool,
}

impl ProductionWalletRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// Get PostgreSQL pool reference
    fn get_pg_pool(&self) -> Result<&PgPool, ServiceError> {
        match &self.pool {
            DatabasePool::Postgres(pool) => Ok(pool),
            _ => Err(ServiceError::DatabaseError(
                "PostgreSQL pool required for production operations".to_string()
            )),
        }
    }

    /// Validate user permissions for wallet operations
    fn validate_permission(&self, user_context: &UserContext, permission: Permission) -> Result<(), ServiceError> {
        if !user_context.has_permission(permission) {
            return Err(ServiceError::AuthorizationError(
                format!("User {} lacks permission {:?}", user_context.user_id, permission)
            ));
        }
        Ok(())
    }

    /// Create a new wallet with user isolation
    #[instrument(skip(self, user_context))]
    pub async fn create_wallet(
        &self,
        user_context: &UserContext,
        name: String,
        encrypted_mnemonic: String,
    ) -> Result<ProductionWallet, ServiceError> {
        self.validate_permission(user_context, Permission::WalletCreate)?;
        
        let pool = self.get_pg_pool()?;
        let wallet_id = Uuid::new_v4();
        
        info!(
            user_id = %user_context.user_id,
            wallet_id = %wallet_id,
            wallet_name = %name,
            "Creating new wallet"
        );

        let wallet = sqlx::query_as::<_, ProductionWallet>(
            r#"
            INSERT INTO wallets (id, user_id, name, encrypted_mnemonic, balance_usd, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            RETURNING id, user_id, name, encrypted_mnemonic, balance_usd, is_active, created_at, updated_at
            "#
        )
        .bind(&wallet_id)
        .bind(&user_context.user_id)
        .bind(&name)
        .bind(&encrypted_mnemonic)
        .bind(Decimal::ZERO)
        .bind(true)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create wallet");
            ServiceError::DatabaseError(format!("Failed to create wallet: {}", e))
        })?;

        // Log audit event
        self.log_audit_event(
            user_context,
            "wallet_created",
            &wallet.id,
            &format!("Created wallet: {}", name),
        ).await?;

        info!(
            user_id = %user_context.user_id,
            wallet_id = %wallet.id,
            "Wallet created successfully"
        );

        Ok(wallet)
    }

    /// Get wallet by ID with user isolation
    #[instrument(skip(self, user_context))]
    pub async fn get_wallet(
        &self,
        user_context: &UserContext,
        wallet_id: Uuid,
    ) -> Result<Option<ProductionWallet>, ServiceError> {
        self.validate_permission(user_context, Permission::WalletRead)?;
        
        let pool = self.get_pg_pool()?;

        let wallet = sqlx::query_as::<_, ProductionWallet>(
            r#"
            SELECT id, user_id, name, encrypted_mnemonic, balance_usd, is_active, created_at, updated_at
            FROM wallets
            WHERE id = $1 AND user_id = $2 AND is_active = true
            "#
        )
        .bind(&wallet_id)
        .bind(&user_context.user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!(error = %e, wallet_id = %wallet_id, "Failed to fetch wallet");
            ServiceError::DatabaseError(format!("Failed to fetch wallet: {}", e))
        })?;

        if wallet.is_some() {
            info!(
                user_id = %user_context.user_id,
                wallet_id = %wallet_id,
                "Wallet retrieved successfully"
            );
        }

        Ok(wallet)
    }

    /// List all wallets for a user
    #[instrument(skip(self, user_context))]
    pub async fn list_user_wallets(
        &self,
        user_context: &UserContext,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ProductionWallet>, ServiceError> {
        self.validate_permission(user_context, Permission::WalletRead)?;
        
        let pool = self.get_pg_pool()?;
        let limit = limit.unwrap_or(50).min(100); // Max 100 wallets per request
        let offset = offset.unwrap_or(0);

        let wallets = sqlx::query_as::<_, ProductionWallet>(
            r#"
            SELECT id, user_id, name, encrypted_mnemonic, balance_usd, is_active, created_at, updated_at
            FROM wallets
            WHERE user_id = $1 AND is_active = true
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&user_context.user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!(error = %e, user_id = %user_context.user_id, "Failed to list wallets");
            ServiceError::DatabaseError(format!("Failed to list wallets: {}", e))
        })?;

        info!(
            user_id = %user_context.user_id,
            wallet_count = wallets.len(),
            "Listed user wallets"
        );

        Ok(wallets)
    }

    /// Update wallet balance
    #[instrument(skip(self, user_context))]
    pub async fn update_wallet_balance(
        &self,
        user_context: &UserContext,
        wallet_id: Uuid,
        new_balance: Decimal,
    ) -> Result<ProductionWallet, ServiceError> {
        self.validate_permission(user_context, Permission::WalletUpdate)?;
        
        let pool = self.get_pg_pool()?;

        let wallet = sqlx::query_as::<_, ProductionWallet>(
            r#"
            UPDATE wallets
            SET balance_usd = $1, updated_at = NOW()
            WHERE id = $2 AND user_id = $3 AND is_active = true
            RETURNING id, user_id, name, encrypted_mnemonic, balance_usd, is_active, created_at, updated_at
            "#
        )
        .bind(&new_balance)
        .bind(&wallet_id)
        .bind(&user_context.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(error = %e, wallet_id = %wallet_id, "Failed to update wallet balance");
            ServiceError::DatabaseError(format!("Failed to update wallet balance: {}", e))
        })?;

        // Log audit event
        self.log_audit_event(
            user_context,
            "wallet_balance_updated",
            &wallet_id,
            &format!("Updated balance to: {}", new_balance),
        ).await?;

        info!(
            user_id = %user_context.user_id,
            wallet_id = %wallet_id,
            new_balance = %new_balance,
            "Wallet balance updated"
        );

        Ok(wallet)
    }

    /// Soft delete wallet (admin only)
    #[instrument(skip(self, user_context))]
    pub async fn delete_wallet(
        &self,
        user_context: &UserContext,
        wallet_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.validate_permission(user_context, Permission::WalletDelete)?;
        
        let pool = self.get_pg_pool()?;

        let result = sqlx::query(
            r#"
            UPDATE wallets
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            "#
        )
        .bind(&wallet_id)
        .bind(&user_context.user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!(error = %e, wallet_id = %wallet_id, "Failed to delete wallet");
            ServiceError::DatabaseError(format!("Failed to delete wallet: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::NotFoundError(
                format!("Wallet {} not found or already deleted", wallet_id)
            ));
        }

        // Log audit event
        self.log_audit_event(
            user_context,
            "wallet_deleted",
            &wallet_id,
            "Wallet soft deleted",
        ).await?;

        info!(
            user_id = %user_context.user_id,
            wallet_id = %wallet_id,
            "Wallet deleted successfully"
        );

        Ok(())
    }

    /// Get wallet statistics for user
    #[instrument(skip(self, user_context))]
    pub async fn get_wallet_statistics(
        &self,
        user_context: &UserContext,
    ) -> Result<WalletStatistics, ServiceError> {
        self.validate_permission(user_context, Permission::WalletRead)?;
        
        let pool = self.get_pg_pool()?;

        let stats = sqlx::query_as::<_, WalletStatistics>(
            r#"
            SELECT 
                COUNT(*) as total_wallets,
                COALESCE(SUM(balance_usd), 0) as total_balance_usd,
                COALESCE(AVG(balance_usd), 0) as average_balance_usd,
                COALESCE(MAX(balance_usd), 0) as max_balance_usd
            FROM wallets
            WHERE user_id = $1 AND is_active = true
            "#
        )
        .bind(&user_context.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(error = %e, user_id = %user_context.user_id, "Failed to get wallet statistics");
            ServiceError::DatabaseError(format!("Failed to get wallet statistics: {}", e))
        })?;

        Ok(stats)
    }

    /// Log audit event for wallet operations
    async fn log_audit_event(
        &self,
        user_context: &UserContext,
        event_type: &str,
        wallet_id: &Uuid,
        description: &str,
    ) -> Result<(), ServiceError> {
        let pool = self.get_pg_pool()?;

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, user_id, event_type, resource_type, resource_id, description, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#
        )
        .bind(Uuid::new_v4())
        .bind(&user_context.user_id)
        .bind(event_type)
        .bind("wallet")
        .bind(wallet_id)
        .bind(description)
        .execute(pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to log audit event");
            ServiceError::DatabaseError(format!("Failed to log audit event: {}", e))
        })?;

        Ok(())
    }
}

/// Wallet statistics for user dashboard
#[derive(Debug, FromRow)]
pub struct WalletStatistics {
    pub total_wallets: i64,
    pub total_balance_usd: Decimal,
    pub average_balance_usd: Decimal,
    pub max_balance_usd: Decimal,
}
