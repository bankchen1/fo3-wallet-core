//! Referral service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    referral_service_server::ReferralService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    referral_guard::ReferralGuard,
};
use crate::models::referral::{
    ReferralCode, ReferralCampaign, ReferralRelationship, ReferralBonus, ReferralMetrics,
    ReferralCodeStatus, ReferralRelationshipStatus, ReferralCampaignType, ReferralCampaignStatus,
    ReferralBonusType, ReferralBonusStatus, ReferralRepository, ReferralAuditTrailEntry,
    TopReferrer, CampaignMetrics, ReferralTreeNode,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Referral service implementation
#[derive(Debug)]
pub struct ReferralServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    referral_guard: Arc<ReferralGuard>,
    referral_repository: Arc<dyn ReferralRepository>,
}

impl ReferralServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        referral_guard: Arc<ReferralGuard>,
        referral_repository: Arc<dyn ReferralRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            referral_guard,
            referral_repository,
        }
    }

    /// Convert proto referral code status to internal enum
    fn proto_to_referral_code_status(proto_status: i32) -> Result<ReferralCodeStatus, Status> {
        match proto_status {
            1 => Ok(ReferralCodeStatus::Active),
            2 => Ok(ReferralCodeStatus::Inactive),
            3 => Ok(ReferralCodeStatus::Expired),
            4 => Ok(ReferralCodeStatus::Suspended),
            5 => Ok(ReferralCodeStatus::Exhausted),
            _ => Err(Status::invalid_argument("Invalid referral code status")),
        }
    }

    /// Convert internal referral code status to proto
    fn referral_code_status_to_proto(status: &ReferralCodeStatus) -> i32 {
        match status {
            ReferralCodeStatus::Active => 1,
            ReferralCodeStatus::Inactive => 2,
            ReferralCodeStatus::Expired => 3,
            ReferralCodeStatus::Suspended => 4,
            ReferralCodeStatus::Exhausted => 5,
        }
    }

    /// Convert proto referral relationship status to internal enum
    fn proto_to_referral_relationship_status(proto_status: i32) -> Result<ReferralRelationshipStatus, Status> {
        match proto_status {
            1 => Ok(ReferralRelationshipStatus::Pending),
            2 => Ok(ReferralRelationshipStatus::Active),
            3 => Ok(ReferralRelationshipStatus::Completed),
            4 => Ok(ReferralRelationshipStatus::Cancelled),
            5 => Ok(ReferralRelationshipStatus::Fraudulent),
            _ => Err(Status::invalid_argument("Invalid referral relationship status")),
        }
    }

    /// Convert internal referral relationship status to proto
    fn referral_relationship_status_to_proto(status: &ReferralRelationshipStatus) -> i32 {
        match status {
            ReferralRelationshipStatus::Pending => 1,
            ReferralRelationshipStatus::Active => 2,
            ReferralRelationshipStatus::Completed => 3,
            ReferralRelationshipStatus::Cancelled => 4,
            ReferralRelationshipStatus::Fraudulent => 5,
        }
    }

    /// Convert proto referral campaign type to internal enum
    fn proto_to_referral_campaign_type(proto_type: i32) -> Result<ReferralCampaignType, Status> {
        match proto_type {
            1 => Ok(ReferralCampaignType::Signup),
            2 => Ok(ReferralCampaignType::FirstTransaction),
            3 => Ok(ReferralCampaignType::SpendingMilestone),
            4 => Ok(ReferralCampaignType::TierUpgrade),
            5 => Ok(ReferralCampaignType::MultiLevel),
            6 => Ok(ReferralCampaignType::TimeLimited),
            _ => Err(Status::invalid_argument("Invalid referral campaign type")),
        }
    }

    /// Convert internal referral campaign type to proto
    fn referral_campaign_type_to_proto(campaign_type: &ReferralCampaignType) -> i32 {
        match campaign_type {
            ReferralCampaignType::Signup => 1,
            ReferralCampaignType::FirstTransaction => 2,
            ReferralCampaignType::SpendingMilestone => 3,
            ReferralCampaignType::TierUpgrade => 4,
            ReferralCampaignType::MultiLevel => 5,
            ReferralCampaignType::TimeLimited => 6,
        }
    }

    /// Convert proto referral campaign status to internal enum
    fn proto_to_referral_campaign_status(proto_status: i32) -> Result<ReferralCampaignStatus, Status> {
        match proto_status {
            1 => Ok(ReferralCampaignStatus::Draft),
            2 => Ok(ReferralCampaignStatus::Active),
            3 => Ok(ReferralCampaignStatus::Paused),
            4 => Ok(ReferralCampaignStatus::Completed),
            5 => Ok(ReferralCampaignStatus::Cancelled),
            _ => Err(Status::invalid_argument("Invalid referral campaign status")),
        }
    }

    /// Convert internal referral campaign status to proto
    fn referral_campaign_status_to_proto(status: &ReferralCampaignStatus) -> i32 {
        match status {
            ReferralCampaignStatus::Draft => 1,
            ReferralCampaignStatus::Active => 2,
            ReferralCampaignStatus::Paused => 3,
            ReferralCampaignStatus::Completed => 4,
            ReferralCampaignStatus::Cancelled => 5,
        }
    }

    /// Convert proto referral bonus type to internal enum
    fn proto_to_referral_bonus_type(proto_type: i32) -> Result<ReferralBonusType, Status> {
        match proto_type {
            1 => Ok(ReferralBonusType::Referrer),
            2 => Ok(ReferralBonusType::Referee),
            3 => Ok(ReferralBonusType::Milestone),
            4 => Ok(ReferralBonusType::TierBonus),
            5 => Ok(ReferralBonusType::CampaignBonus),
            _ => Err(Status::invalid_argument("Invalid referral bonus type")),
        }
    }

    /// Convert internal referral bonus type to proto
    fn referral_bonus_type_to_proto(bonus_type: &ReferralBonusType) -> i32 {
        match bonus_type {
            ReferralBonusType::Referrer => 1,
            ReferralBonusType::Referee => 2,
            ReferralBonusType::Milestone => 3,
            ReferralBonusType::TierBonus => 4,
            ReferralBonusType::CampaignBonus => 5,
        }
    }

    /// Convert proto referral bonus status to internal enum
    fn proto_to_referral_bonus_status(proto_status: i32) -> Result<ReferralBonusStatus, Status> {
        match proto_status {
            1 => Ok(ReferralBonusStatus::Pending),
            2 => Ok(ReferralBonusStatus::Processing),
            3 => Ok(ReferralBonusStatus::Completed),
            4 => Ok(ReferralBonusStatus::Failed),
            5 => Ok(ReferralBonusStatus::Cancelled),
            6 => Ok(ReferralBonusStatus::Expired),
            _ => Err(Status::invalid_argument("Invalid referral bonus status")),
        }
    }

    /// Convert internal referral bonus status to proto
    fn referral_bonus_status_to_proto(status: &ReferralBonusStatus) -> i32 {
        match status {
            ReferralBonusStatus::Pending => 1,
            ReferralBonusStatus::Processing => 2,
            ReferralBonusStatus::Completed => 3,
            ReferralBonusStatus::Failed => 4,
            ReferralBonusStatus::Cancelled => 5,
            ReferralBonusStatus::Expired => 6,
        }
    }

    /// Convert internal referral code to proto
    fn referral_code_to_proto(&self, code: &ReferralCode) -> ReferralCode {
        ReferralCode {
            id: code.id.to_string(),
            user_id: code.user_id.to_string(),
            code: code.code.clone(),
            status: Self::referral_code_status_to_proto(&code.status),
            campaign_id: code.campaign_id.map(|id| id.to_string()).unwrap_or_default(),
            description: code.description.clone().unwrap_or_default(),
            is_custom: code.is_custom,
            max_uses: code.max_uses.unwrap_or(-1),
            current_uses: code.current_uses,
            successful_referrals: code.successful_referrals,
            pending_referrals: code.pending_referrals,
            expires_at: code.expires_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            last_used_at: code.last_used_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            metadata: code.metadata.clone(),
            created_at: code.created_at.to_rfc3339(),
            updated_at: code.updated_at.to_rfc3339(),
        }
    }

    /// Convert internal referral campaign to proto
    fn referral_campaign_to_proto(&self, campaign: &ReferralCampaign) -> ReferralCampaign {
        ReferralCampaign {
            id: campaign.id.to_string(),
            name: campaign.name.clone(),
            description: campaign.description.clone().unwrap_or_default(),
            r#type: Self::referral_campaign_type_to_proto(&campaign.campaign_type),
            status: Self::referral_campaign_status_to_proto(&campaign.status),
            referrer_bonus: campaign.referrer_bonus.to_string(),
            referee_bonus: campaign.referee_bonus.to_string(),
            bonus_currency: campaign.bonus_currency.clone(),
            minimum_transaction_amount: campaign.minimum_transaction_amount.to_string(),
            is_multi_level: campaign.is_multi_level,
            max_levels: campaign.max_levels,
            level_multipliers: campaign.level_multipliers.iter().map(|m| m.to_string()).collect(),
            start_date: campaign.start_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            end_date: campaign.end_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            bonus_expiry_days: campaign.bonus_expiry_days,
            max_referrals_per_user: campaign.max_referrals_per_user.unwrap_or(-1),
            max_total_referrals: campaign.max_total_referrals.unwrap_or(-1),
            max_bonus_per_user: campaign.max_bonus_per_user.map(|b| b.to_string()).unwrap_or_default(),
            total_budget: campaign.total_budget.map(|b| b.to_string()).unwrap_or_default(),
            budget_used: campaign.budget_used.to_string(),
            target_user_tiers: campaign.target_user_tiers.clone(),
            target_countries: campaign.target_countries.clone(),
            excluded_users: campaign.excluded_users.iter().map(|id| id.to_string()).collect(),
            metadata: campaign.metadata.clone(),
            created_at: campaign.created_at.to_rfc3339(),
            updated_at: campaign.updated_at.to_rfc3339(),
            created_by: campaign.created_by.map(|id| id.to_string()).unwrap_or_default(),
        }
    }

    /// Send notification for referral events
    async fn send_referral_notification(
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
            "referral_notification",
            &format!("Type: {:?}, Title: {}, Message: {}", notification_type, title, message),
            None,
        ).await.map_err(|e| Status::internal(format!("Failed to log notification: {}", e)))?;

        Ok(())
    }

    /// Generate unique reference number for referral operations
    fn generate_reference_number(&self) -> String {
        format!("REF-{}-{}", Utc::now().format("%Y%m%d"), Uuid::new_v4().to_string()[..8].to_uppercase())
    }

    /// Convert internal referral relationship to proto
    fn referral_relationship_to_proto(&self, relationship: &ReferralRelationship) -> ReferralRelationship {
        ReferralRelationship {
            id: relationship.id.to_string(),
            referrer_user_id: relationship.referrer_user_id.to_string(),
            referee_user_id: relationship.referee_user_id.to_string(),
            referral_code_id: relationship.referral_code_id.to_string(),
            campaign_id: relationship.campaign_id.map(|id| id.to_string()).unwrap_or_default(),
            status: Self::referral_relationship_status_to_proto(&relationship.status),
            referral_level: relationship.referral_level,
            parent_relationship_id: relationship.parent_relationship_id.map(|id| id.to_string()).unwrap_or_default(),
            signup_completed: relationship.signup_completed,
            first_transaction_completed: relationship.first_transaction_completed,
            kyc_completed: relationship.kyc_completed,
            first_transaction_date: relationship.first_transaction_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            kyc_completion_date: relationship.kyc_completion_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            total_bonuses_earned: relationship.total_bonuses_earned.to_string(),
            total_bonuses_paid: relationship.total_bonuses_paid.to_string(),
            bonuses_pending: relationship.bonuses_pending,
            is_suspicious: relationship.is_suspicious,
            fraud_flags: relationship.fraud_flags.clone(),
            fraud_check_date: relationship.fraud_check_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
            referral_source: relationship.referral_source.clone().unwrap_or_default(),
            ip_address: relationship.ip_address.clone().unwrap_or_default(),
            user_agent: relationship.user_agent.clone().unwrap_or_default(),
            metadata: relationship.metadata.clone(),
            created_at: relationship.created_at.to_rfc3339(),
            updated_at: relationship.updated_at.to_rfc3339(),
        }
    }

    /// Convert internal referral bonus to proto
    fn referral_bonus_to_proto(&self, bonus: &ReferralBonus) -> ReferralBonus {
        ReferralBonus {
            id: bonus.id.to_string(),
            referral_relationship_id: bonus.referral_relationship_id.to_string(),
            campaign_id: bonus.campaign_id.map(|id| id.to_string()).unwrap_or_default(),
            user_id: bonus.user_id.to_string(),
            r#type: Self::referral_bonus_type_to_proto(&bonus.bonus_type),
            status: Self::referral_bonus_status_to_proto(&bonus.status),
            bonus_amount: bonus.bonus_amount.to_string(),
            bonus_currency: bonus.bonus_currency.clone(),
            exchange_rate: bonus.exchange_rate.to_string(),
            milestone_type: bonus.milestone_type.clone().unwrap_or_default(),
            milestone_value: bonus.milestone_value.map(|v| v.to_string()).unwrap_or_default(),
            reward_transaction_id: bonus.reward_transaction_id.map(|id| id.to_string()).unwrap_or_default(),
            processing_fee: bonus.processing_fee.to_string(),
            net_amount: bonus.net_amount.to_string(),
            earned_at: bonus.earned_at.to_rfc3339(),
            processed_at: bonus.processed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            expires_at: bonus.expires_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            description: bonus.description.clone().unwrap_or_default(),
            metadata: bonus.metadata.clone(),
            created_at: bonus.created_at.to_rfc3339(),
            updated_at: bonus.updated_at.to_rfc3339(),
        }
    }

    /// Convert internal referral tree node to proto
    fn referral_tree_node_to_proto(&self, node: &ReferralTreeNode) -> ReferralTreeNode {
        ReferralTreeNode {
            user_id: node.user_id.to_string(),
            username: node.username.clone().unwrap_or_default(),
            level: node.level,
            direct_referrals: node.direct_referrals,
            total_referrals: node.total_referrals,
            total_bonuses_earned: node.total_bonuses_earned.to_string(),
            children: node.children.iter().map(|child| self.referral_tree_node_to_proto(child)).collect(),
            joined_at: node.joined_at.to_rfc3339(),
            is_active: node.is_active,
        }
    }

    /// Convert internal referral metrics to proto
    fn referral_metrics_to_proto(&self, metrics: &ReferralMetrics) -> ReferralMetrics {
        ReferralMetrics {
            total_referral_codes: metrics.total_referral_codes,
            active_referral_codes: metrics.active_referral_codes,
            total_referrals: metrics.total_referrals,
            successful_referrals: metrics.successful_referrals,
            pending_referrals: metrics.pending_referrals,
            total_bonuses_paid: metrics.total_bonuses_paid.to_string(),
            total_bonuses_pending: metrics.total_bonuses_pending.to_string(),
            period_start: metrics.period_start.to_rfc3339(),
            period_end: metrics.period_end.to_rfc3339(),
            period_referrals: metrics.period_referrals,
            period_signups: metrics.period_signups,
            period_bonuses_paid: metrics.period_bonuses_paid.to_string(),
            signup_conversion_rate: metrics.signup_conversion_rate.to_string(),
            transaction_conversion_rate: metrics.transaction_conversion_rate.to_string(),
            average_bonus_per_referral: metrics.average_bonus_per_referral.to_string(),
            roi: metrics.roi.to_string(),
            top_referrers: metrics.top_referrers.iter().map(|r| self.top_referrer_to_proto(r)).collect(),
            top_campaigns: metrics.top_campaigns.iter().map(|c| self.campaign_metrics_to_proto(c)).collect(),
            flagged_relationships: metrics.flagged_relationships,
            cancelled_relationships: metrics.cancelled_relationships,
            fraud_rate: metrics.fraud_rate.to_string(),
            generated_at: metrics.generated_at.to_rfc3339(),
            metadata: metrics.metadata.clone(),
        }
    }

    /// Convert internal top referrer to proto
    fn top_referrer_to_proto(&self, referrer: &TopReferrer) -> TopReferrer {
        TopReferrer {
            user_id: referrer.user_id.to_string(),
            username: referrer.username.clone().unwrap_or_default(),
            total_referrals: referrer.total_referrals,
            successful_referrals: referrer.successful_referrals,
            total_bonuses_earned: referrer.total_bonuses_earned.to_string(),
            conversion_rate: referrer.conversion_rate.to_string(),
        }
    }

    /// Convert internal campaign metrics to proto
    fn campaign_metrics_to_proto(&self, metrics: &CampaignMetrics) -> CampaignMetrics {
        CampaignMetrics {
            campaign_id: metrics.campaign_id.to_string(),
            campaign_name: metrics.campaign_name.clone(),
            total_referrals: metrics.total_referrals,
            successful_referrals: metrics.successful_referrals,
            total_bonuses_paid: metrics.total_bonuses_paid.to_string(),
            budget_utilization: metrics.budget_utilization.to_string(),
            roi: metrics.roi.to_string(),
        }
    }

    /// Convert internal audit trail entry to proto
    fn audit_trail_entry_to_proto(&self, entry: &ReferralAuditTrailEntry) -> ReferralAuditTrailEntry {
        ReferralAuditTrailEntry {
            id: entry.id.to_string(),
            user_id: entry.user_id.to_string(),
            relationship_id: entry.relationship_id.map(|id| id.to_string()).unwrap_or_default(),
            action_type: entry.action_type.clone(),
            entity_type: entry.entity_type.clone(),
            entity_id: entry.entity_id.to_string(),
            old_value: entry.old_value.clone().unwrap_or_default(),
            new_value: entry.new_value.clone().unwrap_or_default(),
            reason: entry.reason.clone().unwrap_or_default(),
            performed_by: entry.performed_by.to_string(),
            ip_address: entry.ip_address.clone().unwrap_or_default(),
            user_agent: entry.user_agent.clone().unwrap_or_default(),
            metadata: entry.metadata.clone(),
            created_at: entry.created_at.to_rfc3339(),
        }
    }
}
