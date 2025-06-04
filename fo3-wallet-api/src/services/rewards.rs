//! Rewards service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    rewards_service_server::RewardsService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    rewards_guard::RewardsGuard,
};
use crate::models::rewards::{
    RewardRule, UserRewards, RewardTransaction, Redemption, RedemptionOption,
    RewardRuleType, RewardRuleStatus, UserRewardTier, RewardTransactionType, RewardTransactionStatus,
    RedemptionType, RedemptionStatus, RewardMetrics, CategoryMetrics, RedemptionMetrics,
    RewardAuditTrailEntry, RewardsRepository,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Rewards service implementation
#[derive(Debug)]
pub struct RewardsServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rewards_guard: Arc<RewardsGuard>,
    rewards_repository: Arc<dyn RewardsRepository>,
}

impl RewardsServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rewards_guard: Arc<RewardsGuard>,
        rewards_repository: Arc<dyn RewardsRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            rewards_guard,
            rewards_repository,
        }
    }

    /// Convert proto reward rule type to internal enum
    fn proto_to_reward_rule_type(proto_type: i32) -> Result<RewardRuleType, Status> {
        match proto_type {
            1 => Ok(RewardRuleType::Transaction),
            2 => Ok(RewardRuleType::Spending),
            3 => Ok(RewardRuleType::Funding),
            4 => Ok(RewardRuleType::Referral),
            5 => Ok(RewardRuleType::Milestone),
            6 => Ok(RewardRuleType::Promotional),
            7 => Ok(RewardRuleType::TierBonus),
            8 => Ok(RewardRuleType::Category),
            _ => Err(Status::invalid_argument("Invalid reward rule type")),
        }
    }

    /// Convert internal reward rule type to proto
    fn reward_rule_type_to_proto(rule_type: &RewardRuleType) -> i32 {
        match rule_type {
            RewardRuleType::Transaction => 1,
            RewardRuleType::Spending => 2,
            RewardRuleType::Funding => 3,
            RewardRuleType::Referral => 4,
            RewardRuleType::Milestone => 5,
            RewardRuleType::Promotional => 6,
            RewardRuleType::TierBonus => 7,
            RewardRuleType::Category => 8,
        }
    }

    /// Convert proto reward rule status to internal enum
    fn proto_to_reward_rule_status(proto_status: i32) -> Result<RewardRuleStatus, Status> {
        match proto_status {
            1 => Ok(RewardRuleStatus::Active),
            2 => Ok(RewardRuleStatus::Inactive),
            3 => Ok(RewardRuleStatus::Expired),
            4 => Ok(RewardRuleStatus::Suspended),
            _ => Err(Status::invalid_argument("Invalid reward rule status")),
        }
    }

    /// Convert internal reward rule status to proto
    fn reward_rule_status_to_proto(status: &RewardRuleStatus) -> i32 {
        match status {
            RewardRuleStatus::Active => 1,
            RewardRuleStatus::Inactive => 2,
            RewardRuleStatus::Expired => 3,
            RewardRuleStatus::Suspended => 4,
        }
    }

    /// Convert proto user reward tier to internal enum
    fn proto_to_user_reward_tier(proto_tier: i32) -> Result<UserRewardTier, Status> {
        match proto_tier {
            1 => Ok(UserRewardTier::Bronze),
            2 => Ok(UserRewardTier::Silver),
            3 => Ok(UserRewardTier::Gold),
            4 => Ok(UserRewardTier::Platinum),
            _ => Err(Status::invalid_argument("Invalid user reward tier")),
        }
    }

    /// Convert internal user reward tier to proto
    fn user_reward_tier_to_proto(tier: &UserRewardTier) -> i32 {
        match tier {
            UserRewardTier::Bronze => 1,
            UserRewardTier::Silver => 2,
            UserRewardTier::Gold => 3,
            UserRewardTier::Platinum => 4,
        }
    }

    /// Convert proto reward transaction type to internal enum
    fn proto_to_reward_transaction_type(proto_type: i32) -> Result<RewardTransactionType, Status> {
        match proto_type {
            1 => Ok(RewardTransactionType::Earned),
            2 => Ok(RewardTransactionType::Redeemed),
            3 => Ok(RewardTransactionType::Expired),
            4 => Ok(RewardTransactionType::Adjusted),
            5 => Ok(RewardTransactionType::Bonus),
            6 => Ok(RewardTransactionType::Penalty),
            _ => Err(Status::invalid_argument("Invalid reward transaction type")),
        }
    }

    /// Convert internal reward transaction type to proto
    fn reward_transaction_type_to_proto(transaction_type: &RewardTransactionType) -> i32 {
        match transaction_type {
            RewardTransactionType::Earned => 1,
            RewardTransactionType::Redeemed => 2,
            RewardTransactionType::Expired => 3,
            RewardTransactionType::Adjusted => 4,
            RewardTransactionType::Bonus => 5,
            RewardTransactionType::Penalty => 6,
        }
    }

    /// Convert proto redemption type to internal enum
    fn proto_to_redemption_type(proto_type: i32) -> Result<RedemptionType, Status> {
        match proto_type {
            1 => Ok(RedemptionType::Cash),
            2 => Ok(RedemptionType::Credit),
            3 => Ok(RedemptionType::GiftCard),
            4 => Ok(RedemptionType::Merchandise),
            5 => Ok(RedemptionType::Discount),
            6 => Ok(RedemptionType::Charity),
            _ => Err(Status::invalid_argument("Invalid redemption type")),
        }
    }

    /// Convert internal redemption type to proto
    fn redemption_type_to_proto(redemption_type: &RedemptionType) -> i32 {
        match redemption_type {
            RedemptionType::Cash => 1,
            RedemptionType::Credit => 2,
            RedemptionType::GiftCard => 3,
            RedemptionType::Merchandise => 4,
            RedemptionType::Discount => 5,
            RedemptionType::Charity => 6,
        }
    }

    /// Convert internal reward rule to proto
    fn reward_rule_to_proto(&self, rule: &RewardRule) -> RewardRule {
        RewardRule {
            id: rule.id.to_string(),
            name: rule.name.clone(),
            description: rule.description.clone().unwrap_or_default(),
            r#type: Self::reward_rule_type_to_proto(&rule.rule_type),
            status: Self::reward_rule_status_to_proto(&rule.status),
            points_per_unit: rule.points_per_unit.to_string(),
            minimum_amount: rule.minimum_amount.map(|a| a.to_string()).unwrap_or_default(),
            maximum_points: rule.maximum_points.map(|p| p.to_string()).unwrap_or_default(),
            minimum_tier: Self::user_reward_tier_to_proto(&rule.minimum_tier),
            categories: rule.categories.clone(),
            currencies: rule.currencies.clone(),
            start_date: rule.start_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            end_date: rule.end_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            days_of_week: rule.days_of_week.clone(),
            start_time: rule.start_time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(),
            end_time: rule.end_time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(),
            max_uses_per_user: rule.max_uses_per_user.unwrap_or(-1),
            max_uses_per_day: rule.max_uses_per_day.unwrap_or(-1),
            max_uses_per_month: rule.max_uses_per_month.unwrap_or(-1),
            total_uses_remaining: rule.total_uses_remaining.unwrap_or(-1),
            metadata: rule.metadata.clone(),
            created_at: rule.created_at.to_rfc3339(),
            updated_at: rule.updated_at.to_rfc3339(),
            created_by: rule.created_by.map(|id| id.to_string()).unwrap_or_default(),
        }
    }

    /// Convert internal user rewards to proto
    fn user_rewards_to_proto(&self, rewards: &UserRewards) -> UserRewards {
        UserRewards {
            user_id: rewards.user_id.to_string(),
            total_points: rewards.total_points.to_string(),
            lifetime_earned: rewards.lifetime_earned.to_string(),
            lifetime_redeemed: rewards.lifetime_redeemed.to_string(),
            pending_points: rewards.pending_points.to_string(),
            expiring_points: rewards.expiring_points.to_string(),
            current_tier: Self::user_reward_tier_to_proto(&rewards.current_tier),
            tier_progress: rewards.tier_progress.to_string(),
            next_tier_threshold: rewards.next_tier_threshold.to_string(),
            tier_multiplier: rewards.tier_multiplier.to_string(),
            tier_benefits: rewards.tier_benefits.clone(),
            next_expiration_date: rewards.next_expiration_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            next_expiration_amount: rewards.next_expiration_amount.to_string(),
            last_activity_date: rewards.last_activity_date.to_rfc3339(),
            tier_upgrade_date: rewards.tier_upgrade_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            metadata: rewards.metadata.clone(),
            created_at: rewards.created_at.to_rfc3339(),
            updated_at: rewards.updated_at.to_rfc3339(),
        }
    }

    /// Generate unique reference number for reward transactions
    fn generate_reference_number(&self) -> String {
        format!("RWD-{}-{}", Utc::now().format("%Y%m%d"), Uuid::new_v4().to_string()[..8].to_uppercase())
    }

    /// Send notification for reward events
    async fn send_reward_notification(
        &self,
        user_id: &Uuid,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        metadata: HashMap<String, String>,
    ) -> Result<(), Status> {
        // Implementation would integrate with NotificationService
        // For now, just log the notification
        self.audit_logger.log_action(
            &user_id.to_string(),
            "reward_notification",
            &format!("Type: {:?}, Title: {}, Message: {}", notification_type, title, message),
            None,
        ).await.map_err(|e| Status::internal(format!("Failed to log notification: {}", e)))?;

        Ok(())
    }

    /// Update user tier based on lifetime earnings
    async fn update_user_tier_if_eligible(&self, user_id: &Uuid) -> Result<Option<UserRewardTier>, Status> {
        if let Ok(Some(mut user_rewards)) = self.rewards_repository.get_user_rewards(user_id).await {
            let current_tier = user_rewards.current_tier.clone();
            let lifetime_earned = user_rewards.lifetime_earned;
            
            // Determine new tier based on lifetime earnings
            let new_tier = if lifetime_earned >= UserRewardTier::Platinum.threshold() {
                UserRewardTier::Platinum
            } else if lifetime_earned >= UserRewardTier::Gold.threshold() {
                UserRewardTier::Gold
            } else if lifetime_earned >= UserRewardTier::Silver.threshold() {
                UserRewardTier::Silver
            } else {
                UserRewardTier::Bronze
            };
            
            // Update tier if changed
            if new_tier > current_tier {
                user_rewards.current_tier = new_tier.clone();
                user_rewards.tier_multiplier = new_tier.multiplier();
                user_rewards.tier_benefits = new_tier.benefits();
                user_rewards.tier_upgrade_date = Some(Utc::now());
                user_rewards.updated_at = Utc::now();
                
                // Update tier progress
                if let Some(next_tier) = new_tier.next_tier() {
                    let current_threshold = new_tier.threshold();
                    let next_threshold = next_tier.threshold();
                    let progress = if next_threshold > current_threshold {
                        ((lifetime_earned - current_threshold) / (next_threshold - current_threshold))
                            .max(Decimal::ZERO).min(Decimal::ONE)
                    } else {
                        Decimal::ONE
                    };
                    user_rewards.tier_progress = progress;
                    user_rewards.next_tier_threshold = next_threshold;
                } else {
                    user_rewards.tier_progress = Decimal::ONE;
                    user_rewards.next_tier_threshold = Decimal::ZERO;
                }
                
                // Save updated rewards
                self.rewards_repository.update_user_rewards(&user_rewards).await
                    .map_err(|e| Status::internal(format!("Failed to update user rewards: {}", e)))?;
                
                // Send tier upgrade notification
                self.send_reward_notification(
                    user_id,
                    NotificationType::RewardTierUpgrade,
                    "Tier Upgrade!",
                    &format!("Congratulations! You've been upgraded to {:?} tier!", new_tier),
                    HashMap::new(),
                ).await?;
                
                return Ok(Some(new_tier));
            }
        }
        
        Ok(None)
    }

    /// Convert internal reward transaction to proto
    fn reward_transaction_to_proto(&self, transaction: &RewardTransaction) -> RewardTransaction {
        RewardTransaction {
            id: transaction.id.to_string(),
            user_id: transaction.user_id.to_string(),
            r#type: Self::reward_transaction_type_to_proto(&transaction.transaction_type),
            status: Self::reward_transaction_status_to_proto(&transaction.status),
            points: transaction.points.to_string(),
            multiplier: transaction.multiplier.to_string(),
            base_points: transaction.base_points.to_string(),
            currency: transaction.currency.clone(),
            exchange_rate: transaction.exchange_rate.map(|r| r.to_string()).unwrap_or_default(),
            source_type: transaction.source_type.clone().unwrap_or_default(),
            source_id: transaction.source_id.clone().unwrap_or_default(),
            reward_rule_id: transaction.reward_rule_id.map(|id| id.to_string()).unwrap_or_default(),
            reference_number: transaction.reference_number.clone(),
            expires_at: transaction.expires_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            is_expired: transaction.is_expired,
            description: transaction.description.clone().unwrap_or_default(),
            metadata: transaction.metadata.clone(),
            created_at: transaction.created_at.to_rfc3339(),
            updated_at: transaction.updated_at.to_rfc3339(),
        }
    }

    /// Convert proto reward transaction status to internal enum
    fn proto_to_reward_transaction_status(proto_status: i32) -> Result<RewardTransactionStatus, Status> {
        match proto_status {
            1 => Ok(RewardTransactionStatus::Pending),
            2 => Ok(RewardTransactionStatus::Completed),
            3 => Ok(RewardTransactionStatus::Failed),
            4 => Ok(RewardTransactionStatus::Cancelled),
            5 => Ok(RewardTransactionStatus::Expired),
            _ => Err(Status::invalid_argument("Invalid reward transaction status")),
        }
    }

    /// Convert internal reward transaction status to proto
    fn reward_transaction_status_to_proto(status: &RewardTransactionStatus) -> i32 {
        match status {
            RewardTransactionStatus::Pending => 1,
            RewardTransactionStatus::Completed => 2,
            RewardTransactionStatus::Failed => 3,
            RewardTransactionStatus::Cancelled => 4,
            RewardTransactionStatus::Expired => 5,
        }
    }

    /// Convert proto redemption status to internal enum
    fn proto_to_redemption_status(proto_status: i32) -> Result<RedemptionStatus, Status> {
        match proto_status {
            1 => Ok(RedemptionStatus::Pending),
            2 => Ok(RedemptionStatus::Processing),
            3 => Ok(RedemptionStatus::Completed),
            4 => Ok(RedemptionStatus::Failed),
            5 => Ok(RedemptionStatus::Cancelled),
            6 => Ok(RedemptionStatus::Expired),
            _ => Err(Status::invalid_argument("Invalid redemption status")),
        }
    }

    /// Convert internal redemption status to proto
    fn redemption_status_to_proto(status: &RedemptionStatus) -> i32 {
        match status {
            RedemptionStatus::Pending => 1,
            RedemptionStatus::Processing => 2,
            RedemptionStatus::Completed => 3,
            RedemptionStatus::Failed => 4,
            RedemptionStatus::Cancelled => 5,
            RedemptionStatus::Expired => 6,
        }
    }

    /// Convert internal redemption to proto
    fn redemption_to_proto(&self, redemption: &Redemption) -> Redemption {
        Redemption {
            id: redemption.id.to_string(),
            user_id: redemption.user_id.to_string(),
            r#type: Self::redemption_type_to_proto(&redemption.redemption_type),
            status: Self::redemption_status_to_proto(&redemption.status),
            points_redeemed: redemption.points_redeemed.to_string(),
            cash_value: redemption.cash_value.to_string(),
            currency: redemption.currency.clone(),
            exchange_rate: redemption.exchange_rate.to_string(),
            target_account: redemption.target_account.clone().unwrap_or_default(),
            gift_card_code: redemption.gift_card_code.clone().unwrap_or_default(),
            merchant_name: redemption.merchant_name.clone().unwrap_or_default(),
            tracking_number: redemption.tracking_number.clone().unwrap_or_default(),
            processing_fee: redemption.processing_fee.to_string(),
            net_amount: redemption.net_amount.to_string(),
            estimated_delivery: redemption.estimated_delivery.map(|d| d.to_rfc3339()).unwrap_or_default(),
            actual_delivery: redemption.actual_delivery.map(|d| d.to_rfc3339()).unwrap_or_default(),
            description: redemption.description.clone().unwrap_or_default(),
            metadata: redemption.metadata.clone(),
            created_at: redemption.created_at.to_rfc3339(),
            updated_at: redemption.updated_at.to_rfc3339(),
            completed_at: redemption.completed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        }
    }

    /// Convert internal redemption option to proto
    fn redemption_option_to_proto(&self, option: &RedemptionOption) -> RedemptionOption {
        RedemptionOption {
            id: option.id.to_string(),
            name: option.name.clone(),
            description: option.description.clone().unwrap_or_default(),
            redemption_type: Self::redemption_type_to_proto(&option.redemption_type),
            points_required: option.points_required.to_string(),
            cash_value: option.cash_value.to_string(),
            currency: option.currency.clone(),
            exchange_rate: option.exchange_rate.to_string(),
            is_active: option.is_active,
            minimum_tier: Self::user_reward_tier_to_proto(&option.minimum_tier),
            max_redemptions_per_user: option.maximum_redemptions_per_user.unwrap_or(-1),
            max_redemptions_per_day: option.maximum_redemptions_per_day.unwrap_or(-1),
            minimum_points_balance: option.minimum_points_balance.to_string(),
            image_url: option.image_url.clone().unwrap_or_default(),
            tags: option.tags.clone(),
            metadata: option.metadata.clone(),
            created_at: option.created_at.to_rfc3339(),
            updated_at: option.updated_at.to_rfc3339(),
        }
    }

    /// Internal helper method for awarding points (used by bulk operations)
    async fn award_points_internal(
        &self,
        user_id: &Uuid,
        points: &Decimal,
        source_type: &str,
        source_id: &str,
        description: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<Decimal, String> {
        // Get or create user rewards
        let mut user_rewards = match self.rewards_repository.get_user_rewards(user_id).await {
            Ok(Some(rewards)) => rewards,
            Ok(None) => {
                let default_rewards = UserRewards {
                    user_id: *user_id,
                    ..Default::default()
                };
                self.rewards_repository.create_user_rewards(&default_rewards).await?
            },
            Err(e) => return Err(e),
        };

        // Apply tier multiplier
        let multiplier = user_rewards.tier_multiplier;
        let final_points = *points * multiplier;

        // Create reward transaction
        let transaction = RewardTransaction {
            id: Uuid::new_v4(),
            user_id: *user_id,
            transaction_type: RewardTransactionType::Earned,
            status: RewardTransactionStatus::Completed,
            points: final_points,
            multiplier,
            base_points: *points,
            currency: "USD".to_string(),
            exchange_rate: None,
            source_type: Some(source_type.to_string()),
            source_id: Some(source_id.to_string()),
            reward_rule_id: None,
            reference_number: self.generate_reference_number(),
            expires_at: None,
            is_expired: false,
            description: Some(description.to_string()),
            metadata: metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save transaction
        self.rewards_repository.create_reward_transaction(&transaction).await?;

        // Update user rewards
        user_rewards.total_points += final_points;
        user_rewards.lifetime_earned += final_points;
        user_rewards.last_activity_date = Utc::now();
        user_rewards.updated_at = Utc::now();

        self.rewards_repository.update_user_rewards(&user_rewards).await?;

        // Check for tier upgrade
        self.update_user_tier_if_eligible(user_id).await.map_err(|e| e.to_string())?;

        Ok(final_points)
    }
}
