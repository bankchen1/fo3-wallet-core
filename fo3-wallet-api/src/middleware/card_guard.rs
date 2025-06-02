//! Card security middleware for transaction validation and limits

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

use crate::state::AppState;
use crate::middleware::auth::AuthContext;
use crate::models::cards::{
    Card, CardTransaction, CardStatus, CardTransactionType, CardTransactionStatus,
    CardRepository, MerchantInfo
};
use crate::models::kyc::KycStatus;

/// Card security guard for validating card operations
pub struct CardGuard {
    state: Arc<AppState>,
}

impl CardGuard {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Validate card issuance eligibility
    pub async fn validate_card_issuance(&self, auth: &AuthContext) -> Result<(), Status> {
        // Check if user has completed KYC
        let kyc_submissions = self.state.kyc_submissions.read()
            .map_err(|_| Status::internal("Failed to read KYC submissions"))?;

        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        let has_approved_kyc = kyc_submissions.values()
            .any(|submission| {
                submission.wallet_id == user_id && submission.status == KycStatus::Approved
            });

        if !has_approved_kyc {
            return Err(Status::failed_precondition(
                "KYC verification required before card issuance"
            ));
        }

        // Check if user already has maximum number of cards (limit: 5)
        let user_cards = self.state.card_repository
            .get_cards_by_user(user_id)
            .map_err(|e| Status::internal(format!("Failed to get user cards: {}", e)))?;

        if user_cards.len() >= 5 {
            return Err(Status::resource_exhausted(
                "Maximum number of cards (5) already issued"
            ));
        }

        Ok(())
    }

    /// Validate card ownership
    pub async fn validate_card_ownership(&self, auth: &AuthContext, card_id: Uuid) -> Result<Card, Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        let card = self.state.card_repository
            .get_card(card_id)
            .map_err(|e| Status::internal(format!("Failed to get card: {}", e)))?
            .ok_or_else(|| Status::not_found("Card not found"))?;

        if card.user_id != user_id {
            return Err(Status::permission_denied("Card does not belong to user"));
        }

        Ok(card)
    }

    /// Validate transaction against card limits and status
    pub async fn validate_transaction(
        &self,
        card: &Card,
        amount: Decimal,
        transaction_type: &CardTransactionType,
        merchant: Option<&MerchantInfo>,
    ) -> Result<(), Status> {
        // Check card status
        match card.status {
            CardStatus::Active => {}
            CardStatus::Frozen => return Err(Status::failed_precondition("Card is frozen")),
            CardStatus::Cancelled => return Err(Status::failed_precondition("Card is cancelled")),
            CardStatus::Expired => return Err(Status::failed_precondition("Card has expired")),
            CardStatus::Blocked => return Err(Status::failed_precondition("Card is blocked")),
            CardStatus::Pending => return Err(Status::failed_precondition("Card is not yet active")),
        }

        // Check if card has expired
        if Utc::now() > card.expires_at {
            return Err(Status::failed_precondition("Card has expired"));
        }

        // Validate amount
        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Transaction amount must be positive"));
        }

        // Check per-transaction limit
        if amount > card.limits.per_transaction_limit {
            return Err(Status::failed_precondition(format!(
                "Transaction amount {} exceeds per-transaction limit {}",
                amount, card.limits.per_transaction_limit
            )));
        }

        // Check balance for purchase transactions
        if matches!(transaction_type, CardTransactionType::Purchase) {
            if card.balance < amount {
                return Err(Status::failed_precondition("Insufficient card balance"));
            }
        }

        // Check daily and monthly limits
        self.validate_spending_limits(card, amount).await?;

        // Validate merchant if provided
        if let Some(merchant_info) = merchant {
            self.validate_merchant(merchant_info)?;
        }

        Ok(())
    }

    /// Validate spending limits (daily and monthly)
    async fn validate_spending_limits(&self, card: &Card, amount: Decimal) -> Result<(), Status> {
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let month_start = now.date_naive()
            .with_day(1).unwrap()
            .and_hms_opt(0, 0, 0).unwrap()
            .and_utc();

        // Get all transactions for this card
        let transactions = self.state.card_repository
            .get_transactions_by_card(card.id)
            .map_err(|e| Status::internal(format!("Failed to get transactions: {}", e)))?;

        // Calculate daily spending
        let daily_spending: Decimal = transactions.iter()
            .filter(|tx| {
                tx.created_at >= today_start &&
                matches!(tx.transaction_type, CardTransactionType::Purchase) &&
                matches!(tx.status, CardTransactionStatus::Approved | CardTransactionStatus::Settled)
            })
            .map(|tx| tx.amount)
            .sum();

        if daily_spending + amount > card.limits.daily_limit {
            return Err(Status::failed_precondition(format!(
                "Transaction would exceed daily limit. Current: {}, Limit: {}, Attempted: {}",
                daily_spending, card.limits.daily_limit, amount
            )));
        }

        // Calculate monthly spending
        let monthly_spending: Decimal = transactions.iter()
            .filter(|tx| {
                tx.created_at >= month_start &&
                matches!(tx.transaction_type, CardTransactionType::Purchase) &&
                matches!(tx.status, CardTransactionStatus::Approved | CardTransactionStatus::Settled)
            })
            .map(|tx| tx.amount)
            .sum();

        if monthly_spending + amount > card.limits.monthly_limit {
            return Err(Status::failed_precondition(format!(
                "Transaction would exceed monthly limit. Current: {}, Limit: {}, Attempted: {}",
                monthly_spending, card.limits.monthly_limit, amount
            )));
        }

        // Check daily transaction count
        let daily_transaction_count = transactions.iter()
            .filter(|tx| {
                tx.created_at >= today_start &&
                matches!(tx.transaction_type, CardTransactionType::Purchase) &&
                matches!(tx.status, CardTransactionStatus::Approved | CardTransactionStatus::Settled)
            })
            .count() as i32;

        if daily_transaction_count >= card.limits.transaction_count_daily {
            return Err(Status::failed_precondition(format!(
                "Daily transaction count limit reached: {}/{}",
                daily_transaction_count, card.limits.transaction_count_daily
            )));
        }

        // Check monthly transaction count
        let monthly_transaction_count = transactions.iter()
            .filter(|tx| {
                tx.created_at >= month_start &&
                matches!(tx.transaction_type, CardTransactionType::Purchase) &&
                matches!(tx.status, CardTransactionStatus::Approved | CardTransactionStatus::Settled)
            })
            .count() as i32;

        if monthly_transaction_count >= card.limits.transaction_count_monthly {
            return Err(Status::failed_precondition(format!(
                "Monthly transaction count limit reached: {}/{}",
                monthly_transaction_count, card.limits.transaction_count_monthly
            )));
        }

        Ok(())
    }

    /// Validate merchant information
    fn validate_merchant(&self, merchant: &MerchantInfo) -> Result<(), Status> {
        if merchant.name.trim().is_empty() {
            return Err(Status::invalid_argument("Merchant name cannot be empty"));
        }

        if merchant.country.len() != 2 {
            return Err(Status::invalid_argument("Merchant country must be 2-letter ISO code"));
        }

        if !merchant.mcc.chars().all(|c| c.is_ascii_digit()) || merchant.mcc.len() != 4 {
            return Err(Status::invalid_argument("Merchant Category Code must be 4 digits"));
        }

        Ok(())
    }

    /// Check for suspicious transaction patterns
    pub async fn check_fraud_patterns(&self, card: &Card, amount: Decimal, merchant: Option<&MerchantInfo>) -> Result<(), Status> {
        let transactions = self.state.card_repository
            .get_transactions_by_card(card.id)
            .map_err(|e| Status::internal(format!("Failed to get transactions: {}", e)))?;

        let recent_transactions: Vec<&CardTransaction> = transactions.iter()
            .filter(|tx| tx.created_at > Utc::now() - Duration::hours(1))
            .collect();

        // Check for rapid successive transactions (more than 5 in 1 hour)
        if recent_transactions.len() >= 5 {
            return Err(Status::failed_precondition(
                "Too many transactions in short time period. Please wait before making another transaction."
            ));
        }

        // Check for duplicate transactions (same amount and merchant within 5 minutes)
        if let Some(merchant_info) = merchant {
            let duplicate_transactions = recent_transactions.iter()
                .filter(|tx| {
                    tx.created_at > Utc::now() - Duration::minutes(5) &&
                    tx.amount == amount &&
                    tx.merchant.as_ref().map(|m| &m.name) == Some(&merchant_info.name)
                })
                .count();

            if duplicate_transactions > 0 {
                return Err(Status::failed_precondition(
                    "Duplicate transaction detected. Please wait before retrying."
                ));
            }
        }

        // Check for unusually large transaction (more than 10x average)
        if !transactions.is_empty() {
            let average_amount: Decimal = transactions.iter()
                .map(|tx| tx.amount)
                .sum::<Decimal>() / Decimal::from(transactions.len());

            if amount > average_amount * Decimal::from(10) {
                return Err(Status::failed_precondition(
                    "Transaction amount is unusually large. Please contact support for assistance."
                ));
            }
        }

        Ok(())
    }

    /// Validate 2FA code for sensitive operations
    pub async fn validate_2fa(&self, auth: &AuthContext, verification_code: &str) -> Result<(), Status> {
        // In a real implementation, this would validate against a TOTP or SMS code
        // For demo purposes, we'll accept a simple pattern
        if verification_code.len() != 6 || !verification_code.chars().all(|c| c.is_ascii_digit()) {
            return Err(Status::invalid_argument("2FA code must be 6 digits"));
        }

        // Mock validation - in production, validate against actual 2FA service
        if verification_code == "000000" {
            return Err(Status::unauthenticated("Invalid 2FA code"));
        }

        Ok(())
    }

    /// Rate limiting for card operations
    pub async fn check_rate_limit(&self, auth: &AuthContext, operation: &str) -> Result<(), Status> {
        // In a real implementation, this would use Redis or similar for distributed rate limiting
        // For demo purposes, we'll implement basic in-memory rate limiting
        
        // Allow different limits for different operations
        let limit = match operation {
            "issue_card" => 1, // 1 card issuance per hour
            "freeze_card" | "unfreeze_card" => 10, // 10 freeze/unfreeze operations per hour
            "transaction" => 100, // 100 transactions per hour
            _ => 50, // Default limit
        };

        // In production, implement proper rate limiting with sliding windows
        // For now, just return OK
        Ok(())
    }
}

/// Velocity limits for different transaction types
#[derive(Debug, Clone)]
pub struct VelocityLimits {
    pub max_transactions_per_hour: i32,
    pub max_amount_per_hour: Decimal,
    pub max_transactions_per_day: i32,
    pub max_amount_per_day: Decimal,
}

impl Default for VelocityLimits {
    fn default() -> Self {
        Self {
            max_transactions_per_hour: 20,
            max_amount_per_hour: Decimal::from(10000), // $10,000 per hour
            max_transactions_per_day: 100,
            max_amount_per_day: Decimal::from(50000), // $50,000 per day
        }
    }
}
