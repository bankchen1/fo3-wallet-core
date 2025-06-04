//! Card funding service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    card_funding_service_server::CardFundingService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    card_funding_guard::CardFundingGuard,
};
use crate::models::card_funding::{
    FundingSource, FundingTransaction, FundingLimits, FeeCalculation, FeeBreakdown,
    CryptoFundingDetails, FundingSourceType, FundingSourceStatus, FundingTransactionStatus,
    CryptoCurrency, FundingSourceLimits, FundingSourceMetadata, CardFundingRepository,
    FundingMetrics, FundingSourceMetrics, CurrencyMetrics,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Card funding service implementation
#[derive(Debug)]
pub struct CardFundingServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    funding_guard: Arc<CardFundingGuard>,
    funding_repository: Arc<dyn CardFundingRepository>,
}

impl CardFundingServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        funding_guard: Arc<CardFundingGuard>,
        funding_repository: Arc<dyn CardFundingRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            funding_guard,
            funding_repository,
        }
    }

    /// Calculate funding fees based on source type and amount
    fn calculate_funding_fees(
        &self,
        source_type: &FundingSourceType,
        amount: &Decimal,
        currency: &str,
    ) -> FeeCalculation {
        let fee_percentage = match source_type {
            FundingSourceType::CryptoWallet => Decimal::from_str("0.025").unwrap(), // 2.5% for crypto
            FundingSourceType::ExternalCard => Decimal::from_str("0.029").unwrap(), // 2.9% for cards
            FundingSourceType::ACH => Decimal::from_str("0.005").unwrap(), // 0.5% for ACH
            FundingSourceType::BankAccount => Decimal::from_str("0.001").unwrap(), // 0.1% for bank
            FundingSourceType::FiatAccount => Decimal::ZERO, // Free for existing fiat accounts
        };

        let fee_amount = amount * fee_percentage;
        let net_amount = amount - fee_amount;

        let mut fee_breakdown = vec![
            FeeBreakdown {
                fee_type: "base_fee".to_string(),
                amount: fee_amount,
                description: format!("{}% funding fee", fee_percentage * Decimal::from(100)),
            }
        ];

        // Add exchange fee for crypto
        let (exchange_fee, total_fee) = if matches!(source_type, FundingSourceType::CryptoWallet) {
            let exchange_fee = amount * Decimal::from_str("0.005").unwrap(); // 0.5% exchange fee
            fee_breakdown.push(FeeBreakdown {
                fee_type: "exchange_fee".to_string(),
                amount: exchange_fee,
                description: "Cryptocurrency exchange fee".to_string(),
            });
            (Some(exchange_fee), fee_amount + exchange_fee)
        } else {
            (None, fee_amount)
        };

        FeeCalculation {
            base_amount: *amount,
            fee_percentage,
            fee_amount,
            net_amount: amount - total_fee,
            exchange_rate: None, // Will be set for crypto transactions
            exchange_fee,
            total_fee,
            fee_breakdown,
        }
    }

    /// Generate unique reference number for funding transactions
    fn generate_reference_number() -> String {
        format!("FND{}", Uuid::new_v4().to_string().replace('-', "").to_uppercase()[..12].to_string())
    }

    /// Convert proto funding source type to model
    fn proto_to_funding_source_type(proto_type: i32) -> Result<FundingSourceType, Status> {
        match FundingSourceType::try_from(proto_type) {
            Ok(funding_source_type::FundingSourceTypeBankAccount) => Ok(FundingSourceType::BankAccount),
            Ok(funding_source_type::FundingSourceTypeCryptoWallet) => Ok(FundingSourceType::CryptoWallet),
            Ok(funding_source_type::FundingSourceTypeAch) => Ok(FundingSourceType::ACH),
            Ok(funding_source_type::FundingSourceTypeExternalCard) => Ok(FundingSourceType::ExternalCard),
            Ok(funding_source_type::FundingSourceTypeFiatAccount) => Ok(FundingSourceType::FiatAccount),
            _ => Err(Status::invalid_argument("Invalid funding source type")),
        }
    }

    /// Convert model funding source type to proto
    fn funding_source_type_to_proto(source_type: &FundingSourceType) -> i32 {
        match source_type {
            FundingSourceType::BankAccount => funding_source_type::FundingSourceTypeBankAccount as i32,
            FundingSourceType::CryptoWallet => funding_source_type::FundingSourceTypeCryptoWallet as i32,
            FundingSourceType::ACH => funding_source_type::FundingSourceTypeAch as i32,
            FundingSourceType::ExternalCard => funding_source_type::FundingSourceTypeExternalCard as i32,
            FundingSourceType::FiatAccount => funding_source_type::FundingSourceTypeFiatAccount as i32,
        }
    }

    /// Convert proto funding source status to model
    fn proto_to_funding_source_status(proto_status: i32) -> Result<FundingSourceStatus, Status> {
        match FundingSourceStatus::try_from(proto_status) {
            Ok(funding_source_status::FundingSourceStatusPending) => Ok(FundingSourceStatus::Pending),
            Ok(funding_source_status::FundingSourceStatusActive) => Ok(FundingSourceStatus::Active),
            Ok(funding_source_status::FundingSourceStatusSuspended) => Ok(FundingSourceStatus::Suspended),
            Ok(funding_source_status::FundingSourceStatusExpired) => Ok(FundingSourceStatus::Expired),
            Ok(funding_source_status::FundingSourceStatusRemoved) => Ok(FundingSourceStatus::Removed),
            _ => Err(Status::invalid_argument("Invalid funding source status")),
        }
    }

    /// Convert model funding source status to proto
    fn funding_source_status_to_proto(status: &FundingSourceStatus) -> i32 {
        match status {
            FundingSourceStatus::Pending => funding_source_status::FundingSourceStatusPending as i32,
            FundingSourceStatus::Active => funding_source_status::FundingSourceStatusActive as i32,
            FundingSourceStatus::Suspended => funding_source_status::FundingSourceStatusSuspended as i32,
            FundingSourceStatus::Expired => funding_source_status::FundingSourceStatusExpired as i32,
            FundingSourceStatus::Removed => funding_source_status::FundingSourceStatusRemoved as i32,
        }
    }

    /// Convert proto funding transaction status to model
    fn proto_to_funding_transaction_status(proto_status: i32) -> Result<FundingTransactionStatus, Status> {
        match FundingTransactionStatus::try_from(proto_status) {
            Ok(funding_transaction_status::FundingTransactionStatusPending) => Ok(FundingTransactionStatus::Pending),
            Ok(funding_transaction_status::FundingTransactionStatusProcessing) => Ok(FundingTransactionStatus::Processing),
            Ok(funding_transaction_status::FundingTransactionStatusCompleted) => Ok(FundingTransactionStatus::Completed),
            Ok(funding_transaction_status::FundingTransactionStatusFailed) => Ok(FundingTransactionStatus::Failed),
            Ok(funding_transaction_status::FundingTransactionStatusCancelled) => Ok(FundingTransactionStatus::Cancelled),
            Ok(funding_transaction_status::FundingTransactionStatusRefunded) => Ok(FundingTransactionStatus::Refunded),
            _ => Err(Status::invalid_argument("Invalid funding transaction status")),
        }
    }

    /// Convert model funding transaction status to proto
    fn funding_transaction_status_to_proto(status: &FundingTransactionStatus) -> i32 {
        match status {
            FundingTransactionStatus::Pending => funding_transaction_status::FundingTransactionStatusPending as i32,
            FundingTransactionStatus::Processing => funding_transaction_status::FundingTransactionStatusProcessing as i32,
            FundingTransactionStatus::Completed => funding_transaction_status::FundingTransactionStatusCompleted as i32,
            FundingTransactionStatus::Failed => funding_transaction_status::FundingTransactionStatusFailed as i32,
            FundingTransactionStatus::Cancelled => funding_transaction_status::FundingTransactionStatusCancelled as i32,
            FundingTransactionStatus::Refunded => funding_transaction_status::FundingTransactionStatusRefunded as i32,
        }
    }

    /// Convert proto crypto currency to model
    fn proto_to_crypto_currency(proto_currency: i32) -> Result<CryptoCurrency, Status> {
        match CryptoCurrency::try_from(proto_currency) {
            Ok(crypto_currency::CryptoCurrencyUsdt) => Ok(CryptoCurrency::USDT),
            Ok(crypto_currency::CryptoCurrencyUsdc) => Ok(CryptoCurrency::USDC),
            Ok(crypto_currency::CryptoCurrencyDai) => Ok(CryptoCurrency::DAI),
            Ok(crypto_currency::CryptoCurrencyBusd) => Ok(CryptoCurrency::BUSD),
            _ => Err(Status::invalid_argument("Invalid crypto currency")),
        }
    }

    /// Convert model crypto currency to proto
    fn crypto_currency_to_proto(currency: &CryptoCurrency) -> i32 {
        match currency {
            CryptoCurrency::USDT => crypto_currency::CryptoCurrencyUsdt as i32,
            CryptoCurrency::USDC => crypto_currency::CryptoCurrencyUsdc as i32,
            CryptoCurrency::DAI => crypto_currency::CryptoCurrencyDai as i32,
            CryptoCurrency::BUSD => crypto_currency::CryptoCurrencyBusd as i32,
        }
    }

    /// Convert model funding source to proto
    fn funding_source_to_proto(&self, source: &FundingSource) -> crate::proto::fo3::wallet::v1::FundingSource {
        crate::proto::fo3::wallet::v1::FundingSource {
            id: source.id.to_string(),
            user_id: source.user_id.to_string(),
            r#type: Self::funding_source_type_to_proto(&source.source_type),
            status: Self::funding_source_status_to_proto(&source.status),
            name: source.name.clone(),
            masked_identifier: source.masked_identifier.clone(),
            currency: source.currency.clone(),
            provider: source.provider.clone(),
            limits: Some(self.funding_source_limits_to_proto(&source.limits)),
            metadata: Some(self.funding_source_metadata_to_proto(&source.metadata)),
            is_primary: source.is_primary,
            is_verified: source.is_verified,
            created_at: source.created_at.to_rfc3339(),
            updated_at: source.updated_at.to_rfc3339(),
            expires_at: source.expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }

    /// Convert model funding source limits to proto
    fn funding_source_limits_to_proto(&self, limits: &FundingSourceLimits) -> FundingSourceLimits {
        FundingSourceLimits {
            daily_limit: limits.daily_limit.to_string(),
            monthly_limit: limits.monthly_limit.to_string(),
            per_transaction_limit: limits.per_transaction_limit.to_string(),
            minimum_amount: limits.minimum_amount.to_string(),
            daily_transaction_count: limits.daily_transaction_count,
            monthly_transaction_count: limits.monthly_transaction_count,
        }
    }

    /// Convert model funding source metadata to proto
    fn funding_source_metadata_to_proto(&self, metadata: &FundingSourceMetadata) -> FundingSourceMetadata {
        match metadata {
            FundingSourceMetadata::BankAccount { account_type, routing_number, bank_name } => {
                FundingSourceMetadata {
                    metadata: Some(funding_source_metadata::Metadata::BankAccount(BankAccountMetadata {
                        account_type: account_type.clone(),
                        routing_number: routing_number.clone(),
                        bank_name: bank_name.clone(),
                    })),
                }
            }
            FundingSourceMetadata::CryptoWallet { currency, network, wallet_address, exchange_name } => {
                FundingSourceMetadata {
                    metadata: Some(funding_source_metadata::Metadata::CryptoWallet(CryptoWalletMetadata {
                        currency: Self::crypto_currency_to_proto(currency),
                        network: network.clone(),
                        wallet_address: wallet_address.clone(),
                        exchange_name: exchange_name.clone().unwrap_or_default(),
                    })),
                }
            }
            FundingSourceMetadata::ExternalCard { card_type, issuer, last_four, expiry_month, expiry_year } => {
                FundingSourceMetadata {
                    metadata: Some(funding_source_metadata::Metadata::ExternalCard(ExternalCardMetadata {
                        card_type: card_type.clone(),
                        issuer: issuer.clone(),
                        last_four: last_four.clone(),
                        expiry_month: expiry_month.clone(),
                        expiry_year: expiry_year.clone(),
                    })),
                }
            }
            FundingSourceMetadata::ACH { ach_type, processor } => {
                FundingSourceMetadata {
                    metadata: Some(funding_source_metadata::Metadata::Ach(AchMetadata {
                        ach_type: ach_type.clone(),
                        processor: processor.clone(),
                    })),
                }
            }
            FundingSourceMetadata::FiatAccount { account_id, account_type } => {
                // For fiat accounts, we'll use bank account metadata structure
                FundingSourceMetadata {
                    metadata: Some(funding_source_metadata::Metadata::BankAccount(BankAccountMetadata {
                        account_type: account_type.clone(),
                        routing_number: account_id.to_string(),
                        bank_name: "FO3 Wallet".to_string(),
                    })),
                }
            }
        }
    }

    /// Convert model funding transaction to proto
    fn funding_transaction_to_proto(&self, transaction: &FundingTransaction) -> crate::proto::fo3::wallet::v1::FundingTransaction {
        crate::proto::fo3::wallet::v1::FundingTransaction {
            id: transaction.id.to_string(),
            user_id: transaction.user_id.to_string(),
            card_id: transaction.card_id.to_string(),
            funding_source_id: transaction.funding_source_id.to_string(),
            status: Self::funding_transaction_status_to_proto(&transaction.status),
            amount: transaction.amount.to_string(),
            currency: transaction.currency.clone(),
            fee_amount: transaction.fee_amount.to_string(),
            fee_percentage: transaction.fee_percentage.to_string(),
            exchange_rate: transaction.exchange_rate.map(|r| r.to_string()).unwrap_or_default(),
            net_amount: transaction.net_amount.to_string(),
            reference_number: transaction.reference_number.clone(),
            external_transaction_id: transaction.external_transaction_id.clone().unwrap_or_default(),
            description: transaction.description.clone().unwrap_or_default(),
            created_at: transaction.created_at.to_rfc3339(),
            completed_at: transaction.completed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            failure_reason: transaction.failure_reason.clone().unwrap_or_default(),
            metadata: transaction.metadata.clone(),
        }
    }

    /// Convert model fee calculation to proto
    fn fee_calculation_to_proto(&self, fee_calc: &FeeCalculation) -> crate::proto::fo3::wallet::v1::FeeCalculation {
        crate::proto::fo3::wallet::v1::FeeCalculation {
            base_amount: fee_calc.base_amount.to_string(),
            fee_percentage: fee_calc.fee_percentage.to_string(),
            fee_amount: fee_calc.fee_amount.to_string(),
            net_amount: fee_calc.net_amount.to_string(),
            exchange_rate: fee_calc.exchange_rate.map(|r| r.to_string()).unwrap_or_default(),
            exchange_fee: fee_calc.exchange_fee.map(|f| f.to_string()).unwrap_or_default(),
            total_fee: fee_calc.total_fee.to_string(),
            fee_breakdown: fee_calc.fee_breakdown.iter().map(|fb| crate::proto::fo3::wallet::v1::FeeBreakdown {
                fee_type: fb.fee_type.clone(),
                amount: fb.amount.to_string(),
                description: fb.description.clone(),
            }).collect(),
        }
    }

    /// Send funding notification
    async fn send_funding_notification(
        &self,
        user_id: &Uuid,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        metadata: HashMap<String, String>,
    ) {
        if let Some(notification_service) = &self.state.notification_service {
            let _ = notification_service.send_notification(
                user_id,
                notification_type,
                NotificationPriority::High,
                title,
                message,
                Some(metadata),
                vec![DeliveryChannel::Push, DeliveryChannel::Email],
            ).await;
        }
    }
}

#[tonic::async_trait]
impl CardFundingService for CardFundingServiceImpl {
    /// Add a new funding source
    async fn add_funding_source(
        &self,
        request: Request<AddFundingSourceRequest>,
    ) -> Result<Response<AddFundingSourceResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate request
        let source_type = Self::proto_to_funding_source_type(req.r#type)?;
        let amount_limit = Decimal::from_str(&req.limits.as_ref()
            .ok_or_else(|| Status::invalid_argument("Funding source limits required"))?
            .per_transaction_limit)
            .map_err(|_| Status::invalid_argument("Invalid per transaction limit"))?;

        // Validate request with security guard
        let auth_context = self.funding_guard
            .validate_funding_source_creation(&request, &source_type, &amount_limit)
            .await?;

        // Parse metadata based on source type
        let metadata = req.metadata.as_ref()
            .ok_or_else(|| Status::invalid_argument("Funding source metadata required"))?;

        let parsed_metadata = match &metadata.metadata {
            Some(funding_source_metadata::Metadata::BankAccount(bank_meta)) => {
                FundingSourceMetadata::BankAccount {
                    account_type: bank_meta.account_type.clone(),
                    routing_number: bank_meta.routing_number.clone(),
                    bank_name: bank_meta.bank_name.clone(),
                }
            }
            Some(funding_source_metadata::Metadata::CryptoWallet(crypto_meta)) => {
                let currency = Self::proto_to_crypto_currency(crypto_meta.currency)?;
                FundingSourceMetadata::CryptoWallet {
                    currency,
                    network: crypto_meta.network.clone(),
                    wallet_address: crypto_meta.wallet_address.clone(),
                    exchange_name: if crypto_meta.exchange_name.is_empty() {
                        None
                    } else {
                        Some(crypto_meta.exchange_name.clone())
                    },
                }
            }
            Some(funding_source_metadata::Metadata::ExternalCard(card_meta)) => {
                FundingSourceMetadata::ExternalCard {
                    card_type: card_meta.card_type.clone(),
                    issuer: card_meta.issuer.clone(),
                    last_four: card_meta.last_four.clone(),
                    expiry_month: card_meta.expiry_month.clone(),
                    expiry_year: card_meta.expiry_year.clone(),
                }
            }
            Some(funding_source_metadata::Metadata::Ach(ach_meta)) => {
                FundingSourceMetadata::ACH {
                    ach_type: ach_meta.ach_type.clone(),
                    processor: ach_meta.processor.clone(),
                }
            }
            None => return Err(Status::invalid_argument("Funding source metadata required")),
        };

        // Parse limits
        let limits_proto = req.limits.as_ref().unwrap();
        let limits = FundingSourceLimits {
            daily_limit: Decimal::from_str(&limits_proto.daily_limit)
                .map_err(|_| Status::invalid_argument("Invalid daily limit"))?,
            monthly_limit: Decimal::from_str(&limits_proto.monthly_limit)
                .map_err(|_| Status::invalid_argument("Invalid monthly limit"))?,
            per_transaction_limit: Decimal::from_str(&limits_proto.per_transaction_limit)
                .map_err(|_| Status::invalid_argument("Invalid per transaction limit"))?,
            minimum_amount: Decimal::from_str(&limits_proto.minimum_amount)
                .map_err(|_| Status::invalid_argument("Invalid minimum amount"))?,
            daily_transaction_count: limits_proto.daily_transaction_count,
            monthly_transaction_count: limits_proto.monthly_transaction_count,
        };

        // Create masked identifier based on metadata
        let masked_identifier = match &parsed_metadata {
            FundingSourceMetadata::BankAccount { routing_number, .. } => {
                format!("****{}", &routing_number[routing_number.len().saturating_sub(4)..])
            }
            FundingSourceMetadata::CryptoWallet { wallet_address, .. } => {
                format!("{}...{}", &wallet_address[..6], &wallet_address[wallet_address.len()-4..])
            }
            FundingSourceMetadata::ExternalCard { last_four, .. } => {
                format!("****-****-****-{}", last_four)
            }
            FundingSourceMetadata::ACH { .. } => {
                format!("ACH-{}", Uuid::new_v4().to_string()[..8].to_uppercase())
            }
            FundingSourceMetadata::FiatAccount { account_id, .. } => {
                format!("FIAT-{}", account_id.to_string()[..8].to_uppercase())
            }
        };

        // Create funding source
        let funding_source = FundingSource {
            id: Uuid::new_v4(),
            user_id: auth_context.user_id,
            source_type,
            status: FundingSourceStatus::Pending, // Requires verification
            name: req.name.clone(),
            masked_identifier,
            currency: req.currency.clone(),
            provider: req.provider.clone(),
            limits,
            metadata: parsed_metadata,
            is_primary: false, // User can set primary later
            is_verified: false, // Requires verification
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None, // Set for cards during verification
            verification_url: None, // Set during verification process
            external_id: None, // Set by external provider
        };

        // Save to repository
        let created_source = self.funding_repository
            .create_funding_source(&funding_source)
            .await
            .map_err(|e| Status::internal(format!("Failed to create funding source: {}", e)))?;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "add_funding_source",
            &format!("Added funding source: {} ({})", created_source.name, created_source.source_type),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_funding_notification(
            &auth_context.user_id,
            NotificationType::FundingSourceAdded,
            "Funding Source Added",
            &format!("Your {} funding source '{}' has been added and is pending verification.",
                created_source.source_type, created_source.name),
            HashMap::from([
                ("funding_source_id".to_string(), created_source.id.to_string()),
                ("source_type".to_string(), created_source.source_type.to_string()),
            ]),
        ).await;

        Ok(Response::new(AddFundingSourceResponse {
            funding_source: Some(self.funding_source_to_proto(&created_source)),
            requires_verification: true,
            verification_url: "".to_string(), // Would be set by verification service
        }))
    }
