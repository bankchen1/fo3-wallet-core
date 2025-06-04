//! Comprehensive tests for the Referral service

#[cfg(test)]
mod tests {
    use super::super::referral::ReferralServiceImpl;
    use crate::models::referral::{
        ReferralCode, ReferralCampaign, ReferralRelationship, ReferralBonus,
        ReferralCodeStatus, ReferralRelationshipStatus, ReferralCampaignType, ReferralCampaignStatus,
        ReferralBonusType, ReferralBonusStatus, InMemoryReferralRepository, ReferralRepository,
    };
    use crate::middleware::{
        auth::AuthService,
        audit::AuditLogger,
        referral_guard::ReferralGuard,
        rate_limit::RateLimiter,
    };
    use crate::state::AppState;
    use crate::proto::fo3::wallet::v1::{
        referral_service_server::ReferralService,
        *,
    };
    use std::sync::Arc;
    use std::collections::HashMap;
    use uuid::Uuid;
    use rust_decimal::Decimal;
    use chrono::Utc;
    use tonic::{Request, Code};

    async fn setup_test_service() -> (ReferralServiceImpl, Arc<dyn ReferralRepository>) {
        let state = Arc::new(AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let audit_logger = Arc::new(AuditLogger::new(state.clone()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let referral_repository = Arc::new(InMemoryReferralRepository::default());
        let referral_guard = Arc::new(ReferralGuard::new(
            auth_service.clone(),
            audit_logger.clone(),
            rate_limiter.clone(),
            referral_repository.clone(),
        ));

        let service = ReferralServiceImpl::new(
            state,
            auth_service,
            audit_logger,
            referral_guard,
            referral_repository.clone(),
        );

        (service, referral_repository)
    }

    fn create_test_request<T>(payload: T) -> Request<T> {
        let mut request = Request::new(payload);
        // Add mock authentication headers
        request.metadata_mut().insert("authorization", "Bearer test-token".parse().unwrap());
        request.metadata_mut().insert("user-id", "550e8400-e29b-41d4-a716-446655440000".parse().unwrap());
        request
    }

    #[tokio::test]
    async fn test_generate_referral_code_success() {
        let (service, _) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        let request = create_test_request(GenerateReferralCodeRequest {
            user_id: user_id.to_string(),
            campaign_id: "".to_string(),
            custom_code: "".to_string(),
            description: "Test referral code".to_string(),
            max_uses: 10,
            expires_at: "".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.generate_referral_code(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.referral_code.is_some());
        
        let code = response.referral_code.unwrap();
        assert_eq!(code.user_id, user_id.to_string());
        assert!(code.code.starts_with("FO3-"));
        assert_eq!(code.max_uses, 10);
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn test_generate_custom_referral_code() {
        let (service, _) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        let custom_code = "MYCUSTOMCODE";
        
        let request = create_test_request(GenerateReferralCodeRequest {
            user_id: user_id.to_string(),
            campaign_id: "".to_string(),
            custom_code: custom_code.to_string(),
            description: "Custom referral code".to_string(),
            max_uses: -1, // Unlimited
            expires_at: "".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.generate_referral_code(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        let code = response.referral_code.unwrap();
        assert_eq!(code.code, custom_code);
        assert_eq!(code.max_uses, -1);
        assert!(code.is_custom);
    }

    #[tokio::test]
    async fn test_validate_referral_code_success() {
        let (service, repository) = setup_test_service().await;

        let referrer_id = Uuid::new_v4();
        let referee_id = Uuid::new_v4();
        
        // Create a referral code
        let referral_code = ReferralCode {
            id: Uuid::new_v4(),
            user_id: referrer_id,
            code: "TESTCODE123".to_string(),
            status: ReferralCodeStatus::Active,
            campaign_id: None,
            description: Some("Test code".to_string()),
            is_custom: true,
            max_uses: Some(10),
            current_uses: 0,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        repository.create_referral_code(&referral_code).await.unwrap();

        let request = create_test_request(ValidateReferralCodeRequest {
            code: "TESTCODE123".to_string(),
            user_id: referee_id.to_string(),
            campaign_id: "".to_string(),
        });

        let response = service.validate_referral_code(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.is_valid);
        assert!(response.referral_code.is_some());
        assert!(response.validation_errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_referral_code_self_referral() {
        let (service, repository) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        
        // Create a referral code
        let referral_code = ReferralCode {
            id: Uuid::new_v4(),
            user_id,
            code: "SELFREF123".to_string(),
            status: ReferralCodeStatus::Active,
            campaign_id: None,
            description: Some("Test code".to_string()),
            is_custom: true,
            max_uses: Some(10),
            current_uses: 0,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        repository.create_referral_code(&referral_code).await.unwrap();

        let request = create_test_request(ValidateReferralCodeRequest {
            code: "SELFREF123".to_string(),
            user_id: user_id.to_string(), // Same user trying to use their own code
            campaign_id: "".to_string(),
        });

        let response = service.validate_referral_code(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(!response.is_valid);
        assert!(response.validation_errors.contains(&"Cannot use your own referral code".to_string()));
    }

    #[tokio::test]
    async fn test_create_referral_relationship_success() {
        let (service, repository) = setup_test_service().await;

        let referrer_id = Uuid::new_v4();
        let referee_id = Uuid::new_v4();
        
        // Create a referral code
        let referral_code = ReferralCode {
            id: Uuid::new_v4(),
            user_id: referrer_id,
            code: "REFCODE123".to_string(),
            status: ReferralCodeStatus::Active,
            campaign_id: None,
            description: Some("Test code".to_string()),
            is_custom: true,
            max_uses: Some(10),
            current_uses: 0,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        repository.create_referral_code(&referral_code).await.unwrap();

        let request = create_test_request(CreateReferralRelationshipRequest {
            referrer_user_id: referrer_id.to_string(),
            referee_user_id: referee_id.to_string(),
            referral_code: "REFCODE123".to_string(),
            campaign_id: "".to_string(),
            referral_source: "web".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.create_referral_relationship(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.relationship.is_some());
        
        let relationship = response.relationship.unwrap();
        assert_eq!(relationship.referrer_user_id, referrer_id.to_string());
        assert_eq!(relationship.referee_user_id, referee_id.to_string());
        assert_eq!(relationship.referral_level, 1);
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn test_create_referral_relationship_self_referral() {
        let (service, repository) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        
        // Create a referral code
        let referral_code = ReferralCode {
            id: Uuid::new_v4(),
            user_id,
            code: "SELFREF456".to_string(),
            status: ReferralCodeStatus::Active,
            campaign_id: None,
            description: Some("Test code".to_string()),
            is_custom: true,
            max_uses: Some(10),
            current_uses: 0,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        repository.create_referral_code(&referral_code).await.unwrap();

        let request = create_test_request(CreateReferralRelationshipRequest {
            referrer_user_id: user_id.to_string(),
            referee_user_id: user_id.to_string(), // Self-referral
            referral_code: "SELFREF456".to_string(),
            campaign_id: "".to_string(),
            referral_source: "web".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.create_referral_relationship(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_list_user_referral_codes() {
        let (service, repository) = setup_test_service().await;

        let user_id = Uuid::new_v4();
        
        // Create multiple referral codes
        for i in 0..3 {
            let referral_code = ReferralCode {
                id: Uuid::new_v4(),
                user_id,
                code: format!("CODE{}", i),
                status: if i == 2 { ReferralCodeStatus::Inactive } else { ReferralCodeStatus::Active },
                campaign_id: None,
                description: Some(format!("Test code {}", i)),
                is_custom: true,
                max_uses: Some(10),
                current_uses: i,
                successful_referrals: 0,
                pending_referrals: 0,
                expires_at: None,
                last_used_at: None,
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            
            repository.create_referral_code(&referral_code).await.unwrap();
        }

        let request = create_test_request(ListUserReferralCodesRequest {
            user_id: user_id.to_string(),
            status: 1, // Active only
            campaign_id: "".to_string(),
            page: 0,
            page_size: 10,
        });

        let response = service.list_user_referral_codes(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert_eq!(response.referral_codes.len(), 2); // Only active codes
        assert_eq!(response.total_count, 2);
    }

    #[tokio::test]
    async fn test_referral_code_generation_patterns() {
        let user_id = Uuid::new_v4();
        
        // Test auto-generated code format
        let auto_code = ReferralCode::generate_code(&user_id, None);
        assert!(auto_code.starts_with("FO3-"));
        assert_eq!(auto_code.len(), 17); // FO3- + 8 chars + - + 4 chars
        
        // Test custom code
        let custom_code = ReferralCode::generate_code(&user_id, Some("MYCUSTOM".to_string()));
        assert_eq!(custom_code, "MYCUSTOM");
    }

    #[tokio::test]
    async fn test_referral_code_validation() {
        let user_id = Uuid::new_v4();
        
        // Test valid active code
        let mut code = ReferralCode {
            id: Uuid::new_v4(),
            user_id,
            code: "TESTCODE".to_string(),
            status: ReferralCodeStatus::Active,
            campaign_id: None,
            description: None,
            is_custom: false,
            max_uses: Some(10),
            current_uses: 5,
            successful_referrals: 0,
            pending_referrals: 0,
            expires_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        assert!(code.is_valid());
        
        // Test inactive code
        code.status = ReferralCodeStatus::Inactive;
        assert!(!code.is_valid());
        
        // Test exhausted code
        code.status = ReferralCodeStatus::Active;
        code.current_uses = 10; // Reached max uses
        assert!(!code.is_valid());
        
        // Test expired code
        code.current_uses = 5;
        code.expires_at = Some(Utc::now() - chrono::Duration::hours(1)); // Expired
        assert!(!code.is_valid());
    }
}
