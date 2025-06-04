//! Rewards service data models

use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveTime};
use serde::{Serialize, Deserialize};

/// Reward rule types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardRuleType {
    Transaction,    // Points per transaction
    Spending,       // Points per dollar spent
    Funding,        // Points for funding activities
    Referral,       // Referral bonuses
    Milestone,      // Achievement milestones
    Promotional,    // Time-limited promotions
    TierBonus,      // Tier-based multipliers
    Category,       // Category-specific rewards
}

/// Reward rule status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardRuleStatus {
    Active,
    Inactive,
    Expired,
    Suspended,
}

/// User reward tiers
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UserRewardTier {
    Bronze,     // 1x multiplier
    Silver,     // 1.5x multiplier
    Gold,       // 2x multiplier
    Platinum,   // 3x multiplier
}

impl UserRewardTier {
    pub fn multiplier(&self) -> Decimal {
        match self {
            UserRewardTier::Bronze => Decimal::from(1),
            UserRewardTier::Silver => Decimal::from_str_exact("1.5").unwrap(),
            UserRewardTier::Gold => Decimal::from(2),
            UserRewardTier::Platinum => Decimal::from(3),
        }
    }

    pub fn threshold(&self) -> Decimal {
        match self {
            UserRewardTier::Bronze => Decimal::ZERO,
            UserRewardTier::Silver => Decimal::from(1000),
            UserRewardTier::Gold => Decimal::from(5000),
            UserRewardTier::Platinum => Decimal::from(25000),
        }
    }

    pub fn next_tier(&self) -> Option<UserRewardTier> {
        match self {
            UserRewardTier::Bronze => Some(UserRewardTier::Silver),
            UserRewardTier::Silver => Some(UserRewardTier::Gold),
            UserRewardTier::Gold => Some(UserRewardTier::Platinum),
            UserRewardTier::Platinum => None,
        }
    }

    pub fn benefits(&self) -> Vec<String> {
        match self {
            UserRewardTier::Bronze => vec![
                "1x points multiplier".to_string(),
                "Basic customer support".to_string(),
            ],
            UserRewardTier::Silver => vec![
                "1.5x points multiplier".to_string(),
                "Priority customer support".to_string(),
                "Monthly bonus rewards".to_string(),
            ],
            UserRewardTier::Gold => vec![
                "2x points multiplier".to_string(),
                "Premium customer support".to_string(),
                "Weekly bonus rewards".to_string(),
                "Exclusive redemption options".to_string(),
            ],
            UserRewardTier::Platinum => vec![
                "3x points multiplier".to_string(),
                "VIP customer support".to_string(),
                "Daily bonus rewards".to_string(),
                "Exclusive redemption options".to_string(),
                "Early access to new features".to_string(),
                "Personal account manager".to_string(),
            ],
        }
    }
}

/// Reward transaction types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardTransactionType {
    Earned,     // Points earned
    Redeemed,   // Points redeemed
    Expired,    // Points expired
    Adjusted,   // Manual adjustment
    Bonus,      // Bonus points
    Penalty,    // Penalty deduction
}

/// Reward transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardTransactionStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

/// Redemption types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedemptionType {
    Cash,           // Points to cash
    Credit,         // Points to account credit
    GiftCard,       // Points to gift cards
    Merchandise,    // Points to physical items
    Discount,       // Points to transaction discounts
    Charity,        // Points to charitable donations
}

/// Redemption status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedemptionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

/// Reward rule entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub rule_type: RewardRuleType,
    pub status: RewardRuleStatus,
    
    // Rule configuration
    pub points_per_unit: Decimal,           // Points awarded per unit
    pub minimum_amount: Option<Decimal>,    // Minimum transaction amount
    pub maximum_points: Option<Decimal>,    // Maximum points per transaction
    pub minimum_tier: UserRewardTier,       // Minimum user tier required
    pub categories: Vec<String>,            // Applicable merchant categories
    pub currencies: Vec<String>,            // Applicable currencies
    
    // Time constraints
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub days_of_week: Vec<i32>,             // 0=Sunday, 1=Monday, etc.
    pub start_time: Option<NaiveTime>,      // HH:MM format
    pub end_time: Option<NaiveTime>,        // HH:MM format
    
    // Usage limits
    pub max_uses_per_user: Option<i32>,     // None for unlimited
    pub max_uses_per_day: Option<i32>,
    pub max_uses_per_month: Option<i32>,
    pub total_uses_remaining: Option<i32>,
    
    // Metadata
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

/// User reward information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRewards {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total_points: Decimal,              // Total available points
    pub lifetime_earned: Decimal,           // Lifetime points earned
    pub lifetime_redeemed: Decimal,         // Lifetime points redeemed
    pub pending_points: Decimal,            // Pending points
    pub expiring_points: Decimal,           // Points expiring soon
    pub current_tier: UserRewardTier,
    pub tier_progress: Decimal,             // Progress to next tier
    pub next_tier_threshold: Decimal,       // Points needed for next tier
    
    // Tier benefits
    pub tier_multiplier: Decimal,           // Current tier multiplier
    pub tier_benefits: Vec<String>,         // List of tier-specific benefits
    
    // Expiration tracking
    pub next_expiration_date: Option<DateTime<Utc>>,
    pub next_expiration_amount: Decimal,
    
    // Metadata
    pub last_activity_date: DateTime<Utc>,
    pub tier_upgrade_date: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for UserRewards {
    fn default() -> Self {
        let now = Utc::now();
        let tier = UserRewardTier::Bronze;
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            total_points: Decimal::ZERO,
            lifetime_earned: Decimal::ZERO,
            lifetime_redeemed: Decimal::ZERO,
            pending_points: Decimal::ZERO,
            expiring_points: Decimal::ZERO,
            current_tier: tier.clone(),
            tier_progress: Decimal::ZERO,
            next_tier_threshold: tier.next_tier().map(|t| t.threshold()).unwrap_or(Decimal::ZERO),
            tier_multiplier: tier.multiplier(),
            tier_benefits: tier.benefits(),
            next_expiration_date: None,
            next_expiration_amount: Decimal::ZERO,
            last_activity_date: now,
            tier_upgrade_date: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Reward transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub transaction_type: RewardTransactionType,
    pub status: RewardTransactionStatus,
    
    // Transaction details
    pub points: Decimal,                    // Points amount (positive or negative)
    pub multiplier: Decimal,                // Tier multiplier applied
    pub base_points: Decimal,               // Base points before multiplier
    pub currency: String,                   // Currency for monetary transactions
    pub exchange_rate: Option<Decimal>,     // Points-to-currency rate
    
    // Source information
    pub source_type: Option<String>,        // transaction, referral, milestone, etc.
    pub source_id: Option<String>,          // ID of source transaction/event
    pub reward_rule_id: Option<Uuid>,       // ID of reward rule applied
    pub reference_number: String,           // Unique reference for tracking
    
    // Expiration
    pub expires_at: Option<DateTime<Utc>>,
    pub is_expired: bool,
    
    // Metadata
    pub description: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Redemption record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redemption {
    pub id: Uuid,
    pub user_id: Uuid,
    pub redemption_type: RedemptionType,
    pub status: RedemptionStatus,
    
    // Redemption details
    pub points_redeemed: Decimal,           // Points used
    pub cash_value: Decimal,                // Cash equivalent
    pub currency: String,                   // Currency for cash redemptions
    pub exchange_rate: Decimal,             // Points-to-cash rate
    
    // Redemption target
    pub target_account: Option<String>,     // Account for cash/credit redemptions
    pub gift_card_code: Option<String>,     // Gift card code for gift card redemptions
    pub merchant_name: Option<String>,      // Merchant for gift cards/merchandise
    pub tracking_number: Option<String>,    // Shipping tracking for merchandise
    
    // Processing information
    pub processing_fee: Decimal,            // Processing fee
    pub net_amount: Decimal,                // Net amount after fees
    pub estimated_delivery: Option<DateTime<Utc>>, // Delivery estimate
    pub actual_delivery: Option<DateTime<Utc>>,    // Actual delivery
    
    // Metadata
    pub description: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Redemption option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionOption {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub redemption_type: RedemptionType,
    pub is_active: bool,

    // Pricing
    pub points_required: Decimal,           // Points needed
    pub cash_value: Decimal,                // Cash equivalent
    pub currency: String,                   // Currency for cash value
    pub processing_fee: Decimal,            // Processing fee percentage

    // Availability
    pub quantity_available: Option<i32>,    // None for unlimited
    pub quantity_redeemed: i32,
    pub minimum_tier: UserRewardTier,       // Minimum tier required

    // Constraints
    pub minimum_points_balance: Decimal,    // Minimum balance required
    pub max_redemptions_per_user: Option<i32>, // Per user limit
    pub max_redemptions_per_day: Option<i32>,  // Daily limit

    // Metadata
    pub image_url: Option<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Reward metrics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardMetrics {
    // Overall metrics
    pub total_points_awarded: Decimal,      // Total points awarded
    pub total_points_redeemed: Decimal,     // Total points redeemed
    pub total_points_expired: Decimal,      // Total points expired
    pub total_cash_value: Decimal,          // Total cash value of rewards
    pub total_users: i64,                   // Total users with rewards
    pub active_users: i64,                  // Users with recent activity

    // Time period metrics
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub period_points_awarded: Decimal,     // Period points awarded
    pub period_points_redeemed: Decimal,    // Period points redeemed
    pub period_transactions: i64,           // Number of reward transactions in period
    pub period_redemptions: i64,            // Number of redemptions in period

    // Tier distribution
    pub bronze_users: i64,
    pub silver_users: i64,
    pub gold_users: i64,
    pub platinum_users: i64,

    // Top categories and redemptions
    pub top_categories: Vec<CategoryMetrics>,
    pub top_redemptions: Vec<RedemptionMetrics>,

    // Metadata
    pub generated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Category metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMetrics {
    pub category: String,
    pub points_awarded: Decimal,            // Points awarded in category
    pub transaction_count: i64,             // Number of transactions in category
    pub average_points: Decimal,            // Average points per transaction
}

/// Redemption metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionMetrics {
    pub redemption_type: RedemptionType,
    pub points_redeemed: Decimal,           // Points redeemed
    pub redemption_count: i64,              // Number of redemptions
    pub average_points: Decimal,            // Average points per redemption
}

/// Audit trail entry for rewards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardAuditTrailEntry {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action_type: String,                // award, redeem, expire, adjust, tier_change
    pub entity_type: String,                // reward_transaction, redemption, user_rewards
    pub entity_id: Uuid,
    pub old_value: Option<String>,          // JSON string of old state
    pub new_value: Option<String>,          // JSON string of new state
    pub reason: Option<String>,
    pub performed_by: Option<Uuid>,         // User ID who performed the action
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Repository trait for reward operations
#[async_trait::async_trait]
pub trait RewardsRepository: Send + Sync {
    // Reward rule operations
    async fn create_reward_rule(&self, rule: &RewardRule) -> Result<RewardRule, String>;
    async fn get_reward_rule(&self, id: &Uuid) -> Result<Option<RewardRule>, String>;
    async fn list_reward_rules(&self, rule_type: Option<RewardRuleType>, status: Option<RewardRuleStatus>, category: Option<String>, currency: Option<String>, active_only: bool, page: i32, page_size: i32) -> Result<(Vec<RewardRule>, i64), String>;
    async fn update_reward_rule(&self, rule: &RewardRule) -> Result<RewardRule, String>;
    async fn delete_reward_rule(&self, id: &Uuid) -> Result<(), String>;

    // User reward operations
    async fn get_user_rewards(&self, user_id: &Uuid) -> Result<Option<UserRewards>, String>;
    async fn create_user_rewards(&self, rewards: &UserRewards) -> Result<UserRewards, String>;
    async fn update_user_rewards(&self, rewards: &UserRewards) -> Result<UserRewards, String>;
    async fn get_reward_balance(&self, user_id: &Uuid, currency: Option<String>) -> Result<Option<UserRewards>, String>;
    async fn update_user_tier(&self, user_id: &Uuid, new_tier: UserRewardTier, reason: Option<String>) -> Result<UserRewards, String>;

    // Reward transaction operations
    async fn create_reward_transaction(&self, transaction: &RewardTransaction) -> Result<RewardTransaction, String>;
    async fn get_reward_transaction(&self, id: &Uuid) -> Result<Option<RewardTransaction>, String>;
    async fn list_reward_transactions(&self, user_id: Option<Uuid>, transaction_type: Option<RewardTransactionType>, status: Option<RewardTransactionStatus>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<RewardTransaction>, i64), String>;
    async fn update_reward_transaction(&self, transaction: &RewardTransaction) -> Result<RewardTransaction, String>;
    async fn expire_points(&self, user_id: Option<Uuid>, expiration_date: DateTime<Utc>, dry_run: bool) -> Result<(Vec<RewardTransaction>, i64), String>;

    // Redemption operations
    async fn create_redemption(&self, redemption: &Redemption) -> Result<Redemption, String>;
    async fn get_redemption(&self, id: &Uuid) -> Result<Option<Redemption>, String>;
    async fn list_redemptions(&self, user_id: Option<Uuid>, redemption_type: Option<RedemptionType>, status: Option<RedemptionStatus>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<Redemption>, i64), String>;
    async fn update_redemption(&self, redemption: &Redemption) -> Result<Redemption, String>;
    async fn cancel_redemption(&self, id: &Uuid, reason: String) -> Result<Redemption, String>;

    // Redemption option operations
    async fn create_redemption_option(&self, option: &RedemptionOption) -> Result<RedemptionOption, String>;
    async fn get_redemption_option(&self, id: &Uuid) -> Result<Option<RedemptionOption>, String>;
    async fn list_redemption_options(&self, redemption_type: Option<RedemptionType>, minimum_tier: Option<UserRewardTier>, minimum_points: Option<Decimal>, active_only: bool, page: i32, page_size: i32) -> Result<(Vec<RedemptionOption>, i64), String>;
    async fn update_redemption_option(&self, option: &RedemptionOption) -> Result<RedemptionOption, String>;

    // Analytics operations
    async fn get_reward_metrics(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>, rule_types: Vec<RewardRuleType>, tiers: Vec<UserRewardTier>, currencies: Vec<String>) -> Result<RewardMetrics, String>;
    async fn get_user_reward_analytics(&self, user_id: &Uuid, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<(UserRewards, Vec<RewardTransaction>, Vec<CategoryMetrics>), String>;

    // Audit operations
    async fn create_audit_entry(&self, entry: &RewardAuditTrailEntry) -> Result<RewardAuditTrailEntry, String>;
    async fn get_audit_trail(&self, user_id: Option<Uuid>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, action_types: Vec<String>, page: i32, page_size: i32) -> Result<(Vec<RewardAuditTrailEntry>, i64), String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryRewardsRepository {
    reward_rules: std::sync::RwLock<HashMap<Uuid, RewardRule>>,
    user_rewards: std::sync::RwLock<HashMap<Uuid, UserRewards>>,
    reward_transactions: std::sync::RwLock<HashMap<Uuid, RewardTransaction>>,
    redemptions: std::sync::RwLock<HashMap<Uuid, Redemption>>,
    redemption_options: std::sync::RwLock<HashMap<Uuid, RedemptionOption>>,
    audit_trail: std::sync::RwLock<Vec<RewardAuditTrailEntry>>,
    reference_numbers: std::sync::RwLock<HashMap<String, Uuid>>,
}
