//! Transaction Manager for Cross-Service Data Consistency
//! 
//! Implements distributed transaction patterns and rollback mechanisms

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};

use crate::error::ServiceError;
use crate::database::connection::DatabasePool;

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    InProgress,
    Committed,
    RolledBack,
    Failed,
}

/// Transaction operation for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOperation {
    pub operation_id: String,
    pub service_name: String,
    pub operation_type: String,
    pub operation_data: serde_json::Value,
    pub rollback_data: Option<serde_json::Value>,
    pub executed_at: DateTime<Utc>,
}

/// Transaction context for managing distributed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContext {
    pub transaction_id: String,
    pub user_id: Option<String>,
    pub status: TransactionStatus,
    pub operations: Vec<TransactionOperation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Transaction result
#[derive(Debug, Clone)]
pub struct TransactionResult {
    pub success: bool,
    pub transaction_id: String,
    pub operations_completed: usize,
    pub operations_rolled_back: usize,
    pub error_message: Option<String>,
    pub completed_at: DateTime<Utc>,
}

/// Transaction manager for coordinating cross-service operations
pub struct TransactionManager {
    /// Active transactions
    active_transactions: Arc<RwLock<HashMap<String, TransactionContext>>>,
    
    /// Database pool for transaction logging
    database_pool: DatabasePool,
    
    /// Default transaction timeout in seconds
    default_timeout_seconds: i64,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(database_pool: DatabasePool) -> Self {
        Self {
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            database_pool,
            default_timeout_seconds: 300, // 5 minutes default timeout
        }
    }

    /// Begin a new distributed transaction
    #[instrument(skip(self))]
    pub async fn begin_transaction(
        &self,
        user_id: Option<String>,
        timeout_seconds: Option<i64>,
    ) -> Result<String, ServiceError> {
        let transaction_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let timeout_duration = timeout_seconds.unwrap_or(self.default_timeout_seconds);
        
        let context = TransactionContext {
            transaction_id: transaction_id.clone(),
            user_id,
            status: TransactionStatus::Pending,
            operations: Vec::new(),
            created_at: now,
            updated_at: now,
            timeout_at: now + chrono::Duration::seconds(timeout_duration),
            metadata: HashMap::new(),
        };

        // Store in active transactions
        {
            let mut transactions = self.active_transactions.write().await;
            transactions.insert(transaction_id.clone(), context.clone());
        }

        // Log transaction start to database
        self.log_transaction_event(&transaction_id, "transaction_started", None).await?;

        info!("Started distributed transaction: {}", transaction_id);
        Ok(transaction_id)
    }

    /// Add an operation to the transaction
    #[instrument(skip(self))]
    pub async fn add_operation(
        &self,
        transaction_id: &str,
        service_name: String,
        operation_type: String,
        operation_data: serde_json::Value,
        rollback_data: Option<serde_json::Value>,
    ) -> Result<String, ServiceError> {
        let operation_id = Uuid::new_v4().to_string();
        let operation = TransactionOperation {
            operation_id: operation_id.clone(),
            service_name,
            operation_type,
            operation_data,
            rollback_data,
            executed_at: Utc::now(),
        };

        // Update transaction context
        {
            let mut transactions = self.active_transactions.write().await;
            if let Some(context) = transactions.get_mut(transaction_id) {
                context.operations.push(operation.clone());
                context.updated_at = Utc::now();
                context.status = TransactionStatus::InProgress;
            } else {
                return Err(ServiceError::NotFound(format!("Transaction not found: {}", transaction_id)));
            }
        }

        // Log operation to database
        self.log_transaction_event(
            transaction_id,
            "operation_added",
            Some(serde_json::json!({
                "operation_id": operation_id,
                "service_name": operation.service_name,
                "operation_type": operation.operation_type
            }))
        ).await?;

        info!("Added operation {} to transaction {}", operation_id, transaction_id);
        Ok(operation_id)
    }

    /// Commit the transaction
    #[instrument(skip(self))]
    pub async fn commit_transaction(&self, transaction_id: &str) -> Result<TransactionResult, ServiceError> {
        let context = {
            let mut transactions = self.active_transactions.write().await;
            if let Some(mut context) = transactions.remove(transaction_id) {
                context.status = TransactionStatus::Committed;
                context.updated_at = Utc::now();
                context
            } else {
                return Err(ServiceError::NotFound(format!("Transaction not found: {}", transaction_id)));
            }
        };

        // Log transaction commit
        self.log_transaction_event(transaction_id, "transaction_committed", None).await?;

        let result = TransactionResult {
            success: true,
            transaction_id: transaction_id.to_string(),
            operations_completed: context.operations.len(),
            operations_rolled_back: 0,
            error_message: None,
            completed_at: Utc::now(),
        };

        info!("Committed transaction {} with {} operations", transaction_id, context.operations.len());
        Ok(result)
    }

    /// Rollback the transaction
    #[instrument(skip(self))]
    pub async fn rollback_transaction(
        &self,
        transaction_id: &str,
        reason: Option<String>,
    ) -> Result<TransactionResult, ServiceError> {
        let context = {
            let mut transactions = self.active_transactions.write().await;
            if let Some(mut context) = transactions.remove(transaction_id) {
                context.status = TransactionStatus::RolledBack;
                context.updated_at = Utc::now();
                context
            } else {
                return Err(ServiceError::NotFound(format!("Transaction not found: {}", transaction_id)));
            }
        };

        // Execute rollback operations in reverse order
        let mut operations_rolled_back = 0;
        for operation in context.operations.iter().rev() {
            if let Some(rollback_data) = &operation.rollback_data {
                match self.execute_rollback_operation(operation, rollback_data).await {
                    Ok(_) => {
                        operations_rolled_back += 1;
                        info!("Rolled back operation: {}", operation.operation_id);
                    }
                    Err(e) => {
                        error!("Failed to rollback operation {}: {}", operation.operation_id, e);
                    }
                }
            }
        }

        // Log transaction rollback
        self.log_transaction_event(
            transaction_id,
            "transaction_rolled_back",
            Some(serde_json::json!({
                "reason": reason,
                "operations_rolled_back": operations_rolled_back
            }))
        ).await?;

        let result = TransactionResult {
            success: true,
            transaction_id: transaction_id.to_string(),
            operations_completed: context.operations.len(),
            operations_rolled_back,
            error_message: reason,
            completed_at: Utc::now(),
        };

        info!("Rolled back transaction {} ({} operations)", transaction_id, operations_rolled_back);
        Ok(result)
    }

    /// Get transaction status
    pub async fn get_transaction_status(&self, transaction_id: &str) -> Result<TransactionContext, ServiceError> {
        let transactions = self.active_transactions.read().await;
        transactions.get(transaction_id)
            .cloned()
            .ok_or_else(|| ServiceError::NotFound(format!("Transaction not found: {}", transaction_id)))
    }

    /// Execute a rollback operation
    async fn execute_rollback_operation(
        &self,
        operation: &TransactionOperation,
        rollback_data: &serde_json::Value,
    ) -> Result<(), ServiceError> {
        // In a real implementation, this would call the appropriate service
        // to execute the rollback operation based on the service_name and operation_type
        
        match operation.service_name.as_str() {
            "wallet_service" => self.rollback_wallet_operation(operation, rollback_data).await,
            "kyc_service" => self.rollback_kyc_operation(operation, rollback_data).await,
            "card_service" => self.rollback_card_operation(operation, rollback_data).await,
            _ => {
                warn!("Unknown service for rollback: {}", operation.service_name);
                Ok(())
            }
        }
    }

    /// Rollback wallet operation
    async fn rollback_wallet_operation(
        &self,
        operation: &TransactionOperation,
        rollback_data: &serde_json::Value,
    ) -> Result<(), ServiceError> {
        info!("Rolling back wallet operation: {} with data: {}", operation.operation_type, rollback_data);
        // Implement wallet-specific rollback logic
        Ok(())
    }

    /// Rollback KYC operation
    async fn rollback_kyc_operation(
        &self,
        operation: &TransactionOperation,
        rollback_data: &serde_json::Value,
    ) -> Result<(), ServiceError> {
        info!("Rolling back KYC operation: {} with data: {}", operation.operation_type, rollback_data);
        // Implement KYC-specific rollback logic
        Ok(())
    }

    /// Rollback card operation
    async fn rollback_card_operation(
        &self,
        operation: &TransactionOperation,
        rollback_data: &serde_json::Value,
    ) -> Result<(), ServiceError> {
        info!("Rolling back card operation: {} with data: {}", operation.operation_type, rollback_data);
        // Implement card-specific rollback logic
        Ok(())
    }

    /// Log transaction event to database
    async fn log_transaction_event(
        &self,
        transaction_id: &str,
        event_type: &str,
        event_data: Option<serde_json::Value>,
    ) -> Result<(), ServiceError> {
        // In a real implementation, this would insert into a transaction_log table
        info!("Transaction log: {} - {} - {:?}", transaction_id, event_type, event_data);
        Ok(())
    }

    /// Cleanup expired transactions
    pub async fn cleanup_expired_transactions(&self) -> Result<usize, ServiceError> {
        let now = Utc::now();
        let mut expired_transactions = Vec::new();

        // Find expired transactions
        {
            let transactions = self.active_transactions.read().await;
            for (id, context) in transactions.iter() {
                if context.timeout_at < now && context.status == TransactionStatus::InProgress {
                    expired_transactions.push(id.clone());
                }
            }
        }

        // Rollback expired transactions
        let mut cleaned_up = 0;
        for transaction_id in expired_transactions {
            match self.rollback_transaction(&transaction_id, Some("Transaction timeout".to_string())).await {
                Ok(_) => cleaned_up += 1,
                Err(e) => error!("Failed to cleanup expired transaction {}: {}", transaction_id, e),
            }
        }

        if cleaned_up > 0 {
            info!("Cleaned up {} expired transactions", cleaned_up);
        }

        Ok(cleaned_up)
    }

    /// Get transaction statistics
    pub async fn get_transaction_stats(&self) -> HashMap<String, serde_json::Value> {
        let transactions = self.active_transactions.read().await;
        
        let mut stats = HashMap::new();
        stats.insert("active_transactions".to_string(), serde_json::json!(transactions.len()));
        
        let mut status_counts = HashMap::new();
        for context in transactions.values() {
            let status_str = format!("{:?}", context.status);
            *status_counts.entry(status_str).or_insert(0u64) += 1;
        }
        stats.insert("status_breakdown".to_string(), serde_json::json!(status_counts));
        
        stats
    }
}
