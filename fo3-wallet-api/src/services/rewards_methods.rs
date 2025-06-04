//! Rewards service method implementations

use super::rewards::RewardsServiceImpl;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    rewards_service_server::RewardsService,
    *,
};
use crate::models::rewards::{
    RewardRule, UserRewards, RewardTransaction, Redemption,
    RewardRuleType, RewardRuleStatus, UserRewardTier, RewardTransactionType, RewardTransactionStatus,
    RedemptionType, RedemptionStatus,
};
use crate::models::notifications::NotificationType;

#[tonic::async_trait]
impl RewardsService for RewardsServiceImpl {
    /// Create a new reward rule
    async fn create_reward_rule(
        &self,
        request: Request<CreateRewardRuleRequest>,
    ) -> Result<Response<CreateRewardRuleResponse>, Status> {
        let req = request.get_ref();
        let rule_proto = req.rule.as_ref()
            .ok_or_else(|| Status::invalid_argument("Reward rule required"))?;

        // Parse and validate request
        let rule_type = Self::proto_to_reward_rule_type(rule_proto.r#type)?;
        let points_per_unit = Decimal::from_str(&rule_proto.points_per_unit)
            .map_err(|_| Status::invalid_argument("Invalid points per unit"))?;
        let minimum_tier = Self::proto_to_user_reward_tier(rule_proto.minimum_tier)?;

        // Validate request with security guard
        let auth_context = self.rewards_guard
            .validate_reward_rule_creation(&request, &rule_proto.name, &points_per_unit, &minimum_tier)
            .await?;

        // Create reward rule
        let rule = RewardRule {
            id: Uuid::new_v4(),
            name: rule_proto.name.clone(),
            description: if rule_proto.description.is_empty() { None } else { Some(rule_proto.description.clone()) },
            rule_type,
            status: if rule_proto.status != 0 {
                Self::proto_to_reward_rule_status(rule_proto.status)?
            } else {
                RewardRuleStatus::Active
            },
            points_per_unit,
            minimum_amount: if rule_proto.minimum_amount.is_empty() {
                None
            } else {
                Some(Decimal::from_str(&rule_proto.minimum_amount)
                    .map_err(|_| Status::invalid_argument("Invalid minimum amount"))?)
            },
            maximum_points: if rule_proto.maximum_points.is_empty() {
                None
            } else {
                Some(Decimal::from_str(&rule_proto.maximum_points)
                    .map_err(|_| Status::invalid_argument("Invalid maximum points"))?)
            },
            minimum_tier,
            categories: rule_proto.categories.clone(),
            currencies: if rule_proto.currencies.is_empty() {
                vec!["USD".to_string()]
            } else {
                rule_proto.currencies.clone()
            },
            start_date: if rule_proto.start_date.is_empty() {
                None
            } else {
                Some(DateTime::parse_from_rfc3339(&rule_proto.start_date)
                    .map_err(|_| Status::invalid_argument("Invalid start date"))?
                    .with_timezone(&Utc))
            },
            end_date: if rule_proto.end_date.is_empty() {
                None
            } else {
                Some(DateTime::parse_from_rfc3339(&rule_proto.end_date)
                    .map_err(|_| Status::invalid_argument("Invalid end date"))?
                    .with_timezone(&Utc))
            },
            days_of_week: rule_proto.days_of_week.clone(),
            start_time: if rule_proto.start_time.is_empty() {
                None
            } else {
                Some(chrono::NaiveTime::parse_from_str(&rule_proto.start_time, "%H:%M")
                    .map_err(|_| Status::invalid_argument("Invalid start time"))?)
            },
            end_time: if rule_proto.end_time.is_empty() {
                None
            } else {
                Some(chrono::NaiveTime::parse_from_str(&rule_proto.end_time, "%H:%M")
                    .map_err(|_| Status::invalid_argument("Invalid end time"))?)
            },
            max_uses_per_user: if rule_proto.max_uses_per_user == -1 { None } else { Some(rule_proto.max_uses_per_user) },
            max_uses_per_day: if rule_proto.max_uses_per_day == -1 { None } else { Some(rule_proto.max_uses_per_day) },
            max_uses_per_month: if rule_proto.max_uses_per_month == -1 { None } else { Some(rule_proto.max_uses_per_month) },
            total_uses_remaining: if rule_proto.total_uses_remaining == -1 { None } else { Some(rule_proto.total_uses_remaining) },
            metadata: rule_proto.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: Some(auth_context.user_id),
        };

        // Save rule
        let created_rule = self.rewards_repository
            .create_reward_rule(&rule)
            .await
            .map_err(|e| Status::internal(format!("Failed to create reward rule: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "create_reward_rule",
            &format!("Created rule: {} ({})", created_rule.name, created_rule.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(CreateRewardRuleResponse {
            rule: Some(self.reward_rule_to_proto(&created_rule)),
            message: "Reward rule created successfully".to_string(),
        }))
    }

    /// Get a reward rule by ID
    async fn get_reward_rule(
        &self,
        request: Request<GetRewardRuleRequest>,
    ) -> Result<Response<GetRewardRuleResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionViewRewards)?;

        // Parse rule ID
        let rule_id = Uuid::parse_str(&req.rule_id)
            .map_err(|_| Status::invalid_argument("Invalid rule ID"))?;

        // Get rule
        let rule = self.rewards_repository
            .get_reward_rule(&rule_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get reward rule: {}", e)))?;

        match rule {
            Some(rule) => Ok(Response::new(GetRewardRuleResponse {
                rule: Some(self.reward_rule_to_proto(&rule)),
            })),
            None => Err(Status::not_found("Reward rule not found")),
        }
    }

    /// List reward rules
    async fn list_reward_rules(
        &self,
        request: Request<ListRewardRulesRequest>,
    ) -> Result<Response<ListRewardRulesResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionViewRewards)?;

        // Parse filters
        let rule_type = if req.r#type != 0 {
            Some(Self::proto_to_reward_rule_type(req.r#type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_reward_rule_status(req.status)?)
        } else {
            None
        };

        let category = if req.category.is_empty() { None } else { Some(req.category.clone()) };
        let currency = if req.currency.is_empty() { None } else { Some(req.currency.clone()) };

        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get rules
        let (rules, total_count) = self.rewards_repository
            .list_reward_rules(rule_type, status, category, currency, req.active_only, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list reward rules: {}", e)))?;

        let proto_rules = rules.iter()
            .map(|rule| self.reward_rule_to_proto(rule))
            .collect();

        Ok(Response::new(ListRewardRulesResponse {
            rules: proto_rules,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Get user rewards
    async fn get_user_rewards(
        &self,
        request: Request<GetUserRewardsRequest>,
    ) -> Result<Response<GetUserRewardsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Check if user can access their own rewards or has admin permissions
        if auth_context.user_id != user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
            return Err(Status::permission_denied("Can only view your own rewards"));
        }

        // Get user rewards
        let rewards = self.rewards_repository
            .get_user_rewards(&user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get user rewards: {}", e)))?;

        match rewards {
            Some(rewards) => Ok(Response::new(GetUserRewardsResponse {
                rewards: Some(self.user_rewards_to_proto(&rewards)),
            })),
            None => {
                // Create default user rewards if not found
                let default_rewards = UserRewards {
                    user_id,
                    ..Default::default()
                };
                
                let created_rewards = self.rewards_repository
                    .create_user_rewards(&default_rewards)
                    .await
                    .map_err(|e| Status::internal(format!("Failed to create user rewards: {}", e)))?;

                Ok(Response::new(GetUserRewardsResponse {
                    rewards: Some(self.user_rewards_to_proto(&created_rewards)),
                }))
            }
        }
    }

    /// Award points to a user
    async fn award_points(
        &self,
        request: Request<AwardPointsRequest>,
    ) -> Result<Response<AwardPointsResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        let points = Decimal::from_str(&req.points)
            .map_err(|_| Status::invalid_argument("Invalid points amount"))?;

        // Validate request with security guard
        let auth_context = self.rewards_guard
            .validate_points_award(&request, &user_id, &points, &req.source_type)
            .await?;

        // Get or create user rewards
        let mut user_rewards = match self.rewards_repository.get_user_rewards(&user_id).await {
            Ok(Some(rewards)) => rewards,
            Ok(None) => {
                let default_rewards = UserRewards {
                    user_id,
                    ..Default::default()
                };
                self.rewards_repository.create_user_rewards(&default_rewards).await
                    .map_err(|e| Status::internal(format!("Failed to create user rewards: {}", e)))?
            },
            Err(e) => return Err(Status::internal(format!("Failed to get user rewards: {}", e))),
        };

        // Apply tier multiplier
        let multiplier = user_rewards.tier_multiplier;
        let final_points = points * multiplier;

        // Create reward transaction
        let transaction = RewardTransaction {
            id: Uuid::new_v4(),
            user_id,
            transaction_type: RewardTransactionType::Earned,
            status: RewardTransactionStatus::Completed,
            points: final_points,
            multiplier,
            base_points: points,
            currency: "USD".to_string(),
            exchange_rate: None,
            source_type: Some(req.source_type.clone()),
            source_id: Some(req.source_id.clone()),
            reward_rule_id: None,
            reference_number: self.generate_reference_number(),
            expires_at: if req.expires_at.is_empty() {
                None
            } else {
                Some(DateTime::parse_from_rfc3339(&req.expires_at)
                    .map_err(|_| Status::invalid_argument("Invalid expiration date"))?
                    .with_timezone(&Utc))
            },
            is_expired: false,
            description: Some(req.description.clone()),
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save transaction
        let created_transaction = self.rewards_repository
            .create_reward_transaction(&transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create reward transaction: {}", e)))?;

        // Update user rewards
        user_rewards.total_points += final_points;
        user_rewards.lifetime_earned += final_points;
        user_rewards.last_activity_date = Utc::now();
        user_rewards.updated_at = Utc::now();

        let updated_rewards = self.rewards_repository
            .update_user_rewards(&user_rewards)
            .await
            .map_err(|e| Status::internal(format!("Failed to update user rewards: {}", e)))?;

        // Check for tier upgrade
        self.update_user_tier_if_eligible(&user_id).await?;

        // Send notification
        self.send_reward_notification(
            &user_id,
            NotificationType::RewardEarned,
            "Points Earned!",
            &format!("You earned {} points!", final_points),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "award_points",
            &format!("Awarded {} points to user {}", final_points, user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(AwardPointsResponse {
            transaction: Some(self.reward_transaction_to_proto(&created_transaction)),
            updated_rewards: Some(self.user_rewards_to_proto(&updated_rewards)),
            message: format!("Successfully awarded {} points", final_points),
        }))
    }

    /// Update a reward rule
    async fn update_reward_rule(
        &self,
        request: Request<UpdateRewardRuleRequest>,
    ) -> Result<Response<UpdateRewardRuleResponse>, Status> {
        let req = request.get_ref();
        let rule_proto = req.rule.as_ref()
            .ok_or_else(|| Status::invalid_argument("Reward rule required"))?;

        // Parse rule ID
        let rule_id = Uuid::parse_str(&rule_proto.id)
            .map_err(|_| Status::invalid_argument("Invalid rule ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Get existing rule
        let mut existing_rule = self.rewards_repository
            .get_reward_rule(&rule_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get reward rule: {}", e)))?
            .ok_or_else(|| Status::not_found("Reward rule not found"))?;

        // Validate update with security guard
        let points_per_unit = Decimal::from_str(&rule_proto.points_per_unit)
            .map_err(|_| Status::invalid_argument("Invalid points per unit"))?;
        let minimum_tier = Self::proto_to_user_reward_tier(rule_proto.minimum_tier)?;

        self.rewards_guard
            .validate_reward_rule_update(&request, &rule_id, &rule_proto.name, &points_per_unit)
            .await?;

        // Update rule fields
        existing_rule.name = rule_proto.name.clone();
        existing_rule.description = if rule_proto.description.is_empty() {
            None
        } else {
            Some(rule_proto.description.clone())
        };
        existing_rule.status = if rule_proto.status != 0 {
            Self::proto_to_reward_rule_status(rule_proto.status)?
        } else {
            existing_rule.status
        };
        existing_rule.points_per_unit = points_per_unit;
        existing_rule.minimum_tier = minimum_tier;
        existing_rule.categories = rule_proto.categories.clone();
        existing_rule.currencies = if rule_proto.currencies.is_empty() {
            existing_rule.currencies
        } else {
            rule_proto.currencies.clone()
        };
        existing_rule.metadata = rule_proto.metadata.clone();
        existing_rule.updated_at = Utc::now();

        // Save updated rule
        let updated_rule = self.rewards_repository
            .update_reward_rule(&existing_rule)
            .await
            .map_err(|e| Status::internal(format!("Failed to update reward rule: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "update_reward_rule",
            &format!("Updated rule: {} ({})", updated_rule.name, updated_rule.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(UpdateRewardRuleResponse {
            rule: Some(self.reward_rule_to_proto(&updated_rule)),
            message: "Reward rule updated successfully".to_string(),
        }))
    }

    /// Delete (deactivate) a reward rule
    async fn delete_reward_rule(
        &self,
        request: Request<DeleteRewardRuleRequest>,
    ) -> Result<Response<DeleteRewardRuleResponse>, Status> {
        let req = request.get_ref();

        // Parse rule ID
        let rule_id = Uuid::parse_str(&req.rule_id)
            .map_err(|_| Status::invalid_argument("Invalid rule ID"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Get existing rule
        let mut existing_rule = self.rewards_repository
            .get_reward_rule(&rule_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get reward rule: {}", e)))?
            .ok_or_else(|| Status::not_found("Reward rule not found"))?;

        // Validate deletion with security guard
        self.rewards_guard
            .validate_reward_rule_deletion(&request, &rule_id)
            .await?;

        // Soft delete by setting status to inactive
        existing_rule.status = RewardRuleStatus::Inactive;
        existing_rule.updated_at = Utc::now();

        // Save updated rule
        let updated_rule = self.rewards_repository
            .update_reward_rule(&existing_rule)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete reward rule: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "delete_reward_rule",
            &format!("Deleted rule: {} ({})", updated_rule.name, updated_rule.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(DeleteRewardRuleResponse {
            message: "Reward rule deleted successfully".to_string(),
        }))
    }

    /// Redeem points for rewards
    async fn redeem_points(
        &self,
        request: Request<RedeemPointsRequest>,
    ) -> Result<Response<RedeemPointsResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        let points_to_redeem = Decimal::from_str(&req.points_to_redeem)
            .map_err(|_| Status::invalid_argument("Invalid points amount"))?;
        let redemption_type = Self::proto_to_redemption_type(req.redemption_type)?;

        // Validate request with security guard
        let auth_context = self.rewards_guard
            .validate_points_redemption(&request, &user_id, &points_to_redeem, &redemption_type)
            .await?;

        // Get user rewards
        let mut user_rewards = self.rewards_repository
            .get_user_rewards(&user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get user rewards: {}", e)))?
            .ok_or_else(|| Status::not_found("User rewards not found"))?;

        // Check if user has enough points
        if user_rewards.total_points < points_to_redeem {
            return Err(Status::failed_precondition("Insufficient points for redemption"));
        }

        // Calculate cash value and fees
        let exchange_rate = Decimal::from_str("0.01").unwrap(); // 1 point = $0.01
        let cash_value = points_to_redeem * exchange_rate;
        let processing_fee = cash_value * Decimal::from_str("0.02").unwrap(); // 2% fee
        let net_amount = cash_value - processing_fee;

        // Create redemption
        let redemption = Redemption {
            id: Uuid::new_v4(),
            user_id,
            redemption_type: redemption_type.clone(),
            status: RedemptionStatus::Pending,
            points_redeemed: points_to_redeem,
            cash_value,
            currency: "USD".to_string(),
            exchange_rate,
            target_account: if req.target_account.is_empty() { None } else { Some(req.target_account.clone()) },
            gift_card_code: None,
            merchant_name: if req.merchant_name.is_empty() { None } else { Some(req.merchant_name.clone()) },
            tracking_number: None,
            processing_fee,
            net_amount,
            estimated_delivery: None,
            actual_delivery: None,
            description: if req.description.is_empty() { None } else { Some(req.description.clone()) },
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        // Save redemption
        let created_redemption = self.rewards_repository
            .create_redemption(&redemption)
            .await
            .map_err(|e| Status::internal(format!("Failed to create redemption: {}", e)))?;

        // Create redemption transaction
        let transaction = RewardTransaction {
            id: Uuid::new_v4(),
            user_id,
            transaction_type: RewardTransactionType::Redeemed,
            status: RewardTransactionStatus::Completed,
            points: -points_to_redeem, // Negative for redemption
            multiplier: Decimal::ONE,
            base_points: -points_to_redeem,
            currency: "USD".to_string(),
            exchange_rate: Some(exchange_rate),
            source_type: Some("redemption".to_string()),
            source_id: Some(created_redemption.id.to_string()),
            reward_rule_id: None,
            reference_number: self.generate_reference_number(),
            expires_at: None,
            is_expired: false,
            description: Some(format!("Redemption: {:?}", redemption_type)),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save transaction
        let created_transaction = self.rewards_repository
            .create_reward_transaction(&transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create reward transaction: {}", e)))?;

        // Update user rewards
        user_rewards.total_points -= points_to_redeem;
        user_rewards.lifetime_redeemed += points_to_redeem;
        user_rewards.last_activity_date = Utc::now();
        user_rewards.updated_at = Utc::now();

        let updated_rewards = self.rewards_repository
            .update_user_rewards(&user_rewards)
            .await
            .map_err(|e| Status::internal(format!("Failed to update user rewards: {}", e)))?;

        // Send notification
        self.send_reward_notification(
            &user_id,
            NotificationType::RewardRedeemed,
            "Points Redeemed!",
            &format!("You redeemed {} points for ${:.2}", points_to_redeem, net_amount),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "redeem_points",
            &format!("Redeemed {} points for user {}", points_to_redeem, user_id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(RedeemPointsResponse {
            redemption: Some(self.redemption_to_proto(&created_redemption)),
            transaction: Some(self.reward_transaction_to_proto(&created_transaction)),
            updated_rewards: Some(self.user_rewards_to_proto(&updated_rewards)),
            message: format!("Successfully redeemed {} points", points_to_redeem),
        }))
    }

    /// Get a redemption by ID
    async fn get_redemption(
        &self,
        request: Request<GetRedemptionRequest>,
    ) -> Result<Response<GetRedemptionResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse redemption ID
        let redemption_id = Uuid::parse_str(&req.redemption_id)
            .map_err(|_| Status::invalid_argument("Invalid redemption ID"))?;

        // Get redemption
        let redemption = self.rewards_repository
            .get_redemption(&redemption_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get redemption: {}", e)))?;

        match redemption {
            Some(redemption) => {
                // Check if user can access this redemption
                if auth_context.user_id != redemption.user_id &&
                   !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
                    return Err(Status::permission_denied("Can only view your own redemptions"));
                }

                Ok(Response::new(GetRedemptionResponse {
                    redemption: Some(self.redemption_to_proto(&redemption)),
                }))
            },
            None => Err(Status::not_found("Redemption not found")),
        }
    }

    /// List redemptions
    async fn list_redemptions(
        &self,
        request: Request<ListRedemptionsRequest>,
    ) -> Result<Response<ListRedemptionsResponse>, Status> {
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

        // Check permissions
        if let Some(uid) = user_id {
            if auth_context.user_id != uid &&
               !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
                return Err(Status::permission_denied("Can only view your own redemptions"));
            }
        } else if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
            // If no user_id specified, default to current user's redemptions
            return Err(Status::permission_denied("Must specify user_id or have admin permissions"));
        }

        let redemption_type = if req.redemption_type != 0 {
            Some(Self::proto_to_redemption_type(req.redemption_type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_redemption_status(req.status)?)
        } else {
            None
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

        // Get redemptions
        let (redemptions, total_count) = self.rewards_repository
            .list_redemptions(user_id, redemption_type, status, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list redemptions: {}", e)))?;

        let proto_redemptions = redemptions.iter()
            .map(|redemption| self.redemption_to_proto(redemption))
            .collect();

        Ok(Response::new(ListRedemptionsResponse {
            redemptions: proto_redemptions,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Create a redemption option
    async fn create_redemption_option(
        &self,
        request: Request<CreateRedemptionOptionRequest>,
    ) -> Result<Response<CreateRedemptionOptionResponse>, Status> {
        let req = request.get_ref();
        let option_proto = req.option.as_ref()
            .ok_or_else(|| Status::invalid_argument("Redemption option required"))?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Parse and validate request
        let redemption_type = Self::proto_to_redemption_type(option_proto.redemption_type)?;
        let minimum_tier = Self::proto_to_user_reward_tier(option_proto.minimum_tier)?;
        let points_required = Decimal::from_str(&option_proto.points_required)
            .map_err(|_| Status::invalid_argument("Invalid points required"))?;
        let cash_value = Decimal::from_str(&option_proto.cash_value)
            .map_err(|_| Status::invalid_argument("Invalid cash value"))?;

        // Create redemption option
        let option = RedemptionOption {
            id: Uuid::new_v4(),
            name: option_proto.name.clone(),
            description: if option_proto.description.is_empty() { None } else { Some(option_proto.description.clone()) },
            redemption_type,
            points_required,
            cash_value,
            currency: if option_proto.currency.is_empty() { "USD".to_string() } else { option_proto.currency.clone() },
            exchange_rate: Decimal::from_str(&option_proto.exchange_rate).unwrap_or(Decimal::from_str("0.01").unwrap()),
            is_active: option_proto.is_active,
            minimum_tier,
            maximum_redemptions_per_user: if option_proto.max_redemptions_per_user == -1 { None } else { Some(option_proto.max_redemptions_per_user) },
            maximum_redemptions_per_day: if option_proto.max_redemptions_per_day == -1 { None } else { Some(option_proto.max_redemptions_per_day) },
            minimum_points_balance: Decimal::from_str(&option_proto.minimum_points_balance).unwrap_or(Decimal::ZERO),
            image_url: if option_proto.image_url.is_empty() { None } else { Some(option_proto.image_url.clone()) },
            tags: option_proto.tags.clone(),
            metadata: option_proto.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save option
        let created_option = self.rewards_repository
            .create_redemption_option(&option)
            .await
            .map_err(|e| Status::internal(format!("Failed to create redemption option: {}", e)))?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "create_redemption_option",
            &format!("Created option: {} ({})", created_option.name, created_option.id),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(CreateRedemptionOptionResponse {
            option: Some(self.redemption_option_to_proto(&created_option)),
            message: "Redemption option created successfully".to_string(),
        }))
    }

    /// List redemption options
    async fn list_redemption_options(
        &self,
        request: Request<ListRedemptionOptionsRequest>,
    ) -> Result<Response<ListRedemptionOptionsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionViewRewards)?;

        // Parse filters
        let redemption_type = if req.redemption_type != 0 {
            Some(Self::proto_to_redemption_type(req.redemption_type)?)
        } else {
            None
        };

        let minimum_tier = if req.minimum_tier != 0 {
            Some(Self::proto_to_user_reward_tier(req.minimum_tier)?)
        } else {
            None
        };

        let minimum_points = if req.minimum_points.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&req.minimum_points)
                .map_err(|_| Status::invalid_argument("Invalid minimum points"))?)
        };

        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get redemption options
        let (options, total_count) = self.rewards_repository
            .list_redemption_options(redemption_type, minimum_tier, minimum_points, req.active_only, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list redemption options: {}", e)))?;

        let proto_options = options.iter()
            .map(|option| self.redemption_option_to_proto(option))
            .collect();

        Ok(Response::new(ListRedemptionOptionsResponse {
            options: proto_options,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Get reward metrics
    async fn get_reward_metrics(
        &self,
        request: Request<GetRewardMetricsRequest>,
    ) -> Result<Response<GetRewardMetricsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionViewRewards)?;

        // Parse date range
        let start_date = DateTime::parse_from_rfc3339(&req.start_date)
            .map_err(|_| Status::invalid_argument("Invalid start date"))?
            .with_timezone(&Utc);
        let end_date = DateTime::parse_from_rfc3339(&req.end_date)
            .map_err(|_| Status::invalid_argument("Invalid end date"))?
            .with_timezone(&Utc);

        // Parse filters
        let rule_types = req.rule_types.iter()
            .map(|&t| Self::proto_to_reward_rule_type(t))
            .collect::<Result<Vec<_>, _>>()?;

        let tiers = req.tiers.iter()
            .map(|&t| Self::proto_to_user_reward_tier(t))
            .collect::<Result<Vec<_>, _>>()?;

        let currencies = req.currencies.clone();

        // Get metrics
        let metrics = self.rewards_repository
            .get_reward_metrics(start_date, end_date, rule_types, tiers, currencies)
            .await
            .map_err(|e| Status::internal(format!("Failed to get reward metrics: {}", e)))?;

        // Convert to proto
        let proto_metrics = RewardMetrics {
            total_points_awarded: metrics.total_points_awarded.to_string(),
            total_points_redeemed: metrics.total_points_redeemed.to_string(),
            total_points_expired: metrics.total_points_expired.to_string(),
            total_cash_value: metrics.total_cash_value.to_string(),
            total_users: metrics.total_users,
            active_users: metrics.active_users,
            period_start: metrics.period_start.to_rfc3339(),
            period_end: metrics.period_end.to_rfc3339(),
            period_points_awarded: metrics.period_points_awarded.to_string(),
            period_points_redeemed: metrics.period_points_redeemed.to_string(),
            period_transactions: metrics.period_transactions,
            period_redemptions: metrics.period_redemptions,
            bronze_users: metrics.bronze_users,
            silver_users: metrics.silver_users,
            gold_users: metrics.gold_users,
            platinum_users: metrics.platinum_users,
            top_categories: metrics.top_categories.iter().map(|c| CategoryMetrics {
                category: c.category.clone(),
                points_awarded: c.points_awarded.to_string(),
                transaction_count: c.transaction_count,
                average_points: c.average_points.to_string(),
            }).collect(),
            top_redemptions: metrics.top_redemptions.iter().map(|r| RedemptionMetrics {
                redemption_type: Self::redemption_type_to_proto(&r.redemption_type),
                points_redeemed: r.points_redeemed.to_string(),
                redemption_count: r.redemption_count,
                average_points: r.average_points.to_string(),
            }).collect(),
            generated_at: metrics.generated_at.to_rfc3339(),
            metadata: metrics.metadata,
        };

        Ok(Response::new(GetRewardMetricsResponse {
            metrics: Some(proto_metrics),
        }))
    }

    /// List reward transactions
    async fn list_reward_transactions(
        &self,
        request: Request<ListRewardTransactionsRequest>,
    ) -> Result<Response<ListRewardTransactionsResponse>, Status> {
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

        // Check permissions
        if let Some(uid) = user_id {
            if auth_context.user_id != uid &&
               !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
                return Err(Status::permission_denied("Can only view your own transactions"));
            }
        } else if !self.auth_service.has_permission(&auth_context, Permission::PermissionManageRewards).await? {
            return Err(Status::permission_denied("Must specify user_id or have admin permissions"));
        }

        let transaction_type = if req.transaction_type != 0 {
            Some(Self::proto_to_reward_transaction_type(req.transaction_type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_reward_transaction_status(req.status)?)
        } else {
            None
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

        // Get transactions
        let (transactions, total_count) = self.rewards_repository
            .list_reward_transactions(user_id, transaction_type, status, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list reward transactions: {}", e)))?;

        let proto_transactions = transactions.iter()
            .map(|transaction| self.reward_transaction_to_proto(transaction))
            .collect();

        Ok(Response::new(ListRewardTransactionsResponse {
            transactions: proto_transactions,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Update user tier manually
    async fn update_user_tier(
        &self,
        request: Request<UpdateUserTierRequest>,
    ) -> Result<Response<UpdateUserTierResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        let new_tier = Self::proto_to_user_reward_tier(req.new_tier)?;

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Update user tier
        let updated_rewards = self.rewards_repository
            .update_user_tier(&user_id, new_tier.clone(), Some(req.reason.clone()))
            .await
            .map_err(|e| Status::internal(format!("Failed to update user tier: {}", e)))?;

        // Send notification
        self.send_reward_notification(
            &user_id,
            NotificationType::RewardTierUpgrade,
            "Tier Updated!",
            &format!("Your tier has been updated to {:?}", new_tier),
            HashMap::new(),
        ).await?;

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "update_user_tier",
            &format!("Updated tier to {:?} for user {}: {}", new_tier, user_id, req.reason),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(UpdateUserTierResponse {
            updated_rewards: Some(self.user_rewards_to_proto(&updated_rewards)),
            message: format!("User tier updated to {:?}", new_tier),
        }))
    }

    /// Expire points
    async fn expire_points(
        &self,
        request: Request<ExpirePointsRequest>,
    ) -> Result<Response<ExpirePointsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Parse parameters
        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let expiration_date = DateTime::parse_from_rfc3339(&req.expiration_date)
            .map_err(|_| Status::invalid_argument("Invalid expiration date"))?
            .with_timezone(&Utc);

        // Expire points
        let (expired_transactions, users_affected) = self.rewards_repository
            .expire_points(user_id, expiration_date, req.dry_run)
            .await
            .map_err(|e| Status::internal(format!("Failed to expire points: {}", e)))?;

        let total_points_expired: Decimal = expired_transactions.iter()
            .map(|tx| tx.points.abs())
            .sum();

        // Log the action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "expire_points",
            &format!("Expired {} points for {} users (dry_run: {})", total_points_expired, users_affected, req.dry_run),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        Ok(Response::new(ExpirePointsResponse {
            expired_transactions: expired_transactions.iter()
                .map(|tx| self.reward_transaction_to_proto(tx))
                .collect(),
            total_points_expired: total_points_expired.to_string(),
            users_affected: users_affected as i64,
            message: if req.dry_run {
                format!("Would expire {} points for {} users", total_points_expired, users_affected)
            } else {
                format!("Expired {} points for {} users", total_points_expired, users_affected)
            },
        }))
    }

    /// Get audit trail
    async fn get_audit_trail(
        &self,
        request: Request<GetAuditTrailRequest>,
    ) -> Result<Response<GetAuditTrailResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionViewRewards)?;

        // Parse filters
        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
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

        let action_types = req.action_types.clone();
        let page = if req.page > 0 { req.page } else { 0 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get audit trail
        let (audit_entries, total_count) = self.rewards_repository
            .get_audit_trail(user_id, start_date, end_date, action_types, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get audit trail: {}", e)))?;

        let proto_entries = audit_entries.iter().map(|entry| AuditTrailEntry {
            id: entry.id.to_string(),
            user_id: entry.user_id.map(|id| id.to_string()).unwrap_or_default(),
            action_type: entry.action_type.clone(),
            entity_type: entry.entity_type.clone(),
            entity_id: entry.entity_id.to_string(),
            old_value: entry.old_value.clone().unwrap_or_default(),
            new_value: entry.new_value.clone().unwrap_or_default(),
            reason: entry.reason.clone().unwrap_or_default(),
            performed_by: entry.performed_by.map(|id| id.to_string()).unwrap_or_default(),
            ip_address: entry.ip_address.clone().unwrap_or_default(),
            user_agent: entry.user_agent.clone().unwrap_or_default(),
            metadata: entry.metadata.clone(),
            created_at: entry.created_at.to_rfc3339(),
        }).collect();

        Ok(Response::new(GetAuditTrailResponse {
            entries: proto_entries,
            total_count: total_count as i64,
            page,
            page_size,
        }))
    }

    /// Bulk award points to multiple users
    async fn bulk_award_points(
        &self,
        request: Request<BulkAwardPointsRequest>,
    ) -> Result<Response<BulkAwardPointsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate permissions
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionManageRewards)?;

        // Validate bulk operation limits
        if req.awards.len() > 1000 {
            return Err(Status::invalid_argument("Maximum 1000 awards per bulk operation"));
        }

        let mut successful_awards = Vec::new();
        let mut failed_awards = Vec::new();
        let mut total_points_awarded = Decimal::ZERO;

        // Process each award
        for award in &req.awards {
            let user_id = match Uuid::parse_str(&award.user_id) {
                Ok(id) => id,
                Err(_) => {
                    failed_awards.push(BulkAwardResult {
                        user_id: award.user_id.clone(),
                        success: false,
                        error_message: "Invalid user ID".to_string(),
                        points_awarded: "0".to_string(),
                    });
                    continue;
                }
            };

            let points = match Decimal::from_str(&award.points) {
                Ok(p) => p,
                Err(_) => {
                    failed_awards.push(BulkAwardResult {
                        user_id: award.user_id.clone(),
                        success: false,
                        error_message: "Invalid points amount".to_string(),
                        points_awarded: "0".to_string(),
                    });
                    continue;
                }
            };

            // Validate points amount
            if points <= Decimal::ZERO || points > Decimal::from(10000) {
                failed_awards.push(BulkAwardResult {
                    user_id: award.user_id.clone(),
                    success: false,
                    error_message: "Points amount must be between 0 and 10000".to_string(),
                    points_awarded: "0".to_string(),
                });
                continue;
            }

            // Try to award points
            match self.award_points_internal(&user_id, &points, &award.source_type, &award.source_id, &award.description, &award.metadata).await {
                Ok(final_points) => {
                    successful_awards.push(BulkAwardResult {
                        user_id: award.user_id.clone(),
                        success: true,
                        error_message: String::new(),
                        points_awarded: final_points.to_string(),
                    });
                    total_points_awarded += final_points;
                },
                Err(e) => {
                    failed_awards.push(BulkAwardResult {
                        user_id: award.user_id.clone(),
                        success: false,
                        error_message: e,
                        points_awarded: "0".to_string(),
                    });
                }
            }
        }

        // Log the bulk operation
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "bulk_award_points",
            &format!("Bulk awarded {} points to {} users ({} successful, {} failed)",
                total_points_awarded, req.awards.len(), successful_awards.len(), failed_awards.len()),
            request.remote_addr().map(|addr| addr.ip()),
        ).await.map_err(|e| Status::internal(format!("Failed to log action: {}", e)))?;

        let mut results = successful_awards;
        results.extend(failed_awards);

        Ok(Response::new(BulkAwardPointsResponse {
            results,
            total_processed: req.awards.len() as i64,
            successful_count: successful_awards.len() as i64,
            failed_count: failed_awards.len() as i64,
            total_points_awarded: total_points_awarded.to_string(),
            message: format!("Processed {} awards: {} successful, {} failed",
                req.awards.len(), successful_awards.len(), failed_awards.len()),
        }))
    }
}
