//! Referral service method implementations

use super::referral::ReferralServiceImpl;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    referral_service_server::ReferralService,
    *,
};
use crate::models::referral::{
    ReferralCode, ReferralCampaign, ReferralRelationship, ReferralBonus,
    ReferralCodeStatus, ReferralRelationshipStatus, ReferralCampaignType, ReferralCampaignStatus,
    ReferralBonusType, ReferralBonusStatus,
};
use crate::models::notifications::NotificationType;

#[tonic::async_trait]
impl ReferralService for ReferralServiceImpl {
    /// Generate a new referral code
    async fn generate_referral_code(
        &self,
        request: Request<GenerateReferralCodeRequest>,
    ) -> Result<Response<GenerateReferralCodeResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate request
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        let campaign_id = if req.campaign_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.campaign_id)
                .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?)
        };

        let custom_code = if req.custom_code.is_empty() { None } else { Some(req.custom_code.as_str()) };

        // Validate request with security guard
        let auth_context = self.referral_guard
            .validate_referral_code_generation(&request, &user_id, custom_code, campaign_id.as_ref())
            .await?;

        // Generate referral code
        let code_string = ReferralCode::generate_code(&user_id, req.custom_code.clone().into());
        
        let referral_code = ReferralCode {
            id: Uuid::new_v4(),
            user_id,
            code: code_string,
            status: ReferralCodeStatus::Active,
            campaign_id,
            description: if req.description.is_empty() { None } else { Some(req.description.clone()) },
            is_custom: !req.custom_code.is_empty(),
            max_uses: if req.max_uses == -1 { None } else { Some(req.max_uses) },
            current_uses: 0,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: if req.expires_at.is_empty() {
                None
            } else {
                Some(DateTime::parse_from_rfc3339(&req.expires_at)
                    .map_err(|_| Status::invalid_argument("Invalid expiration date"))?
                    .with_timezone(&Utc))
            },
            last_used_at: None,
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save referral code
        let created_code = self.referral_repository
            .create_referral_code(&referral_code)
            .await
            .map_err(|e| Status::internal(format!("Failed to create referral code: {}", e)))?;

        // Send notification
        self.send_referral_notification(
            &user_id,
            NotificationType::ReferralCodeGenerated,
            "Referral Code Generated!",
            &format!("Your referral code {} is ready to share!", created_code.code),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "generate_referral_code",
            &format!("Generated code: {} for user {}", created_code.code, user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GenerateReferralCodeResponse {
            referral_code: Some(self.referral_code_to_proto(&created_code)),
            message: "Referral code generated successfully".to_string(),
        }))
    }

    /// Get a referral code by ID or code
    async fn get_referral_code(
        &self,
        request: Request<GetReferralCodeRequest>,
    ) -> Result<Response<GetReferralCodeResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Try to parse as UUID first, then as code string
        let referral_code = if let Ok(id) = Uuid::parse_str(&req.code_id) {
            self.referral_repository
                .get_referral_code(&id)
                .await
                .map_err(|e| Status::internal(format!("Failed to get referral code: {}", e)))?
        } else {
            self.referral_repository
                .get_referral_code_by_code(&req.code_id)
                .await
                .map_err(|e| Status::internal(format!("Failed to get referral code: {}", e)))?
        };

        match referral_code {
            Some(code) => {
                // Check if user can access this code
                if auth_context.user_id != code.user_id && 
                   !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
                    return Err(Status::permission_denied("Can only view your own referral codes"));
                }

                Ok(Response::new(GetReferralCodeResponse {
                    referral_code: Some(self.referral_code_to_proto(&code)),
                }))
            },
            None => Err(Status::not_found("Referral code not found")),
        }
    }

    /// Validate a referral code
    async fn validate_referral_code(
        &self,
        request: Request<ValidateReferralCodeRequest>,
    ) -> Result<Response<ValidateReferralCodeResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Get referral code
        let referral_code = self.referral_repository
            .get_referral_code_by_code(&req.code)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral code: {}", e)))?;

        match referral_code {
            Some(code) => {
                let mut validation_errors = Vec::new();
                let mut is_valid = true;

                // Check if code is valid
                if !code.is_valid() {
                    is_valid = false;
                    validation_errors.push(format!("Code status: {:?}", code.status));
                }

                // Check if user is trying to use their own code
                if code.user_id == user_id {
                    is_valid = false;
                    validation_errors.push("Cannot use your own referral code".to_string());
                }

                // Check campaign eligibility if specified
                if !req.campaign_id.is_empty() {
                    if let Ok(campaign_id) = Uuid::parse_str(&req.campaign_id) {
                        if code.campaign_id != Some(campaign_id) {
                            is_valid = false;
                            validation_errors.push("Code not valid for this campaign".to_string());
                        }
                    }
                }

                // Check for existing relationship
                if let Ok((existing_relationships, _)) = self.referral_repository.list_referral_relationships(
                    None,
                    Some(code.user_id),
                    Some(user_id),
                    None,
                    None,
                    None,
                    None,
                    0,
                    1,
                ).await {
                    if !existing_relationships.is_empty() {
                        is_valid = false;
                        validation_errors.push("Referral relationship already exists".to_string());
                    }
                }

                let validation_message = if is_valid {
                    "Referral code is valid".to_string()
                } else {
                    format!("Referral code is invalid: {}", validation_errors.join(", "))
                };

                Ok(Response::new(ValidateReferralCodeResponse {
                    is_valid,
                    validation_message,
                    referral_code: if is_valid { Some(self.referral_code_to_proto(&code)) } else { None },
                    validation_errors,
                }))
            },
            None => Ok(Response::new(ValidateReferralCodeResponse {
                is_valid: false,
                validation_message: "Referral code not found".to_string(),
                referral_code: None,
                validation_errors: vec!["Code does not exist".to_string()],
            })),
        }
    }

    /// Create a referral relationship
    async fn create_referral_relationship(
        &self,
        request: Request<CreateReferralRelationshipRequest>,
    ) -> Result<Response<CreateReferralRelationshipResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let referrer_user_id = Uuid::parse_str(&req.referrer_user_id)
            .map_err(|_| Status::invalid_argument("Invalid referrer user ID"))?;
        let referee_user_id = Uuid::parse_str(&req.referee_user_id)
            .map_err(|_| Status::invalid_argument("Invalid referee user ID"))?;

        let campaign_id = if req.campaign_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.campaign_id)
                .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?)
        };

        // Validate request with security guard
        let auth_context = self.referral_guard
            .validate_referral_relationship_creation(&request, &referrer_user_id, &referee_user_id, &req.referral_code)
            .await?;

        // Get referral code
        let referral_code = self.referral_repository
            .get_referral_code_by_code(&req.referral_code)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral code: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral code not found"))?;

        // Create referral relationship
        let relationship = ReferralRelationship {
            id: Uuid::new_v4(),
            referrer_user_id,
            referee_user_id,
            referral_code_id: referral_code.id,
            campaign_id,
            status: ReferralRelationshipStatus::Pending,
            referral_level: 1, // Direct referral
            parent_relationship_id: None,
            signup_completed: false,
            first_transaction_completed: false,
            kyc_completed: false,
            first_transaction_date: None,
            kyc_completion_date: None,
            total_bonuses_earned: Decimal::ZERO,
            total_bonuses_paid: Decimal::ZERO,
            bonuses_pending: 0,
            is_suspicious: false,
            fraud_flags: Vec::new(),
            fraud_check_date: None,
            referral_source: Some(req.referral_source.clone()),
            ip_address: request.remote_addr().map(|addr| addr.ip().to_string()),
            user_agent: None, // Would be extracted from headers
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save relationship
        let created_relationship = self.referral_repository
            .create_referral_relationship(&relationship)
            .await
            .map_err(|e| Status::internal(format!("Failed to create referral relationship: {}", e)))?;

        // Update referral code usage
        let mut updated_code = referral_code.clone();
        updated_code.current_uses += 1;
        updated_code.pending_referrals += 1;
        updated_code.last_used_at = Some(Utc::now());
        updated_code.updated_at = Utc::now();

        self.referral_repository
            .update_referral_code(&updated_code)
            .await
            .map_err(|e| Status::internal(format!("Failed to update referral code: {}", e)))?;

        // Send notifications
        self.send_referral_notification(
            &referrer_user_id,
            NotificationType::ReferralSuccess,
            "New Referral!",
            "Someone used your referral code and joined!",
            HashMap::new(),
        ).await?;

        self.send_referral_notification(
            &referee_user_id,
            NotificationType::ReferralWelcome,
            "Welcome!",
            "You've successfully joined through a referral!",
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "create_referral_relationship",
            &format!("Created relationship: {} -> {}", referrer_user_id, referee_user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(CreateReferralRelationshipResponse {
            relationship: Some(self.referral_relationship_to_proto(&created_relationship)),
            message: "Referral relationship created successfully".to_string(),
        }))
    }

    /// List user referral codes
    async fn list_user_referral_codes(
        &self,
        request: Request<ListUserReferralCodesRequest>,
    ) -> Result<Response<ListUserReferralCodesResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Check if user can access these codes
        if auth_context.user_id != user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
            return Err(Status::permission_denied("Can only view your own referral codes"));
        }

        // Parse filters
        let status = if req.status != 0 {
            Some(Self::proto_to_referral_code_status(req.status)?)
        } else {
            None
        };

        let campaign_id = if req.campaign_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.campaign_id)
                .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?)
        };

        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get codes
        let (codes, total_count) = self.referral_repository
            .list_user_referral_codes(&user_id, status, campaign_id, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list referral codes: {}", e)))?;

        let proto_codes = codes.iter()
            .map(|code| self.referral_code_to_proto(code))
            .collect();

        Ok(Response::new(ListUserReferralCodesResponse {
            referral_codes: proto_codes,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Deactivate a referral code
    async fn deactivate_referral_code(
        &self,
        request: Request<DeactivateReferralCodeRequest>,
    ) -> Result<Response<DeactivateReferralCodeResponse>, Status> {
        let req = request.get_ref();

        // Parse code ID
        let code_id = Uuid::parse_str(&req.code_id)
            .map_err(|_| Status::invalid_argument("Invalid code ID"))?;

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get existing code
        let mut referral_code = self.referral_repository
            .get_referral_code(&code_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral code: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral code not found"))?;

        // Check if user can deactivate this code
        if auth_context.user_id != referral_code.user_id &&
           !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
            return Err(Status::permission_denied("Can only deactivate your own referral codes"));
        }

        // Validate deactivation with security guard
        self.referral_guard
            .validate_referral_code_deactivation(&request, &code_id, &req.reason)
            .await?;

        // Deactivate code
        referral_code.status = ReferralCodeStatus::Inactive;
        referral_code.updated_at = Utc::now();

        let updated_code = self.referral_repository
            .update_referral_code(&referral_code)
            .await
            .map_err(|e| Status::internal(format!("Failed to deactivate referral code: {}", e)))?;

        // Send notification
        self.send_referral_notification(
            &referral_code.user_id,
            NotificationType::ReferralCodeDeactivated,
            "Referral Code Deactivated",
            &format!("Your referral code {} has been deactivated", referral_code.code),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "deactivate_referral_code",
            &format!("Deactivated code: {} - Reason: {}", referral_code.code, req.reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(DeactivateReferralCodeResponse {
            referral_code: Some(self.referral_code_to_proto(&updated_code)),
            message: "Referral code deactivated successfully".to_string(),
        }))
    }

    /// Get a referral relationship by ID
    async fn get_referral_relationship(
        &self,
        request: Request<GetReferralRelationshipRequest>,
    ) -> Result<Response<GetReferralRelationshipResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse relationship ID
        let relationship_id = Uuid::parse_str(&req.relationship_id)
            .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?;

        // Get relationship
        let relationship = self.referral_repository
            .get_referral_relationship(&relationship_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral relationship: {}", e)))?;

        match relationship {
            Some(relationship) => {
                // Check if user can access this relationship
                if auth_context.user_id != relationship.referrer_user_id &&
                   auth_context.user_id != relationship.referee_user_id &&
                   !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
                    return Err(Status::permission_denied("Can only view your own referral relationships"));
                }

                Ok(Response::new(GetReferralRelationshipResponse {
                    relationship: Some(self.referral_relationship_to_proto(&relationship)),
                }))
            },
            None => Err(Status::not_found("Referral relationship not found")),
        }
    }

    /// List referral relationships
    async fn list_referral_relationships(
        &self,
        request: Request<ListReferralRelationshipsRequest>,
    ) -> Result<Response<ListReferralRelationshipsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse filters
        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let referrer_user_id = if req.referrer_user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.referrer_user_id)
                .map_err(|_| Status::invalid_argument("Invalid referrer user ID"))?)
        };

        let referee_user_id = if req.referee_user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.referee_user_id)
                .map_err(|_| Status::invalid_argument("Invalid referee user ID"))?)
        };

        // Check permissions
        let has_admin_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await?;

        if !has_admin_permission {
            // Non-admin users can only view their own relationships
            if let Some(uid) = user_id {
                if auth_context.user_id != uid {
                    return Err(Status::permission_denied("Can only view your own referral relationships"));
                }
            } else if let Some(referrer_id) = referrer_user_id {
                if auth_context.user_id != referrer_id {
                    return Err(Status::permission_denied("Can only view your own referral relationships"));
                }
            } else if let Some(referee_id) = referee_user_id {
                if auth_context.user_id != referee_id {
                    return Err(Status::permission_denied("Can only view your own referral relationships"));
                }
            } else {
                return Err(Status::permission_denied("Must specify user_id or have admin permissions"));
            }
        }

        let status = if req.status != 0 {
            Some(Self::proto_to_referral_relationship_status(req.status)?)
        } else {
            None
        };

        let campaign_id = if req.campaign_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.campaign_id)
                .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?)
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get relationships
        let (relationships, total_count) = self.referral_repository
            .list_referral_relationships(user_id, referrer_user_id, referee_user_id, status, campaign_id, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list referral relationships: {}", e)))?;

        let proto_relationships = relationships.iter()
            .map(|relationship| self.referral_relationship_to_proto(relationship))
            .collect();

        Ok(Response::new(ListReferralRelationshipsResponse {
            relationships: proto_relationships,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Process a referral bonus
    async fn process_referral_bonus(
        &self,
        request: Request<ProcessReferralBonusRequest>,
    ) -> Result<Response<ProcessReferralBonusResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let relationship_id = Uuid::parse_str(&req.relationship_id)
            .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?;
        let bonus_type = Self::proto_to_referral_bonus_type(req.bonus_type)?;
        let bonus_amount = Decimal::from_str(&req.bonus_amount)
            .map_err(|_| Status::invalid_argument("Invalid bonus amount"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageReferrals)?;

        // Get referral relationship
        let mut relationship = self.referral_repository
            .get_referral_relationship(&relationship_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral relationship: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral relationship not found"))?;

        // Validate bonus processing with security guard
        self.referral_guard
            .validate_referral_bonus_processing(&request, &relationship_id, &bonus_type, &bonus_amount)
            .await?;

        // Determine recipient based on bonus type
        let recipient_user_id = match bonus_type {
            ReferralBonusType::Referrer | ReferralBonusType::Milestone | ReferralBonusType::TierBonus => {
                relationship.referrer_user_id
            },
            ReferralBonusType::Referee => {
                relationship.referee_user_id
            },
            _ => relationship.referrer_user_id, // Default to referrer
        };

        // Create referral bonus
        let bonus = ReferralBonus {
            id: Uuid::new_v4(),
            referral_relationship_id: relationship_id,
            campaign_id: relationship.campaign_id,
            user_id: recipient_user_id,
            bonus_type: bonus_type.clone(),
            status: ReferralBonusStatus::Processing,
            bonus_amount,
            bonus_currency: if req.bonus_currency.is_empty() { "points".to_string() } else { req.bonus_currency.clone() },
            exchange_rate: Decimal::from_str(&req.exchange_rate).unwrap_or(Decimal::ONE),
            milestone_type: if req.milestone_type.is_empty() { None } else { Some(req.milestone_type.clone()) },
            milestone_value: if req.milestone_value.is_empty() {
                None
            } else {
                Some(Decimal::from_str(&req.milestone_value).unwrap_or(Decimal::ZERO))
            },
            reward_transaction_id: None, // Will be set after reward processing
            processing_fee: bonus_amount * Decimal::from_str("0.02").unwrap(), // 2% processing fee
            net_amount: bonus_amount * Decimal::from_str("0.98").unwrap(),
            earned_at: Utc::now(),
            processed_at: None,
            expires_at: if req.expires_at.is_empty() {
                None
            } else {
                Some(DateTime::parse_from_rfc3339(&req.expires_at)
                    .map_err(|_| Status::invalid_argument("Invalid expiration date"))?
                    .with_timezone(&Utc))
            },
            description: if req.description.is_empty() { None } else { Some(req.description.clone()) },
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save bonus
        let mut created_bonus = self.referral_repository
            .create_referral_bonus(&bonus)
            .await
            .map_err(|e| Status::internal(format!("Failed to create referral bonus: {}", e)))?;

        // Process bonus through rewards service if currency is points
        if created_bonus.bonus_currency == "points" {
            match self.rewards_service.award_points_internal(
                &recipient_user_id,
                &created_bonus.net_amount,
                "referral_bonus",
                &created_bonus.id.to_string(),
                &format!("Referral bonus: {:?}", bonus_type),
                &created_bonus.metadata,
            ).await {
                Ok(_) => {
                    created_bonus.status = ReferralBonusStatus::Completed;
                    created_bonus.processed_at = Some(Utc::now());
                },
                Err(e) => {
                    created_bonus.status = ReferralBonusStatus::Failed;
                    created_bonus.description = Some(format!("Processing failed: {}", e));
                }
            }
        } else {
            // For non-points currencies, mark as completed (would integrate with payment processor)
            created_bonus.status = ReferralBonusStatus::Completed;
            created_bonus.processed_at = Some(Utc::now());
        }

        created_bonus.updated_at = Utc::now();

        // Update bonus status
        let final_bonus = self.referral_repository
            .update_referral_bonus(&created_bonus)
            .await
            .map_err(|e| Status::internal(format!("Failed to update referral bonus: {}", e)))?;

        // Update relationship bonus tracking
        relationship.total_bonuses_earned += bonus_amount;
        if final_bonus.status == ReferralBonusStatus::Completed {
            relationship.total_bonuses_paid += final_bonus.net_amount;
        } else {
            relationship.bonuses_pending += 1;
        }
        relationship.updated_at = Utc::now();

        self.referral_repository
            .update_referral_relationship(&relationship)
            .await
            .map_err(|e| Status::internal(format!("Failed to update referral relationship: {}", e)))?;

        // Send notification
        self.send_referral_notification(
            &recipient_user_id,
            NotificationType::ReferralBonusEarned,
            "Referral Bonus Earned!",
            &format!("You earned a {} bonus of {} {}!",
                format!("{:?}", bonus_type).to_lowercase(),
                final_bonus.net_amount,
                final_bonus.bonus_currency),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "process_referral_bonus",
            &format!("Processed {} bonus of {} {} for user {}",
                format!("{:?}", bonus_type), bonus_amount, final_bonus.bonus_currency, recipient_user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(ProcessReferralBonusResponse {
            bonus: Some(self.referral_bonus_to_proto(&final_bonus)),
            message: format!("Referral bonus processed successfully: {} {}",
                final_bonus.net_amount, final_bonus.bonus_currency),
        }))
    }

    /// Get referral bonuses for a specific relationship
    async fn get_referral_bonuses(
        &self,
        request: Request<GetReferralBonusesRequest>,
    ) -> Result<Response<GetReferralBonusesResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse relationship ID
        let relationship_id = Uuid::parse_str(&req.relationship_id)
            .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?;

        // Get relationship to check permissions
        let relationship = self.referral_repository
            .get_referral_relationship(&relationship_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral relationship: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral relationship not found"))?;

        // Check if user can access these bonuses
        if auth_context.user_id != relationship.referrer_user_id &&
           auth_context.user_id != relationship.referee_user_id &&
           !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await? {
            return Err(Status::permission_denied("Can only view bonuses for your own referral relationships"));
        }

        // Get bonuses for this relationship
        let bonuses = self.referral_repository
            .get_referral_bonuses_by_relationship(&relationship_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral bonuses: {}", e)))?;

        let proto_bonuses = bonuses.iter()
            .map(|bonus| self.referral_bonus_to_proto(bonus))
            .collect();

        Ok(Response::new(GetReferralBonusesResponse {
            bonuses: proto_bonuses,
        }))
    }

    /// List referral bonuses with filtering
    async fn list_referral_bonuses(
        &self,
        request: Request<ListReferralBonusesRequest>,
    ) -> Result<Response<ListReferralBonusesResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse filters
        let relationship_id = if req.relationship_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.relationship_id)
                .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?)
        };

        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let bonus_type = if req.bonus_type != 0 {
            Some(Self::proto_to_referral_bonus_type(req.bonus_type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_referral_bonus_status(req.status)?)
        } else {
            None
        };

        // Check permissions
        let has_admin_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await?;

        if !has_admin_permission {
            // Non-admin users can only view their own bonuses
            if let Some(uid) = user_id {
                if auth_context.user_id != uid {
                    return Err(Status::permission_denied("Can only view your own referral bonuses"));
                }
            } else {
                return Err(Status::permission_denied("Must specify user_id or have admin permissions"));
            }
        }

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get bonuses
        let (bonuses, total_count) = self.referral_repository
            .list_referral_bonuses(relationship_id, user_id, bonus_type, status, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list referral bonuses: {}", e)))?;

        let proto_bonuses = bonuses.iter()
            .map(|bonus| self.referral_bonus_to_proto(bonus))
            .collect();

        Ok(Response::new(ListReferralBonusesResponse {
            bonuses: proto_bonuses,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Create a referral campaign
    async fn create_referral_campaign(
        &self,
        request: Request<CreateReferralCampaignRequest>,
    ) -> Result<Response<CreateReferralCampaignResponse>, Status> {
        let req = request.get_ref();
        let campaign_proto = req.campaign.as_ref()
            .ok_or_else(|| Status::invalid_argument("Referral campaign required"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageReferrals)?;

        // Parse and validate campaign data
        let campaign_type = Self::proto_to_referral_campaign_type(campaign_proto.r#type)?;
        let referrer_bonus = Decimal::from_str(&campaign_proto.referrer_bonus)
            .map_err(|_| Status::invalid_argument("Invalid referrer bonus"))?;
        let referee_bonus = Decimal::from_str(&campaign_proto.referee_bonus)
            .map_err(|_| Status::invalid_argument("Invalid referee bonus"))?;
        let minimum_transaction_amount = Decimal::from_str(&campaign_proto.minimum_transaction_amount)
            .map_err(|_| Status::invalid_argument("Invalid minimum transaction amount"))?;

        // Validate campaign with security guard
        self.referral_guard
            .validate_referral_campaign_creation(&request, &campaign_proto.name, &referrer_bonus, &referee_bonus)
            .await?;

        // Parse level multipliers
        let level_multipliers: Result<Vec<Decimal>, _> = campaign_proto.level_multipliers.iter()
            .map(|m| Decimal::from_str(m))
            .collect();
        let level_multipliers = level_multipliers
            .map_err(|_| Status::invalid_argument("Invalid level multipliers"))?;

        // Parse optional fields
        let start_date = if campaign_proto.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&campaign_proto.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if campaign_proto.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&campaign_proto.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        let max_bonus_per_user = if campaign_proto.max_bonus_per_user.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&campaign_proto.max_bonus_per_user)
                .map_err(|_| Status::invalid_argument("Invalid max bonus per user"))?)
        };

        let total_budget = if campaign_proto.total_budget.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&campaign_proto.total_budget)
                .map_err(|_| Status::invalid_argument("Invalid total budget"))?)
        };

        // Parse excluded users
        let excluded_users: Result<Vec<Uuid>, _> = campaign_proto.excluded_users.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let excluded_users = excluded_users
            .map_err(|_| Status::invalid_argument("Invalid excluded user IDs"))?;

        // Create campaign
        let campaign = ReferralCampaign {
            id: Uuid::new_v4(),
            name: campaign_proto.name.clone(),
            description: if campaign_proto.description.is_empty() { None } else { Some(campaign_proto.description.clone()) },
            campaign_type,
            status: ReferralCampaignStatus::Draft, // Start as draft
            referrer_bonus,
            referee_bonus,
            bonus_currency: if campaign_proto.bonus_currency.is_empty() { "points".to_string() } else { campaign_proto.bonus_currency.clone() },
            minimum_transaction_amount,
            is_multi_level: campaign_proto.is_multi_level,
            max_levels: campaign_proto.max_levels,
            level_multipliers,
            start_date,
            end_date,
            bonus_expiry_days: campaign_proto.bonus_expiry_days,
            max_referrals_per_user: if campaign_proto.max_referrals_per_user == -1 { None } else { Some(campaign_proto.max_referrals_per_user) },
            max_total_referrals: if campaign_proto.max_total_referrals == -1 { None } else { Some(campaign_proto.max_total_referrals) },
            max_bonus_per_user,
            total_budget,
            budget_used: Decimal::ZERO,
            target_user_tiers: campaign_proto.target_user_tiers.clone(),
            target_countries: campaign_proto.target_countries.clone(),
            excluded_users,
            metadata: campaign_proto.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: Some(auth_context.user_id),
        };

        // Save campaign
        let created_campaign = self.referral_repository
            .create_referral_campaign(&campaign)
            .await
            .map_err(|e| Status::internal(format!("Failed to create referral campaign: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "create_referral_campaign",
            &format!("Created campaign: {} ({})", created_campaign.name, created_campaign.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(CreateReferralCampaignResponse {
            campaign: Some(self.referral_campaign_to_proto(&created_campaign)),
            message: "Referral campaign created successfully".to_string(),
        }))
    }

    /// Get a referral campaign by ID
    async fn get_referral_campaign(
        &self,
        request: Request<GetReferralCampaignRequest>,
    ) -> Result<Response<GetReferralCampaignResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate campaign ID
        let campaign_id = Uuid::parse_str(&req.campaign_id)
            .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?;

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get campaign from repository
        let campaign = self.referral_repository
            .get_referral_campaign(&campaign_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral campaign: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral campaign not found"))?;

        // Check permissions: campaign creators can view their campaigns, admins can view all
        let has_manage_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false);
        let is_campaign_creator = campaign.created_by == Some(auth_context.user_id);

        if !has_manage_permission && !is_campaign_creator {
            return Err(Status::permission_denied("Insufficient permissions to view this campaign"));
        }

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_referral_campaign",
            &format!("Retrieved campaign: {} ({})", campaign.name, campaign.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetReferralCampaignResponse {
            campaign: Some(self.referral_campaign_to_proto(&campaign)),
        }))
    }

    /// List referral campaigns with filtering and pagination
    async fn list_referral_campaigns(
        &self,
        request: Request<ListReferralCampaignsRequest>,
    ) -> Result<Response<ListReferralCampaignsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse filtering parameters
        let campaign_type = if req.r#type == 0 {
            None
        } else {
            Some(Self::proto_to_referral_campaign_type(req.r#type)?)
        };

        let status = if req.status == 0 {
            None
        } else {
            Some(Self::proto_to_referral_campaign_status(req.status)?)
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        // Validate pagination parameters
        let page = req.page.max(0);
        let page_size = req.page_size.clamp(1, 100).max(20); // Default 20, max 100

        // Get campaigns from repository
        let (mut campaigns, total_count) = self.referral_repository
            .list_referral_campaigns(campaign_type, status, req.active_only, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list referral campaigns: {}", e)))?;

        // Apply permission-based filtering: non-admin users see only their created campaigns
        let has_manage_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false);
        if !has_manage_permission {
            campaigns.retain(|campaign| campaign.created_by == Some(auth_context.user_id));
        }

        // Convert to proto
        let campaign_protos: Vec<_> = campaigns.iter()
            .map(|campaign| self.referral_campaign_to_proto(campaign))
            .collect();

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "list_referral_campaigns",
            &format!("Listed {} campaigns (page: {}, size: {})", campaign_protos.len(), page, page_size),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(ListReferralCampaignsResponse {
            campaigns: campaign_protos,
            total_count,
            page,
            page_size,
        }))
    }

    /// Update a referral campaign
    async fn update_referral_campaign(
        &self,
        request: Request<UpdateReferralCampaignRequest>,
    ) -> Result<Response<UpdateReferralCampaignResponse>, Status> {
        let req = request.get_ref();
        let campaign_proto = req.campaign.as_ref()
            .ok_or_else(|| Status::invalid_argument("Referral campaign required"))?;

        // Parse and validate campaign ID
        let campaign_id = Uuid::parse_str(&campaign_proto.id)
            .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get existing campaign
        let existing_campaign = self.referral_repository
            .get_referral_campaign(&campaign_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral campaign: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral campaign not found"))?;

        // Check permissions: only campaign creators or admins can update
        let has_manage_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false);
        let is_campaign_creator = existing_campaign.created_by == Some(auth_context.user_id);

        if !has_manage_permission && !is_campaign_creator {
            return Err(Status::permission_denied("Insufficient permissions to update this campaign"));
        }

        // Validate campaign update with security guard
        self.referral_guard
            .validate_referral_campaign_update(&request, &campaign_proto.name, &existing_campaign.status)
            .await?;

        // Parse and validate updated fields
        let campaign_type = Self::proto_to_referral_campaign_type(campaign_proto.r#type)?;
        let referrer_bonus = Decimal::from_str(&campaign_proto.referrer_bonus)
            .map_err(|_| Status::invalid_argument("Invalid referrer bonus"))?;
        let referee_bonus = Decimal::from_str(&campaign_proto.referee_bonus)
            .map_err(|_| Status::invalid_argument("Invalid referee bonus"))?;
        let minimum_transaction_amount = Decimal::from_str(&campaign_proto.minimum_transaction_amount)
            .map_err(|_| Status::invalid_argument("Invalid minimum transaction amount"))?;

        // Parse level multipliers
        let level_multipliers: Result<Vec<Decimal>, _> = campaign_proto.level_multipliers.iter()
            .map(|m| Decimal::from_str(m))
            .collect();
        let level_multipliers = level_multipliers
            .map_err(|_| Status::invalid_argument("Invalid level multipliers"))?;

        // Parse optional fields
        let start_date = if campaign_proto.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&campaign_proto.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if campaign_proto.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&campaign_proto.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        let max_bonus_per_user = if campaign_proto.max_bonus_per_user.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&campaign_proto.max_bonus_per_user)
                .map_err(|_| Status::invalid_argument("Invalid max bonus per user"))?)
        };

        let total_budget = if campaign_proto.total_budget.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&campaign_proto.total_budget)
                .map_err(|_| Status::invalid_argument("Invalid total budget"))?)
        };

        // Parse excluded users
        let excluded_users: Result<Vec<Uuid>, _> = campaign_proto.excluded_users.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let excluded_users = excluded_users
            .map_err(|_| Status::invalid_argument("Invalid excluded user IDs"))?;

        // Business logic validation: prevent updates to active campaigns that would break existing referrals
        if existing_campaign.status == ReferralCampaignStatus::Active {
            // Only allow certain fields to be updated for active campaigns
            if existing_campaign.campaign_type != campaign_type {
                return Err(Status::failed_precondition("Cannot change campaign type for active campaigns"));
            }
            if existing_campaign.referrer_bonus != referrer_bonus || existing_campaign.referee_bonus != referee_bonus {
                return Err(Status::failed_precondition("Cannot change bonus amounts for active campaigns"));
            }
        }

        // Create updated campaign
        let updated_campaign = ReferralCampaign {
            id: campaign_id,
            name: campaign_proto.name.clone(),
            description: if campaign_proto.description.is_empty() { None } else { Some(campaign_proto.description.clone()) },
            campaign_type,
            status: Self::proto_to_referral_campaign_status(campaign_proto.status)?,
            referrer_bonus,
            referee_bonus,
            bonus_currency: if campaign_proto.bonus_currency.is_empty() { "points".to_string() } else { campaign_proto.bonus_currency.clone() },
            minimum_transaction_amount,
            is_multi_level: campaign_proto.is_multi_level,
            max_levels: campaign_proto.max_levels,
            level_multipliers,
            start_date,
            end_date,
            bonus_expiry_days: campaign_proto.bonus_expiry_days,
            max_referrals_per_user: if campaign_proto.max_referrals_per_user == -1 { None } else { Some(campaign_proto.max_referrals_per_user) },
            max_total_referrals: if campaign_proto.max_total_referrals == -1 { None } else { Some(campaign_proto.max_total_referrals) },
            max_bonus_per_user,
            total_budget,
            budget_used: Decimal::from_str(&campaign_proto.budget_used)
                .map_err(|_| Status::invalid_argument("Invalid budget used"))?,
            target_user_tiers: campaign_proto.target_user_tiers.clone(),
            target_countries: campaign_proto.target_countries.clone(),
            excluded_users,
            metadata: campaign_proto.metadata.clone(),
            created_at: existing_campaign.created_at, // Preserve original creation time
            updated_at: Utc::now(),
            created_by: existing_campaign.created_by, // Preserve original creator
        };

        // Save updated campaign
        let saved_campaign = self.referral_repository
            .update_referral_campaign(&updated_campaign)
            .await
            .map_err(|e| Status::internal(format!("Failed to update referral campaign: {}", e)))?;

        // Log the action with before/after values
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "update_referral_campaign",
            &format!("Updated campaign: {} ({}). Status: {:?} -> {:?}",
                    saved_campaign.name, saved_campaign.id,
                    existing_campaign.status, saved_campaign.status),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(UpdateReferralCampaignResponse {
            campaign: Some(self.referral_campaign_to_proto(&saved_campaign)),
            message: "Referral campaign updated successfully".to_string(),
        }))
    }

    /// Delete (soft delete) a referral campaign
    async fn delete_referral_campaign(
        &self,
        request: Request<DeleteReferralCampaignRequest>,
    ) -> Result<Response<DeleteReferralCampaignResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate campaign ID
        let campaign_id = Uuid::parse_str(&req.campaign_id)
            .map_err(|_| Status::invalid_argument("Invalid campaign ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get existing campaign
        let existing_campaign = self.referral_repository
            .get_referral_campaign(&campaign_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral campaign: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral campaign not found"))?;

        // Validate campaign deletion with security guard (requires admin permissions)
        self.referral_guard
            .validate_referral_campaign_deletion(&request, &existing_campaign.status, &req.reason)
            .await?;

        // Validate that campaign can be safely deleted (no active referrals or pending bonuses)
        // For now, we'll implement basic validation - in production this would check for active referrals
        if existing_campaign.status == ReferralCampaignStatus::Active {
            // Check if there are active referrals using this campaign
            // This is a simplified check - in production you'd query the relationships table
            return Err(Status::failed_precondition(
                "Cannot delete active campaign with ongoing referrals. Please pause the campaign first."
            ));
        }

        // Implement soft delete by setting status to CANCELLED
        let cancelled_campaign = ReferralCampaign {
            id: existing_campaign.id,
            name: existing_campaign.name.clone(),
            description: existing_campaign.description.clone(),
            campaign_type: existing_campaign.campaign_type.clone(),
            status: ReferralCampaignStatus::Cancelled, // Soft delete
            referrer_bonus: existing_campaign.referrer_bonus,
            referee_bonus: existing_campaign.referee_bonus,
            bonus_currency: existing_campaign.bonus_currency.clone(),
            minimum_transaction_amount: existing_campaign.minimum_transaction_amount,
            is_multi_level: existing_campaign.is_multi_level,
            max_levels: existing_campaign.max_levels,
            level_multipliers: existing_campaign.level_multipliers.clone(),
            start_date: existing_campaign.start_date,
            end_date: existing_campaign.end_date,
            bonus_expiry_days: existing_campaign.bonus_expiry_days,
            max_referrals_per_user: existing_campaign.max_referrals_per_user,
            max_total_referrals: existing_campaign.max_total_referrals,
            max_bonus_per_user: existing_campaign.max_bonus_per_user,
            total_budget: existing_campaign.total_budget,
            budget_used: existing_campaign.budget_used,
            target_user_tiers: existing_campaign.target_user_tiers.clone(),
            target_countries: existing_campaign.target_countries.clone(),
            excluded_users: existing_campaign.excluded_users.clone(),
            metadata: existing_campaign.metadata.clone(),
            created_at: existing_campaign.created_at,
            updated_at: Utc::now(),
            created_by: existing_campaign.created_by,
        };

        // Save the cancelled campaign (soft delete)
        self.referral_repository
            .update_referral_campaign(&cancelled_campaign)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete referral campaign: {}", e)))?;

        // Log the action with reason for audit trail
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "delete_referral_campaign",
            &format!("Deleted campaign: {} ({}). Reason: {}",
                    cancelled_campaign.name, cancelled_campaign.id, req.reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(DeleteReferralCampaignResponse {
            message: "Referral campaign deleted successfully".to_string(),
        }))
    }

    /// Get referral tree for multi-level visualization
    async fn get_referral_tree(
        &self,
        request: Request<GetReferralTreeRequest>,
    ) -> Result<Response<GetReferralTreeResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check permissions: users can view their own tree, admins can view any tree
        let has_view_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false);
        let is_own_tree = auth_context.user_id == user_id;

        if !has_view_permission && !is_own_tree {
            return Err(Status::permission_denied("Insufficient permissions to view this referral tree"));
        }

        // Validate analytics access with security guard
        self.referral_guard
            .validate_analytics_access(&request, Some(&user_id))
            .await?;

        // Validate parameters
        let max_depth = req.max_depth.clamp(1, 10); // Limit depth to prevent performance issues
        let include_inactive = req.include_inactive;

        // Get referral tree from repository
        let tree = self.referral_repository
            .get_referral_tree(&user_id, max_depth, include_inactive)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral tree: {}", e)))?;

        // Count total nodes in tree
        fn count_nodes(node: &ReferralTreeNode) -> i64 {
            1 + node.children.iter().map(count_nodes).sum::<i64>()
        }
        let total_nodes = count_nodes(&tree);

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_referral_tree",
            &format!("Retrieved tree for user: {} (depth: {}, nodes: {})", user_id, max_depth, total_nodes),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetReferralTreeResponse {
            root: Some(self.referral_tree_node_to_proto(&tree)),
            total_nodes,
            max_depth,
        }))
    }

    /// Get referral statistics for a user
    async fn get_referral_stats(
        &self,
        request: Request<GetReferralStatsRequest>,
    ) -> Result<Response<GetReferralStatsResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check permissions: users can view their own stats, admins can view any stats
        let has_view_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false);
        let is_own_stats = auth_context.user_id == user_id;

        if !has_view_permission && !is_own_stats {
            return Err(Status::permission_denied("Insufficient permissions to view these referral statistics"));
        }

        // Validate analytics access with security guard
        self.referral_guard
            .validate_analytics_access(&request, Some(&user_id))
            .await?;

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            Utc::now() - chrono::Duration::days(30) // Default to last 30 days
        } else {
            DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc)
        };

        let end_date = if req.end_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Get referral statistics from repository
        let (total_referrals, successful_referrals, pending_referrals, total_bonuses_earned, total_bonuses_pending, conversion_rate, campaign_stats) =
            self.referral_repository
                .get_referral_stats(&user_id, start_date, end_date)
                .await
                .map_err(|e| Status::internal(format!("Failed to get referral stats: {}", e)))?;

        // Convert campaign stats to proto
        let campaign_protos: Vec<_> = campaign_stats.iter()
            .map(|stats| self.campaign_metrics_to_proto(stats))
            .collect();

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_referral_stats",
            &format!("Retrieved stats for user: {} (period: {} to {})", user_id, start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetReferralStatsResponse {
            total_referrals,
            successful_referrals,
            pending_referrals,
            total_bonuses_earned: total_bonuses_earned.to_string(),
            total_bonuses_pending: total_bonuses_pending.to_string(),
            conversion_rate: conversion_rate.to_string(),
            campaign_stats: campaign_protos,
        }))
    }

    /// Get referral metrics for business analytics and insights
    async fn get_referral_metrics(
        &self,
        request: Request<GetReferralMetricsRequest>,
    ) -> Result<Response<GetReferralMetricsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate admin permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check admin permissions for business analytics
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required to view referral metrics"));
        }

        // Validate analytics access with security guard
        self.referral_guard
            .validate_analytics_access(&request, None)
            .await?;

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            Utc::now() - chrono::Duration::days(30) // Default to last 30 days
        } else {
            DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc)
        };

        let end_date = if req.end_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Parse campaign IDs filter
        let campaign_ids: Result<Vec<Uuid>, _> = req.campaign_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let campaign_ids = campaign_ids
            .map_err(|_| Status::invalid_argument("Invalid campaign IDs"))?;

        // Parse user IDs filter
        let user_ids: Result<Vec<Uuid>, _> = req.user_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let user_ids = user_ids
            .map_err(|_| Status::invalid_argument("Invalid user IDs"))?;

        // Get referral metrics from repository
        let metrics = self.referral_repository
            .get_referral_metrics(start_date, end_date, campaign_ids, user_ids, req.include_fraud_metrics)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral metrics: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_referral_metrics",
            &format!("Retrieved business metrics (period: {} to {}, campaigns: {}, users: {})",
                    start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"),
                    req.campaign_ids.len(), req.user_ids.len()),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetReferralMetricsResponse {
            metrics: Some(self.referral_metrics_to_proto(&metrics)),
        }))
    }

    /// Claim referral bonus for a user
    async fn claim_referral_bonus(
        &self,
        request: Request<ClaimReferralBonusRequest>,
    ) -> Result<Response<ClaimReferralBonusResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate bonus ID
        let bonus_id = Uuid::parse_str(&req.bonus_id)
            .map_err(|_| Status::invalid_argument("Invalid bonus ID"))?;

        // Parse and validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check permissions: users can only claim their own bonuses, admins can claim any
        let has_manage_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false);
        let is_own_bonus = auth_context.user_id == user_id;

        if !has_manage_permission && !is_own_bonus {
            return Err(Status::permission_denied("Insufficient permissions to claim this bonus"));
        }

        // Get bonus from repository
        let bonus = self.referral_repository
            .get_referral_bonus(&bonus_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get referral bonus: {}", e)))?
            .ok_or_else(|| Status::not_found("Referral bonus not found"))?;

        // Validate bonus ownership
        if bonus.user_id != user_id {
            return Err(Status::invalid_argument("Bonus does not belong to the specified user"));
        }

        // Validate bonus eligibility
        if bonus.status != ReferralBonusStatus::Pending {
            return Err(Status::failed_precondition("Bonus is not in pending status"));
        }

        // Check if bonus has expired
        if let Some(expires_at) = bonus.expires_at {
            if expires_at < Utc::now() {
                return Err(Status::failed_precondition("Bonus has expired"));
            }
        }

        // Validate bonus claiming with security guard
        self.referral_guard
            .validate_referral_bonus_processing(&request, &bonus.referral_relationship_id, &bonus.bonus_type, &bonus.bonus_amount)
            .await?;

        // Create reward transaction (integration with RewardsService)
        // For now, we'll simulate this - in production this would call the actual RewardsService
        let reward_transaction_id = Uuid::new_v4().to_string();

        // Update bonus status to completed
        let mut claimed_bonus = bonus.clone();
        claimed_bonus.status = ReferralBonusStatus::Completed;
        claimed_bonus.processed_at = Some(Utc::now());
        claimed_bonus.reward_transaction_id = Some(reward_transaction_id.clone());
        claimed_bonus.updated_at = Utc::now();

        // Save updated bonus
        let updated_bonus = self.referral_repository
            .update_referral_bonus(&claimed_bonus)
            .await
            .map_err(|e| Status::internal(format!("Failed to update referral bonus: {}", e)))?;

        // Send notification to user about successful claim
        self.send_referral_notification(
            &user_id,
            NotificationType::Reward,
            "Bonus Claimed!",
            &format!("You've successfully claimed a referral bonus of {} {}",
                    updated_bonus.bonus_amount, updated_bonus.bonus_currency),
            HashMap::from([
                ("bonus_id".to_string(), bonus_id.to_string()),
                ("amount".to_string(), updated_bonus.bonus_amount.to_string()),
                ("currency".to_string(), updated_bonus.bonus_currency.clone()),
            ]),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "claim_referral_bonus",
            &format!("Claimed bonus: {} ({} {}) for user: {}",
                    bonus_id, updated_bonus.bonus_amount, updated_bonus.bonus_currency, user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(ClaimReferralBonusResponse {
            bonus: Some(self.referral_bonus_to_proto(&updated_bonus)),
            reward_transaction_id,
            message: "Referral bonus claimed successfully".to_string(),
        }))
    }

    /// Get detailed user-specific referral analytics
    async fn get_user_referral_analytics(
        &self,
        request: Request<GetUserReferralAnalyticsRequest>,
    ) -> Result<Response<GetUserReferralAnalyticsResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check permissions: users can only view their own analytics
        let has_view_permission = self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false);
        let is_own_analytics = auth_context.user_id == user_id;

        if !has_view_permission && !is_own_analytics {
            return Err(Status::permission_denied("Insufficient permissions to view these analytics"));
        }

        // Validate analytics access with security guard
        self.referral_guard
            .validate_administrative_access(&request, "user_analytics")
            .await?;

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            Utc::now() - chrono::Duration::days(90) // Default to last 90 days for analytics
        } else {
            DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc)
        };

        let end_date = if req.end_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Get detailed analytics from repository
        let (total_referrals, successful_referrals, pending_referrals, total_bonuses_earned, total_bonuses_pending, conversion_rate, campaign_stats, recent_bonuses) =
            self.referral_repository
                .get_user_referral_analytics(&user_id, start_date, end_date)
                .await
                .map_err(|e| Status::internal(format!("Failed to get user referral analytics: {}", e)))?;

        // Get referral tree if requested
        let referral_tree = if req.include_tree {
            Some(self.referral_repository
                .get_referral_tree(&user_id, 5, false) // Max 5 levels for analytics
                .await
                .map_err(|e| Status::internal(format!("Failed to get referral tree: {}", e)))?)
        } else {
            None
        };

        // Calculate projected monthly earnings if requested
        let projected_monthly_earnings = if req.include_projections {
            // Simple projection based on recent performance
            let days_in_period = (end_date - start_date).num_days() as f64;
            let monthly_projection = if days_in_period > 0.0 {
                (total_bonuses_earned * rust_decimal::Decimal::from_f64(30.0 / days_in_period).unwrap_or_default())
                    .max(rust_decimal::Decimal::ZERO)
            } else {
                rust_decimal::Decimal::ZERO
            };
            monthly_projection.to_string()
        } else {
            "0".to_string()
        };

        // Generate AI-like insights
        let mut insights = HashMap::new();
        insights.insert("performance_trend".to_string(),
            if conversion_rate > rust_decimal::Decimal::from_f64(0.1).unwrap() {
                "Above average conversion rate".to_string()
            } else {
                "Room for improvement in conversion".to_string()
            });
        insights.insert("activity_level".to_string(),
            if total_referrals > 10 {
                "High activity user".to_string()
            } else {
                "Moderate activity user".to_string()
            });

        // Convert campaign stats to proto
        let campaign_protos: Vec<_> = campaign_stats.iter()
            .map(|stats| self.campaign_metrics_to_proto(stats))
            .collect();

        // Convert recent bonuses to proto
        let bonus_protos: Vec<_> = recent_bonuses.iter()
            .map(|bonus| self.referral_bonus_to_proto(bonus))
            .collect();

        // Create stats response
        let stats_response = GetReferralStatsResponse {
            total_referrals,
            successful_referrals,
            pending_referrals,
            total_bonuses_earned: total_bonuses_earned.to_string(),
            total_bonuses_pending: total_bonuses_pending.to_string(),
            conversion_rate: conversion_rate.to_string(),
            campaign_stats: campaign_protos,
        };

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_user_referral_analytics",
            &format!("Retrieved analytics for user: {} (period: {} to {})", user_id, start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetUserReferralAnalyticsResponse {
            stats: Some(stats_response),
            referral_tree: referral_tree.map(|tree| self.referral_tree_node_to_proto(&tree)),
            recent_bonuses: bonus_protos,
            projected_monthly_earnings,
            insights,
        }))
    }

    /// Bulk process referral bonuses for operational efficiency
    async fn bulk_process_bonuses(
        &self,
        request: Request<BulkProcessBonusesRequest>,
    ) -> Result<Response<BulkProcessBonusesResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate admin permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check admin permissions for bulk operations
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required for bulk bonus processing"));
        }

        // Validate bulk operation with security guard
        self.referral_guard
            .validate_administrative_access(&request, "bulk_process_bonuses")
            .await?;

        // Parse and validate relationship IDs
        let relationship_ids: Result<Vec<Uuid>, _> = req.relationship_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let relationship_ids = relationship_ids
            .map_err(|_| Status::invalid_argument("Invalid relationship IDs"))?;

        // Validate input parameters
        if relationship_ids.is_empty() {
            return Err(Status::invalid_argument("At least one relationship ID is required"));
        }

        if relationship_ids.len() > 1000 {
            return Err(Status::invalid_argument("Maximum 1000 relationships can be processed in a single batch"));
        }

        if req.milestone_type.is_empty() {
            return Err(Status::invalid_argument("Milestone type is required"));
        }

        if req.reason.is_empty() {
            return Err(Status::invalid_argument("Reason for bulk processing is required"));
        }

        // Generate batch ID if not provided
        let batch_id = if req.batch_id.is_empty() {
            format!("BULK-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), Uuid::new_v4().to_string()[..8].to_uppercase())
        } else {
            req.batch_id.clone()
        };

        // Process bonuses in bulk
        let (processed_bonuses, successful_count, failed_count, error_messages) =
            self.referral_repository
                .bulk_process_bonuses(relationship_ids.clone(), req.milestone_type.clone(), batch_id.clone(), req.reason.clone())
                .await
                .map_err(|e| Status::internal(format!("Failed to bulk process bonuses: {}", e)))?;

        // Convert processed bonuses to proto
        let bonus_protos: Vec<_> = processed_bonuses.iter()
            .map(|bonus| self.referral_bonus_to_proto(bonus))
            .collect();

        // Send notifications for successful bonus processing
        for bonus in &processed_bonuses {
            if bonus.status == ReferralBonusStatus::Completed {
                self.send_referral_notification(
                    &bonus.user_id,
                    NotificationType::Reward,
                    "Bonus Processed!",
                    &format!("Your referral bonus of {} {} has been processed",
                            bonus.bonus_amount, bonus.bonus_currency),
                    HashMap::from([
                        ("bonus_id".to_string(), bonus.id.to_string()),
                        ("batch_id".to_string(), batch_id.clone()),
                        ("amount".to_string(), bonus.bonus_amount.to_string()),
                        ("currency".to_string(), bonus.bonus_currency.clone()),
                    ]),
                ).await.unwrap_or_else(|e| {
                    // Log notification failure but don't fail the entire operation
                    eprintln!("Failed to send notification for bonus {}: {}", bonus.id, e);
                });
            }
        }

        // Log the bulk operation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "bulk_process_bonuses",
            &format!("Processed {} relationships in batch {} (successful: {}, failed: {}). Reason: {}",
                    relationship_ids.len(), batch_id, successful_count, failed_count, req.reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(BulkProcessBonusesResponse {
            bonuses: bonus_protos,
            successful_bonuses: successful_count,
            failed_bonuses: failed_count,
            error_messages,
            batch_id,
        }))
    }

    /// Flag suspicious activity for fraud detection and investigation
    async fn flag_suspicious_activity(
        &self,
        request: Request<FlagSuspiciousActivityRequest>,
    ) -> Result<Response<FlagSuspiciousActivityResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate relationship ID
        let relationship_id = Uuid::parse_str(&req.relationship_id)
            .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?;

        // Extract authentication and validate admin permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check admin permissions for fraud operations
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required for fraud detection operations"));
        }

        // Validate fraud operation with security guard
        self.referral_guard
            .validate_administrative_access(&request, "flag_suspicious_activity")
            .await?;

        // Validate input parameters
        if req.fraud_flags.is_empty() {
            return Err(Status::invalid_argument("At least one fraud flag is required"));
        }

        if req.reason.is_empty() {
            return Err(Status::invalid_argument("Reason for flagging is required"));
        }

        if req.reason.len() > 1000 {
            return Err(Status::invalid_argument("Reason too long (max 1000 characters)"));
        }

        // Validate fraud flags
        let valid_fraud_flags = vec![
            "velocity_fraud", "pattern_anomaly", "duplicate_device", "suspicious_ip",
            "manual_review", "kyc_mismatch", "transaction_anomaly", "referral_farming"
        ];

        for flag in &req.fraud_flags {
            if !valid_fraud_flags.contains(&flag.as_str()) {
                return Err(Status::invalid_argument(format!("Invalid fraud flag: {}", flag)));
            }
        }

        // Flag the suspicious activity
        let updated_relationship = self.referral_repository
            .flag_suspicious_activity(&relationship_id, req.fraud_flags.clone(), req.reason.clone(), req.auto_suspend)
            .await
            .map_err(|e| Status::internal(format!("Failed to flag suspicious activity: {}", e)))?;

        // Send notification to affected users if auto-suspended
        if req.auto_suspend && updated_relationship.status == ReferralRelationshipStatus::Fraudulent {
            // Notify referrer
            self.send_referral_notification(
                &updated_relationship.referrer_user_id,
                NotificationType::Security,
                "Referral Relationship Suspended",
                "One of your referral relationships has been suspended due to suspicious activity. Please contact support if you believe this is an error.",
                HashMap::from([
                    ("relationship_id".to_string(), relationship_id.to_string()),
                    ("reason".to_string(), "suspicious_activity".to_string()),
                ]),
            ).await.unwrap_or_else(|e| {
                eprintln!("Failed to send notification to referrer {}: {}", updated_relationship.referrer_user_id, e);
            });

            // Notify referee
            self.send_referral_notification(
                &updated_relationship.referee_user_id,
                NotificationType::Security,
                "Account Under Review",
                "Your account is under review due to suspicious activity. Please contact support for more information.",
                HashMap::from([
                    ("relationship_id".to_string(), relationship_id.to_string()),
                    ("reason".to_string(), "suspicious_activity".to_string()),
                ]),
            ).await.unwrap_or_else(|e| {
                eprintln!("Failed to send notification to referee {}: {}", updated_relationship.referee_user_id, e);
            });
        }

        // Log the fraud detection action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "flag_suspicious_activity",
            &format!("Flagged relationship {} with flags: {:?}. Auto-suspend: {}. Reason: {}",
                    relationship_id, req.fraud_flags, req.auto_suspend, req.reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        let message = if req.auto_suspend {
            "Suspicious activity flagged and relationship suspended successfully"
        } else {
            "Suspicious activity flagged successfully"
        };

        Ok(Response::new(FlagSuspiciousActivityResponse {
            relationship: Some(self.referral_relationship_to_proto(&updated_relationship)),
            message: message.to_string(),
        }))
    }

    /// Get comprehensive audit trail for compliance and regulatory requirements
    async fn get_referral_audit_trail(
        &self,
        request: Request<GetReferralAuditTrailRequest>,
    ) -> Result<Response<GetReferralAuditTrailResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate admin permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check admin permissions for audit trail access
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionViewReports).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required for audit trail access"));
        }

        // Validate audit access with security guard
        self.referral_guard
            .validate_administrative_access(&request, "get_audit_trail")
            .await?;

        // Parse optional user ID filter
        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        // Parse optional relationship ID filter
        let relationship_id = if req.relationship_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.relationship_id)
                .map_err(|_| Status::invalid_argument("Invalid relationship ID"))?)
        };

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        // Validate date range if both provided
        if let (Some(start), Some(end)) = (start_date, end_date) {
            if start >= end {
                return Err(Status::invalid_argument("Start date must be before end date"));
            }
        }

        // Validate pagination parameters
        let page = req.page.max(0);
        let page_size = req.page_size.clamp(1, 1000).max(50); // Default 50, max 1000

        // Validate action types
        let valid_action_types = vec![
            "create", "update", "bonus", "flag", "suspend", "activate", "delete", "claim"
        ];

        for action_type in &req.action_types {
            if !valid_action_types.contains(&action_type.as_str()) {
                return Err(Status::invalid_argument(format!("Invalid action type: {}", action_type)));
            }
        }

        // Get audit trail from repository
        let (audit_entries, total_count) = self.referral_repository
            .get_audit_trail(user_id, relationship_id, start_date, end_date, req.action_types.clone(), page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get audit trail: {}", e)))?;

        // Convert audit entries to proto
        let entry_protos: Vec<_> = audit_entries.iter()
            .map(|entry| self.audit_trail_entry_to_proto(entry))
            .collect();

        // Log the audit trail access
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "get_referral_audit_trail",
            &format!("Retrieved {} audit entries (page: {}, size: {}, filters: user_id={:?}, relationship_id={:?}, actions={:?})",
                    entry_protos.len(), page, page_size, user_id, relationship_id, req.action_types),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(GetReferralAuditTrailResponse {
            entries: entry_protos,
            total_count,
            page,
            page_size,
        }))
    }

    /// Recalculate referral metrics for data consistency and integrity
    async fn recalculate_referral_metrics(
        &self,
        request: Request<RecalculateReferralMetricsRequest>,
    ) -> Result<Response<RecalculateReferralMetricsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate admin permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Check admin permissions for metrics recalculation
        if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageReferrals).await.unwrap_or(false) {
            return Err(Status::permission_denied("Admin permissions required for metrics recalculation"));
        }

        // Validate recalculation access with security guard
        self.referral_guard
            .validate_administrative_access(&request, "recalculate_metrics")
            .await?;

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            Utc::now() - chrono::Duration::days(30) // Default to last 30 days
        } else {
            DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc)
        };

        let end_date = if req.end_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Validate date range is not too large (max 1 year)
        let max_duration = chrono::Duration::days(365);
        if end_date - start_date > max_duration {
            return Err(Status::invalid_argument("Date range too large (maximum 1 year)"));
        }

        // Parse campaign IDs filter
        let campaign_ids: Result<Vec<Uuid>, _> = req.campaign_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let campaign_ids = campaign_ids
            .map_err(|_| Status::invalid_argument("Invalid campaign IDs"))?;

        // Record start time for processing duration
        let processing_start = std::time::Instant::now();

        // Perform metrics recalculation
        let (relationships_processed, bonuses_recalculated) = self.referral_repository
            .recalculate_referral_metrics(start_date, end_date, campaign_ids.clone(), req.force_recalculation)
            .await
            .map_err(|e| Status::internal(format!("Failed to recalculate referral metrics: {}", e)))?;

        // Calculate processing time
        let processing_duration = processing_start.elapsed();
        let processing_time = format!("{}ms", processing_duration.as_millis());

        // Send notification to admin about completion
        self.send_referral_notification(
            &auth_context.user_id,
            NotificationType::System,
            "Metrics Recalculation Complete",
            &format!("Referral metrics recalculation completed. Processed {} relationships and {} bonuses in {}",
                    relationships_processed, bonuses_recalculated, processing_time),
            HashMap::from([
                ("relationships_processed".to_string(), relationships_processed.to_string()),
                ("bonuses_recalculated".to_string(), bonuses_recalculated.to_string()),
                ("processing_time".to_string(), processing_time.clone()),
                ("start_date".to_string(), start_date.to_rfc3339()),
                ("end_date".to_string(), end_date.to_rfc3339()),
            ]),
        ).await.unwrap_or_else(|e| {
            eprintln!("Failed to send recalculation notification: {}", e);
        });

        // Log the recalculation operation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "recalculate_referral_metrics",
            &format!("Recalculated metrics for period {} to {} (campaigns: {:?}, force: {}, processed: {}, bonuses: {}, time: {})",
                    start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"),
                    campaign_ids, req.force_recalculation, relationships_processed, bonuses_recalculated, processing_time),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        let message = if req.force_recalculation {
            "Referral metrics force recalculation completed successfully"
        } else {
            "Referral metrics recalculation completed successfully"
        };

        Ok(Response::new(RecalculateReferralMetricsResponse {
            message: message.to_string(),
            relationships_processed,
            bonuses_recalculated,
            processing_time,
        }))
    }
}
