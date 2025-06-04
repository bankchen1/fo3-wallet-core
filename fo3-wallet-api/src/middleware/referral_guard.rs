//! Referral security guard middleware

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
use crate::models::referral::{
    ReferralRepository, ReferralCodeStatus, ReferralCampaignType, ReferralCampaignStatus, ReferralBonusType,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// Referral security guard for validation and fraud prevention
#[derive(Debug)]
pub struct ReferralGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    referral_repository: Arc<dyn ReferralRepository>,
}

impl ReferralGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        referral_repository: Arc<dyn ReferralRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            referral_repository,
        }
    }

    /// Validate referral code generation
    pub async fn validate_referral_code_generation<T>(
        &self,
        request: &Request<T>,
        user_id: &Uuid,
        custom_code: Option<&str>,
        campaign_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can generate codes for themselves or has admin permissions
        if auth_context.user_id != *user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::ManageReferrals).await? {
            return Err(Status::permission_denied("Can only generate referral codes for yourself"));
        }

        // Rate limiting for code generation
        let rate_limit_key = format!("referral_code_generation:{}", user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 10, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for referral code generation"));
        }

        // Validate custom code if provided
        if let Some(code) = custom_code {
            if code.is_empty() {
                return Err(Status::invalid_argument("Custom code cannot be empty"));
            }
            
            if code.len() < 3 || code.len() > 50 {
                return Err(Status::invalid_argument("Custom code must be between 3 and 50 characters"));
            }
            
            // Check for inappropriate content (basic validation)
            let code_lower = code.to_lowercase();
            let forbidden_words = ["admin", "test", "fake", "spam", "scam"];
            if forbidden_words.iter().any(|word| code_lower.contains(word)) {
                return Err(Status::invalid_argument("Custom code contains forbidden content"));
            }
            
            // Check if code already exists
            if let Ok(Some(_)) = self.referral_repository.get_referral_code_by_code(code).await {
                return Err(Status::already_exists("Referral code already exists"));
            }
        }

        // Check user's existing active codes limit
        if let Ok((existing_codes, _)) = self.referral_repository.list_user_referral_codes(
            user_id,
            Some(ReferralCodeStatus::Active),
            campaign_id.copied(),
            0,
            100,
        ).await {
            if existing_codes.len() >= 10 {
                return Err(Status::failed_precondition("Maximum number of active referral codes reached"));
            }
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_code_generation",
            &format!("User: {}, Custom: {:?}, Campaign: {:?}", user_id, custom_code, campaign_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate referral relationship creation
    pub async fn validate_referral_relationship_creation<T>(
        &self,
        request: &Request<T>,
        referrer_user_id: &Uuid,
        referee_user_id: &Uuid,
        referral_code: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can create relationships for themselves or has admin permissions
        if auth_context.user_id != *referee_user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::ManageReferrals).await? {
            return Err(Status::permission_denied("Can only create referral relationships for yourself"));
        }

        // Rate limiting for relationship creation
        let rate_limit_key = format!("referral_relationship_creation:{}", referee_user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 5, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for referral relationship creation"));
        }

        // Validate referral code
        let code_record = self.referral_repository.get_referral_code_by_code(referral_code).await
            .map_err(|e| Status::internal(format!("Failed to validate referral code: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral code not found"))?;

        if !code_record.is_valid() {
            return Err(Status::failed_precondition("Referral code is not valid"));
        }

        if code_record.user_id != *referrer_user_id {
            return Err(Status::invalid_argument("Referral code does not belong to the specified referrer"));
        }

        // Prevent self-referrals
        if referrer_user_id == referee_user_id {
            return Err(Status::invalid_argument("Users cannot refer themselves"));
        }

        // Check for existing relationship
        if let Ok((existing_relationships, _)) = self.referral_repository.list_referral_relationships(
            None,
            Some(*referrer_user_id),
            Some(*referee_user_id),
            None,
            None,
            None,
            None,
            0,
            1,
        ).await {
            if !existing_relationships.is_empty() {
                return Err(Status::already_exists("Referral relationship already exists between these users"));
            }
        }

        // Check for suspicious patterns
        if let Err(e) = self.check_referral_patterns(referrer_user_id, referee_user_id).await {
            return Err(Status::failed_precondition(&format!("Suspicious referral pattern detected: {}", e)));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_relationship_creation",
            &format!("Referrer: {}, Referee: {}, Code: {}", referrer_user_id, referee_user_id, referral_code),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate referral campaign creation
    pub async fn validate_referral_campaign_creation<T>(
        &self,
        request: &Request<T>,
        campaign_name: &str,
        campaign_type: &ReferralCampaignType,
        referrer_bonus: &Decimal,
        referee_bonus: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions for campaign management
        if !self.auth_service.has_permission(&auth_context, Permission::ManageReferrals).await? {
            return Err(Status::permission_denied("Insufficient permissions to create referral campaigns"));
        }

        // Rate limiting for campaign creation
        let rate_limit_key = format!("referral_campaign_creation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 5, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for referral campaign creation"));
        }

        // Validate campaign parameters
        if campaign_name.is_empty() {
            return Err(Status::invalid_argument("Campaign name cannot be empty"));
        }

        if campaign_name.len() > 255 {
            return Err(Status::invalid_argument("Campaign name too long (max 255 characters)"));
        }

        if *referrer_bonus < Decimal::ZERO {
            return Err(Status::invalid_argument("Referrer bonus cannot be negative"));
        }

        if *referee_bonus < Decimal::ZERO {
            return Err(Status::invalid_argument("Referee bonus cannot be negative"));
        }

        if *referrer_bonus > Decimal::from(10000) {
            return Err(Status::invalid_argument("Referrer bonus too high (max 10,000)"));
        }

        if *referee_bonus > Decimal::from(10000) {
            return Err(Status::invalid_argument("Referee bonus too high (max 10,000)"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_campaign_creation",
            &format!("Campaign: {}, Type: {:?}, Referrer Bonus: {}, Referee Bonus: {}", 
                    campaign_name, campaign_type, referrer_bonus, referee_bonus),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate bonus processing
    pub async fn validate_bonus_processing<T>(
        &self,
        request: &Request<T>,
        relationship_id: &Uuid,
        milestone_type: &str,
        force_processing: bool,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions for bonus processing
        if !self.auth_service.has_permission(&auth_context, Permission::ManageReferrals).await? {
            return Err(Status::permission_denied("Insufficient permissions to process referral bonuses"));
        }

        // Rate limiting for bonus processing
        let rate_limit_key = format!("referral_bonus_processing:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 50, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for referral bonus processing"));
        }

        // Validate milestone type
        let valid_milestones = ["signup", "first_transaction", "kyc", "tier_upgrade", "spending_milestone"];
        if !valid_milestones.contains(&milestone_type) {
            return Err(Status::invalid_argument("Invalid milestone type"));
        }

        // Check if relationship exists
        let relationship = self.referral_repository.get_referral_relationship(relationship_id).await
            .map_err(|e| Status::internal(format!("Failed to get referral relationship: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral relationship not found"))?;

        // Check if relationship is eligible for bonus (unless forced)
        if !force_processing && !relationship.is_eligible_for_bonus(milestone_type) {
            return Err(Status::failed_precondition("Referral relationship not eligible for bonus"));
        }

        // Check for suspicious activity
        if relationship.is_suspicious {
            return Err(Status::failed_precondition("Cannot process bonuses for flagged relationships"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_bonus_processing",
            &format!("Relationship: {}, Milestone: {}, Forced: {}", relationship_id, milestone_type, force_processing),
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
                return Err(Status::permission_denied("Can only view your own referral analytics"));
            }
        } else {
            // Accessing system-wide analytics requires admin permissions
            if !self.auth_service.has_permission(&auth_context, Permission::ViewReports).await? {
                return Err(Status::permission_denied("Insufficient permissions to view system referral analytics"));
            }
        }

        // Rate limiting for analytics access
        let rate_limit_key = format!("referral_analytics_access:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 30, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for referral analytics access"));
        }

        Ok(auth_context)
    }

    /// Check for suspicious referral patterns
    async fn check_referral_patterns(&self, referrer_user_id: &Uuid, referee_user_id: &Uuid) -> Result<(), String> {
        // Get recent relationships for the referrer
        let end_date = Utc::now();
        let start_date = end_date - Duration::hours(24);
        
        if let Ok((relationships, _)) = self.referral_repository.list_referral_relationships(
            None,
            Some(*referrer_user_id),
            None,
            None,
            None,
            Some(start_date),
            Some(end_date),
            0,
            100,
        ).await {
            // Check for too many referrals in short time
            if relationships.len() > 20 {
                return Err("Too many referrals in 24 hours".to_string());
            }
            
            // Check for patterns that might indicate fraud
            let unique_ips: std::collections::HashSet<_> = relationships
                .iter()
                .filter_map(|rel| rel.ip_address.as_ref())
                .collect();
            
            if relationships.len() > 5 && unique_ips.len() == 1 {
                return Err("Multiple referrals from same IP address".to_string());
            }
        }

        // Check if referee has been referred by multiple users recently
        if let Ok((referee_relationships, _)) = self.referral_repository.list_referral_relationships(
            None,
            None,
            Some(*referee_user_id),
            None,
            None,
            Some(start_date),
            Some(end_date),
            0,
            10,
        ).await {
            if referee_relationships.len() > 1 {
                return Err("User has multiple recent referral relationships".to_string());
            }
        }

        Ok(())
    }

    /// Validate referral code deactivation request
    pub async fn validate_referral_code_deactivation<T>(
        &self,
        request: &Request<T>,
        code_id: &Uuid,
        reason: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "referral_code_deactivation",
            10, // 10 deactivations per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate reason
        if reason.is_empty() || reason.len() > 500 {
            return Err(Status::invalid_argument("Deactivation reason must be 1-500 characters"));
        }

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_code_deactivation",
            &format!("Validated deactivation for code: {} - Reason: {}", code_id, reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }

    /// Validate referral bonus processing request
    pub async fn validate_referral_bonus_processing<T>(
        &self,
        request: &Request<T>,
        relationship_id: &Uuid,
        bonus_type: &ReferralBonusType,
        bonus_amount: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageReferrals)?;

        // Rate limiting
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "referral_bonus_processing",
            50, // 50 bonus processes per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate bonus amount
        if *bonus_amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Bonus amount must be positive"));
        }

        if *bonus_amount > Decimal::from(10000) {
            return Err(Status::invalid_argument("Bonus amount too large"));
        }

        // Check for suspicious bonus patterns
        self.check_bonus_fraud_patterns(relationship_id, bonus_type, bonus_amount).await
            .map_err(|e| Status::failed_precondition(format!("Fraud check failed: {}", e)))?;

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_bonus_processing",
            &format!("Validated bonus processing: {} {:?} for relationship {}", bonus_amount, bonus_type, relationship_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }

    /// Check for suspicious bonus patterns
    async fn check_bonus_fraud_patterns(
        &self,
        relationship_id: &Uuid,
        bonus_type: &ReferralBonusType,
        bonus_amount: &Decimal,
    ) -> Result<(), String> {
        // Check for excessive bonus amounts in short time period
        let start_date = Utc::now() - Duration::hours(24);
        let end_date = Utc::now();

        if let Ok(bonuses) = self.referral_repository.list_referral_bonuses(
            Some(*relationship_id),
            None,
            Some(bonus_type.clone()),
            None,
            Some(start_date),
            Some(end_date),
            0,
            100,
        ).await {
            let total_amount: Decimal = bonuses.0.iter()
                .map(|bonus| bonus.bonus_amount)
                .sum();

            if total_amount + bonus_amount > Decimal::from(5000) {
                return Err("Excessive bonus amount in 24 hours".to_string());
            }

            if bonuses.0.len() > 10 {
                return Err("Too many bonuses processed in 24 hours".to_string());
            }
        }

        Ok(())
    }

    /// Validate referral campaign creation request
    pub async fn validate_referral_campaign_creation<T>(
        &self,
        request: &Request<T>,
        campaign_name: &str,
        referrer_bonus: &Decimal,
        referee_bonus: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageReferrals)?;

        // Rate limiting
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "referral_campaign_creation",
            5, // 5 campaign creations per hour
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate campaign name
        if campaign_name.is_empty() || campaign_name.len() > 100 {
            return Err(Status::invalid_argument("Campaign name must be 1-100 characters"));
        }

        // Validate bonus amounts
        if *referrer_bonus <= Decimal::ZERO || *referrer_bonus > Decimal::from(1000) {
            return Err(Status::invalid_argument("Referrer bonus must be between 0 and 1000"));
        }

        if *referee_bonus <= Decimal::ZERO || *referee_bonus > Decimal::from(1000) {
            return Err(Status::invalid_argument("Referee bonus must be between 0 and 1000"));
        }

        // Log the validation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_campaign_creation",
            &format!("Validated campaign creation: {} (referrer: {}, referee: {})", campaign_name, referrer_bonus, referee_bonus),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log validation: {}", e)))?;

        Ok(auth_context)
    }

    /// Validate referral campaign update request
    pub async fn validate_referral_campaign_update<T>(
        &self,
        request: &Request<T>,
        campaign_name: &str,
        current_status: &ReferralCampaignStatus,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageReferrals)?;

        // Rate limiting - 10 campaign updates per hour
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "referral_campaign_update",
            10,
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate campaign name
        if campaign_name.is_empty() {
            return Err(Status::invalid_argument("Campaign name cannot be empty"));
        }

        if campaign_name.len() > 100 {
            return Err(Status::invalid_argument("Campaign name too long (max 100 characters)"));
        }

        // Business logic validation based on current status
        match current_status {
            ReferralCampaignStatus::Completed => {
                return Err(Status::failed_precondition("Cannot update completed campaigns"));
            }
            ReferralCampaignStatus::Cancelled => {
                return Err(Status::failed_precondition("Cannot update cancelled campaigns"));
            }
            _ => {} // Allow updates for Draft, Active, and Paused campaigns
        }

        // Fraud prevention - check for suspicious update patterns
        if self.is_suspicious_campaign_activity(&auth_context.user_id, "update").await {
            return Err(Status::permission_denied("Suspicious campaign update activity detected"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_campaign_update",
            &format!("Campaign: {}, Current Status: {:?}", campaign_name, current_status),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Validate referral campaign deletion request
    pub async fn validate_referral_campaign_deletion<T>(
        &self,
        request: &Request<T>,
        current_status: &ReferralCampaignStatus,
        reason: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check admin permissions - only admins can delete campaigns
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
            return Err(Status::permission_denied("Admin permissions required to delete campaigns"));
        }

        // Rate limiting - 5 campaign deletions per hour
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "referral_campaign_deletion",
            5,
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Validate reason is provided
        if reason.is_empty() {
            return Err(Status::invalid_argument("Deletion reason is required"));
        }

        if reason.len() > 500 {
            return Err(Status::invalid_argument("Deletion reason too long (max 500 characters)"));
        }

        // Business logic validation based on current status
        match current_status {
            ReferralCampaignStatus::Cancelled => {
                return Err(Status::failed_precondition("Campaign is already cancelled"));
            }
            _ => {} // Allow deletion for all other statuses
        }

        // Fraud prevention - check for suspicious deletion patterns
        if self.is_suspicious_campaign_activity(&auth_context.user_id, "deletion").await {
            return Err(Status::permission_denied("Suspicious campaign deletion activity detected"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_referral_campaign_deletion",
            &format!("Status: {:?}, Reason: {}", current_status, reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Check for suspicious campaign activity patterns
    async fn is_suspicious_campaign_activity(&self, user_id: &Uuid, activity_type: &str) -> bool {
        // Simplified fraud detection - in production this would be more sophisticated
        // Check for rapid successive operations, unusual patterns, etc.

        // For now, just return false - implement actual fraud detection logic here
        false
    }

    /// Validate analytics access request
    pub async fn validate_analytics_access<T>(
        &self,
        request: &Request<T>,
        target_user_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting for analytics operations - 50 requests per hour
        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            "analytics_access",
            50,
            Duration::hours(1),
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Check if user is accessing their own data or has admin permissions
        if let Some(target_id) = target_user_id {
            let has_view_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false);
            let is_own_data = auth_context.user_id == *target_id;

            if !has_view_permission && !is_own_data {
                return Err(Status::permission_denied("Insufficient permissions to access analytics data"));
            }
        } else {
            // For global analytics, require admin permissions
            if !self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false) {
                return Err(Status::permission_denied("Admin permissions required for global analytics"));
            }
        }

        // Fraud prevention - check for suspicious analytics access patterns
        if self.is_suspicious_analytics_activity(&auth_context.user_id).await {
            return Err(Status::permission_denied("Suspicious analytics access activity detected"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_analytics_access",
            &format!("Target user: {:?}", target_user_id.map(|id| id.to_string())),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Check for suspicious analytics activity patterns
    async fn is_suspicious_analytics_activity(&self, user_id: &Uuid) -> bool {
        // Simplified fraud detection for analytics access
        // In production, this would check for:
        // - Rapid successive analytics requests
        // - Unusual access patterns
        // - Access to large amounts of data
        // - Requests from suspicious IP addresses

        // For now, just return false - implement actual fraud detection logic here
        false
    }

    /// Validate administrative access request
    pub async fn validate_administrative_access<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check admin permissions for administrative operations
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required for administrative operations"));
        }

        // Rate limiting based on operation type
        let (rate_limit, duration) = match operation_type {
            "bulk_process_bonuses" => (5, Duration::hours(1)),  // 5 bulk operations per hour
            "flag_suspicious_activity" => (20, Duration::hours(1)), // 20 fraud flags per hour
            "get_audit_trail" => (30, Duration::hours(1)),     // 30 audit queries per hour
            "recalculate_metrics" => (3, Duration::hours(1)),  // 3 recalculations per hour
            "user_analytics" => (50, Duration::hours(1)),      // 50 analytics requests per hour
            _ => (10, Duration::hours(1)),                      // Default rate limit
        };

        self.rate_limiter.check_rate_limit(
            &auth_context.user_id.to_string(),
            &format!("admin_{}", operation_type),
            rate_limit,
            duration,
        ).await.map_err(|e| Status::resource_exhausted(format!("Rate limit exceeded: {}", e)))?;

        // Fraud prevention - check for suspicious administrative activity
        if self.is_suspicious_administrative_activity(&auth_context.user_id, operation_type).await {
            return Err(Status::permission_denied("Suspicious administrative activity detected"));
        }

        // Log the validation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "validate_administrative_access",
            &format!("Operation: {}", operation_type),
            request.remote_addr().map(|addr| addr.ip()),
        ).await?;

        Ok(auth_context)
    }

    /// Check for suspicious administrative activity patterns
    async fn is_suspicious_administrative_activity(&self, user_id: &Uuid, operation_type: &str) -> bool {
        // Simplified fraud detection for administrative operations
        // In production, this would check for:
        // - Rapid successive administrative operations
        // - Unusual patterns in bulk operations
        // - Suspicious fraud flagging patterns
        // - Excessive audit trail access
        // - Unusual metrics recalculation requests

        // For now, just return false - implement actual fraud detection logic here
        false
    }
}
