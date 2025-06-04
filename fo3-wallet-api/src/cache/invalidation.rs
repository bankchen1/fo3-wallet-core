//! Cache invalidation strategies and event-driven cache management
//!
//! Provides intelligent cache invalidation based on data changes and business logic.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::ServiceError;
use super::{Cache, CacheKey, cache_manager::CacheManager};

/// Cache invalidation event types
#[derive(Debug, Clone, PartialEq)]
pub enum InvalidationEvent {
    // User-related events
    UserUpdated(Uuid),
    UserDeleted(Uuid),
    UserSessionExpired(Uuid),
    UserPermissionsChanged(Uuid),
    
    // Wallet events
    WalletBalanceChanged(Uuid),
    TransactionCompleted(Uuid),
    TransactionFailed(Uuid),
    
    // KYC events
    KycStatusChanged(Uuid),
    KycDocumentUploaded(Uuid),
    KycDocumentDeleted(Uuid),
    
    // Card events
    CardCreated(Uuid),
    CardUpdated(Uuid),
    CardDeleted(Uuid),
    CardLimitsChanged(Uuid),
    CardTransactionCompleted(Uuid),
    
    // Pricing events
    AssetPriceUpdated(String),
    MarketDataUpdated(String),
    ExchangeRateUpdated(String),
    
    // DeFi events
    YieldRateUpdated(String),
    PoolDataUpdated(String),
    StakingRewardsUpdated(Uuid),
    
    // System events
    FeatureFlagChanged(String),
    SystemConfigUpdated(String),
    ServiceHealthChanged(String),
    
    // Bulk operations
    BulkUserUpdate(Vec<Uuid>),
    BulkPriceUpdate(Vec<String>),
    SystemMaintenance,
}

/// Cache invalidation strategy
#[derive(Debug, Clone)]
pub enum InvalidationStrategy {
    /// Immediate invalidation
    Immediate,
    /// Delayed invalidation (useful for batching)
    Delayed(std::time::Duration),
    /// Conditional invalidation based on criteria
    Conditional(InvalidationCondition),
    /// No invalidation (for read-only data)
    None,
}

/// Invalidation condition
#[derive(Debug, Clone)]
pub enum InvalidationCondition {
    /// Invalidate if cache age exceeds threshold
    AgeThreshold(std::time::Duration),
    /// Invalidate if access count exceeds threshold
    AccessThreshold(u64),
    /// Invalidate based on custom logic
    Custom(String),
}

/// Cache invalidation manager
pub struct CacheInvalidationManager {
    cache_manager: Arc<CacheManager>,
    invalidation_rules: Arc<RwLock<HashMap<String, InvalidationStrategy>>>,
    pending_invalidations: Arc<RwLock<Vec<PendingInvalidation>>>,
    stats: Arc<RwLock<InvalidationStats>>,
}

/// Pending invalidation entry
#[derive(Debug, Clone)]
struct PendingInvalidation {
    event: InvalidationEvent,
    scheduled_at: DateTime<Utc>,
    strategy: InvalidationStrategy,
}

/// Invalidation statistics
#[derive(Debug, Clone)]
pub struct InvalidationStats {
    pub total_events: u64,
    pub immediate_invalidations: u64,
    pub delayed_invalidations: u64,
    pub conditional_invalidations: u64,
    pub failed_invalidations: u64,
    pub cache_entries_invalidated: u64,
}

impl InvalidationStats {
    pub fn new() -> Self {
        Self {
            total_events: 0,
            immediate_invalidations: 0,
            delayed_invalidations: 0,
            conditional_invalidations: 0,
            failed_invalidations: 0,
            cache_entries_invalidated: 0,
        }
    }
}

impl CacheInvalidationManager {
    /// Create new cache invalidation manager
    pub fn new(cache_manager: Arc<CacheManager>) -> Self {
        let mut rules = HashMap::new();
        
        // Set up default invalidation rules
        Self::setup_default_rules(&mut rules);
        
        Self {
            cache_manager,
            invalidation_rules: Arc::new(RwLock::new(rules)),
            pending_invalidations: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(InvalidationStats::new())),
        }
    }
    
    /// Set up default invalidation rules
    fn setup_default_rules(rules: &mut HashMap<String, InvalidationStrategy>) {
        // Immediate invalidation for critical data
        rules.insert("user_session".to_string(), InvalidationStrategy::Immediate);
        rules.insert("user_permissions".to_string(), InvalidationStrategy::Immediate);
        rules.insert("wallet_balance".to_string(), InvalidationStrategy::Immediate);
        rules.insert("card_limits".to_string(), InvalidationStrategy::Immediate);
        rules.insert("kyc_status".to_string(), InvalidationStrategy::Immediate);
        
        // Delayed invalidation for less critical data
        rules.insert("transaction_history".to_string(), 
                    InvalidationStrategy::Delayed(std::time::Duration::from_secs(30)));
        rules.insert("spending_insights".to_string(), 
                    InvalidationStrategy::Delayed(std::time::Duration::from_secs(60)));
        rules.insert("user_analytics".to_string(), 
                    InvalidationStrategy::Delayed(std::time::Duration::from_secs(120)));
        
        // Conditional invalidation for market data
        rules.insert("asset_price".to_string(), 
                    InvalidationStrategy::Conditional(
                        InvalidationCondition::AgeThreshold(std::time::Duration::from_secs(60))
                    ));
        rules.insert("market_data".to_string(), 
                    InvalidationStrategy::Conditional(
                        InvalidationCondition::AgeThreshold(std::time::Duration::from_secs(300))
                    ));
        
        // No invalidation for static data
        rules.insert("system_config".to_string(), InvalidationStrategy::None);
        rules.insert("feature_flags".to_string(), InvalidationStrategy::None);
    }
    
    /// Handle invalidation event
    pub async fn handle_event(&self, event: InvalidationEvent) -> Result<(), ServiceError> {
        debug!("Handling invalidation event: {:?}", event);
        
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        drop(stats);
        
        let strategy = self.get_strategy_for_event(&event).await;
        
        match strategy {
            InvalidationStrategy::Immediate => {
                self.execute_immediate_invalidation(&event).await?;
                let mut stats = self.stats.write().await;
                stats.immediate_invalidations += 1;
            }
            InvalidationStrategy::Delayed(delay) => {
                self.schedule_delayed_invalidation(event, delay).await?;
                let mut stats = self.stats.write().await;
                stats.delayed_invalidations += 1;
            }
            InvalidationStrategy::Conditional(condition) => {
                self.execute_conditional_invalidation(&event, &condition).await?;
                let mut stats = self.stats.write().await;
                stats.conditional_invalidations += 1;
            }
            InvalidationStrategy::None => {
                debug!("No invalidation required for event: {:?}", event);
            }
        }
        
        Ok(())
    }
    
    /// Get invalidation strategy for event
    async fn get_strategy_for_event(&self, event: &InvalidationEvent) -> InvalidationStrategy {
        let rules = self.invalidation_rules.read().await;
        
        let rule_key = match event {
            InvalidationEvent::UserUpdated(_) | 
            InvalidationEvent::UserDeleted(_) | 
            InvalidationEvent::UserSessionExpired(_) => "user_session",
            InvalidationEvent::UserPermissionsChanged(_) => "user_permissions",
            InvalidationEvent::WalletBalanceChanged(_) => "wallet_balance",
            InvalidationEvent::TransactionCompleted(_) | 
            InvalidationEvent::TransactionFailed(_) => "transaction_history",
            InvalidationEvent::KycStatusChanged(_) => "kyc_status",
            InvalidationEvent::CardLimitsChanged(_) => "card_limits",
            InvalidationEvent::AssetPriceUpdated(_) => "asset_price",
            InvalidationEvent::MarketDataUpdated(_) => "market_data",
            InvalidationEvent::FeatureFlagChanged(_) => "feature_flags",
            InvalidationEvent::SystemConfigUpdated(_) => "system_config",
            _ => "default",
        };
        
        rules.get(rule_key)
            .cloned()
            .unwrap_or(InvalidationStrategy::Immediate)
    }
    
    /// Execute immediate invalidation
    async fn execute_immediate_invalidation(&self, event: &InvalidationEvent) -> Result<(), ServiceError> {
        let keys_to_invalidate = self.get_cache_keys_for_event(event);
        let mut total_invalidated = 0u64;
        
        for key in keys_to_invalidate {
            match self.cache_manager.delete(&key).await {
                Ok(()) => {
                    total_invalidated += 1;
                    debug!("Invalidated cache key: {:?}", key);
                }
                Err(e) => {
                    warn!("Failed to invalidate cache key {:?}: {}", key, e);
                    let mut stats = self.stats.write().await;
                    stats.failed_invalidations += 1;
                }
            }
        }
        
        // Also invalidate by pattern for bulk operations
        if let Some(pattern) = self.get_invalidation_pattern_for_event(event) {
            match self.cache_manager.invalidate_pattern(&pattern).await {
                Ok(count) => {
                    total_invalidated += count;
                    info!("Invalidated {} cache entries with pattern: {}", count, pattern);
                }
                Err(e) => {
                    warn!("Failed to invalidate pattern {}: {}", pattern, e);
                    let mut stats = self.stats.write().await;
                    stats.failed_invalidations += 1;
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.cache_entries_invalidated += total_invalidated;
        
        Ok(())
    }
    
    /// Schedule delayed invalidation
    async fn schedule_delayed_invalidation(&self, event: InvalidationEvent, delay: std::time::Duration) -> Result<(), ServiceError> {
        let scheduled_at = Utc::now() + chrono::Duration::from_std(delay).unwrap();
        
        let pending = PendingInvalidation {
            event,
            scheduled_at,
            strategy: InvalidationStrategy::Delayed(delay),
        };
        
        let mut pending_invalidations = self.pending_invalidations.write().await;
        pending_invalidations.push(pending);
        
        debug!("Scheduled delayed invalidation for {:?}", scheduled_at);
        Ok(())
    }
    
    /// Execute conditional invalidation
    async fn execute_conditional_invalidation(&self, event: &InvalidationEvent, condition: &InvalidationCondition) -> Result<(), ServiceError> {
        match condition {
            InvalidationCondition::AgeThreshold(_threshold) => {
                // For now, execute immediate invalidation
                // In production, you would check cache entry age
                self.execute_immediate_invalidation(event).await
            }
            InvalidationCondition::AccessThreshold(_threshold) => {
                // For now, execute immediate invalidation
                // In production, you would check access count
                self.execute_immediate_invalidation(event).await
            }
            InvalidationCondition::Custom(_logic) => {
                // For now, execute immediate invalidation
                // In production, you would implement custom logic
                self.execute_immediate_invalidation(event).await
            }
        }
    }
    
    /// Get cache keys to invalidate for event
    fn get_cache_keys_for_event(&self, event: &InvalidationEvent) -> Vec<CacheKey> {
        match event {
            InvalidationEvent::UserUpdated(user_id) |
            InvalidationEvent::UserDeleted(user_id) |
            InvalidationEvent::UserSessionExpired(user_id) => {
                vec![
                    CacheKey::UserSession(*user_id),
                    CacheKey::UserPermissions(*user_id),
                    CacheKey::UserAnalytics(*user_id),
                ]
            }
            InvalidationEvent::WalletBalanceChanged(user_id) => {
                vec![
                    CacheKey::WalletBalance(*user_id),
                    CacheKey::PendingTransactions(*user_id),
                ]
            }
            InvalidationEvent::TransactionCompleted(user_id) |
            InvalidationEvent::TransactionFailed(user_id) => {
                vec![
                    CacheKey::TransactionHistory(*user_id),
                    CacheKey::WalletBalance(*user_id),
                    CacheKey::SpendingInsights(*user_id),
                ]
            }
            InvalidationEvent::KycStatusChanged(user_id) => {
                vec![
                    CacheKey::KycStatus(*user_id),
                    CacheKey::ComplianceCheck(*user_id),
                ]
            }
            InvalidationEvent::CardLimitsChanged(user_id) => {
                vec![
                    CacheKey::CardLimits(*user_id),
                ]
            }
            InvalidationEvent::AssetPriceUpdated(symbol) => {
                vec![
                    CacheKey::AssetPrice(symbol.clone()),
                    CacheKey::MarketData(symbol.clone()),
                ]
            }
            InvalidationEvent::FeatureFlagChanged(flag) => {
                vec![
                    CacheKey::FeatureFlags(flag.clone()),
                ]
            }
            _ => Vec::new(),
        }
    }
    
    /// Get invalidation pattern for event
    fn get_invalidation_pattern_for_event(&self, event: &InvalidationEvent) -> Option<String> {
        match event {
            InvalidationEvent::BulkUserUpdate(user_ids) => {
                if user_ids.len() > 10 {
                    Some("session:*".to_string())
                } else {
                    None
                }
            }
            InvalidationEvent::BulkPriceUpdate(symbols) => {
                if symbols.len() > 5 {
                    Some("price:*".to_string())
                } else {
                    None
                }
            }
            InvalidationEvent::SystemMaintenance => {
                Some("*".to_string())
            }
            _ => None,
        }
    }
    
    /// Process pending invalidations
    pub async fn process_pending_invalidations(&self) -> Result<(), ServiceError> {
        let now = Utc::now();
        let mut pending_invalidations = self.pending_invalidations.write().await;
        
        let mut to_process = Vec::new();
        pending_invalidations.retain(|pending| {
            if pending.scheduled_at <= now {
                to_process.push(pending.clone());
                false
            } else {
                true
            }
        });
        
        drop(pending_invalidations);
        
        for pending in to_process {
            debug!("Processing pending invalidation: {:?}", pending.event);
            if let Err(e) = self.execute_immediate_invalidation(&pending.event).await {
                warn!("Failed to process pending invalidation: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get invalidation statistics
    pub async fn get_stats(&self) -> InvalidationStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Add custom invalidation rule
    pub async fn add_rule(&self, key: String, strategy: InvalidationStrategy) {
        let mut rules = self.invalidation_rules.write().await;
        rules.insert(key, strategy);
    }
    
    /// Remove invalidation rule
    pub async fn remove_rule(&self, key: &str) {
        let mut rules = self.invalidation_rules.write().await;
        rules.remove(key);
    }
}
