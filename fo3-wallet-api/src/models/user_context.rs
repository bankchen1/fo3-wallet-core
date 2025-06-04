//! User Context and Multi-User Architecture
//!
//! Provides user isolation, authentication context, and RBAC enforcement
//! for production-ready multi-user database operations.

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

/// User context for database operations with isolation and RBAC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub permissions: HashSet<Permission>,
    pub tier: UserTier,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
}

/// User roles for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserRole {
    BasicUser,
    PremiumUser,
    Admin,
    SuperAdmin,
    Viewer,
}

/// User tier for financial limits and features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

/// Granular permissions for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Wallet permissions
    WalletCreate,
    WalletRead,
    WalletUpdate,
    WalletDelete,
    
    // Transaction permissions
    TransactionCreate,
    TransactionRead,
    TransactionUpdate,
    TransactionCancel,
    
    // Card permissions
    CardCreate,
    CardRead,
    CardUpdate,
    CardDelete,
    CardFreeze,
    CardUnfreeze,
    
    // KYC permissions
    KycSubmit,
    KycRead,
    KycUpdate,
    KycApprove,
    KycReject,
    
    // Fiat gateway permissions
    FiatDeposit,
    FiatWithdraw,
    FiatRead,
    BankAccountAdd,
    BankAccountRead,
    BankAccountDelete,
    
    // Admin permissions
    UserManagement,
    SystemConfiguration,
    AuditLogRead,
    FinancialReporting,
    
    // Trading permissions
    TradingBasic,
    TradingAdvanced,
    TradingAdmin,
    
    // DeFi permissions
    DefiBasic,
    DefiAdvanced,
    DefiAdmin,
}

/// Financial limits based on user tier and role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimits {
    pub daily_transaction_limit_usd: f64,
    pub monthly_transaction_limit_usd: f64,
    pub single_transaction_limit_usd: f64,
    pub daily_withdrawal_limit_usd: f64,
    pub monthly_withdrawal_limit_usd: f64,
    pub card_daily_limit_usd: f64,
    pub card_monthly_limit_usd: f64,
}

impl UserContext {
    /// Create a new user context
    pub fn new(
        user_id: Uuid,
        username: String,
        email: String,
        role: UserRole,
        tier: UserTier,
    ) -> Self {
        let permissions = Self::get_default_permissions(role);
        
        Self {
            user_id,
            username,
            email,
            role,
            permissions,
            tier,
            is_active: true,
            created_at: Utc::now(),
            last_login: None,
            session_id: None,
        }
    }
    
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.is_active && self.permissions.contains(&permission)
    }
    
    /// Check if user can access another user's data (admin only)
    pub fn can_access_user_data(&self, target_user_id: Uuid) -> bool {
        if !self.is_active {
            return false;
        }
        
        // Users can always access their own data
        if self.user_id == target_user_id {
            return true;
        }
        
        // Only admins can access other users' data
        matches!(self.role, UserRole::Admin | UserRole::SuperAdmin)
    }
    
    /// Get financial limits for the user
    pub fn get_limits(&self) -> UserLimits {
        match (self.role, self.tier) {
            (UserRole::BasicUser, UserTier::Bronze) => UserLimits {
                daily_transaction_limit_usd: 1000.0,
                monthly_transaction_limit_usd: 10000.0,
                single_transaction_limit_usd: 500.0,
                daily_withdrawal_limit_usd: 500.0,
                monthly_withdrawal_limit_usd: 5000.0,
                card_daily_limit_usd: 1000.0,
                card_monthly_limit_usd: 10000.0,
            },
            (UserRole::BasicUser, UserTier::Silver) => UserLimits {
                daily_transaction_limit_usd: 2500.0,
                monthly_transaction_limit_usd: 25000.0,
                single_transaction_limit_usd: 1000.0,
                daily_withdrawal_limit_usd: 1000.0,
                monthly_withdrawal_limit_usd: 10000.0,
                card_daily_limit_usd: 2500.0,
                card_monthly_limit_usd: 25000.0,
            },
            (UserRole::PremiumUser, UserTier::Gold) => UserLimits {
                daily_transaction_limit_usd: 10000.0,
                monthly_transaction_limit_usd: 100000.0,
                single_transaction_limit_usd: 5000.0,
                daily_withdrawal_limit_usd: 5000.0,
                monthly_withdrawal_limit_usd: 50000.0,
                card_daily_limit_usd: 10000.0,
                card_monthly_limit_usd: 100000.0,
            },
            (UserRole::PremiumUser, UserTier::Platinum) => UserLimits {
                daily_transaction_limit_usd: 50000.0,
                monthly_transaction_limit_usd: 500000.0,
                single_transaction_limit_usd: 25000.0,
                daily_withdrawal_limit_usd: 25000.0,
                monthly_withdrawal_limit_usd: 250000.0,
                card_daily_limit_usd: 50000.0,
                card_monthly_limit_usd: 500000.0,
            },
            (UserRole::Admin | UserRole::SuperAdmin, _) => UserLimits {
                daily_transaction_limit_usd: 1000000.0,
                monthly_transaction_limit_usd: 10000000.0,
                single_transaction_limit_usd: 500000.0,
                daily_withdrawal_limit_usd: 500000.0,
                monthly_withdrawal_limit_usd: 5000000.0,
                card_daily_limit_usd: 1000000.0,
                card_monthly_limit_usd: 10000000.0,
            },
            _ => UserLimits {
                daily_transaction_limit_usd: 100.0,
                monthly_transaction_limit_usd: 1000.0,
                single_transaction_limit_usd: 50.0,
                daily_withdrawal_limit_usd: 50.0,
                monthly_withdrawal_limit_usd: 500.0,
                card_daily_limit_usd: 100.0,
                card_monthly_limit_usd: 1000.0,
            },
        }
    }
    
    /// Get default permissions for a role
    fn get_default_permissions(role: UserRole) -> HashSet<Permission> {
        let mut permissions = HashSet::new();
        
        match role {
            UserRole::BasicUser => {
                permissions.extend([
                    Permission::WalletCreate,
                    Permission::WalletRead,
                    Permission::WalletUpdate,
                    Permission::TransactionCreate,
                    Permission::TransactionRead,
                    Permission::CardCreate,
                    Permission::CardRead,
                    Permission::CardUpdate,
                    Permission::CardFreeze,
                    Permission::KycSubmit,
                    Permission::KycRead,
                    Permission::FiatDeposit,
                    Permission::FiatWithdraw,
                    Permission::FiatRead,
                    Permission::BankAccountAdd,
                    Permission::BankAccountRead,
                    Permission::TradingBasic,
                    Permission::DefiBasic,
                ]);
            },
            UserRole::PremiumUser => {
                permissions.extend([
                    Permission::WalletCreate,
                    Permission::WalletRead,
                    Permission::WalletUpdate,
                    Permission::TransactionCreate,
                    Permission::TransactionRead,
                    Permission::TransactionCancel,
                    Permission::CardCreate,
                    Permission::CardRead,
                    Permission::CardUpdate,
                    Permission::CardFreeze,
                    Permission::CardUnfreeze,
                    Permission::KycSubmit,
                    Permission::KycRead,
                    Permission::KycUpdate,
                    Permission::FiatDeposit,
                    Permission::FiatWithdraw,
                    Permission::FiatRead,
                    Permission::BankAccountAdd,
                    Permission::BankAccountRead,
                    Permission::BankAccountDelete,
                    Permission::TradingBasic,
                    Permission::TradingAdvanced,
                    Permission::DefiBasic,
                    Permission::DefiAdvanced,
                ]);
            },
            UserRole::Admin => {
                permissions.extend([
                    Permission::WalletCreate,
                    Permission::WalletRead,
                    Permission::WalletUpdate,
                    Permission::WalletDelete,
                    Permission::TransactionCreate,
                    Permission::TransactionRead,
                    Permission::TransactionUpdate,
                    Permission::TransactionCancel,
                    Permission::CardCreate,
                    Permission::CardRead,
                    Permission::CardUpdate,
                    Permission::CardDelete,
                    Permission::CardFreeze,
                    Permission::CardUnfreeze,
                    Permission::KycSubmit,
                    Permission::KycRead,
                    Permission::KycUpdate,
                    Permission::KycApprove,
                    Permission::KycReject,
                    Permission::FiatDeposit,
                    Permission::FiatWithdraw,
                    Permission::FiatRead,
                    Permission::BankAccountAdd,
                    Permission::BankAccountRead,
                    Permission::BankAccountDelete,
                    Permission::UserManagement,
                    Permission::AuditLogRead,
                    Permission::FinancialReporting,
                    Permission::TradingBasic,
                    Permission::TradingAdvanced,
                    Permission::TradingAdmin,
                    Permission::DefiBasic,
                    Permission::DefiAdvanced,
                    Permission::DefiAdmin,
                ]);
            },
            UserRole::SuperAdmin => {
                // Super admin gets all permissions
                permissions.extend([
                    Permission::WalletCreate,
                    Permission::WalletRead,
                    Permission::WalletUpdate,
                    Permission::WalletDelete,
                    Permission::TransactionCreate,
                    Permission::TransactionRead,
                    Permission::TransactionUpdate,
                    Permission::TransactionCancel,
                    Permission::CardCreate,
                    Permission::CardRead,
                    Permission::CardUpdate,
                    Permission::CardDelete,
                    Permission::CardFreeze,
                    Permission::CardUnfreeze,
                    Permission::KycSubmit,
                    Permission::KycRead,
                    Permission::KycUpdate,
                    Permission::KycApprove,
                    Permission::KycReject,
                    Permission::FiatDeposit,
                    Permission::FiatWithdraw,
                    Permission::FiatRead,
                    Permission::BankAccountAdd,
                    Permission::BankAccountRead,
                    Permission::BankAccountDelete,
                    Permission::UserManagement,
                    Permission::SystemConfiguration,
                    Permission::AuditLogRead,
                    Permission::FinancialReporting,
                    Permission::TradingBasic,
                    Permission::TradingAdvanced,
                    Permission::TradingAdmin,
                    Permission::DefiBasic,
                    Permission::DefiAdvanced,
                    Permission::DefiAdmin,
                ]);
            },
            UserRole::Viewer => {
                permissions.extend([
                    Permission::WalletRead,
                    Permission::TransactionRead,
                    Permission::CardRead,
                    Permission::KycRead,
                    Permission::FiatRead,
                    Permission::BankAccountRead,
                    Permission::AuditLogRead,
                ]);
            },
        }
        
        permissions
    }
    
    /// Update last login timestamp
    pub fn update_last_login(&mut self) {
        self.last_login = Some(Utc::now());
    }
    
    /// Set session ID
    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }
    
    /// Clear session
    pub fn clear_session(&mut self) {
        self.session_id = None;
    }
}
