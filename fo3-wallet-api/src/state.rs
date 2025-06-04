//! Application state management

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use fo3_wallet::{
    account::Wallet,
    transaction::provider::{ProviderConfig, ProviderType},
};

use crate::models::kyc::{KycSubmission, KycRepository};
use crate::models::fiat_gateway::{BankAccount, FiatTransaction, TransactionLimits};
use crate::models::pricing::{PricingRepository, InMemoryPricingRepository};
use crate::models::notifications::{NotificationRepository, InMemoryNotificationRepository};
use crate::models::cards::{CardRepository, InMemoryCardRepository};
use crate::models::spending_insights::{SpendingInsightsRepository, InMemorySpendingInsightsRepository};
use crate::storage::{DocumentStorage, DocumentStorageConfig};
use crate::database::connection::DatabasePool;
use crate::database::repositories::{SqlxKycRepository, SqlxWalletRepository, SqlxCardRepository, SqlxFiatRepository};
use crate::database::repositories::wallet_repository::WalletRepository;
use crate::services::integration::{ServiceCoordinator, TransactionManager, EventDispatcher, HealthMonitor};
use base64::{Engine as _, engine::general_purpose};

/// Application state shared across gRPC services
pub struct AppState {
    /// Database connection pool
    pub database_pool: DatabasePool,
    /// Provider configuration for blockchain interactions
    pub provider_config: ProviderConfig,
    /// Document storage service
    pub document_storage: Arc<DocumentStorage>,
    /// KYC repository for KYC submissions and documents
    pub kyc_repository: Arc<dyn KycRepository<Error = crate::error::ServiceError>>,
    /// Wallet repository for wallet management
    pub wallet_repository: Arc<dyn WalletRepository<Error = crate::error::ServiceError>>,
    /// Pricing repository for price data and caching
    pub pricing_repository: Arc<dyn PricingRepository>,
    /// Notification repository for notifications and preferences
    pub notification_repository: Arc<dyn NotificationRepository>,
    /// Card repository for virtual card management
    pub card_repository: Arc<dyn CardRepository>,
    /// Spending insights repository for financial analytics
    pub spending_insights_repository: Arc<dyn SpendingInsightsRepository>,
    /// Fiat repository for banking operations
    pub fiat_repository: Arc<SqlxFiatRepository>,

    // Phase 3: Service Integration & Real-time Features
    /// Service coordinator for cross-service operations
    pub service_coordinator: Arc<ServiceCoordinator>,
    /// Transaction manager for distributed transactions
    pub transaction_manager: Arc<TransactionManager>,
    /// Event dispatcher for real-time notifications
    pub event_dispatcher: Arc<EventDispatcher>,
    /// Health monitor for service monitoring
    pub health_monitor: Arc<HealthMonitor>,

    // Legacy in-memory storage for backward compatibility during migration
    /// In-memory wallet storage (deprecated - use wallet_repository)
    pub wallets: RwLock<HashMap<String, Wallet>>,
    /// In-memory KYC submissions storage (deprecated - use kyc_repository)
    pub kyc_submissions: RwLock<HashMap<String, KycSubmission>>,
    /// In-memory fiat accounts storage (deprecated - use fiat_repository)
    pub fiat_accounts: RwLock<HashMap<String, BankAccount>>,
    /// In-memory fiat transactions storage (deprecated - use fiat_repository)
    pub fiat_transactions: RwLock<HashMap<String, FiatTransaction>>,
    /// In-memory transaction limits storage (deprecated - use fiat_repository)
    pub fiat_limits: RwLock<HashMap<String, TransactionLimits>>,
}

impl AppState {
    pub fn new(database_pool: DatabasePool) -> Self {
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

        // Initialize database-backed repositories
        let kyc_repository: Arc<dyn KycRepository<Error = crate::error::ServiceError>> = Arc::new(SqlxKycRepository::new(database_pool.clone()));
        let wallet_repository: Arc<dyn WalletRepository<Error = crate::error::ServiceError>> = Arc::new(SqlxWalletRepository::new(database_pool.clone()));
        let fiat_repository = Arc::new(SqlxFiatRepository::new(database_pool.clone()));

        // Initialize Phase 3 integration services
        let event_dispatcher = Arc::new(EventDispatcher::new());
        let transaction_manager = Arc::new(TransactionManager::new(database_pool.clone()));
        let health_monitor = Arc::new(HealthMonitor::new(
            database_pool.clone(),
            event_dispatcher.clone(),
            None, // Use default config
        ));

        // Note: ServiceCoordinator will be initialized after AppState creation to avoid circular dependency

        Self {
            database_pool,
            provider_config,
            document_storage,
            kyc_repository,
            wallet_repository,
            pricing_repository,
            notification_repository,
            card_repository,
            spending_insights_repository,
            fiat_repository,
            service_coordinator: Arc::new(ServiceCoordinator::new(Arc::new(AppState::create_placeholder()))),
            transaction_manager,
            event_dispatcher,
            health_monitor,

            // Legacy in-memory storage for backward compatibility
            wallets: RwLock::new(HashMap::new()),
            kyc_submissions: RwLock::new(HashMap::new()),
            fiat_accounts: RwLock::new(HashMap::new()),
            fiat_transactions: RwLock::new(HashMap::new()),
            fiat_limits: RwLock::new(HashMap::new()),
        }
    }

    /// Create minimal state for service coordinator initialization
    fn create_minimal_state(database_pool: DatabasePool) -> AppState {
        // Create a minimal AppState without service coordinator to avoid circular dependency
        // The service coordinator will be properly initialized later
        let kyc_repository: Arc<dyn KycRepository<Error = crate::error::ServiceError>> = Arc::new(SqlxKycRepository::new(database_pool.clone()));
        let wallet_repository: Arc<dyn WalletRepository<Error = crate::error::ServiceError>> = Arc::new(SqlxWalletRepository::new(database_pool.clone()));

        // Create a dummy service coordinator that will be replaced
        let dummy_state = AppState {
            database_pool: database_pool.clone(),
            provider_config: ProviderConfig::default(),
            document_storage: Arc::new(DocumentStorage::new(DocumentStorageConfig::default())),
            kyc_repository: kyc_repository.clone(),
            wallet_repository: wallet_repository.clone(),
            pricing_repository: Arc::new(InMemoryPricingRepository::new()),
            notification_repository: Arc::new(InMemoryNotificationRepository::new()),
            card_repository: Arc::new(InMemoryCardRepository::new()),
            spending_insights_repository: Arc::new(InMemorySpendingInsightsRepository::new()),
            fiat_repository: Arc::new(SqlxFiatRepository::new(database_pool.clone())),
            service_coordinator: Arc::new(ServiceCoordinator::new(Arc::new(AppState::default()))), // Temporary placeholder
            transaction_manager: Arc::new(TransactionManager::new(database_pool.clone())),
            event_dispatcher: Arc::new(EventDispatcher::new()),
            health_monitor: Arc::new(HealthMonitor::new(database_pool.clone(), Arc::new(EventDispatcher::new()), None)),
            wallets: RwLock::new(HashMap::new()),
            kyc_submissions: RwLock::new(HashMap::new()),
            fiat_accounts: RwLock::new(HashMap::new()),
            fiat_transactions: RwLock::new(HashMap::new()),
            fiat_limits: RwLock::new(HashMap::new()),
        };

        dummy_state
    }

    /// Create a placeholder AppState for initialization
    fn create_placeholder() -> AppState {
        // This is a temporary placeholder to avoid circular dependencies
        // In a real implementation, you might use a different pattern like dependency injection
        panic!("Placeholder AppState should not be used directly")
    }
}

// Note: Default implementation removed due to async requirements

impl AppState {
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
