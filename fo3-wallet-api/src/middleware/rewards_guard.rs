//! Rewards security guard middleware

use std::sync::Arc;
use tonic::{Request, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    rate_limit::RateLimiter,
};
use crate::models::rewards::{
    RewardsRepository, UserRewardTier, RewardTransactionType, RedemptionType,
    RewardTransaction, Redemption,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// Rewards security guard for validation and fraud prevention
#[derive(Debug)]
pub struct RewardsGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    rewards_repository: Arc<dyn RewardsRepository>,
}

impl RewardsGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        rewards_repository: Arc<dyn RewardsRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            rewards_repository,
        }
    }

    /// Validate reward rule creation
    pub async fn validate_reward_rule_creation<T>(
        &self,
        request: &Request<T>,
        rule_name: &str,
        points_per_unit: &Decimal,
        minimum_tier: &UserRewardTier,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions for reward rule management
        if !self.auth_service.has_permission(&auth_context, Permission::ManageRewards).await? {
            return Err(Status::permission_denied("Insufficient permissions to create reward rules"));
        }

        // Rate limiting for rule creation
        let rate_limit_key = format!("reward_rule_creation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 10, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for reward rule creation"));
        }

        // Validate rule parameters
        if rule_name.is_empty() {
            return Err(Status::invalid_argument("Rule name cannot be empty"));
        }

        if rule_name.len() > 255 {
            return Err(Status::invalid_argument("Rule name too long (max 255 characters)"));
        }

        if *points_per_unit < Decimal::ZERO {
            return Err(Status::invalid_argument("Points per unit cannot be negative"));
        }

        if *points_per_unit > Decimal::from(10000) {
            return Err(Status::invalid_argument("Points per unit too high (max 10,000)"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_reward_rule_creation",
            &format!("Rule: {}, Points: {}, Tier: {:?}", rule_name, points_per_unit, minimum_tier),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate points awarding
    pub async fn validate_points_award<T>(
        &self,
        request: &Request<T>,
        user_id: &Uuid,
        points: &Decimal,
        source_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions for awarding points
        if !self.auth_service.has_permission(&auth_context, Permission::ManageRewards).await? {
            return Err(Status::permission_denied("Insufficient permissions to award points"));
        }

        // Rate limiting for points awarding
        let rate_limit_key = format!("points_award:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 100, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for points awarding"));
        }

        // Validate points amount
        if *points <= Decimal::ZERO {
            return Err(Status::invalid_argument("Points amount must be positive"));
        }

        if *points > Decimal::from(100000) {
            return Err(Status::invalid_argument("Points amount too high (max 100,000 per award)"));
        }

        // Validate source type
        let valid_sources = ["transaction", "referral", "milestone", "promotional", "manual", "bonus"];
        if !valid_sources.contains(&source_type) {
            return Err(Status::invalid_argument("Invalid source type"));
        }

        // Check for suspicious patterns (e.g., too many awards to same user)
        if let Err(e) = self.check_award_patterns(user_id, points).await {
            return Err(Status::failed_precondition(&format!("Suspicious award pattern detected: {}", e)));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_points_award",
            &format!("User: {}, Points: {}, Source: {}", user_id, points, source_type),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate points redemption
    pub async fn validate_points_redemption<T>(
        &self,
        request: &Request<T>,
        user_id: &Uuid,
        points_to_redeem: &Decimal,
        redemption_type: &RedemptionType,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can redeem their own points or has admin permissions
        if auth_context.user_id != *user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::ManageRewards).await? {
            return Err(Status::permission_denied("Can only redeem your own points"));
        }

        // Rate limiting for redemptions
        let rate_limit_key = format!("points_redemption:{}", user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 10, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for points redemption"));
        }

        // Validate redemption amount
        if *points_to_redeem <= Decimal::ZERO {
            return Err(Status::invalid_argument("Redemption amount must be positive"));
        }

        if *points_to_redeem > Decimal::from(1000000) {
            return Err(Status::invalid_argument("Redemption amount too high (max 1,000,000 per redemption)"));
        }

        // Check user's available balance
        if let Ok(Some(user_rewards)) = self.rewards_repository.get_user_rewards(user_id).await {
            if user_rewards.total_points < *points_to_redeem {
                return Err(Status::failed_precondition("Insufficient points balance"));
            }
        } else {
            return Err(Status::not_found("User rewards not found"));
        }

        // Check for suspicious redemption patterns
        if let Err(e) = self.check_redemption_patterns(user_id, points_to_redeem, redemption_type).await {
            return Err(Status::failed_precondition(&format!("Suspicious redemption pattern detected: {}", e)));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_points_redemption",
            &format!("User: {}, Points: {}, Type: {:?}", user_id, points_to_redeem, redemption_type),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate tier update
    pub async fn validate_tier_update<T>(
        &self,
        request: &Request<T>,
        user_id: &Uuid,
        new_tier: &UserRewardTier,
        force_update: bool,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions for tier management
        if !self.auth_service.has_permission(&auth_context, Permission::ManageRewards).await? {
            return Err(Status::permission_denied("Insufficient permissions to update user tiers"));
        }

        // Rate limiting for tier updates
        let rate_limit_key = format!("tier_update:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 20, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for tier updates"));
        }

        // Validate tier progression unless forced
        if !force_update {
            if let Ok(Some(user_rewards)) = self.rewards_repository.get_user_rewards(user_id).await {
                // Check if user has earned enough points for the new tier
                let required_threshold = new_tier.threshold();
                if user_rewards.lifetime_earned < required_threshold {
                    return Err(Status::failed_precondition(
                        &format!("User has not earned enough points for {:?} tier (required: {}, earned: {})", 
                                new_tier, required_threshold, user_rewards.lifetime_earned)
                    ));
                }

                // Prevent tier downgrades unless forced
                if *new_tier < user_rewards.current_tier {
                    return Err(Status::failed_precondition("Cannot downgrade tier without force flag"));
                }
            }
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_tier_update",
            &format!("User: {}, New Tier: {:?}, Forced: {}", user_id, new_tier, force_update),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate analytics access
    pub async fn validate_analytics_access<T>(
        &self,
        request: &Request<T>,
        requested_user_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if accessing own analytics or has admin permissions
        if let Some(user_id) = requested_user_id {
            if auth_context.user_id != *user_id && 
               !self.auth_service.has_permission(&auth_context, Permission::ViewReports).await? {
                return Err(Status::permission_denied("Can only view your own analytics"));
            }
        } else {
            // Accessing system-wide analytics requires admin permissions
            if !self.auth_service.has_permission(&auth_context, Permission::ViewReports).await? {
                return Err(Status::permission_denied("Insufficient permissions to view system analytics"));
            }
        }

        // Rate limiting for analytics access
        let rate_limit_key = format!("analytics_access:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 50, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for analytics access"));
        }

        Ok(auth_context)
    }

    /// Check for suspicious award patterns
    async fn check_award_patterns(&self, user_id: &Uuid, points: &Decimal) -> Result<(), String> {
        // Get recent transactions for the user
        let end_date = Utc::now();
        let start_date = end_date - Duration::hours(24);
        
        if let Ok((transactions, _)) = self.rewards_repository.list_reward_transactions(
            Some(*user_id),
            Some(RewardTransactionType::Earned),
            None,
            Some(start_date),
            Some(end_date),
            0,
            100,
        ).await {
            // Check for too many awards in short time
            if transactions.len() > 50 {
                return Err("Too many awards in 24 hours".to_string());
            }

            // Check for unusually large awards
            let total_points: Decimal = transactions.iter().map(|tx| tx.points).sum();
            if total_points + points > Decimal::from(50000) {
                return Err("Daily award limit exceeded".to_string());
            }
        }

        Ok(())
    }

    /// Check for suspicious redemption patterns
    async fn check_redemption_patterns(
        &self,
        user_id: &Uuid,
        points: &Decimal,
        redemption_type: &RedemptionType,
    ) -> Result<(), String> {
        // Get recent redemptions for the user
        let end_date = Utc::now();
        let start_date = end_date - Duration::hours(24);
        
        if let Ok((redemptions, _)) = self.rewards_repository.list_redemptions(
            Some(*user_id),
            Some(redemption_type.clone()),
            None,
            Some(start_date),
            Some(end_date),
            0,
            100,
        ).await {
            // Check for too many redemptions in short time
            if redemptions.len() > 10 {
                return Err("Too many redemptions in 24 hours".to_string());
            }

            // Check for unusually large redemptions
            let total_points: Decimal = redemptions.iter().map(|r| r.points_redeemed).sum();
            if total_points + points > Decimal::from(100000) {
                return Err("Daily redemption limit exceeded".to_string());
            }
        }

        Ok(())
    }

    /// Validate reward rule update request
    pub async fn validate_reward_rule_update<T>(
        &self,
        request: &Request<T>,
        rule_id: &Uuid,
        name: &str,
        points_per_unit: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Rate limiting
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "reward_rule_update",
            10, // 10 updates per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate rule name
        if name.is_empty() || name.len() > 100 {
            return Err(Status::invalid_argument("Rule name must be 1-100 characters"));
        }

        // Validate points per unit
        if *points_per_unit <= Decimal::ZERO || *points_per_unit > Decimal::from(1000) {
            return Err(Status::invalid_argument("Points per unit must be between 0 and 1000"));
        }

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_reward_rule_update",
            &format!("Validated update for rule: {}", rule_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }

    /// Validate reward rule deletion request
    pub async fn validate_reward_rule_deletion<T>(
        &self,
        request: &Request<T>,
        rule_id: &Uuid,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Rate limiting
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "reward_rule_deletion",
            5, // 5 deletions per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_reward_rule_deletion",
            &format!("Validated deletion for rule: {}", rule_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }

    /// Validate points redemption request
    pub async fn validate_points_redemption<T>(
        &self,
        request: &Request<T>,
        user_id: &Uuid,
        points: &Decimal,
        redemption_type: &RedemptionType,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can redeem their own points or has admin permissions
        if auth_context.user_id != *user_id {
            self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;
        }

        // Rate limiting for redemptions
        self.rate_limiter.check_rate_limit(
            &user_id.to_string(),
            "points_redemption",
            10, // 10 redemptions per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate points amount
        if *points <= Decimal::ZERO {
            return Err(Status::invalid_argument("Points amount must be positive"));
        }

        if *points > Decimal::from(100000) {
            return Err(Status::invalid_argument("Points amount too large"));
        }

        // Validate redemption type
        match redemption_type {
            RedemptionType::Cash => {
                if *points < Decimal::from(1000) {
                    return Err(Status::invalid_argument("Minimum 1000 points required for cash redemption"));
                }
            },
            RedemptionType::GiftCard => {
                if *points < Decimal::from(500) {
                    return Err(Status::invalid_argument("Minimum 500 points required for gift card redemption"));
                }
            },
            _ => {} // Other types have no minimum
        }

        // Check for suspicious redemption patterns
        self.check_redemption_fraud_patterns(user_id, points, redemption_type.clone()).await
            .map_err(|e| Status::failed_precondition(format!("Fraud check failed: {}", e)))?;

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_points_redemption",
            &format!("Validated redemption: {} points for user {}", points, user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }
}
