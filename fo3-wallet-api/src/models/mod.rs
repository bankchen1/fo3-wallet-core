//! Data models for the FO3 Wallet API

pub mod kyc;
pub mod pii_protection;
pub mod fiat_gateway;
pub mod pricing;
pub mod notifications;
pub mod cards;
pub mod spending_insights;

pub use kyc::{
    KycStatus, DocumentType, PersonalInfo, Address, Document, KycSubmission, KycRepository
};
pub use pii_protection::{
    DataRetentionPolicy, PiiClassification, DataSubjectRight, DataProcessingRequest,
    ProcessingStatus, PiiAnonymizer, DataRetentionManager, ComplianceAuditEntry, ComplianceManager
};
pub use fiat_gateway::{
    PaymentProvider, AccountType, TransactionStatus, TransactionType, BankAccount,
    FiatTransaction, TransactionLimits, FiatGatewayRepository
};
pub use pricing::{
    Asset, Price, PricePoint, FiatRate, PricingMetrics, AssetType, PriceSource, TimeInterval,
    PricingRepository, InMemoryPricingRepository
};
pub use notifications::{
    Notification, NotificationPreferences, PriceAlert, NotificationMetrics,
    NotificationType, NotificationPriority, DeliveryChannel, PriceAlertCondition,
    NotificationRepository, InMemoryNotificationRepository, NotificationEventData
};
pub use cards::{
    Card, CardTransaction, CardLimits, CardStatus, CardType, CardTransactionStatus,
    CardTransactionType, MerchantCategory, MerchantInfo, CardMetrics,
    CardRepository, InMemoryCardRepository
};
pub use spending_insights::{
    Budget, SpendingAlert, CategorySpending, SpendingDataPoint, MerchantSpending,
    LocationInsight, SpendingPattern, CashflowAnalysis, PlatformInsights,
    TimePeriod, BudgetStatus, AlertType, SpendingInsightsRepository,
    InMemorySpendingInsightsRepository
};
