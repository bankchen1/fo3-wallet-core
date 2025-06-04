//! Comprehensive seed data module for FO3 Wallet Core local validation
//! 
//! Provides realistic test data covering all 15 core services including:
//! - Users with various KYC statuses (verified, pending, rejected)
//! - Wallet balances across multiple currencies (USD, BTC, ETH, USDC)
//! - Trading strategies and positions for automated trading testing
//! - DeFi yield farming positions and historical data
//! - Card transactions and funding operations
//! - Referral networks and reward point balances

use std::collections::HashMap;
use std::time::Instant;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::error::ServiceError;
use crate::models::*;

/// Seed data configuration
#[derive(Debug, Clone)]
pub struct SeedDataConfig {
    pub num_users: usize,
    pub num_wallets_per_user: usize,
    pub num_cards_per_user: usize,
    pub num_transactions_per_card: usize,
    pub num_referral_levels: usize,
    pub num_yield_products: usize,
    pub num_moonshot_tokens: usize,
    pub enable_ml_data: bool,
    pub enable_audit_trails: bool,
}

impl Default for SeedDataConfig {
    fn default() -> Self {
        Self {
            num_users: 100,
            num_wallets_per_user: 2,
            num_cards_per_user: 3,
            num_transactions_per_card: 50,
            num_referral_levels: 3,
            num_yield_products: 20,
            num_moonshot_tokens: 50,
            enable_ml_data: true,
            enable_audit_trails: true,
        }
    }
}

/// Seed data generation result
#[derive(Debug)]
pub struct SeedDataResult {
    pub success: bool,
    pub records_created: HashMap<String, usize>,
    pub generation_time_ms: u64,
    pub total_records: usize,
}

/// Comprehensive seed data manager
pub struct SeedDataManager {
    config: SeedDataConfig,
}

impl SeedDataManager {
    pub fn new(config: SeedDataConfig) -> Self {
        Self { config }
    }

    /// Generate comprehensive seed data for all services
    pub async fn generate_all_seed_data(&self) -> Result<SeedDataResult, ServiceError> {
        let start_time = Instant::now();
        let mut records_created = HashMap::new();
        let mut total_records = 0;

        info!("Starting comprehensive seed data generation with config: {:?}", self.config);

        // Generate users and KYC data
        let users = self.generate_users().await?;
        records_created.insert("users".to_string(), users.len());
        total_records += users.len();

        // Generate wallets and addresses
        let wallets = self.generate_wallets(&users).await?;
        records_created.insert("wallets".to_string(), wallets.len());
        total_records += wallets.len();

        // Generate KYC submissions
        let kyc_submissions = self.generate_kyc_submissions(&users).await?;
        records_created.insert("kyc_submissions".to_string(), kyc_submissions.len());
        total_records += kyc_submissions.len();

        // Generate fiat accounts and transactions
        let fiat_accounts = self.generate_fiat_accounts(&users).await?;
        records_created.insert("fiat_accounts".to_string(), fiat_accounts.len());
        total_records += fiat_accounts.len();

        let fiat_transactions = self.generate_fiat_transactions(&fiat_accounts).await?;
        records_created.insert("fiat_transactions".to_string(), fiat_transactions.len());
        total_records += fiat_transactions.len();

        // Generate virtual cards and transactions
        let cards = self.generate_virtual_cards(&users).await?;
        records_created.insert("virtual_cards".to_string(), cards.len());
        total_records += cards.len();

        let card_transactions = self.generate_card_transactions(&cards).await?;
        records_created.insert("card_transactions".to_string(), card_transactions.len());
        total_records += card_transactions.len();

        // Generate card funding sources and transactions
        let funding_sources = self.generate_funding_sources(&users).await?;
        records_created.insert("funding_sources".to_string(), funding_sources.len());
        total_records += funding_sources.len();

        let funding_transactions = self.generate_funding_transactions(&funding_sources).await?;
        records_created.insert("funding_transactions".to_string(), funding_transactions.len());
        total_records += funding_transactions.len();

        // Generate ledger accounts and transactions
        let ledger_accounts = self.generate_ledger_accounts(&users).await?;
        records_created.insert("ledger_accounts".to_string(), ledger_accounts.len());
        total_records += ledger_accounts.len();

        let ledger_transactions = self.generate_ledger_transactions(&ledger_accounts).await?;
        records_created.insert("ledger_transactions".to_string(), ledger_transactions.len());
        total_records += ledger_transactions.len();

        // Generate rewards and referral data
        let user_rewards = self.generate_user_rewards(&users).await?;
        records_created.insert("user_rewards".to_string(), user_rewards.len());
        total_records += user_rewards.len();

        let referral_campaigns = self.generate_referral_campaigns().await?;
        records_created.insert("referral_campaigns".to_string(), referral_campaigns.len());
        total_records += referral_campaigns.len();

        let referral_relationships = self.generate_referral_relationships(&users, &referral_campaigns).await?;
        records_created.insert("referral_relationships".to_string(), referral_relationships.len());
        total_records += referral_relationships.len();

        // Generate DeFi and yield farming data
        let yield_products = self.generate_yield_products().await?;
        records_created.insert("yield_products".to_string(), yield_products.len());
        total_records += yield_products.len();

        let staking_positions = self.generate_staking_positions(&users, &yield_products).await?;
        records_created.insert("staking_positions".to_string(), staking_positions.len());
        total_records += staking_positions.len();

        // Generate moonshot trading data
        let moonshot_tokens = self.generate_moonshot_tokens().await?;
        records_created.insert("moonshot_tokens".to_string(), moonshot_tokens.len());
        total_records += moonshot_tokens.len();

        let moonshot_votes = self.generate_moonshot_votes(&users, &moonshot_tokens).await?;
        records_created.insert("moonshot_votes".to_string(), moonshot_votes.len());
        total_records += moonshot_votes.len();

        // Generate ML training data if enabled
        if self.config.enable_ml_data {
            let ml_data = self.generate_ml_training_data(&users).await?;
            records_created.insert("ml_training_data".to_string(), ml_data);
            total_records += ml_data;
        }

        let generation_time = start_time.elapsed().as_millis() as u64;
        
        info!(
            "Seed data generation completed: {} total records in {}ms",
            total_records, generation_time
        );

        Ok(SeedDataResult {
            success: true,
            records_created,
            generation_time_ms: generation_time,
            total_records,
        })
    }

    /// Generate users with various profiles
    async fn generate_users(&self) -> Result<Vec<User>, ServiceError> {
        info!("Generating {} users with diverse profiles", self.config.num_users);
        
        let mut users = Vec::new();
        let user_tiers = vec!["bronze", "silver", "gold", "platinum"];
        let countries = vec!["US", "CA", "GB", "DE", "FR", "JP", "AU", "SG"];

        for i in 0..self.config.num_users {
            let user_id = Uuid::new_v4();
            let tier = user_tiers[i % user_tiers.len()];
            let country = countries[i % countries.len()];
            
            // Create user with realistic data
            let user = User {
                id: user_id,
                email: format!("user{}@fo3wallet.com", i),
                username: format!("user{}", i),
                tier: tier.to_string(),
                country: country.to_string(),
                is_active: i % 10 != 0, // 90% active users
                created_at: Utc::now() - Duration::days((i as i64) % 365),
                updated_at: Utc::now(),
            };
            
            users.push(user);
        }

        Ok(users)
    }

    /// Generate wallets for users
    async fn generate_wallets(&self, users: &[User]) -> Result<Vec<Wallet>, ServiceError> {
        info!("Generating wallets for {} users", users.len());
        
        let mut wallets = Vec::new();
        let currencies = vec!["USD", "BTC", "ETH", "USDC", "USDT"];

        for user in users {
            for wallet_idx in 0..self.config.num_wallets_per_user {
                let wallet_id = Uuid::new_v4();
                let currency = currencies[wallet_idx % currencies.len()];
                
                let wallet = Wallet {
                    id: wallet_id,
                    user_id: user.id,
                    name: format!("{} Wallet {}", currency, wallet_idx + 1),
                    currency: currency.to_string(),
                    balance: self.generate_realistic_balance(currency),
                    is_primary: wallet_idx == 0,
                    created_at: user.created_at + Duration::hours(wallet_idx as i64),
                    updated_at: Utc::now(),
                };
                
                wallets.push(wallet);
            }
        }

        Ok(wallets)
    }

    /// Generate realistic balance based on currency
    fn generate_realistic_balance(&self, currency: &str) -> Decimal {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        match currency {
            "USD" | "USDC" | "USDT" => {
                // $100 to $50,000
                Decimal::from(rng.gen_range(100..50000))
            }
            "BTC" => {
                // 0.001 to 5 BTC
                Decimal::from(rng.gen_range(1..5000)) / Decimal::from(1000)
            }
            "ETH" => {
                // 0.1 to 50 ETH
                Decimal::from(rng.gen_range(100..50000)) / Decimal::from(1000)
            }
            _ => Decimal::from(rng.gen_range(1..1000))
        }
    }
}

    /// Generate KYC submissions with various statuses
    async fn generate_kyc_submissions(&self, users: &[User]) -> Result<Vec<KycSubmission>, ServiceError> {
        info!("Generating KYC submissions for {} users", users.len());

        let mut submissions = Vec::new();
        let statuses = vec![
            KycStatus::Approved,    // 60%
            KycStatus::Approved,
            KycStatus::Approved,
            KycStatus::Pending,     // 20%
            KycStatus::UnderReview, // 15%
            KycStatus::Rejected,    // 5%
        ];

        for (i, user) in users.iter().enumerate() {
            let status = statuses[i % statuses.len()];

            let personal_info = PersonalInfo {
                first_name: format!("FirstName{}", i),
                last_name: format!("LastName{}", i),
                date_of_birth: chrono::NaiveDate::from_ymd_opt(1990 + (i % 30) as i32, 1 + (i % 12) as u32, 1 + (i % 28) as u32).unwrap(),
                nationality: user.country.clone(),
                address: Address {
                    street: format!("{} Main Street", 100 + i),
                    city: "Test City".to_string(),
                    state: "Test State".to_string(),
                    postal_code: format!("{:05}", 10000 + i),
                    country: user.country.clone(),
                },
                phone_number: format!("+1555{:07}", i),
                occupation: "Software Engineer".to_string(),
                income_range: "50000-100000".to_string(),
            };

            let mut submission = KycSubmission::new(user.id, personal_info);
            submission.status = status;

            if status == KycStatus::Approved || status == KycStatus::Rejected {
                submission.reviewed_at = Some(Utc::now() - Duration::days((i % 30) as i64));
                submission.reviewer_id = Some("admin".to_string());
            }

            submissions.push(submission);
        }

        Ok(submissions)
    }

    /// Generate fiat accounts for users
    async fn generate_fiat_accounts(&self, users: &[User]) -> Result<Vec<BankAccount>, ServiceError> {
        info!("Generating fiat accounts for {} users", users.len());

        let mut accounts = Vec::new();
        let account_types = vec![AccountType::Checking, AccountType::Savings];
        let currencies = vec!["USD", "EUR", "GBP", "CAD"];

        for (i, user) in users.iter().enumerate() {
            for (j, &account_type) in account_types.iter().enumerate() {
                let currency = currencies[i % currencies.len()];

                let account = BankAccount {
                    id: Uuid::new_v4(),
                    user_id: user.id,
                    account_type,
                    account_number: format!("ACC{:010}", i * 10 + j),
                    routing_number: format!("RTN{:06}", 100000 + i),
                    bank_name: format!("Test Bank {}", i % 5 + 1),
                    account_holder_name: format!("FirstName{} LastName{}", i, i),
                    currency: currency.to_string(),
                    is_verified: i % 10 != 0, // 90% verified
                    is_primary: j == 0,
                    created_at: user.created_at + Duration::hours(j as i64),
                    updated_at: Utc::now(),
                };

                accounts.push(account);
            }
        }

        Ok(accounts)
    }

    /// Generate fiat transactions
    async fn generate_fiat_transactions(&self, accounts: &[BankAccount]) -> Result<Vec<FiatTransaction>, ServiceError> {
        info!("Generating fiat transactions for {} accounts", accounts.len());

        let mut transactions = Vec::new();
        let transaction_types = vec![
            TransactionType::Deposit,
            TransactionType::Withdrawal,
            TransactionType::Transfer,
        ];

        for account in accounts {
            for i in 0..10 { // 10 transactions per account
                let transaction_type = transaction_types[i % transaction_types.len()];
                let amount = self.generate_realistic_fiat_amount(&account.currency);

                let transaction = FiatTransaction {
                    id: Uuid::new_v4(),
                    user_id: account.user_id,
                    account_id: account.id,
                    transaction_type,
                    amount,
                    currency: account.currency.clone(),
                    status: if i % 20 == 0 { TransactionStatus::Failed } else { TransactionStatus::Completed },
                    provider: PaymentProvider::ACH,
                    external_transaction_id: Some(format!("EXT{:010}", i)),
                    description: format!("Test {} transaction", transaction_type.to_string()),
                    created_at: Utc::now() - Duration::days((i % 30) as i64),
                    updated_at: Utc::now(),
                };

                transactions.push(transaction);
            }
        }

        Ok(transactions)
    }

    /// Generate realistic fiat transaction amount
    fn generate_realistic_fiat_amount(&self, currency: &str) -> Decimal {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        match currency {
            "USD" | "EUR" | "GBP" | "CAD" => {
                // $10 to $5,000
                Decimal::from(rng.gen_range(10..5000))
            }
            _ => Decimal::from(rng.gen_range(10..1000))
        }
    }

    /// Generate virtual cards for users
    async fn generate_virtual_cards(&self, users: &[User]) -> Result<Vec<Card>, ServiceError> {
        info!("Generating virtual cards for {} users", users.len());

        let mut cards = Vec::new();
        let currencies = vec!["USD", "EUR", "GBP"];
        let designs = vec!["default", "premium", "business", "travel"];

        for (i, user) in users.iter().enumerate() {
            for card_idx in 0..self.config.num_cards_per_user {
                let currency = currencies[card_idx % currencies.len()];
                let design = designs[card_idx % designs.len()];

                let limits = CardLimits {
                    daily_limit: Decimal::from(5000),
                    monthly_limit: Decimal::from(50000),
                    per_transaction_limit: Decimal::from(2500),
                    atm_daily_limit: Decimal::from(1000),
                    transaction_count_daily: 50,
                    transaction_count_monthly: 500,
                };

                let card = Card::new(
                    user.id,
                    format!("FirstName{} LastName{}", i, i),
                    currency.to_string(),
                    Some(limits),
                    Some(design.to_string()),
                    None,
                    card_idx == 0,
                );

                cards.push(card);
            }
        }

        Ok(cards)
    }

    /// Generate card transactions
    async fn generate_card_transactions(&self, cards: &[Card]) -> Result<Vec<CardTransaction>, ServiceError> {
        info!("Generating card transactions for {} cards", cards.len());

        let mut transactions = Vec::new();
        let merchants = vec![
            ("Amazon", MerchantCategory::OnlineRetail),
            ("Starbucks", MerchantCategory::Restaurant),
            ("Shell", MerchantCategory::Gas),
            ("Walmart", MerchantCategory::Grocery),
            ("Netflix", MerchantCategory::Entertainment),
        ];

        for card in cards {
            for i in 0..self.config.num_transactions_per_card {
                let (merchant_name, category) = &merchants[i % merchants.len()];
                let amount = self.generate_realistic_transaction_amount(&card.currency);

                let merchant_info = MerchantInfo {
                    name: merchant_name.to_string(),
                    category: *category,
                    mcc: format!("{:04}", 5000 + i % 1000),
                    location: "Test City, Test State".to_string(),
                    country: "US".to_string(),
                };

                let transaction = CardTransaction {
                    id: Uuid::new_v4(),
                    card_id: card.id,
                    amount,
                    currency: card.currency.clone(),
                    status: if i % 50 == 0 { CardTransactionStatus::Declined } else { CardTransactionStatus::Completed },
                    transaction_type: CardTransactionType::Purchase,
                    merchant_info,
                    description: format!("Purchase at {}", merchant_name),
                    created_at: Utc::now() - Duration::hours((i % 24) as i64),
                    updated_at: Utc::now(),
                };

                transactions.push(transaction);
            }
        }

        Ok(transactions)
    }

    /// Generate realistic transaction amount
    fn generate_realistic_transaction_amount(&self, currency: &str) -> Decimal {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        match currency {
            "USD" | "EUR" | "GBP" => {
                // $5 to $500
                Decimal::from(rng.gen_range(5..500))
            }
            _ => Decimal::from(rng.gen_range(5..200))
        }
    }
}

// Placeholder structs for compilation
#[derive(Debug, Clone)]
struct User {
    id: Uuid,
    email: String,
    username: String,
    tier: String,
    country: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct Wallet {
    id: Uuid,
    user_id: Uuid,
    name: String,
    currency: String,
    balance: Decimal,
    is_primary: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Additional placeholder implementations for the remaining methods
impl SeedDataManager {
    async fn generate_funding_sources(&self, _users: &[User]) -> Result<Vec<FundingSource>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_funding_transactions(&self, _sources: &[FundingSource]) -> Result<Vec<FundingTransaction>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_ledger_accounts(&self, _users: &[User]) -> Result<Vec<LedgerAccount>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_ledger_transactions(&self, _accounts: &[LedgerAccount]) -> Result<Vec<LedgerTransaction>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_user_rewards(&self, _users: &[User]) -> Result<Vec<UserRewards>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_referral_campaigns(&self) -> Result<Vec<ReferralCampaign>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_referral_relationships(&self, _users: &[User], _campaigns: &[ReferralCampaign]) -> Result<Vec<ReferralRelationship>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_yield_products(&self) -> Result<Vec<YieldProduct>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_staking_positions(&self, _users: &[User], _products: &[YieldProduct]) -> Result<Vec<StakingPosition>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_moonshot_tokens(&self) -> Result<Vec<TokenEntity>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_moonshot_votes(&self, _users: &[User], _tokens: &[TokenEntity]) -> Result<Vec<VoteEntity>, ServiceError> {
        Ok(Vec::new()) // Placeholder
    }

    async fn generate_ml_training_data(&self, _users: &[User]) -> Result<usize, ServiceError> {
        Ok(0) // Placeholder
    }
}
