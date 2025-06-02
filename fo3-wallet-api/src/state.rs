//! Application state management

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use fo3_wallet::{
    account::Wallet,
    transaction::provider::{ProviderConfig, ProviderType},
};

use crate::models::kyc::KycSubmission;
use crate::models::fiat_gateway::{BankAccount, FiatTransaction, TransactionLimits};
use crate::models::pricing::{PricingRepository, InMemoryPricingRepository};
use crate::models::notifications::{NotificationRepository, InMemoryNotificationRepository};
use crate::models::cards::{CardRepository, InMemoryCardRepository};
use crate::models::spending_insights::{SpendingInsightsRepository, InMemorySpendingInsightsRepository};
use crate::storage::{DocumentStorage, DocumentStorageConfig};
use base64::{Engine as _, engine::general_purpose};

/// Application state shared across gRPC services
pub struct AppState {
    /// In-memory wallet storage (in production, this would be a database)
    pub wallets: RwLock<HashMap<String, Wallet>>,
    /// Provider configuration for blockchain interactions
    pub provider_config: ProviderConfig,
    /// In-memory KYC submissions storage (in production, this would be a database)
    pub kyc_submissions: RwLock<HashMap<String, KycSubmission>>,
    /// Document storage service
    pub document_storage: Arc<DocumentStorage>,
    /// In-memory fiat accounts storage (in production, this would be a database)
    pub fiat_accounts: RwLock<HashMap<String, BankAccount>>,
    /// In-memory fiat transactions storage (in production, this would be a database)
    pub fiat_transactions: RwLock<HashMap<String, FiatTransaction>>,
    /// In-memory transaction limits storage (in production, this would be a database)
    pub fiat_limits: RwLock<HashMap<String, TransactionLimits>>,
    /// Pricing repository for price data and caching
    pub pricing_repository: Arc<dyn PricingRepository>,
    /// Notification repository for notifications and preferences
    pub notification_repository: Arc<dyn NotificationRepository>,
    /// Card repository for virtual card management
    pub card_repository: Arc<dyn CardRepository>,
    /// Spending insights repository for financial analytics
    pub spending_insights_repository: Arc<dyn SpendingInsightsRepository>,
}

impl AppState {
    pub fn new() -> Self {
        // Default to Ethereum mainnet
        let provider_config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: std::env::var("ETHEREUM_RPC_URL")
                .unwrap_or_else(|_| "https://mainnet.infura.io/v3/your-api-key".to_string()),
            api_key: std::env::var("ETHEREUM_API_KEY").ok(),
            timeout: Some(30),
        };

        // Initialize document storage
        let storage_config = DocumentStorageConfig {
            storage_path: std::env::var("KYC_STORAGE_PATH")
                .unwrap_or_else(|_| "./data/kyc_documents".to_string())
                .into(),
            max_file_size: std::env::var("KYC_MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse()
                .unwrap_or(10 * 1024 * 1024),
            encryption_key: Self::load_encryption_key(),
            ..Default::default()
        };

        let document_storage = Arc::new(
            DocumentStorage::new(storage_config)
                .expect("Failed to initialize document storage")
        );

        // Initialize pricing repository
        let pricing_repository: Arc<dyn PricingRepository> = Arc::new(InMemoryPricingRepository::new());

        // Initialize notification repository
        let notification_repository: Arc<dyn NotificationRepository> = Arc::new(InMemoryNotificationRepository::new());

        // Initialize card repository
        let card_repository: Arc<dyn CardRepository> = Arc::new(InMemoryCardRepository::new());

        // Initialize spending insights repository
        let spending_insights_repository: Arc<dyn SpendingInsightsRepository> = Arc::new(InMemorySpendingInsightsRepository::new());

        Self {
            wallets: RwLock::new(HashMap::new()),
            provider_config,
            kyc_submissions: RwLock::new(HashMap::new()),
            document_storage,
            fiat_accounts: RwLock::new(HashMap::new()),
            fiat_transactions: RwLock::new(HashMap::new()),
            fiat_limits: RwLock::new(HashMap::new()),
            pricing_repository,
            notification_repository,
            card_repository,
            spending_insights_repository,
        }
    }

    /// Load encryption key from environment or generate a default one
    fn load_encryption_key() -> [u8; 32] {
        if let Ok(key_str) = std::env::var("KYC_ENCRYPTION_KEY") {
            if let Ok(key_bytes) = general_purpose::STANDARD.decode(&key_str) {
                if key_bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&key_bytes);
                    return key;
                }
            }
        }

        // In production, this should be a proper key loaded from secure storage
        tracing::warn!("Using default encryption key - this is not secure for production!");
        [0u8; 32]
    }

    #[cfg(feature = "solana")]
    pub fn get_solana_config(&self) -> ProviderConfig {
        ProviderConfig {
            provider_type: ProviderType::Http,
            url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
            api_key: std::env::var("SOLANA_API_KEY").ok(),
            timeout: Some(30),
        }
    }

    pub fn get_bitcoin_config(&self) -> ProviderConfig {
        ProviderConfig {
            provider_type: ProviderType::Http,
            url: std::env::var("BITCOIN_RPC_URL")
                .unwrap_or_else(|_| "https://blockstream.info/api".to_string()),
            api_key: std::env::var("BITCOIN_API_KEY").ok(),
            timeout: Some(30),
        }
    }

    pub fn add_wallet(&self, wallet: Wallet) -> Result<(), String> {
        let id = wallet.id().to_string();
        let mut wallets = self.wallets.write().unwrap();
        if wallets.contains_key(&id) {
            return Err("Wallet already exists".to_string());
        }
        wallets.insert(id, wallet);
        Ok(())
    }

    pub fn get_wallet(&self, id: &str) -> Option<Wallet> {
        self.wallets.read().unwrap().get(id).cloned()
    }

    pub fn get_all_wallets(&self) -> Vec<Wallet> {
        self.wallets.read().unwrap().values().cloned().collect()
    }

    pub fn remove_wallet(&self, id: &str) -> bool {
        let mut wallets = self.wallets.write().unwrap();
        wallets.remove(id).is_some()
    }
}
