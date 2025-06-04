//! Referral service data models

use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Referral code status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralCodeStatus {
    Active,
    Inactive,
    Expired,
    Suspended,
    Exhausted,    // Max uses reached
}

/// Referral relationship status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralRelationshipStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Fraudulent,
}

/// Referral campaign types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralCampaignType {
    Signup,             // Bonus for successful signup
    FirstTransaction,   // Bonus for first transaction
    SpendingMilestone,  // Bonus for spending milestones
    TierUpgrade,        // Bonus for tier upgrades
    MultiLevel,         // Multi-level referral bonuses
    TimeLimited,        // Time-limited promotions
}

/// Referral campaign status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralCampaignStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// Referral bonus types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralBonusType {
    Referrer,       // Bonus for the referrer
    Referee,        // Bonus for the referee
    Milestone,      // Milestone-based bonus
    TierBonus,      // Tier-based multiplier bonus
    CampaignBonus,  // Campaign-specific bonus
}

/// Referral bonus status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferralBonusStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

/// Referral code entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralCode {
    pub id: Uuid,
    pub user_id: Uuid,                     // Owner of the referral code
    pub code: String,                      // Unique referral code
    pub status: ReferralCodeStatus,
    
    // Code configuration
    pub campaign_id: Option<Uuid>,         // Associated campaign
    pub description: Option<String>,
    pub is_custom: bool,                   // Custom vs auto-generated
    
    // Usage tracking
    pub max_uses: Option<i32>,             // None for unlimited
    pub current_uses: i32,
    pub successful_referrals: i32,
    pub pending_referrals: i32,
    
    // Time constraints
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    
    // Metadata
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ReferralCode {
    /// Check if the referral code is valid for use
    pub fn is_valid(&self) -> bool {
        match self.status {
            ReferralCodeStatus::Active => {
                // Check expiration
                if let Some(expires_at) = self.expires_at {
                    if Utc::now() > expires_at {
                        return false;
                    }
                }
                
                // Check usage limits
                if let Some(max_uses) = self.max_uses {
                    if self.current_uses >= max_uses {
                        return false;
                    }
                }
                
                true
            },
            _ => false,
        }
    }

    /// Generate a unique referral code
    pub fn generate_code(user_id: &Uuid, custom_code: Option<String>) -> String {
        if let Some(code) = custom_code {
            code
        } else {
            // Generate code format: FO3-{first 8 chars of user_id}-{random 4 chars}
            let user_prefix = user_id.to_string().replace("-", "")[..8].to_uppercase();
            let random_suffix = Uuid::new_v4().to_string().replace("-", "")[..4].to_uppercase();
            format!("FO3-{}-{}", user_prefix, random_suffix)
        }
    }
}

/// Referral campaign entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralCampaign {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub campaign_type: ReferralCampaignType,
    pub status: ReferralCampaignStatus,
    
    // Campaign configuration
    pub referrer_bonus: Decimal,            // Bonus for referrer
    pub referee_bonus: Decimal,             // Bonus for referee
    pub bonus_currency: String,             // Currency (points, USD, etc.)
    pub minimum_transaction_amount: Decimal, // Minimum transaction for bonus
    
    // Multi-level configuration
    pub is_multi_level: bool,
    pub max_levels: i32,                    // Maximum referral levels
    pub level_multipliers: Vec<Decimal>,    // Multipliers for each level
    
    // Time constraints
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub bonus_expiry_days: i32,             // Days until bonus expires
    
    // Usage limits
    pub max_referrals_per_user: Option<i32>, // None for unlimited
    pub max_total_referrals: Option<i32>,   // None for unlimited
    pub max_bonus_per_user: Option<Decimal>, // Maximum bonus per user
    pub total_budget: Option<Decimal>,      // Total campaign budget
    pub budget_used: Decimal,               // Budget used so far
    
    // Targeting
    pub target_user_tiers: Vec<String>,     // Target user tiers
    pub target_countries: Vec<String>,      // Target countries
    pub excluded_users: Vec<Uuid>,          // Excluded user IDs
    
    // Metadata
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

impl ReferralCampaign {
    /// Check if the campaign is currently active
    pub fn is_active(&self) -> bool {
        if self.status != ReferralCampaignStatus::Active {
            return false;
        }
        
        let now = Utc::now();
        
        // Check start date
        if let Some(start_date) = self.start_date {
            if now < start_date {
                return false;
            }
        }
        
        // Check end date
        if let Some(end_date) = self.end_date {
            if now > end_date {
                return false;
            }
        }
        
        // Check budget
        if let Some(total_budget) = self.total_budget {
            if self.budget_used >= total_budget {
                return false;
            }
        }
        
        true
    }

    /// Check if user is eligible for this campaign
    pub fn is_user_eligible(&self, user_tier: &str, country: Option<&str>) -> bool {
        // Check tier eligibility
        if !self.target_user_tiers.is_empty() && !self.target_user_tiers.contains(&user_tier.to_string()) {
            return false;
        }
        
        // Check country eligibility
        if !self.target_countries.is_empty() {
            if let Some(user_country) = country {
                if !self.target_countries.contains(&user_country.to_string()) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        true
    }
}

/// Referral relationship entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralRelationship {
    pub id: Uuid,
    pub referrer_user_id: Uuid,            // User who made the referral
    pub referee_user_id: Uuid,             // User who was referred
    pub referral_code_id: Uuid,            // Referral code used
    pub campaign_id: Option<Uuid>,         // Associated campaign
    pub status: ReferralRelationshipStatus,
    
    // Relationship details
    pub referral_level: i32,               // 1 = direct, 2 = sub-referral, etc.
    pub parent_relationship_id: Option<Uuid>, // For multi-level tracking
    
    // Milestone tracking
    pub signup_completed: bool,
    pub first_transaction_completed: bool,
    pub kyc_completed: bool,
    pub first_transaction_date: Option<DateTime<Utc>>,
    pub kyc_completion_date: Option<DateTime<Utc>>,
    
    // Bonus tracking
    pub total_bonuses_earned: Decimal,     // Total bonuses earned from this relationship
    pub total_bonuses_paid: Decimal,       // Total bonuses paid out
    pub bonuses_pending: i32,              // Number of pending bonuses
    
    // Fraud detection
    pub is_suspicious: bool,
    pub fraud_flags: Vec<String>,          // List of fraud indicators
    pub fraud_check_date: Option<DateTime<Utc>>,
    
    // Metadata
    pub referral_source: Option<String>,   // web, mobile, email, etc.
    pub ip_address: Option<String>,        // IP address at time of referral
    pub user_agent: Option<String>,        // User agent at time of referral
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ReferralRelationship {
    /// Check if relationship is eligible for bonuses
    pub fn is_eligible_for_bonus(&self, milestone_type: &str) -> bool {
        if self.status != ReferralRelationshipStatus::Active {
            return false;
        }
        
        if self.is_suspicious {
            return false;
        }
        
        match milestone_type {
            "signup" => self.signup_completed,
            "first_transaction" => self.first_transaction_completed,
            "kyc" => self.kyc_completed,
            _ => true,
        }
    }

    /// Update milestone completion
    pub fn update_milestone(&mut self, milestone_type: &str) {
        match milestone_type {
            "signup" => {
                self.signup_completed = true;
            },
            "first_transaction" => {
                self.first_transaction_completed = true;
                self.first_transaction_date = Some(Utc::now());
            },
            "kyc" => {
                self.kyc_completed = true;
                self.kyc_completion_date = Some(Utc::now());
            },
            _ => {},
        }
        self.updated_at = Utc::now();
    }
}

/// Referral bonus entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralBonus {
    pub id: Uuid,
    pub referral_relationship_id: Uuid,    // Associated referral relationship
    pub campaign_id: Option<Uuid>,         // Associated campaign
    pub user_id: Uuid,                     // User receiving the bonus
    pub bonus_type: ReferralBonusType,
    pub status: ReferralBonusStatus,

    // Bonus details
    pub bonus_amount: Decimal,             // Bonus amount
    pub bonus_currency: String,            // Currency (points, USD, etc.)
    pub exchange_rate: Decimal,            // Exchange rate if applicable
    pub milestone_type: Option<String>,    // signup, first_transaction, etc.
    pub milestone_value: Option<Decimal>,  // Milestone value if applicable

    // Processing details
    pub reward_transaction_id: Option<Uuid>, // Associated reward transaction
    pub processing_fee: Decimal,           // Processing fee
    pub net_amount: Decimal,               // Net amount after fees

    // Time tracking
    pub earned_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,

    // Metadata
    pub description: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Referral metrics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralMetrics {
    // Overall metrics
    pub total_referral_codes: i64,
    pub active_referral_codes: i64,
    pub total_referrals: i64,
    pub successful_referrals: i64,
    pub pending_referrals: i64,
    pub total_bonuses_paid: Decimal,
    pub total_bonuses_pending: Decimal,

    // Time period metrics
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub period_referrals: i64,
    pub period_signups: i64,
    pub period_bonuses_paid: Decimal,

    // Conversion metrics
    pub signup_conversion_rate: Decimal,
    pub transaction_conversion_rate: Decimal,
    pub average_bonus_per_referral: Decimal,
    pub roi: Decimal,                      // Return on investment

    // Top performers
    pub top_referrers: Vec<TopReferrer>,
    pub top_campaigns: Vec<CampaignMetrics>,

    // Fraud metrics
    pub flagged_relationships: i64,
    pub cancelled_relationships: i64,
    pub fraud_rate: Decimal,

    // Metadata
    pub generated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Top referrer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopReferrer {
    pub user_id: Uuid,
    pub username: Option<String>,          // Optional display name
    pub total_referrals: i64,
    pub successful_referrals: i64,
    pub total_bonuses_earned: Decimal,
    pub conversion_rate: Decimal,
}

/// Campaign metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetrics {
    pub campaign_id: Uuid,
    pub campaign_name: String,
    pub total_referrals: i64,
    pub successful_referrals: i64,
    pub total_bonuses_paid: Decimal,
    pub budget_utilization: Decimal,       // Percentage of budget used
    pub roi: Decimal,                      // Return on investment
}

/// Referral tree node for hierarchical display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralTreeNode {
    pub user_id: Uuid,
    pub username: Option<String>,          // Optional display name
    pub level: i32,                        // Referral level (1 = direct, 2 = sub, etc.)
    pub direct_referrals: i64,
    pub total_referrals: i64,              // Including sub-referrals
    pub total_bonuses_earned: Decimal,
    pub children: Vec<ReferralTreeNode>,   // Sub-referrals
    pub joined_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Audit trail entry for referrals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralAuditTrailEntry {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub relationship_id: Option<Uuid>,     // Associated relationship
    pub action_type: String,               // create, update, bonus, flag, suspend
    pub entity_type: String,               // referral_code, relationship, campaign, bonus
    pub entity_id: Uuid,
    pub old_value: Option<String>,         // JSON string of old state
    pub new_value: Option<String>,         // JSON string of new state
    pub reason: Option<String>,
    pub performed_by: Option<Uuid>,        // User ID who performed the action
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Repository trait for referral operations
#[async_trait::async_trait]
pub trait ReferralRepository: Send + Sync {
    // Referral code operations
    async fn create_referral_code(&self, code: &ReferralCode) -> Result<ReferralCode, String>;
    async fn get_referral_code(&self, id: &Uuid) -> Result<Option<ReferralCode>, String>;
    async fn get_referral_code_by_code(&self, code: &str) -> Result<Option<ReferralCode>, String>;
    async fn list_user_referral_codes(&self, user_id: &Uuid, status: Option<ReferralCodeStatus>, campaign_id: Option<Uuid>, page: i32, page_size: i32) -> Result<(Vec<ReferralCode>, i64), String>;
    async fn update_referral_code(&self, code: &ReferralCode) -> Result<ReferralCode, String>;
    async fn deactivate_referral_code(&self, id: &Uuid, reason: String) -> Result<ReferralCode, String>;

    // Referral campaign operations
    async fn create_referral_campaign(&self, campaign: &ReferralCampaign) -> Result<ReferralCampaign, String>;
    async fn get_referral_campaign(&self, id: &Uuid) -> Result<Option<ReferralCampaign>, String>;
    async fn list_referral_campaigns(&self, campaign_type: Option<ReferralCampaignType>, status: Option<ReferralCampaignStatus>, active_only: bool, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<ReferralCampaign>, i64), String>;
    async fn update_referral_campaign(&self, campaign: &ReferralCampaign) -> Result<ReferralCampaign, String>;
    async fn delete_referral_campaign(&self, id: &Uuid) -> Result<(), String>;

    // Referral relationship operations
    async fn create_referral_relationship(&self, relationship: &ReferralRelationship) -> Result<ReferralRelationship, String>;
    async fn get_referral_relationship(&self, id: &Uuid) -> Result<Option<ReferralRelationship>, String>;
    async fn list_referral_relationships(&self, user_id: Option<Uuid>, referrer_user_id: Option<Uuid>, referee_user_id: Option<Uuid>, status: Option<ReferralRelationshipStatus>, campaign_id: Option<Uuid>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<ReferralRelationship>, i64), String>;
    async fn update_referral_relationship(&self, relationship: &ReferralRelationship) -> Result<ReferralRelationship, String>;
    async fn get_referral_tree(&self, user_id: &Uuid, max_depth: i32, include_inactive: bool) -> Result<ReferralTreeNode, String>;
    async fn get_referral_stats(&self, user_id: &Uuid, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<(i64, i64, i64, Decimal, Decimal, Decimal, Vec<CampaignMetrics>), String>;

    // Referral bonus operations
    async fn create_referral_bonus(&self, bonus: &ReferralBonus) -> Result<ReferralBonus, String>;
    async fn get_referral_bonus(&self, id: &Uuid) -> Result<Option<ReferralBonus>, String>;
    async fn list_referral_bonuses(&self, user_id: Option<Uuid>, relationship_id: Option<Uuid>, campaign_id: Option<Uuid>, status: Option<ReferralBonusStatus>, bonus_type: Option<ReferralBonusType>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<ReferralBonus>, i64), String>;
    async fn update_referral_bonus(&self, bonus: &ReferralBonus) -> Result<ReferralBonus, String>;
    async fn process_referral_bonuses(&self, relationship_id: &Uuid, milestone_type: &str, milestone_value: Option<Decimal>, force_processing: bool) -> Result<Vec<ReferralBonus>, String>;

    // Analytics operations
    async fn get_referral_metrics(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>, campaign_ids: Vec<Uuid>, user_ids: Vec<Uuid>, include_fraud_metrics: bool) -> Result<ReferralMetrics, String>;
    async fn get_user_referral_analytics(&self, user_id: &Uuid, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<(i64, i64, i64, Decimal, Decimal, Decimal, Vec<CampaignMetrics>, Vec<ReferralBonus>), String>;

    // Administrative operations
    async fn flag_suspicious_activity(&self, relationship_id: &Uuid, fraud_flags: Vec<String>, reason: String, auto_suspend: bool) -> Result<ReferralRelationship, String>;
    async fn bulk_process_bonuses(&self, relationship_ids: Vec<Uuid>, milestone_type: String, batch_id: String, reason: String) -> Result<(Vec<ReferralBonus>, i64, i64, Vec<String>), String>;

    // Audit operations
    async fn create_audit_entry(&self, entry: &ReferralAuditTrailEntry) -> Result<ReferralAuditTrailEntry, String>;
    async fn get_audit_trail(&self, user_id: Option<Uuid>, relationship_id: Option<Uuid>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, action_types: Vec<String>, page: i32, page_size: i32) -> Result<(Vec<ReferralAuditTrailEntry>, i64), String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryReferralRepository {
    referral_codes: std::sync::RwLock<HashMap<Uuid, ReferralCode>>,
    referral_campaigns: std::sync::RwLock<HashMap<Uuid, ReferralCampaign>>,
    referral_relationships: std::sync::RwLock<HashMap<Uuid, ReferralRelationship>>,
    referral_bonuses: std::sync::RwLock<HashMap<Uuid, ReferralBonus>>,
    audit_trail: std::sync::RwLock<Vec<ReferralAuditTrailEntry>>,
    code_lookup: std::sync::RwLock<HashMap<String, Uuid>>, // code -> id mapping
}
