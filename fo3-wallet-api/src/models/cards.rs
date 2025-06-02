//! Card models and repository for virtual card management

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use rust_decimal::Decimal;
use rand::{Rng, thread_rng};
use sha2::{Sha256, Digest};

/// Card status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardStatus {
    Active,
    Frozen,
    Expired,
    Cancelled,
    Pending,
    Blocked,
}

impl Default for CardStatus {
    fn default() -> Self {
        CardStatus::Pending
    }
}

/// Card type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Virtual,
    Physical, // Future implementation
}

impl Default for CardType {
    fn default() -> Self {
        CardType::Virtual
    }
}

/// Card transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardTransactionStatus {
    Pending,
    Approved,
    Declined,
    Reversed,
    Settled,
}

/// Card transaction type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardTransactionType {
    Purchase,
    Refund,
    Authorization,
    TopUp,
    Fee,
}

/// Merchant category enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MerchantCategory {
    Grocery,
    Restaurant,
    GasStation,
    Retail,
    Entertainment,
    Travel,
    Healthcare,
    Education,
    Utilities,
    Other,
}

/// Card spending limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLimits {
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub per_transaction_limit: Decimal,
    pub atm_daily_limit: Decimal,
    pub transaction_count_daily: i32,
    pub transaction_count_monthly: i32,
}

impl Default for CardLimits {
    fn default() -> Self {
        Self {
            daily_limit: Decimal::from(5000), // $5,000 daily limit
            monthly_limit: Decimal::from(50000), // $50,000 monthly limit
            per_transaction_limit: Decimal::from(2500), // $2,500 per transaction
            atm_daily_limit: Decimal::from(1000), // $1,000 ATM daily limit
            transaction_count_daily: 50,
            transaction_count_monthly: 500,
        }
    }
}

/// Merchant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantInfo {
    pub name: String,
    pub category: String,
    pub category_code: MerchantCategory,
    pub location: String,
    pub country: String,
    pub mcc: String, // Merchant Category Code (4-digit)
}

/// Virtual card entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: Uuid,
    pub user_id: Uuid,
    pub card_type: CardType,
    pub status: CardStatus,
    pub encrypted_card_number: String, // Encrypted full card number
    pub masked_number: String, // Masked display (****-****-****-1234)
    pub cardholder_name: String,
    pub expiry_month: String,
    pub expiry_year: String,
    pub encrypted_cvv: String, // Encrypted CVV
    pub encrypted_pin: String, // Encrypted PIN
    pub currency: String,
    pub balance: Decimal,
    pub limits: CardLimits,
    pub design_id: String,
    pub linked_account_id: Option<Uuid>, // Linked fiat account
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub frozen_at: Option<DateTime<Utc>>,
    pub frozen_reason: Option<String>,
}

impl Card {
    /// Create a new virtual card
    pub fn new(
        user_id: Uuid,
        cardholder_name: String,
        currency: String,
        limits: Option<CardLimits>,
        design_id: Option<String>,
        linked_account_id: Option<Uuid>,
        is_primary: bool,
    ) -> Self {
        let card_id = Uuid::new_v4();
        let (card_number, masked_number) = Self::generate_card_number();
        let (expiry_month, expiry_year, expires_at) = Self::generate_expiry();
        let cvv = Self::generate_cvv();
        let pin = Self::generate_pin();

        Self {
            id: card_id,
            user_id,
            card_type: CardType::Virtual,
            status: CardStatus::Pending,
            encrypted_card_number: Self::encrypt_sensitive_data(&card_number),
            masked_number,
            cardholder_name,
            expiry_month,
            expiry_year,
            encrypted_cvv: Self::encrypt_sensitive_data(&cvv),
            encrypted_pin: Self::encrypt_sensitive_data(&pin),
            currency,
            balance: Decimal::ZERO,
            limits: limits.unwrap_or_default(),
            design_id: design_id.unwrap_or_else(|| "default".to_string()),
            linked_account_id,
            is_primary,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at,
            frozen_at: None,
            frozen_reason: None,
        }
    }

    /// Generate a virtual card number (16 digits)
    fn generate_card_number() -> (String, String) {
        let mut rng = thread_rng();
        
        // Use a test BIN (Bank Identification Number) for virtual cards
        // 4000 is a common test prefix for Visa
        let mut card_number = String::from("4000");
        
        // Generate 12 more digits
        for _ in 0..12 {
            card_number.push_str(&rng.gen_range(0..10).to_string());
        }
        
        // Create masked version
        let last_four = &card_number[12..];
        let masked = format!("****-****-****-{}", last_four);
        
        (card_number, masked)
    }

    /// Generate card expiry (3 years from now)
    fn generate_expiry() -> (String, String, DateTime<Utc>) {
        let expires_at = Utc::now() + chrono::Duration::days(3 * 365); // 3 years
        let expiry_month = format!("{:02}", expires_at.month());
        let expiry_year = format!("{:02}", expires_at.year() % 100);
        
        (expiry_month, expiry_year, expires_at)
    }

    /// Generate CVV (3 digits)
    fn generate_cvv() -> String {
        let mut rng = thread_rng();
        format!("{:03}", rng.gen_range(100..1000))
    }

    /// Generate PIN (4 digits)
    fn generate_pin() -> String {
        let mut rng = thread_rng();
        format!("{:04}", rng.gen_range(1000..10000))
    }

    /// Encrypt sensitive data (placeholder implementation)
    fn encrypt_sensitive_data(data: &str) -> String {
        // In production, use proper encryption with AES-256-GCM
        // For now, use a simple hash for demonstration
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Decrypt sensitive data (placeholder implementation)
    pub fn decrypt_sensitive_data(encrypted_data: &str) -> Result<String, String> {
        // In production, implement proper decryption
        // For now, return error as we can't reverse the hash
        Err("Decryption not implemented in demo".to_string())
    }

    /// Update card status
    pub fn update_status(&mut self, status: CardStatus, reason: Option<String>) {
        self.status = status.clone();
        self.updated_at = Utc::now();
        
        match status {
            CardStatus::Frozen => {
                self.frozen_at = Some(Utc::now());
                self.frozen_reason = reason;
            }
            CardStatus::Active => {
                self.frozen_at = None;
                self.frozen_reason = None;
            }
            _ => {}
        }
    }

    /// Update card limits
    pub fn update_limits(&mut self, limits: CardLimits) {
        self.limits = limits;
        self.updated_at = Utc::now();
    }

    /// Add funds to card balance
    pub fn add_balance(&mut self, amount: Decimal) -> Result<(), String> {
        if amount <= Decimal::ZERO {
            return Err("Amount must be positive".to_string());
        }
        
        self.balance += amount;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Deduct funds from card balance
    pub fn deduct_balance(&mut self, amount: Decimal) -> Result<(), String> {
        if amount <= Decimal::ZERO {
            return Err("Amount must be positive".to_string());
        }
        
        if self.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        
        self.balance -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if card can process a transaction
    pub fn can_process_transaction(&self, amount: Decimal) -> Result<(), String> {
        match self.status {
            CardStatus::Active => {}
            CardStatus::Frozen => return Err("Card is frozen".to_string()),
            CardStatus::Cancelled => return Err("Card is cancelled".to_string()),
            CardStatus::Expired => return Err("Card has expired".to_string()),
            CardStatus::Blocked => return Err("Card is blocked".to_string()),
            CardStatus::Pending => return Err("Card is not yet active".to_string()),
        }

        if self.balance < amount {
            return Err("Insufficient balance".to_string());
        }

        if amount > self.limits.per_transaction_limit {
            return Err("Transaction exceeds per-transaction limit".to_string());
        }

        Ok(())
    }
}

/// Card transaction entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTransaction {
    pub id: Uuid,
    pub card_id: Uuid,
    pub user_id: Uuid,
    pub transaction_type: CardTransactionType,
    pub status: CardTransactionStatus,
    pub amount: Decimal,
    pub currency: String,
    pub fee_amount: Decimal,
    pub net_amount: Decimal, // amount + fee_amount
    pub merchant: Option<MerchantInfo>,
    pub description: String,
    pub reference_number: String,
    pub authorization_code: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub authorized_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub decline_reason: Option<String>,
}

impl CardTransaction {
    /// Create a new card transaction
    pub fn new(
        card_id: Uuid,
        user_id: Uuid,
        transaction_type: CardTransactionType,
        amount: Decimal,
        currency: String,
        merchant: Option<MerchantInfo>,
        description: String,
    ) -> Self {
        let fee_amount = Self::calculate_fee(&transaction_type, amount);
        let net_amount = amount + fee_amount;

        Self {
            id: Uuid::new_v4(),
            card_id,
            user_id,
            transaction_type,
            status: CardTransactionStatus::Pending,
            amount,
            currency,
            fee_amount,
            net_amount,
            merchant,
            description,
            reference_number: Self::generate_reference_number(),
            authorization_code: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            authorized_at: None,
            settled_at: None,
            decline_reason: None,
        }
    }

    /// Calculate transaction fee
    fn calculate_fee(transaction_type: &CardTransactionType, amount: Decimal) -> Decimal {
        match transaction_type {
            CardTransactionType::Purchase => Decimal::ZERO, // No fee for purchases
            CardTransactionType::TopUp => amount * Decimal::from_str_exact("0.001").unwrap(), // 0.1% fee
            CardTransactionType::Fee => Decimal::ZERO, // Fee transactions don't have additional fees
            _ => Decimal::ZERO,
        }
    }

    /// Generate a reference number
    fn generate_reference_number() -> String {
        let mut rng = thread_rng();
        format!("FO3-{:08}", rng.gen_range(10000000..100000000))
    }

    /// Approve the transaction
    pub fn approve(&mut self, authorization_code: String) {
        self.status = CardTransactionStatus::Approved;
        self.authorization_code = Some(authorization_code);
        self.authorized_at = Some(Utc::now());
    }

    /// Decline the transaction
    pub fn decline(&mut self, reason: String) {
        self.status = CardTransactionStatus::Declined;
        self.decline_reason = Some(reason);
    }

    /// Settle the transaction
    pub fn settle(&mut self) {
        self.status = CardTransactionStatus::Settled;
        self.settled_at = Some(Utc::now());
    }
}

/// Card metrics for admin dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardMetrics {
    pub total_cards_issued: i64,
    pub active_cards: i64,
    pub frozen_cards: i64,
    pub cancelled_cards: i64,
    pub total_transaction_volume: Decimal,
    pub total_transactions: i64,
    pub average_transaction_amount: Decimal,
    pub declined_transactions: i64,
    pub decline_rate: f64,
    pub transactions_by_category: HashMap<String, i64>,
    pub volume_by_currency: HashMap<String, Decimal>,
}

impl Default for CardMetrics {
    fn default() -> Self {
        Self {
            total_cards_issued: 0,
            active_cards: 0,
            frozen_cards: 0,
            cancelled_cards: 0,
            total_transaction_volume: Decimal::ZERO,
            total_transactions: 0,
            average_transaction_amount: Decimal::ZERO,
            declined_transactions: 0,
            decline_rate: 0.0,
            transactions_by_category: HashMap::new(),
            volume_by_currency: HashMap::new(),
        }
    }
}

/// Card repository trait for data access
pub trait CardRepository: Send + Sync {
    /// Create a new card
    fn create_card(&self, card: Card) -> Result<Card, String>;
    
    /// Get card by ID
    fn get_card(&self, card_id: Uuid) -> Result<Option<Card>, String>;
    
    /// Get cards by user ID
    fn get_cards_by_user(&self, user_id: Uuid) -> Result<Vec<Card>, String>;
    
    /// Update card
    fn update_card(&self, card: Card) -> Result<Card, String>;
    
    /// Delete card (soft delete)
    fn delete_card(&self, card_id: Uuid) -> Result<(), String>;
    
    /// Create transaction
    fn create_transaction(&self, transaction: CardTransaction) -> Result<CardTransaction, String>;
    
    /// Get transaction by ID
    fn get_transaction(&self, transaction_id: Uuid) -> Result<Option<CardTransaction>, String>;
    
    /// Get transactions by card ID
    fn get_transactions_by_card(&self, card_id: Uuid) -> Result<Vec<CardTransaction>, String>;
    
    /// Update transaction
    fn update_transaction(&self, transaction: CardTransaction) -> Result<CardTransaction, String>;
    
    /// Get card metrics
    fn get_card_metrics(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<CardMetrics, String>;
    
    /// List all cards (admin)
    fn list_all_cards(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Card>, String>;
}

/// In-memory card repository implementation
pub struct InMemoryCardRepository {
    cards: Arc<RwLock<HashMap<Uuid, Card>>>,
    transactions: Arc<RwLock<HashMap<Uuid, CardTransaction>>>,
    user_cards: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // user_id -> card_ids
    card_transactions: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // card_id -> transaction_ids
}

impl InMemoryCardRepository {
    pub fn new() -> Self {
        Self {
            cards: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            user_cards: Arc::new(RwLock::new(HashMap::new())),
            card_transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CardRepository for InMemoryCardRepository {
    fn create_card(&self, card: Card) -> Result<Card, String> {
        let mut cards = self.cards.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_cards = self.user_cards.write().map_err(|_| "Failed to acquire write lock")?;
        
        let card_id = card.id;
        let user_id = card.user_id;
        
        cards.insert(card_id, card.clone());
        user_cards.entry(user_id).or_insert_with(Vec::new).push(card_id);
        
        Ok(card)
    }
    
    fn get_card(&self, card_id: Uuid) -> Result<Option<Card>, String> {
        let cards = self.cards.read().map_err(|_| "Failed to acquire read lock")?;
        Ok(cards.get(&card_id).cloned())
    }
    
    fn get_cards_by_user(&self, user_id: Uuid) -> Result<Vec<Card>, String> {
        let cards = self.cards.read().map_err(|_| "Failed to acquire read lock")?;
        let user_cards = self.user_cards.read().map_err(|_| "Failed to acquire read lock")?;
        
        let card_ids = user_cards.get(&user_id).unwrap_or(&Vec::new());
        let user_card_list = card_ids.iter()
            .filter_map(|id| cards.get(id).cloned())
            .collect();
            
        Ok(user_card_list)
    }
    
    fn update_card(&self, card: Card) -> Result<Card, String> {
        let mut cards = self.cards.write().map_err(|_| "Failed to acquire write lock")?;
        
        if !cards.contains_key(&card.id) {
            return Err("Card not found".to_string());
        }
        
        cards.insert(card.id, card.clone());
        Ok(card)
    }
    
    fn delete_card(&self, card_id: Uuid) -> Result<(), String> {
        let mut cards = self.cards.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_cards = self.user_cards.write().map_err(|_| "Failed to acquire write lock")?;
        
        if let Some(card) = cards.remove(&card_id) {
            // Remove from user's card list
            if let Some(user_card_list) = user_cards.get_mut(&card.user_id) {
                user_card_list.retain(|&id| id != card_id);
            }
        }
        
        Ok(())
    }
    
    fn create_transaction(&self, transaction: CardTransaction) -> Result<CardTransaction, String> {
        let mut transactions = self.transactions.write().map_err(|_| "Failed to acquire write lock")?;
        let mut card_transactions = self.card_transactions.write().map_err(|_| "Failed to acquire write lock")?;
        
        let transaction_id = transaction.id;
        let card_id = transaction.card_id;
        
        transactions.insert(transaction_id, transaction.clone());
        card_transactions.entry(card_id).or_insert_with(Vec::new).push(transaction_id);
        
        Ok(transaction)
    }
    
    fn get_transaction(&self, transaction_id: Uuid) -> Result<Option<CardTransaction>, String> {
        let transactions = self.transactions.read().map_err(|_| "Failed to acquire read lock")?;
        Ok(transactions.get(&transaction_id).cloned())
    }
    
    fn get_transactions_by_card(&self, card_id: Uuid) -> Result<Vec<CardTransaction>, String> {
        let transactions = self.transactions.read().map_err(|_| "Failed to acquire read lock")?;
        let card_transactions = self.card_transactions.read().map_err(|_| "Failed to acquire read lock")?;
        
        let transaction_ids = card_transactions.get(&card_id).unwrap_or(&Vec::new());
        let card_transaction_list = transaction_ids.iter()
            .filter_map(|id| transactions.get(id).cloned())
            .collect();
            
        Ok(card_transaction_list)
    }
    
    fn update_transaction(&self, transaction: CardTransaction) -> Result<CardTransaction, String> {
        let mut transactions = self.transactions.write().map_err(|_| "Failed to acquire write lock")?;
        
        if !transactions.contains_key(&transaction.id) {
            return Err("Transaction not found".to_string());
        }
        
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction)
    }
    
    fn get_card_metrics(&self, _start_date: DateTime<Utc>, _end_date: DateTime<Utc>) -> Result<CardMetrics, String> {
        let cards = self.cards.read().map_err(|_| "Failed to acquire read lock")?;
        let transactions = self.transactions.read().map_err(|_| "Failed to acquire read lock")?;
        
        let total_cards_issued = cards.len() as i64;
        let active_cards = cards.values().filter(|c| c.status == CardStatus::Active).count() as i64;
        let frozen_cards = cards.values().filter(|c| c.status == CardStatus::Frozen).count() as i64;
        let cancelled_cards = cards.values().filter(|c| c.status == CardStatus::Cancelled).count() as i64;
        
        let total_transactions = transactions.len() as i64;
        let total_transaction_volume = transactions.values()
            .map(|t| t.amount)
            .fold(Decimal::ZERO, |acc, amount| acc + amount);
        
        let average_transaction_amount = if total_transactions > 0 {
            total_transaction_volume / Decimal::from(total_transactions)
        } else {
            Decimal::ZERO
        };
        
        let declined_transactions = transactions.values()
            .filter(|t| t.status == CardTransactionStatus::Declined)
            .count() as i64;
        
        let decline_rate = if total_transactions > 0 {
            declined_transactions as f64 / total_transactions as f64
        } else {
            0.0
        };
        
        Ok(CardMetrics {
            total_cards_issued,
            active_cards,
            frozen_cards,
            cancelled_cards,
            total_transaction_volume,
            total_transactions,
            average_transaction_amount,
            declined_transactions,
            decline_rate,
            transactions_by_category: HashMap::new(), // TODO: Implement category aggregation
            volume_by_currency: HashMap::new(), // TODO: Implement currency aggregation
        })
    }
    
    fn list_all_cards(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Card>, String> {
        let cards = self.cards.read().map_err(|_| "Failed to acquire read lock")?;
        
        let mut all_cards: Vec<Card> = cards.values().cloned().collect();
        all_cards.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // Sort by creation date, newest first
        
        let start = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(start + limit, all_cards.len())
        } else {
            all_cards.len()
        };
        
        if start >= all_cards.len() {
            return Ok(Vec::new());
        }
        
        Ok(all_cards[start..end].to_vec())
    }
}

impl Default for InMemoryCardRepository {
    fn default() -> Self {
        Self::new()
    }
}
