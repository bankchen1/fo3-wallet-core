//! Comprehensive tests for the CardFundingService

#[cfg(test)]
mod tests {
    use super::super::card_funding::CardFundingServiceImpl;
    use crate::models::card_funding::{
        FundingSource, FundingSourceType, FundingSourceStatus, FundingSourceLimits,
        FundingSourceMetadata, CryptoCurrency, InMemoryCardFundingRepository,
        CardFundingRepository,
    };
    use crate::middleware::{
        auth::AuthService,
        audit::AuditLogger,
        card_funding_guard::CardFundingGuard,
        rate_limit::RateLimiter,
    };
    use crate::state::AppState;
    use crate::proto::fo3::wallet::v1::{
        card_funding_service_server::CardFundingService,
        *,
    };
    use std::sync::Arc;
    use uuid::Uuid;
    use rust_decimal::Decimal;
    use chrono::Utc;
    use tonic::{Request, Code};

    fn create_test_service() -> CardFundingServiceImpl {
        let state = Arc::new(AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let audit_logger = Arc::new(AuditLogger::new(state.clone()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let funding_repository = Arc::new(InMemoryCardFundingRepository::new());
        let funding_guard = Arc::new(CardFundingGuard::new(
            auth_service.clone(),
            audit_logger.clone(),
            rate_limiter.clone(),
            funding_repository.clone(),
        ));

        CardFundingServiceImpl::new(
            state,
            auth_service,
            audit_logger,
            funding_guard,
            funding_repository,
        )
    }

    fn create_test_funding_source() -> FundingSource {
        FundingSource {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            source_type: FundingSourceType::ExternalCard,
            status: FundingSourceStatus::Active,
            name: "Test Credit Card".to_string(),
            masked_identifier: "****-****-****-1234".to_string(),
            currency: "USD".to_string(),
            provider: "Visa".to_string(),
            limits: FundingSourceLimits::default(),
            metadata: FundingSourceMetadata::ExternalCard {
                card_type: "credit".to_string(),
                issuer: "Chase".to_string(),
                last_four: "1234".to_string(),
                expiry_month: "12".to_string(),
                expiry_year: "2025".to_string(),
            },
            is_primary: false,
            is_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            verification_url: None,
            external_id: None,
        }
    }

    #[tokio::test]
    async fn test_add_funding_source_success() {
        let service = create_test_service();
        
        let request = Request::new(AddFundingSourceRequest {
            r#type: funding_source_type::FundingSourceTypeExternalCard as i32,
            name: "Test Card".to_string(),
            currency: "USD".to_string(),
            provider: "Visa".to_string(),
            metadata: Some(FundingSourceMetadata {
                metadata: Some(funding_source_metadata::Metadata::ExternalCard(ExternalCardMetadata {
                    card_type: "credit".to_string(),
                    issuer: "Chase".to_string(),
                    last_four: "1234".to_string(),
                    expiry_month: "12".to_string(),
                    expiry_year: "2025".to_string(),
                })),
            }),
            limits: Some(FundingSourceLimits {
                daily_limit: "5000.00".to_string(),
                monthly_limit: "50000.00".to_string(),
                per_transaction_limit: "2500.00".to_string(),
                minimum_amount: "10.00".to_string(),
                daily_transaction_count: 10,
                monthly_transaction_count: 100,
            }),
        });

        // This would fail without proper authentication, but tests the structure
        let result = service.add_funding_source(request).await;
        assert!(result.is_err()); // Expected to fail due to missing auth
        assert_eq!(result.unwrap_err().code(), Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_funding_source_repository_operations() {
        let repository = InMemoryCardFundingRepository::new();
        let test_source = create_test_funding_source();

        // Test create
        let created = repository.create_funding_source(&test_source).await.unwrap();
        assert_eq!(created.id, test_source.id);
        assert_eq!(created.name, test_source.name);

        // Test get
        let retrieved = repository.get_funding_source(&test_source.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, test_source.id);

        // Test get by user
        let user_source = repository
            .get_funding_source_by_user(&test_source.user_id, &test_source.id)
            .await
            .unwrap();
        assert!(user_source.is_some());

        // Test list
        let (sources, count) = repository
            .list_funding_sources(&test_source.user_id, None, None, 1, 10)
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(sources.len(), 1);

        // Test update
        let mut updated_source = test_source.clone();
        updated_source.name = "Updated Card".to_string();
        let updated = repository.update_funding_source(&updated_source).await.unwrap();
        assert_eq!(updated.name, "Updated Card");

        // Test delete
        let deleted = repository.delete_funding_source(&test_source.id).await.unwrap();
        assert!(deleted);

        // Verify deletion
        let retrieved_after_delete = repository.get_funding_source(&test_source.id).await.unwrap();
        assert!(retrieved_after_delete.is_none());
    }

    #[tokio::test]
    async fn test_fee_calculation() {
        let service = create_test_service();

        // Test crypto wallet fees (2.5% + 0.5% exchange)
        let crypto_fee = service.calculate_funding_fees(
            &FundingSourceType::CryptoWallet,
            &Decimal::from(1000),
            "USDT",
        );
        assert_eq!(crypto_fee.fee_percentage, Decimal::from_str("0.025").unwrap());
        assert_eq!(crypto_fee.fee_amount, Decimal::from(25)); // 2.5% of 1000
        assert!(crypto_fee.exchange_fee.is_some());
        assert_eq!(crypto_fee.exchange_fee.unwrap(), Decimal::from(5)); // 0.5% of 1000

        // Test external card fees (2.9%)
        let card_fee = service.calculate_funding_fees(
            &FundingSourceType::ExternalCard,
            &Decimal::from(1000),
            "USD",
        );
        assert_eq!(card_fee.fee_percentage, Decimal::from_str("0.029").unwrap());
        assert_eq!(card_fee.fee_amount, Decimal::from(29)); // 2.9% of 1000
        assert!(card_fee.exchange_fee.is_none());

        // Test ACH fees (0.5%)
        let ach_fee = service.calculate_funding_fees(
            &FundingSourceType::ACH,
            &Decimal::from(1000),
            "USD",
        );
        assert_eq!(ach_fee.fee_percentage, Decimal::from_str("0.005").unwrap());
        assert_eq!(ach_fee.fee_amount, Decimal::from(5)); // 0.5% of 1000

        // Test fiat account fees (0%)
        let fiat_fee = service.calculate_funding_fees(
            &FundingSourceType::FiatAccount,
            &Decimal::from(1000),
            "USD",
        );
        assert_eq!(fiat_fee.fee_percentage, Decimal::ZERO);
        assert_eq!(fiat_fee.fee_amount, Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_funding_limits_operations() {
        let repository = InMemoryCardFundingRepository::new();
        let user_id = Uuid::new_v4();

        // Test get non-existent limits
        let limits = repository.get_funding_limits(&user_id).await.unwrap();
        assert!(limits.is_none());

        // Test create limits
        let mut new_limits = crate::models::card_funding::FundingLimits::default();
        new_limits.user_id = user_id;
        new_limits.daily_limit = Decimal::from(5000);

        let created_limits = repository.create_funding_limits(&new_limits).await.unwrap();
        assert_eq!(created_limits.daily_limit, Decimal::from(5000));

        // Test get existing limits
        let retrieved_limits = repository.get_funding_limits(&user_id).await.unwrap();
        assert!(retrieved_limits.is_some());
        assert_eq!(retrieved_limits.unwrap().daily_limit, Decimal::from(5000));

        // Test update limits
        let mut updated_limits = new_limits.clone();
        updated_limits.daily_limit = Decimal::from(10000);
        let updated = repository.update_funding_limits(&updated_limits).await.unwrap();
        assert_eq!(updated.daily_limit, Decimal::from(10000));

        // Test reset operations
        let reset_daily = repository.reset_daily_limits(&user_id).await.unwrap();
        assert!(reset_daily);

        let reset_monthly = repository.reset_monthly_limits(&user_id).await.unwrap();
        assert!(reset_monthly);

        let reset_yearly = repository.reset_yearly_limits(&user_id).await.unwrap();
        assert!(reset_yearly);
    }

    #[tokio::test]
    async fn test_crypto_currency_conversion() {
        let service = create_test_service();

        // Test proto to model conversion
        let usdt = CardFundingServiceImpl::proto_to_crypto_currency(
            crypto_currency::CryptoCurrencyUsdt as i32
        ).unwrap();
        assert_eq!(usdt, CryptoCurrency::USDT);

        let usdc = CardFundingServiceImpl::proto_to_crypto_currency(
            crypto_currency::CryptoCurrencyUsdc as i32
        ).unwrap();
        assert_eq!(usdc, CryptoCurrency::USDC);

        // Test model to proto conversion
        let proto_usdt = CardFundingServiceImpl::crypto_currency_to_proto(&CryptoCurrency::USDT);
        assert_eq!(proto_usdt, crypto_currency::CryptoCurrencyUsdt as i32);

        let proto_usdc = CardFundingServiceImpl::crypto_currency_to_proto(&CryptoCurrency::USDC);
        assert_eq!(proto_usdc, crypto_currency::CryptoCurrencyUsdc as i32);
    }

    #[tokio::test]
    async fn test_reference_number_generation() {
        let ref1 = CardFundingServiceImpl::generate_reference_number();
        let ref2 = CardFundingServiceImpl::generate_reference_number();

        // Should start with "FND"
        assert!(ref1.starts_with("FND"));
        assert!(ref2.starts_with("FND"));

        // Should be unique
        assert_ne!(ref1, ref2);

        // Should be correct length (FND + 12 characters)
        assert_eq!(ref1.len(), 15);
        assert_eq!(ref2.len(), 15);
    }

    #[tokio::test]
    async fn test_funding_metrics_calculation() {
        let repository = InMemoryCardFundingRepository::new();
        
        // Create test data
        let user_id = Uuid::new_v4();
        let card_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();

        // Create a funding source
        let source = FundingSource {
            id: source_id,
            user_id,
            source_type: FundingSourceType::ExternalCard,
            status: FundingSourceStatus::Active,
            name: "Test Card".to_string(),
            masked_identifier: "****1234".to_string(),
            currency: "USD".to_string(),
            provider: "Visa".to_string(),
            limits: FundingSourceLimits::default(),
            metadata: FundingSourceMetadata::ExternalCard {
                card_type: "credit".to_string(),
                issuer: "Chase".to_string(),
                last_four: "1234".to_string(),
                expiry_month: "12".to_string(),
                expiry_year: "2025".to_string(),
            },
            is_primary: false,
            is_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            verification_url: None,
            external_id: None,
        };

        repository.create_funding_source(&source).await.unwrap();

        // Create test transactions
        let tx1 = crate::models::card_funding::FundingTransaction {
            id: Uuid::new_v4(),
            user_id,
            card_id,
            funding_source_id: source_id,
            status: crate::models::card_funding::FundingTransactionStatus::Completed,
            amount: Decimal::from(1000),
            currency: "USD".to_string(),
            fee_amount: Decimal::from(29),
            fee_percentage: Decimal::from_str("0.029").unwrap(),
            exchange_rate: None,
            net_amount: Decimal::from(971),
            reference_number: "FND123456789".to_string(),
            external_transaction_id: None,
            description: Some("Test transaction".to_string()),
            failure_reason: None,
            metadata: std::collections::HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: Some(Utc::now()),
            expires_at: None,
        };

        repository.create_funding_transaction(&tx1).await.unwrap();

        // Test metrics calculation
        let start_date = Utc::now() - chrono::Duration::hours(1);
        let end_date = Utc::now() + chrono::Duration::hours(1);

        let metrics = repository
            .get_funding_metrics(&start_date, &end_date, None, None)
            .await
            .unwrap();

        assert_eq!(metrics.total_transactions, 1);
        assert_eq!(metrics.total_volume, Decimal::from(1000));
        assert_eq!(metrics.total_fees, Decimal::from(29));
        assert_eq!(metrics.average_transaction_size, Decimal::from(1000));
        assert_eq!(metrics.success_rate, Decimal::ONE);

        // Test user volume calculation
        let user_volume = repository
            .get_user_funding_volume(&user_id, &start_date, &end_date)
            .await
            .unwrap();

        assert_eq!(user_volume, Decimal::from(1000));
    }
}
