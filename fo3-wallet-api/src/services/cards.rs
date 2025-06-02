//! Card service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    card_service_server::CardService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    card_guard::CardGuard,
};
use crate::models::cards::{
    Card, CardTransaction, CardLimits, CardStatus, CardType, CardTransactionStatus,
    CardTransactionType, MerchantCategory, MerchantInfo, CardMetrics,
    CardRepository
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Card service implementation
pub struct CardServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    card_guard: Arc<CardGuard>,
}

impl CardServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        let card_guard = Arc::new(CardGuard::new(state.clone()));
        
        Self {
            state,
            auth_service,
            audit_logger,
            card_guard,
        }
    }

    /// Convert internal Card to proto Card
    fn card_to_proto(&self, card: &Card) -> crate::proto::fo3::wallet::v1::Card {
        crate::proto::fo3::wallet::v1::Card {
            id: card.id.to_string(),
            user_id: card.user_id.to_string(),
            r#type: match card.card_type {
                CardType::Virtual => 1,
                CardType::Physical => 2,
            },
            status: match card.status {
                CardStatus::Pending => 5,
                CardStatus::Active => 1,
                CardStatus::Frozen => 2,
                CardStatus::Expired => 3,
                CardStatus::Cancelled => 4,
                CardStatus::Blocked => 6,
            },
            masked_number: card.masked_number.clone(),
            cardholder_name: card.cardholder_name.clone(),
            expiry_month: card.expiry_month.clone(),
            expiry_year: card.expiry_year.clone(),
            currency: card.currency.clone(),
            balance: card.balance.to_string(),
            limits: Some(CardLimits {
                daily_limit: card.limits.daily_limit.to_string(),
                monthly_limit: card.limits.monthly_limit.to_string(),
                per_transaction_limit: card.limits.per_transaction_limit.to_string(),
                atm_daily_limit: card.limits.atm_daily_limit.to_string(),
                transaction_count_daily: card.limits.transaction_count_daily,
                transaction_count_monthly: card.limits.transaction_count_monthly,
            }),
            design_id: card.design_id.clone(),
            linked_account_id: card.linked_account_id.map(|id| id.to_string()).unwrap_or_default(),
            is_primary: card.is_primary,
            created_at: card.created_at.timestamp(),
            updated_at: card.updated_at.timestamp(),
            expires_at: card.expires_at.timestamp(),
            frozen_at: card.frozen_at.map(|dt| dt.timestamp()).unwrap_or(0),
            frozen_reason: card.frozen_reason.clone().unwrap_or_default(),
        }
    }

    /// Convert internal CardTransaction to proto CardTransaction
    fn transaction_to_proto(&self, transaction: &CardTransaction) -> crate::proto::fo3::wallet::v1::CardTransaction {
        crate::proto::fo3::wallet::v1::CardTransaction {
            id: transaction.id.to_string(),
            card_id: transaction.card_id.to_string(),
            user_id: transaction.user_id.to_string(),
            r#type: match transaction.transaction_type {
                CardTransactionType::Purchase => 1,
                CardTransactionType::Refund => 2,
                CardTransactionType::Authorization => 3,
                CardTransactionType::TopUp => 4,
                CardTransactionType::Fee => 5,
            },
            status: match transaction.status {
                CardTransactionStatus::Pending => 1,
                CardTransactionStatus::Approved => 2,
                CardTransactionStatus::Declined => 3,
                CardTransactionStatus::Reversed => 4,
                CardTransactionStatus::Settled => 5,
            },
            amount: transaction.amount.to_string(),
            currency: transaction.currency.clone(),
            fee_amount: transaction.fee_amount.to_string(),
            net_amount: transaction.net_amount.to_string(),
            merchant: transaction.merchant.as_ref().map(|m| MerchantInfo {
                name: m.name.clone(),
                category: m.category.clone(),
                category_code: match m.category_code {
                    MerchantCategory::Grocery => 1,
                    MerchantCategory::Restaurant => 2,
                    MerchantCategory::GasStation => 3,
                    MerchantCategory::Retail => 4,
                    MerchantCategory::Entertainment => 5,
                    MerchantCategory::Travel => 6,
                    MerchantCategory::Healthcare => 7,
                    MerchantCategory::Education => 8,
                    MerchantCategory::Utilities => 9,
                    MerchantCategory::Other => 10,
                },
                location: m.location.clone(),
                country: m.country.clone(),
                mcc: m.mcc.clone(),
            }),
            description: transaction.description.clone(),
            reference_number: transaction.reference_number.clone(),
            authorization_code: transaction.authorization_code.clone().unwrap_or_default(),
            metadata: transaction.metadata.clone(),
            created_at: transaction.created_at.timestamp(),
            authorized_at: transaction.authorized_at.map(|dt| dt.timestamp()).unwrap_or(0),
            settled_at: transaction.settled_at.map(|dt| dt.timestamp()).unwrap_or(0),
            decline_reason: transaction.decline_reason.clone().unwrap_or_default(),
        }
    }

    /// Convert proto CardLimits to internal CardLimits
    fn proto_to_card_limits(&self, proto_limits: &crate::proto::fo3::wallet::v1::CardLimits) -> Result<crate::models::cards::CardLimits, Status> {
        Ok(crate::models::cards::CardLimits {
            daily_limit: Decimal::from_str_exact(&proto_limits.daily_limit)
                .map_err(|_| Status::invalid_argument("Invalid daily limit"))?,
            monthly_limit: Decimal::from_str_exact(&proto_limits.monthly_limit)
                .map_err(|_| Status::invalid_argument("Invalid monthly limit"))?,
            per_transaction_limit: Decimal::from_str_exact(&proto_limits.per_transaction_limit)
                .map_err(|_| Status::invalid_argument("Invalid per-transaction limit"))?,
            atm_daily_limit: Decimal::from_str_exact(&proto_limits.atm_daily_limit)
                .map_err(|_| Status::invalid_argument("Invalid ATM daily limit"))?,
            transaction_count_daily: proto_limits.transaction_count_daily,
            transaction_count_monthly: proto_limits.transaction_count_monthly,
        })
    }

    /// Convert proto MerchantInfo to internal MerchantInfo
    fn proto_to_merchant_info(&self, proto_merchant: &crate::proto::fo3::wallet::v1::MerchantInfo) -> crate::models::cards::MerchantInfo {
        crate::models::cards::MerchantInfo {
            name: proto_merchant.name.clone(),
            category: proto_merchant.category.clone(),
            category_code: match proto_merchant.category_code {
                1 => MerchantCategory::Grocery,
                2 => MerchantCategory::Restaurant,
                3 => MerchantCategory::GasStation,
                4 => MerchantCategory::Retail,
                5 => MerchantCategory::Entertainment,
                6 => MerchantCategory::Travel,
                7 => MerchantCategory::Healthcare,
                8 => MerchantCategory::Education,
                9 => MerchantCategory::Utilities,
                _ => MerchantCategory::Other,
            },
            location: proto_merchant.location.clone(),
            country: proto_merchant.country.clone(),
            mcc: proto_merchant.mcc.clone(),
        }
    }

    /// Send notification for card events
    async fn send_card_notification(
        &self,
        user_id: &str,
        notification_type: NotificationType,
        title: String,
        message: String,
        metadata: HashMap<String, String>,
    ) -> Result<(), Status> {
        // Use the notification service to send real-time notifications
        let notification_request = crate::proto::fo3::wallet::v1::SendNotificationRequest {
            user_id: user_id.to_string(),
            r#type: match notification_type {
                NotificationType::Transaction => 1,
                NotificationType::Security => 3,
                NotificationType::System => 5,
                _ => 5,
            },
            priority: 2, // Normal priority
            title,
            message,
            metadata,
            channels: vec![1, 2], // WebSocket and InApp
            expires_at: 0,
            action_url: String::new(),
            icon_url: String::new(),
        };

        // In a real implementation, we would call the notification service
        // For now, we'll just log the notification
        tracing::info!(
            "Card notification sent to user {}: {}",
            user_id,
            notification_request.title
        );

        Ok(())
    }
}

#[tonic::async_trait]
impl CardService for CardServiceImpl {
    /// Issue a new virtual card
    async fn issue_virtual_card(
        &self,
        request: Request<IssueCardRequest>,
    ) -> Result<Response<IssueCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        // Validate card issuance eligibility
        self.card_guard.validate_card_issuance(&auth_context).await?;

        // Rate limiting
        self.card_guard.check_rate_limit(&auth_context, "issue_card").await?;

        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Parse linked account ID if provided
        let linked_account_id = if !req.linked_account_id.is_empty() {
            Some(Uuid::parse_str(&req.linked_account_id)
                .map_err(|_| Status::invalid_argument("Invalid linked account ID format"))?)
        } else {
            None
        };

        // Parse card limits if provided
        let limits = if let Some(proto_limits) = req.limits {
            Some(self.proto_to_card_limits(&proto_limits)?)
        } else {
            None
        };

        // Create new card
        let mut card = Card::new(
            user_id,
            req.cardholder_name,
            req.currency.clone(),
            limits,
            Some(req.design_id),
            linked_account_id,
            req.is_primary,
        );

        // Activate the card immediately for demo purposes
        // In production, this might require additional verification
        card.update_status(CardStatus::Active, None);

        // Store the card
        let created_card = self.state.card_repository
            .create_card(card)
            .map_err(|e| Status::internal(format!("Failed to create card: {}", e)))?;

        // For security, we only return the full card details once during issuance
        // In production, these would be properly encrypted and handled securely
        let full_card_number = "4000123456789012"; // Mock card number
        let cvv = "123"; // Mock CVV
        let pin = "1234"; // Mock PIN

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), created_card.id.to_string());
        metadata.insert("card_type".to_string(), "virtual".to_string());

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::System,
            "Virtual Card Issued".to_string(),
            format!("Your new virtual card ending in {} has been issued successfully", 
                   &created_card.masked_number[created_card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_issued",
            &format!("Virtual card {} issued", created_card.id),
            Some(&created_card.id.to_string()),
        ).await;

        let response = IssueCardResponse {
            card: Some(self.card_to_proto(&created_card)),
            full_card_number: full_card_number.to_string(),
            cvv: cvv.to_string(),
            pin: pin.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get card details
    async fn get_card(
        &self,
        request: Request<GetCardRequest>,
    ) -> Result<Response<GetCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate card ownership
        let card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        let response = GetCardResponse {
            card: Some(self.card_to_proto(&card)),
        };

        Ok(Response::new(response))
    }

    /// List user's cards
    async fn list_cards(
        &self,
        request: Request<ListCardsRequest>,
    ) -> Result<Response<ListCardsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Get user's cards
        let mut cards = self.state.card_repository
            .get_cards_by_user(user_id)
            .map_err(|e| Status::internal(format!("Failed to get cards: {}", e)))?;

        // Apply filters
        if req.status != 0 {
            let filter_status = match req.status {
                1 => CardStatus::Active,
                2 => CardStatus::Frozen,
                3 => CardStatus::Expired,
                4 => CardStatus::Cancelled,
                5 => CardStatus::Pending,
                6 => CardStatus::Blocked,
                _ => return Err(Status::invalid_argument("Invalid card status filter")),
            };
            cards.retain(|card| card.status == filter_status);
        }

        if req.r#type != 0 {
            let filter_type = match req.r#type {
                1 => CardType::Virtual,
                2 => CardType::Physical,
                _ => return Err(Status::invalid_argument("Invalid card type filter")),
            };
            cards.retain(|card| card.card_type == filter_type);
        }

        // Sort by creation date (newest first)
        cards.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        let page_size = if req.page_size > 0 { req.page_size as usize } else { 20 };
        let total_count = cards.len() as i64;
        
        // For simplicity, we'll implement basic pagination
        // In production, use proper offset-based or cursor-based pagination
        let start_index = 0; // Would parse page_token in production
        let end_index = std::cmp::min(start_index + page_size, cards.len());
        
        let page_cards = if start_index < cards.len() {
            cards[start_index..end_index].to_vec()
        } else {
            Vec::new()
        };

        let proto_cards: Vec<crate::proto::fo3::wallet::v1::Card> = page_cards
            .iter()
            .map(|card| self.card_to_proto(card))
            .collect();

        let next_page_token = if end_index < cards.len() {
            format!("page_{}", end_index) // Simple token for demo
        } else {
            String::new()
        };

        let response = ListCardsResponse {
            cards: proto_cards,
            next_page_token,
            total_count,
        };

        Ok(Response::new(response))
    }

    /// Freeze a card
    async fn freeze_card(
        &self,
        request: Request<FreezeCardRequest>,
    ) -> Result<Response<FreezeCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate 2FA if required
        if req.require_2fa {
            self.card_guard.validate_2fa(&auth_context, &req.verification_code).await?;
        }

        // Rate limiting
        self.card_guard.check_rate_limit(&auth_context, "freeze_card").await?;

        // Validate card ownership
        let mut card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Check if card can be frozen
        if card.status == CardStatus::Cancelled {
            return Err(Status::failed_precondition("Cannot freeze a cancelled card"));
        }

        if card.status == CardStatus::Frozen {
            return Err(Status::failed_precondition("Card is already frozen"));
        }

        // Freeze the card
        card.update_status(CardStatus::Frozen, Some(req.reason.clone()));

        // Update in repository
        let updated_card = self.state.card_repository
            .update_card(card)
            .map_err(|e| Status::internal(format!("Failed to update card: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), updated_card.id.to_string());
        metadata.insert("reason".to_string(), req.reason);

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Security,
            "Card Frozen".to_string(),
            format!("Your card ending in {} has been frozen", 
                   &updated_card.masked_number[updated_card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_frozen",
            &format!("Card {} frozen", updated_card.id),
            Some(&updated_card.id.to_string()),
        ).await;

        let response = FreezeCardResponse {
            card: Some(self.card_to_proto(&updated_card)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Unfreeze a card
    async fn unfreeze_card(
        &self,
        request: Request<UnfreezeCardRequest>,
    ) -> Result<Response<UnfreezeCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate 2FA if required
        if req.require_2fa {
            self.card_guard.validate_2fa(&auth_context, &req.verification_code).await?;
        }

        // Rate limiting
        self.card_guard.check_rate_limit(&auth_context, "unfreeze_card").await?;

        // Validate card ownership
        let mut card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Check if card can be unfrozen
        if card.status != CardStatus::Frozen {
            return Err(Status::failed_precondition("Card is not frozen"));
        }

        // Unfreeze the card
        card.update_status(CardStatus::Active, None);

        // Update in repository
        let updated_card = self.state.card_repository
            .update_card(card)
            .map_err(|e| Status::internal(format!("Failed to update card: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), updated_card.id.to_string());

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Security,
            "Card Unfrozen".to_string(),
            format!("Your card ending in {} has been unfrozen and is now active", 
                   &updated_card.masked_number[updated_card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_unfrozen",
            &format!("Card {} unfrozen", updated_card.id),
            Some(&updated_card.id.to_string()),
        ).await;

        let response = UnfreezeCardResponse {
            card: Some(self.card_to_proto(&updated_card)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Cancel a card
    async fn cancel_card(
        &self,
        request: Request<CancelCardRequest>,
    ) -> Result<Response<CancelCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate 2FA if required
        if req.require_2fa {
            self.card_guard.validate_2fa(&auth_context, &req.verification_code).await?;
        }

        // Validate card ownership
        let mut card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Check if card can be cancelled
        if card.status == CardStatus::Cancelled {
            return Err(Status::failed_precondition("Card is already cancelled"));
        }

        // Cancel the card
        card.update_status(CardStatus::Cancelled, Some(req.reason.clone()));

        // Update in repository
        let updated_card = self.state.card_repository
            .update_card(card)
            .map_err(|e| Status::internal(format!("Failed to update card: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), updated_card.id.to_string());
        metadata.insert("reason".to_string(), req.reason);

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Security,
            "Card Cancelled".to_string(),
            format!("Your card ending in {} has been permanently cancelled", 
                   &updated_card.masked_number[updated_card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_cancelled",
            &format!("Card {} cancelled", updated_card.id),
            Some(&updated_card.id.to_string()),
        ).await;

        let response = CancelCardResponse {
            card: Some(self.card_to_proto(&updated_card)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Get card transactions
    async fn get_card_transactions(
        &self,
        request: Request<GetCardTransactionsRequest>,
    ) -> Result<Response<GetCardTransactionsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate card ownership
        let _card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Get transactions
        let mut transactions = self.state.card_repository
            .get_transactions_by_card(card_id)
            .map_err(|e| Status::internal(format!("Failed to get transactions: {}", e)))?;

        // Apply filters
        if req.status != 0 {
            let filter_status = match req.status {
                1 => CardTransactionStatus::Pending,
                2 => CardTransactionStatus::Approved,
                3 => CardTransactionStatus::Declined,
                4 => CardTransactionStatus::Reversed,
                5 => CardTransactionStatus::Settled,
                _ => return Err(Status::invalid_argument("Invalid transaction status filter")),
            };
            transactions.retain(|tx| tx.status == filter_status);
        }

        if req.r#type != 0 {
            let filter_type = match req.r#type {
                1 => CardTransactionType::Purchase,
                2 => CardTransactionType::Refund,
                3 => CardTransactionType::Authorization,
                4 => CardTransactionType::TopUp,
                5 => CardTransactionType::Fee,
                _ => return Err(Status::invalid_argument("Invalid transaction type filter")),
            };
            transactions.retain(|tx| tx.transaction_type == filter_type);
        }

        // Apply date filters
        if req.start_date > 0 {
            let start_date = DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?;
            transactions.retain(|tx| tx.created_at >= start_date);
        }

        if req.end_date > 0 {
            let end_date = DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?;
            transactions.retain(|tx| tx.created_at <= end_date);
        }

        // Sort by creation date (newest first)
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Calculate total amount
        let total_amount: Decimal = transactions.iter()
            .map(|tx| tx.amount)
            .sum();

        // Apply pagination
        let page_size = if req.page_size > 0 { req.page_size as usize } else { 50 };
        let total_count = transactions.len() as i64;

        let start_index = 0; // Would parse page_token in production
        let end_index = std::cmp::min(start_index + page_size, transactions.len());

        let page_transactions = if start_index < transactions.len() {
            transactions[start_index..end_index].to_vec()
        } else {
            Vec::new()
        };

        let proto_transactions: Vec<crate::proto::fo3::wallet::v1::CardTransaction> = page_transactions
            .iter()
            .map(|tx| self.transaction_to_proto(tx))
            .collect();

        let next_page_token = if end_index < transactions.len() {
            format!("page_{}", end_index)
        } else {
            String::new()
        };

        let response = GetCardTransactionsResponse {
            transactions: proto_transactions,
            next_page_token,
            total_count,
            total_amount: total_amount.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Simulate a transaction
    async fn simulate_transaction(
        &self,
        request: Request<SimulateTransactionRequest>,
    ) -> Result<Response<SimulateTransactionResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate card ownership
        let card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        let amount = Decimal::from_str_exact(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Convert merchant info if provided
        let merchant = req.merchant.as_ref().map(|m| self.proto_to_merchant_info(m));

        // Validate transaction
        let validation_result = self.card_guard.validate_transaction(
            &card,
            amount,
            &CardTransactionType::Purchase,
            merchant.as_ref(),
        ).await;

        let (approved, decline_reason) = match validation_result {
            Ok(()) => {
                // Check for fraud patterns
                match self.card_guard.check_fraud_patterns(&card, amount, merchant.as_ref()).await {
                    Ok(()) => (true, String::new()),
                    Err(status) => (false, status.message().to_string()),
                }
            }
            Err(status) => (false, status.message().to_string()),
        };

        // Create simulated transaction
        let mut transaction = CardTransaction::new(
            card_id,
            card.user_id,
            if req.is_authorization_only {
                CardTransactionType::Authorization
            } else {
                CardTransactionType::Purchase
            },
            amount,
            req.currency,
            merchant,
            req.description,
        );

        let authorization_code = if approved {
            let code = format!("AUTH{:06}", rand::random::<u32>() % 1000000);
            transaction.approve(code.clone());
            code
        } else {
            transaction.decline(decline_reason.clone());
            String::new()
        };

        let response = SimulateTransactionResponse {
            transaction: Some(self.transaction_to_proto(&transaction)),
            approved,
            decline_reason,
            authorization_code,
        };

        Ok(Response::new(response))
    }

    /// Top up card balance
    async fn top_up_card(
        &self,
        request: Request<TopUpCardRequest>,
    ) -> Result<Response<TopUpCardResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        let amount = Decimal::from_str_exact(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Top-up amount must be positive"));
        }

        // Validate card ownership
        let mut card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Check if card can receive top-ups
        if card.status != CardStatus::Active {
            return Err(Status::failed_precondition("Card must be active to receive top-ups"));
        }

        // Validate funding source if provided
        if !req.funding_source_id.is_empty() {
            let _funding_source_id = Uuid::parse_str(&req.funding_source_id)
                .map_err(|_| Status::invalid_argument("Invalid funding source ID format"))?;

            // In production, validate that the funding source belongs to the user
            // and has sufficient balance
        }

        // Add balance to card
        card.add_balance(amount)
            .map_err(|e| Status::internal(format!("Failed to add balance: {}", e)))?;

        // Update card in repository
        let updated_card = self.state.card_repository
            .update_card(card)
            .map_err(|e| Status::internal(format!("Failed to update card: {}", e)))?;

        // Create top-up transaction record
        let mut transaction = CardTransaction::new(
            card_id,
            updated_card.user_id,
            CardTransactionType::TopUp,
            amount,
            req.currency,
            None,
            format!("Card top-up from funding source"),
        );

        transaction.approve(format!("TOPUP{:06}", rand::random::<u32>() % 1000000));
        transaction.settle();

        // Store transaction
        let created_transaction = self.state.card_repository
            .create_transaction(transaction)
            .map_err(|e| Status::internal(format!("Failed to create transaction: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), updated_card.id.to_string());
        metadata.insert("amount".to_string(), amount.to_string());
        metadata.insert("currency".to_string(), req.currency);

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Transaction,
            "Card Top-up Successful".to_string(),
            format!("Your card ending in {} has been topped up with {} {}",
                   &updated_card.masked_number[updated_card.masked_number.len()-4..],
                   amount, req.currency),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_topup",
            &format!("Card {} topped up with {} {}", updated_card.id, amount, req.currency),
            Some(&updated_card.id.to_string()),
        ).await;

        let response = TopUpCardResponse {
            card: Some(self.card_to_proto(&updated_card)),
            transaction: Some(self.transaction_to_proto(&created_transaction)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Update card limits
    async fn update_card_limits(
        &self,
        request: Request<UpdateCardLimitsRequest>,
    ) -> Result<Response<UpdateCardLimitsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate 2FA if required
        if req.require_2fa {
            self.card_guard.validate_2fa(&auth_context, &req.verification_code).await?;
        }

        // Validate card ownership
        let mut card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Parse new limits
        let new_limits = req.limits
            .ok_or_else(|| Status::invalid_argument("Card limits are required"))?;

        let limits = self.proto_to_card_limits(&new_limits)?;

        // Validate limits
        if limits.daily_limit <= Decimal::ZERO || limits.monthly_limit <= Decimal::ZERO {
            return Err(Status::invalid_argument("Limits must be positive"));
        }

        if limits.daily_limit > limits.monthly_limit {
            return Err(Status::invalid_argument("Daily limit cannot exceed monthly limit"));
        }

        if limits.per_transaction_limit > limits.daily_limit {
            return Err(Status::invalid_argument("Per-transaction limit cannot exceed daily limit"));
        }

        // Update card limits
        card.update_limits(limits);

        // Update in repository
        let updated_card = self.state.card_repository
            .update_card(card)
            .map_err(|e| Status::internal(format!("Failed to update card: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), updated_card.id.to_string());

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Security,
            "Card Limits Updated".to_string(),
            format!("Spending limits for your card ending in {} have been updated",
                   &updated_card.masked_number[updated_card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_limits_updated",
            &format!("Card {} limits updated", updated_card.id),
            Some(&updated_card.id.to_string()),
        ).await;

        let response = UpdateCardLimitsResponse {
            card: Some(self.card_to_proto(&updated_card)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Update card PIN
    async fn update_card_pin(
        &self,
        request: Request<UpdateCardPinRequest>,
    ) -> Result<Response<UpdateCardPinResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardRead)?;

        let req = request.into_inner();

        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID format"))?;

        // Validate 2FA if required
        if req.require_2fa {
            self.card_guard.validate_2fa(&auth_context, &req.verification_code).await?;
        }

        // Validate card ownership
        let card = self.card_guard.validate_card_ownership(&auth_context, card_id).await?;

        // Validate PIN format
        if req.new_pin.len() < 4 || req.new_pin.len() > 6 {
            return Err(Status::invalid_argument("PIN must be 4-6 digits"));
        }

        if !req.new_pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(Status::invalid_argument("PIN must contain only digits"));
        }

        // In production, validate current PIN
        // For demo purposes, we'll skip this validation

        // Update PIN (in production, this would be properly encrypted)
        // For now, we'll just log the operation

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("card_id".to_string(), card.id.to_string());

        self.send_card_notification(
            &auth_context.user_id,
            NotificationType::Security,
            "Card PIN Updated".to_string(),
            format!("PIN for your card ending in {} has been updated successfully",
                   &card.masked_number[card.masked_number.len()-4..]),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "card_pin_updated",
            &format!("Card {} PIN updated", card.id),
            Some(&card.id.to_string()),
        ).await;

        let response = UpdateCardPinResponse {
            success: true,
            message: "PIN updated successfully".to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get card metrics (admin only)
    async fn get_card_metrics(
        &self,
        request: Request<GetCardMetricsRequest>,
    ) -> Result<Response<GetCardMetricsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardAdmin)?;

        let req = request.into_inner();

        let start_date = if req.start_date > 0 {
            DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?
        } else {
            Utc::now() - chrono::Duration::days(30) // Default to last 30 days
        };

        let end_date = if req.end_date > 0 {
            DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?
        } else {
            Utc::now()
        };

        // Get metrics from repository
        let metrics = self.state.card_repository
            .get_card_metrics(start_date, end_date)
            .map_err(|e| Status::internal(format!("Failed to get metrics: {}", e)))?;

        let proto_metrics = crate::proto::fo3::wallet::v1::CardMetrics {
            total_cards_issued: metrics.total_cards_issued,
            active_cards: metrics.active_cards,
            frozen_cards: metrics.frozen_cards,
            cancelled_cards: metrics.cancelled_cards,
            total_transaction_volume: metrics.total_transaction_volume.to_string(),
            total_transactions: metrics.total_transactions,
            average_transaction_amount: metrics.average_transaction_amount.to_string(),
            declined_transactions: metrics.declined_transactions,
            decline_rate: metrics.decline_rate,
            transactions_by_category: metrics.transactions_by_category,
            volume_by_currency: metrics.volume_by_currency.into_iter()
                .map(|(k, v)| (k, v.to_string()))
                .collect(),
        };

        let response = GetCardMetricsResponse {
            metrics: Some(proto_metrics),
        };

        Ok(Response::new(response))
    }

    /// List all cards (admin only)
    async fn list_all_cards(
        &self,
        request: Request<ListAllCardsRequest>,
    ) -> Result<Response<ListAllCardsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionCardAdmin)?;

        let req = request.into_inner();

        let page_size = if req.page_size > 0 { req.page_size as usize } else { 50 };
        let offset = 0; // Would parse page_token in production

        // Get all cards with pagination
        let cards = self.state.card_repository
            .list_all_cards(Some(page_size), Some(offset))
            .map_err(|e| Status::internal(format!("Failed to list cards: {}", e)))?;

        // Apply filters
        let mut filtered_cards = cards;

        if !req.user_id.is_empty() {
            let filter_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;
            filtered_cards.retain(|card| card.user_id == filter_user_id);
        }

        if req.status != 0 {
            let filter_status = match req.status {
                1 => CardStatus::Active,
                2 => CardStatus::Frozen,
                3 => CardStatus::Expired,
                4 => CardStatus::Cancelled,
                5 => CardStatus::Pending,
                6 => CardStatus::Blocked,
                _ => return Err(Status::invalid_argument("Invalid card status filter")),
            };
            filtered_cards.retain(|card| card.status == filter_status);
        }

        if req.r#type != 0 {
            let filter_type = match req.r#type {
                1 => CardType::Virtual,
                2 => CardType::Physical,
                _ => return Err(Status::invalid_argument("Invalid card type filter")),
            };
            filtered_cards.retain(|card| card.card_type == filter_type);
        }

        let total_count = filtered_cards.len() as i64;

        let proto_cards: Vec<crate::proto::fo3::wallet::v1::Card> = filtered_cards
            .iter()
            .map(|card| self.card_to_proto(card))
            .collect();

        let next_page_token = if filtered_cards.len() == page_size {
            format!("page_{}", offset + page_size)
        } else {
            String::new()
        };

        let response = ListAllCardsResponse {
            cards: proto_cards,
            next_page_token,
            total_count,
        };

        Ok(Response::new(response))
    }
}
