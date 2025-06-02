//! Integration tests for Fiat Gateway payment providers and compliance

use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;

use fo3_wallet_api::{
    models::fiat_gateway::{
        BankAccount, FiatTransaction, TransactionLimits, PaymentProvider, AccountType,
        TransactionType, TransactionStatus
    },
    services::payment_providers::{
        PaymentProviderTrait, AchProvider, VisaProvider, PaymentProviderError
    },
    middleware::fiat_guard::FiatGuard,
};

/// Test ACH provider functionality
#[tokio::test]
async fn test_ach_provider_withdrawal() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let bank_account = BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Ach,
        AccountType::Checking,
        "Test Account".to_string(),
        "encrypted_account".to_string(),
        "****1234".to_string(),
        Some("021000021".to_string()),
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    );

    let transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(bank_account.id),
        TransactionType::Withdrawal,
        Decimal::from(1000),
        "USD".to_string(),
        PaymentProvider::Ach,
        Some("Test withdrawal".to_string()),
    );

    // Test successful withdrawal
    let result = provider.process_withdrawal(&transaction, &bank_account).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.external_transaction_id.is_some());
    assert_eq!(response.status, "processing");
    assert!(response.metadata.contains_key("provider"));
    assert_eq!(response.metadata["provider"], "ach");

    // Test withdrawal above limit
    let large_transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(bank_account.id),
        TransactionType::Withdrawal,
        Decimal::from(150000), // Above ACH limit
        "USD".to_string(),
        PaymentProvider::Ach,
        Some("Large withdrawal".to_string()),
    );

    let result = provider.process_withdrawal(&large_transaction, &bank_account).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        PaymentProviderError::TransactionDeclined(msg) => {
            assert!(msg.contains("exceeds ACH limits"));
        }
        _ => panic!("Expected TransactionDeclined error"),
    }
}

#[tokio::test]
async fn test_ach_provider_deposit() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let bank_account = BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Ach,
        AccountType::Checking,
        "Test Account".to_string(),
        "encrypted_account".to_string(),
        "****5678".to_string(),
        Some("021000021".to_string()),
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    );

    let transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(bank_account.id),
        TransactionType::Deposit,
        Decimal::from(2000),
        "USD".to_string(),
        PaymentProvider::Ach,
        Some("Test deposit".to_string()),
    );

    let result = provider.process_deposit(&transaction, &bank_account).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.external_transaction_id.is_some());
    assert_eq!(response.status, "processing");
}

#[tokio::test]
async fn test_ach_bank_account_verification() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let bank_account = BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Ach,
        AccountType::Checking,
        "Test Account".to_string(),
        "encrypted_account".to_string(),
        "****9999".to_string(),
        Some("021000021".to_string()),
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    );

    let result = provider.verify_bank_account(&bank_account).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.verification_method, "micro_deposits");
    assert!(response.verification_data.is_some());
    
    let verification_data = response.verification_data.unwrap();
    assert!(verification_data["amounts"].is_array());
    assert_eq!(verification_data["amounts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_visa_provider_instant_processing() {
    let provider = VisaProvider::new(
        "test_visa_key".to_string(),
        "https://test.visa.com".to_string(),
        "test_visa_secret".to_string(),
    );

    let bank_account = BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Visa,
        AccountType::CreditCard,
        "Test Card".to_string(),
        "encrypted_card".to_string(),
        "****1111".to_string(),
        None,
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    );

    let transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(bank_account.id),
        TransactionType::Withdrawal,
        Decimal::from(500),
        "USD".to_string(),
        PaymentProvider::Visa,
        Some("Card withdrawal".to_string()),
    );

    let result = provider.process_withdrawal(&transaction, &bank_account).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.status, "completed"); // Visa is instant
    assert_eq!(response.metadata["processing_time"], "instant");

    // Test card limit
    let large_transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(bank_account.id),
        TransactionType::Withdrawal,
        Decimal::from(10000), // Above card limit
        "USD".to_string(),
        PaymentProvider::Visa,
        Some("Large card withdrawal".to_string()),
    );

    let result = provider.process_withdrawal(&large_transaction, &bank_account).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        PaymentProviderError::TransactionDeclined(msg) => {
            assert!(msg.contains("exceeds card limits"));
        }
        _ => panic!("Expected TransactionDeclined error"),
    }
}

#[tokio::test]
async fn test_visa_instant_verification() {
    let provider = VisaProvider::new(
        "test_visa_key".to_string(),
        "https://test.visa.com".to_string(),
        "test_visa_secret".to_string(),
    );

    let bank_account = BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Visa,
        AccountType::CreditCard,
        "Test Card".to_string(),
        "encrypted_card".to_string(),
        "****2222".to_string(),
        None,
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    );

    let result = provider.verify_bank_account(&bank_account).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.verification_method, "instant");
    assert!(response.verification_data.is_none()); // No additional data needed for instant
}

#[tokio::test]
async fn test_transaction_status_checking() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let external_id = "ACH_12345";
    let result = provider.get_transaction_status(external_id).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.external_transaction_id.unwrap(), external_id);
    assert_eq!(response.status, "completed");
}

#[tokio::test]
async fn test_transaction_cancellation() {
    let ach_provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let visa_provider = VisaProvider::new(
        "test_visa_key".to_string(),
        "https://test.visa.com".to_string(),
        "test_visa_secret".to_string(),
    );

    // ACH cancellation should work
    let result = ach_provider.cancel_transaction("ACH_12345").await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.status, "cancelled");

    // Visa cancellation should fail (cards can't be cancelled)
    let result = visa_provider.cancel_transaction("VISA_12345").await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        PaymentProviderError::InvalidRequest(msg) => {
            assert!(msg.contains("cannot be cancelled"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

#[tokio::test]
async fn test_webhook_signature_validation() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "webhook_secret".to_string(),
    );

    let payload = b"test webhook payload";
    
    // Test with correct signature
    use sha2::{Sha256, Digest};
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;
    
    let mut mac = HmacSha256::new_from_slice(b"webhook_secret").unwrap();
    mac.update(payload);
    let correct_signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
    
    let is_valid = provider.validate_webhook_signature(payload, &correct_signature, "webhook_secret");
    assert!(is_valid);

    // Test with incorrect signature
    let is_valid = provider.validate_webhook_signature(payload, "invalid_signature", "webhook_secret");
    assert!(!is_valid);
}

#[tokio::test]
async fn test_webhook_payload_parsing() {
    let provider = AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    );

    let webhook_payload = r#"{
        "event_type": "transaction.completed",
        "transaction_id": "txn_123",
        "external_id": "ACH_456",
        "status": "completed",
        "amount": "1000.00",
        "currency": "USD"
    }"#;

    let result = provider.parse_webhook_payload(webhook_payload.as_bytes());
    assert!(result.is_ok());

    let event = result.unwrap();
    assert_eq!(event.event_type, "transaction.completed");
    assert_eq!(event.transaction_id, "txn_123");
    assert_eq!(event.external_transaction_id, "ACH_456");
    assert_eq!(event.status, "completed");
    assert_eq!(event.amount, Some(Decimal::from(1000)));
    assert_eq!(event.currency, Some("USD".to_string()));
}

#[tokio::test]
async fn test_transaction_limits_validation() {
    let user_id = Uuid::new_v4();
    let mut limits = TransactionLimits::default_for_user(user_id, "USD".to_string());

    // Test valid transaction
    let result = limits.validate_transaction(
        Decimal::from(5000),
        TransactionType::Withdrawal,
        Decimal::from(2000), // Current daily usage
        Decimal::from(20000), // Current monthly usage
    );
    assert!(result.is_ok());

    // Test exceeding daily limit
    let result = limits.validate_transaction(
        Decimal::from(9000), // Would exceed daily limit (2000 + 9000 > 10000)
        TransactionType::Withdrawal,
        Decimal::from(2000),
        Decimal::from(20000),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("daily"));

    // Test exceeding monthly limit
    let result = limits.validate_transaction(
        Decimal::from(5000),
        TransactionType::Withdrawal,
        Decimal::from(1000),
        Decimal::from(96000), // Would exceed monthly limit (96000 + 5000 > 100000)
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("monthly"));

    // Test exceeding single transaction limit
    let result = limits.validate_transaction(
        Decimal::from(60000), // Exceeds single transaction limit of 50000
        TransactionType::Withdrawal,
        Decimal::from(0),
        Decimal::from(0),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("single transaction"));
}

#[tokio::test]
async fn test_transaction_approval_requirements() {
    let user_id = Uuid::new_v4();
    let limits = TransactionLimits::default_for_user(user_id, "USD".to_string());

    // Test transaction below approval threshold
    assert!(!limits.requires_approval(Decimal::from(5000)));

    // Test transaction above approval threshold
    assert!(limits.requires_approval(Decimal::from(15000)));

    // Test transaction exactly at threshold
    assert!(!limits.requires_approval(Decimal::from(10000)));
}

#[tokio::test]
async fn test_transaction_state_transitions() {
    let mut transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(Uuid::new_v4()),
        TransactionType::Withdrawal,
        Decimal::from(1000),
        "USD".to_string(),
        PaymentProvider::Ach,
        Some("Test transaction".to_string()),
    );

    // Initial state should be Pending
    assert_eq!(transaction.status, TransactionStatus::Pending);
    assert!(transaction.can_be_cancelled());
    assert!(!transaction.is_final());

    // Test approval
    transaction.approve("admin_123".to_string(), Some("Approved by admin".to_string()));
    assert_eq!(transaction.status, TransactionStatus::Approved);
    assert_eq!(transaction.approver_id, Some("admin_123".to_string()));
    assert_eq!(transaction.approval_notes, Some("Approved by admin".to_string()));

    // Test status update to completed
    transaction.update_status(TransactionStatus::Completed, None);
    assert_eq!(transaction.status, TransactionStatus::Completed);
    assert!(transaction.is_final());
    assert!(!transaction.can_be_cancelled());
    assert!(transaction.completed_at.is_some());

    // Test rejection workflow
    let mut rejected_transaction = FiatTransaction::new(
        Uuid::new_v4(),
        Some(Uuid::new_v4()),
        TransactionType::Withdrawal,
        Decimal::from(2000),
        "USD".to_string(),
        PaymentProvider::Ach,
        Some("Test rejection".to_string()),
    );

    rejected_transaction.reject(
        "admin_456".to_string(),
        "Insufficient documentation".to_string(),
        Some("Need additional KYC docs".to_string()),
    );

    assert_eq!(rejected_transaction.status, TransactionStatus::Rejected);
    assert_eq!(rejected_transaction.approver_id, Some("admin_456".to_string()));
    assert_eq!(rejected_transaction.failure_reason, Some("Insufficient documentation".to_string()));
    assert_eq!(rejected_transaction.approval_notes, Some("Need additional KYC docs".to_string()));
    assert!(rejected_transaction.is_final());
}

#[tokio::test]
async fn test_concurrent_transaction_processing() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let provider = Arc::new(AchProvider::new(
        "test_key".to_string(),
        "https://test.ach.com".to_string(),
        "test_secret".to_string(),
    ));

    let bank_account = Arc::new(BankAccount::new(
        Uuid::new_v4(),
        PaymentProvider::Ach,
        AccountType::Checking,
        "Concurrent Test Account".to_string(),
        "encrypted_account".to_string(),
        "****0000".to_string(),
        Some("021000021".to_string()),
        Some("Test Bank".to_string()),
        "USD".to_string(),
        "US".to_string(),
    ));

    let mut handles = Vec::new();

    // Process 10 transactions concurrently
    for i in 0..10 {
        let provider_clone = provider.clone();
        let account_clone = bank_account.clone();
        
        let handle = tokio::spawn(async move {
            let transaction = FiatTransaction::new(
                Uuid::new_v4(),
                Some(account_clone.id),
                TransactionType::Withdrawal,
                Decimal::from(100 + i), // Different amounts
                "USD".to_string(),
                PaymentProvider::Ach,
                Some(format!("Concurrent transaction {}", i)),
            );

            provider_clone.process_withdrawal(&transaction, &account_clone).await
        });

        handles.push(handle);
    }

    // Wait for all transactions to complete
    let results = futures::future::join_all(handles).await;

    // All transactions should succeed
    for result in results {
        let provider_result = result.unwrap();
        assert!(provider_result.is_ok());
        let response = provider_result.unwrap();
        assert!(response.success);
    }
}
