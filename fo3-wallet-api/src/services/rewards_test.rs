//! Comprehensive tests for the Rewards service

#[cfg(test)]
mod tests {
    use super::super::rewards::RewardsServiceImpl;
    use crate::models::rewards::{
        RewardRule, UserRewards, RewardTransaction, Redemption, RedemptionOption,
        RewardRuleType, RewardRuleStatus, UserRewardTier, RewardTransactionType, RewardTransactionStatus,
        RedemptionType, RedemptionStatus, InMemoryRewardsRepository, RewardsRepository,
    };
    use crate::middleware::{
        auth::AuthService,
        audit::AuditLogger,
        rewards_guard::RewardsGuard,
        rate_limit::RateLimiter,
    };
    use crate::state::AppState;
    use crate::proto::fo3::wallet::v1::{
        rewards_service_server::RewardsService,
        *,
    };
    use std::sync::Arc;
    use std::collections::HashMap;
    use uuid::Uuid;
    use rust_decimal::Decimal;
    use chrono::Utc;
    use tonic::{Request, Code};

    async fn setup_test_service() -> (RewardsServiceImpl, Arc<dyn RewardsRepository>) {
        let state = Arc::new(AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let audit_logger = Arc::new(AuditLogger::new(state.clone()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let rewards_repository = Arc::new(InMemoryRewardsRepository::default());
        let rewards_guard = Arc::new(RewardsGuard::new(
            auth_service.clone(),
            audit_logger.clone(),
            rate_limiter.clone(),
            rewards_repository.clone(),
        ));

        let service = RewardsServiceImpl::new(
            state,
            auth_service,
            audit_logger,
            rewards_guard,
            rewards_repository.clone(),
        );

        (service, rewards_repository)
    }

    fn create_test_request<T>(payload: T) -> Request<T> {
        let mut request = Request::new(payload);
        // Add mock authentication headers
        request.metadata_mut().insert("authorization", "Bearer test-token".parse().unwrap());
        request.metadata_mut().insert("user-id", "550e8400-e29b-41d4-a716-446655440000".parse().unwrap());
        request
    }

    #[tokio::test]
    async fn test_create_reward_rule_success() {
        let (service, _) = setup_test_service().await;

        let rule = RewardRule {
            id: "".to_string(),
            name: "Test Transaction Reward".to_string(),
            description: "Earn 1 point per dollar spent".to_string(),
            r#type: 2, // SPENDING
            status: 1, // ACTIVE
            points_per_unit: "1.0".to_string(),
            minimum_amount: "10.0".to_string(),
            maximum_points: "1000.0".to_string(),
            minimum_tier: 1, // BRONZE
            categories: vec!["grocery".to_string(), "restaurant".to_string()],
            currencies: vec!["USD".to_string()],
            start_date: "".to_string(),
            end_date: "".to_string(),
            days_of_week: vec![],
            start_time: "".to_string(),
            end_time: "".to_string(),
            max_uses_per_user: -1,
            max_uses_per_day: -1,
            max_uses_per_month: -1,
            total_uses_remaining: -1,
            metadata: HashMap::new(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
            created_by: "".to_string(),
        };

        let request = create_test_request(CreateRewardRuleRequest {
            rule: Some(rule),
        });

        let response = service.create_reward_rule(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.rule.is_some());
        assert_eq!(response.rule.unwrap().name, "Test Transaction Reward");
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn test_create_reward_rule_invalid_points() {
        let (service, _) = setup_test_service().await;

        let rule = RewardRule {
            id: "".to_string(),
            name: "Invalid Rule".to_string(),
            description: "".to_string(),
            r#type: 2,
            status: 1,
            points_per_unit: "-1.0".to_string(), // Invalid negative points
            minimum_amount: "".to_string(),
            maximum_points: "".to_string(),
            minimum_tier: 1,
            categories: vec![],
            currencies: vec![],
            start_date: "".to_string(),
            end_date: "".to_string(),
            days_of_week: vec![],
            start_time: "".to_string(),
            end_time: "".to_string(),
            max_uses_per_user: -1,
            max_uses_per_day: -1,
            max_uses_per_month: -1,
            total_uses_remaining: -1,
            metadata: HashMap::new(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
            created_by: "".to_string(),
        };

        let request = create_test_request(CreateRewardRuleRequest {
            rule: Some(rule),
        });

        let response = service.create_reward_rule(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_get_user_rewards_creates_default() {
        let (service, _) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        let request = create_test_request(GetUserRewardsRequest {
            user_id: user_id.to_string(),
        });

        let response = service.get_user_rewards(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.rewards.is_some());

        let rewards = response.rewards.unwrap();
        assert_eq!(rewards.user_id, user_id.to_string());
        assert_eq!(rewards.total_points, "0");
        assert_eq!(rewards.current_tier, 1); // BRONZE
    }

    #[tokio::test]
    async fn test_award_points_success() {
        let (service, repository) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        
        // First create user rewards
        let user_rewards = UserRewards {
            user_id,
            ..Default::default()
        };
        repository.create_user_rewards(&user_rewards).await.unwrap();

        let request = create_test_request(AwardPointsRequest {
            user_id: user_id.to_string(),
            points: "100.0".to_string(),
            source_type: "transaction".to_string(),
            source_id: "tx-123".to_string(),
            description: "Test transaction reward".to_string(),
            metadata: HashMap::new(),
            expires_at: "".to_string(),
        });

        let response = service.award_points(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.transaction.is_some());
        assert!(response.updated_rewards.is_some());

        let transaction = response.transaction.unwrap();
        assert_eq!(transaction.points, "100"); // Bronze tier has 1x multiplier
        assert_eq!(transaction.source_type, "transaction");

        let rewards = response.updated_rewards.unwrap();
        assert_eq!(rewards.total_points, "100");
        assert_eq!(rewards.lifetime_earned, "100");
    }

    #[tokio::test]
    async fn test_award_points_with_tier_multiplier() {
        let (service, repository) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        
        // Create user rewards with Gold tier
        let mut user_rewards = UserRewards {
            user_id,
            current_tier: UserRewardTier::Gold,
            tier_multiplier: UserRewardTier::Gold.multiplier(),
            ..Default::default()
        };
        repository.create_user_rewards(&user_rewards).await.unwrap();

        let request = create_test_request(AwardPointsRequest {
            user_id: user_id.to_string(),
            points: "100.0".to_string(),
            source_type: "transaction".to_string(),
            source_id: "tx-123".to_string(),
            description: "Test transaction reward".to_string(),
            metadata: HashMap::new(),
            expires_at: "".to_string(),
        });

        let response = service.award_points(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        let transaction = response.transaction.unwrap();
        assert_eq!(transaction.points, "200"); // Gold tier has 2x multiplier
        assert_eq!(transaction.base_points, "100");
        assert_eq!(transaction.multiplier, "2");
    }

    #[tokio::test]
    async fn test_award_points_invalid_amount() {
        let (service, _) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        let request = create_test_request(AwardPointsRequest {
            user_id: user_id.to_string(),
            points: "-50.0".to_string(), // Invalid negative points
            source_type: "transaction".to_string(),
            source_id: "tx-123".to_string(),
            description: "Test transaction reward".to_string(),
            metadata: HashMap::new(),
            expires_at: "".to_string(),
        });

        let response = service.award_points(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_list_reward_rules() {
        let (service, repository) = setup_test_service().await;

        // Create test reward rules
        let rule1 = RewardRule {
            id: Uuid::new_v4(),
            name: "Transaction Reward".to_string(),
            description: Some("Earn points for transactions".to_string()),
            rule_type: RewardRuleType::Transaction,
            status: RewardRuleStatus::Active,
            points_per_unit: Decimal::from(1),
            minimum_amount: Some(Decimal::from(10)),
            maximum_points: Some(Decimal::from(1000)),
            minimum_tier: UserRewardTier::Bronze,
            categories: vec!["grocery".to_string()],
            currencies: vec!["USD".to_string()],
            start_date: None,
            end_date: None,
            days_of_week: vec![],
            start_time: None,
            end_time: None,
            max_uses_per_user: None,
            max_uses_per_day: None,
            max_uses_per_month: None,
            total_uses_remaining: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: Some(Uuid::new_v4()),
        };

        let rule2 = RewardRule {
            id: Uuid::new_v4(),
            name: "Referral Bonus".to_string(),
            description: Some("Earn points for referrals".to_string()),
            rule_type: RewardRuleType::Referral,
            status: RewardRuleStatus::Active,
            points_per_unit: Decimal::from(500),
            minimum_amount: None,
            maximum_points: None,
            minimum_tier: UserRewardTier::Bronze,
            categories: vec![],
            currencies: vec!["USD".to_string()],
            start_date: None,
            end_date: None,
            days_of_week: vec![],
            start_time: None,
            end_time: None,
            max_uses_per_user: Some(10),
            max_uses_per_day: None,
            max_uses_per_month: None,
            total_uses_remaining: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: Some(Uuid::new_v4()),
        };

        repository.create_reward_rule(&rule1).await.unwrap();
        repository.create_reward_rule(&rule2).await.unwrap();

        let request = create_test_request(ListRewardRulesRequest {
            r#type: 0, // All types
            status: 1, // Active only
            category: "".to_string(),
            currency: "".to_string(),
            active_only: true,
            page: 0,
            page_size: 10,
        });

        let response = service.list_reward_rules(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert_eq!(response.rules.len(), 2);
        assert_eq!(response.total_count, 2);
    }

    #[tokio::test]
    async fn test_tier_progression() {
        let user_id = Uuid::new_v4();
        
        // Test tier thresholds
        assert_eq!(UserRewardTier::Bronze.threshold(), Decimal::ZERO);
        assert_eq!(UserRewardTier::Silver.threshold(), Decimal::from(1000));
        assert_eq!(UserRewardTier::Gold.threshold(), Decimal::from(5000));
        assert_eq!(UserRewardTier::Platinum.threshold(), Decimal::from(25000));

        // Test tier multipliers
        assert_eq!(UserRewardTier::Bronze.multiplier(), Decimal::from(1));
        assert_eq!(UserRewardTier::Silver.multiplier(), Decimal::from_str_exact("1.5").unwrap());
        assert_eq!(UserRewardTier::Gold.multiplier(), Decimal::from(2));
        assert_eq!(UserRewardTier::Platinum.multiplier(), Decimal::from(3));

        // Test tier progression
        assert_eq!(UserRewardTier::Bronze.next_tier(), Some(UserRewardTier::Silver));
        assert_eq!(UserRewardTier::Silver.next_tier(), Some(UserRewardTier::Gold));
        assert_eq!(UserRewardTier::Gold.next_tier(), Some(UserRewardTier::Platinum));
        assert_eq!(UserRewardTier::Platinum.next_tier(), None);
    }

    #[tokio::test]
    async fn test_user_rewards_default() {
        let user_id = Uuid::new_v4();
        let mut rewards = UserRewards::default();
        rewards.user_id = user_id;

        assert_eq!(rewards.total_points, Decimal::ZERO);
        assert_eq!(rewards.current_tier, UserRewardTier::Bronze);
        assert_eq!(rewards.tier_multiplier, Decimal::from(1));
        assert_eq!(rewards.next_tier_threshold, Decimal::from(1000));
    }
}
