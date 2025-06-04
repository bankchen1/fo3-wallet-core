//! Data models for the FO3 Wallet API

pub mod kyc;
pub mod pii_protection;
pub mod fiat_gateway;
pub mod pricing;
pub mod notifications;
pub mod cards;
pub mod spending_insights;
pub mod card_funding;
pub mod card_funding_impl;
pub mod ledger;
pub mod ledger_repository;
pub mod ledger_impl;
pub mod ledger_impl_remaining;
pub mod rewards;
pub mod rewards_impl;
pub mod referral;
pub mod referral_impl;
pub mod wallet_connect;
pub mod dapp_signing;
pub mod user_context;
pub mod earn;
pub mod moonshot;

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
pub use card_funding::{
    FundingSourceType, FundingSourceStatus, FundingTransactionStatus, CryptoCurrency,
    FundingSourceLimits, FundingSourceMetadata, FundingSource, FundingTransaction,
    FeeCalculation, FeeBreakdown, FundingLimits, CryptoFundingDetails,
    FundingMetrics, FundingSourceMetrics, CurrencyMetrics, CardFundingRepository,
    InMemoryCardFundingRepository
};
pub use rewards::{
    RewardRuleType, RewardRuleStatus, UserRewardTier, RewardTransactionType, RewardTransactionStatus,
    RedemptionType, RedemptionStatus, RewardRule, UserRewards, RewardTransaction, Redemption,
    RedemptionOption, RewardMetrics, CategoryMetrics, RedemptionMetrics, RewardAuditTrailEntry,
    RewardsRepository, InMemoryRewardsRepository
};
pub use referral::{
    ReferralCodeStatus, ReferralRelationshipStatus, ReferralCampaignType, ReferralCampaignStatus,
    ReferralBonusType, ReferralBonusStatus, ReferralCode, ReferralCampaign, ReferralRelationship,
    ReferralBonus, ReferralMetrics, TopReferrer, CampaignMetrics, ReferralTreeNode,
    ReferralAuditTrailEntry, ReferralRepository, InMemoryReferralRepository
};
pub use ledger::{
    AccountType, AccountStatus, TransactionStatus, JournalEntryStatus, EntryType, ReportType,
    LedgerAccount, LedgerTransaction, JournalEntry, AccountBalance, TrialBalanceEntry,
    BalanceSheetItem, BalanceSheetSection, FinancialReport, AuditTrailEntry,
    AccountReconciliation, ValidationIssue, LedgerMetrics, AccountBalanceSnapshot,
    LedgerRepository, InMemoryLedgerRepository
};
pub use wallet_connect::{
    SessionStatus, RequestType, RequestStatus, KeyType as WalletConnectKeyType,
    WalletConnectSession, DAppInfo, SessionRequest, SessionAnalytics,
    WalletConnectRepository, InMemoryWalletConnectRepository
};
pub use dapp_signing::{
    SignatureType, TransactionType as DAppTransactionType, ValidationStatus, RiskLevel,
    KeyType as DAppKeyType, SignatureResult, SimulationResult, ValidationResult,
    SigningHistoryEntry, SigningAnalytics, DAppSigningRepository, InMemoryDAppSigningRepository
};
pub use earn::{
    YieldProductType, ProtocolType, RiskLevel as EarnRiskLevel, PositionStatus,
    KeyType as EarnKeyType, YieldProduct, YieldCalculation, YieldBreakdown,
    StakingPosition, LendingPosition, VaultPosition, EarnAnalytics, PortfolioSummary,
    PositionSummary, YieldChartData, YieldDataPoint, RiskAssessment, RiskFactor,
    PortfolioOptimization, OptimizationSuggestion, EarnRepository, InMemoryEarnRepository
};
pub use moonshot::{
    TokenStatus, VoteType, ProposalStatus, TokenEntity, TokenMetrics, VoteEntity,
    ProposalEntity, SentimentData, SentimentSource, PredictionData,
    MoonshotRepository, InMemoryMoonshotRepository
};
pub use user_context::{
    UserContext, UserRole, UserTier, Permission, UserLimits
};
