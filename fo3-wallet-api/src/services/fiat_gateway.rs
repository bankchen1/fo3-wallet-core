//! Fiat Gateway service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::proto::fo3::wallet::v1::{
    fiat_gateway_service_server::FiatGatewayService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    fiat_guard::FiatGuard,
};
use crate::models::fiat_gateway::{
    BankAccount, FiatTransaction, TransactionLimits, PaymentProvider, AccountType,
    TransactionType, TransactionStatus
};
use crate::services::payment_providers::{PaymentProviderTrait, AchProvider, VisaProvider};

/// Fiat Gateway service implementation
pub struct FiatGatewayServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    fiat_guard: Arc<FiatGuard>,
    payment_providers: HashMap<PaymentProvider, Box<dyn PaymentProviderTrait>>,
}

impl FiatGatewayServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        fiat_guard: Arc<FiatGuard>,
    ) -> Self {
        let mut payment_providers: HashMap<PaymentProvider, Box<dyn PaymentProviderTrait>> = HashMap::new();
        
        // Initialize mock payment providers
        payment_providers.insert(
            PaymentProvider::Ach,
            Box::new(AchProvider::new(
                "mock_ach_key".to_string(),
                "https://api.mock-ach.com".to_string(),
                "mock_webhook_secret".to_string(),
            ))
        );
        
        payment_providers.insert(
            PaymentProvider::Visa,
            Box::new(VisaProvider::new(
                "mock_visa_key".to_string(),
                "https://api.mock-visa.com".to_string(),
                "mock_visa_secret".to_string(),
            ))
        );

        Self {
            state,
            auth_service,
            audit_logger,
            fiat_guard,
            payment_providers,
        }
    }

    /// Convert proto PaymentProvider to model PaymentProvider
    fn convert_payment_provider_from_proto(provider: i32) -> Result<PaymentProvider, Status> {
        match crate::proto::fo3::wallet::v1::PaymentProvider::try_from(provider) {
            Ok(crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderAch) => Ok(PaymentProvider::Ach),
            Ok(crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderVisa) => Ok(PaymentProvider::Visa),
            Ok(crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderPaypal) => Ok(PaymentProvider::PayPal),
            Ok(crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderWire) => Ok(PaymentProvider::Wire),
            Ok(crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderSepa) => Ok(PaymentProvider::Sepa),
            _ => Err(Status::invalid_argument("Invalid payment provider")),
        }
    }

    /// Convert model PaymentProvider to proto PaymentProvider
    fn convert_payment_provider_to_proto(provider: PaymentProvider) -> crate::proto::fo3::wallet::v1::PaymentProvider {
        match provider {
            PaymentProvider::Ach => crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderAch,
            PaymentProvider::Visa => crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderVisa,
            PaymentProvider::PayPal => crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderPaypal,
            PaymentProvider::Wire => crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderWire,
            PaymentProvider::Sepa => crate::proto::fo3::wallet::v1::PaymentProvider::PaymentProviderSepa,
        }
    }

    /// Convert proto AccountType to model AccountType
    fn convert_account_type_from_proto(account_type: i32) -> Result<AccountType, Status> {
        match crate::proto::fo3::wallet::v1::AccountType::try_from(account_type) {
            Ok(crate::proto::fo3::wallet::v1::AccountType::AccountTypeChecking) => Ok(AccountType::Checking),
            Ok(crate::proto::fo3::wallet::v1::AccountType::AccountTypeSavings) => Ok(AccountType::Savings),
            Ok(crate::proto::fo3::wallet::v1::AccountType::AccountTypeCreditCard) => Ok(AccountType::CreditCard),
            Ok(crate::proto::fo3::wallet::v1::AccountType::AccountTypePaypal) => Ok(AccountType::PayPal),
            _ => Err(Status::invalid_argument("Invalid account type")),
        }
    }

    /// Convert model BankAccount to proto BankAccount
    fn convert_bank_account_to_proto(account: &BankAccount) -> crate::proto::fo3::wallet::v1::BankAccount {
        crate::proto::fo3::wallet::v1::BankAccount {
            id: account.id.to_string(),
            user_id: account.user_id.to_string(),
            provider: Self::convert_payment_provider_to_proto(account.provider) as i32,
            account_type: match account.account_type {
                AccountType::Checking => crate::proto::fo3::wallet::v1::AccountType::AccountTypeChecking as i32,
                AccountType::Savings => crate::proto::fo3::wallet::v1::AccountType::AccountTypeSavings as i32,
                AccountType::CreditCard => crate::proto::fo3::wallet::v1::AccountType::AccountTypeCreditCard as i32,
                AccountType::PayPal => crate::proto::fo3::wallet::v1::AccountType::AccountTypePaypal as i32,
            },
            account_name: account.account_name.clone(),
            masked_account_number: account.masked_account_number.clone(),
            routing_number: account.routing_number.clone().unwrap_or_default(),
            bank_name: account.bank_name.clone().unwrap_or_default(),
            is_verified: account.is_verified,
            is_primary: account.is_primary,
            created_at: account.created_at.timestamp(),
            verified_at: account.verified_at.map(|dt| dt.timestamp()).unwrap_or(0),
            currency: account.currency.clone(),
            country: account.country.clone(),
        }
    }

    /// Convert model FiatTransaction to proto FiatTransaction
    fn convert_transaction_to_proto(transaction: &FiatTransaction) -> crate::proto::fo3::wallet::v1::FiatTransaction {
        crate::proto::fo3::wallet::v1::FiatTransaction {
            id: transaction.id.to_string(),
            user_id: transaction.user_id.to_string(),
            r#type: match transaction.transaction_type {
                TransactionType::Deposit => crate::proto::fo3::wallet::v1::TransactionType::TransactionTypeDeposit as i32,
                TransactionType::Withdrawal => crate::proto::fo3::wallet::v1::TransactionType::TransactionTypeWithdrawal as i32,
            },
            status: match transaction.status {
                TransactionStatus::Pending => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusPending as i32,
                TransactionStatus::Processing => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusProcessing as i32,
                TransactionStatus::Completed => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusCompleted as i32,
                TransactionStatus::Failed => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusFailed as i32,
                TransactionStatus::Cancelled => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusCancelled as i32,
                TransactionStatus::RequiresApproval => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusRequiresApproval as i32,
                TransactionStatus::Approved => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusApproved as i32,
                TransactionStatus::Rejected => crate::proto::fo3::wallet::v1::TransactionStatus::TransactionStatusRejected as i32,
            },
            amount: transaction.amount.to_string(),
            currency: transaction.currency.clone(),
            fee_amount: transaction.fee_amount.to_string(),
            net_amount: transaction.net_amount.to_string(),
            bank_account_id: transaction.bank_account_id.map(|id| id.to_string()).unwrap_or_default(),
            provider: Self::convert_payment_provider_to_proto(transaction.provider) as i32,
            external_transaction_id: transaction.external_transaction_id.clone().unwrap_or_default(),
            reference_number: transaction.reference_number.clone().unwrap_or_default(),
            description: transaction.description.clone().unwrap_or_default(),
            created_at: transaction.created_at.timestamp(),
            updated_at: transaction.updated_at.timestamp(),
            completed_at: transaction.completed_at.map(|dt| dt.timestamp()).unwrap_or(0),
            failure_reason: transaction.failure_reason.clone().unwrap_or_default(),
            approval_notes: transaction.approval_notes.clone().unwrap_or_default(),
            approver_id: transaction.approver_id.clone().unwrap_or_default(),
            metadata: transaction.metadata.as_ref()
                .and_then(|m| serde_json::to_string(m).ok())
                .map(|s| [(String::new(), s)].into_iter().collect())
                .unwrap_or_default(),
        }
    }

    /// Encrypt sensitive account data
    fn encrypt_account_number(&self, account_number: &str) -> String {
        // In a real implementation, this would use proper encryption
        // For now, we'll just base64 encode it as a placeholder
        base64::engine::general_purpose::STANDARD.encode(account_number.as_bytes())
    }

    /// Create masked account number (show only last 4 digits)
    fn mask_account_number(&self, account_number: &str) -> String {
        if account_number.len() <= 4 {
            "*".repeat(account_number.len())
        } else {
            format!("****{}", &account_number[account_number.len()-4..])
        }
    }
}

#[tonic::async_trait]
impl FiatGatewayService for FiatGatewayServiceImpl {
    /// Bind a bank account to user's profile
    async fn bind_bank_account(
        &self,
        request: Request<BindBankAccountRequest>,
    ) -> Result<Response<BindBankAccountResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check deposit permission (binding accounts is required for deposits)
        self.fiat_guard.check_deposit_permission(auth_context)?;

        let req = request.into_inner();

        // Validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Users can only bind accounts to their own profile
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Can only bind accounts to your own profile"));
        }

        // Convert proto enums
        let provider = Self::convert_payment_provider_from_proto(req.provider)?;
        let account_type = Self::convert_account_type_from_proto(req.account_type)?;

        // Validate required fields
        if req.account_name.is_empty() || req.account_number.is_empty() {
            return Err(Status::invalid_argument("Account name and number are required"));
        }

        // Create bank account
        let encrypted_account_number = self.encrypt_account_number(&req.account_number);
        let masked_account_number = self.mask_account_number(&req.account_number);

        let mut bank_account = BankAccount::new(
            user_id,
            provider,
            account_type,
            req.account_name,
            encrypted_account_number,
            masked_account_number,
            if req.routing_number.is_empty() { None } else { Some(req.routing_number) },
            if req.bank_name.is_empty() { None } else { Some(req.bank_name) },
            req.currency,
            req.country,
        );

        if req.set_as_primary {
            bank_account.set_primary(true);
        }

        // Store in state (in real implementation, this would be in database)
        {
            let mut accounts = self.state.fiat_accounts.write().unwrap();
            accounts.insert(bank_account.id.to_string(), bank_account.clone());
        }

        // Initiate verification process
        let verification_method = match provider {
            PaymentProvider::Ach => "micro_deposits",
            PaymentProvider::Visa => "instant",
            _ => "manual",
        };

        let verification_amounts = if verification_method == "micro_deposits" {
            vec!["0.12".to_string(), "0.34".to_string()]
        } else {
            vec![]
        };

        // Log audit event
        self.audit_logger.log_audit_event(
            auth_context,
            crate::middleware::audit::AuditEventType::FiatAccountBound,
            "bind_bank_account",
            true,
            Some(&bank_account.id.to_string()),
            None,
        ).await;

        let response = BindBankAccountResponse {
            account: Some(Self::convert_bank_account_to_proto(&bank_account)),
            verification_method: verification_method.to_string(),
            verification_amounts,
        };

        Ok(Response::new(response))
    }

    /// Get user's bank accounts
    async fn get_bank_accounts(
        &self,
        request: Request<GetBankAccountsRequest>,
    ) -> Result<Response<GetBankAccountsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only view their own accounts
        if auth_context.user_id != req.user_id {
            self.fiat_guard.check_admin_permission(auth_context)?;
        }

        // Get accounts from state (in real implementation, this would query database)
        let accounts = {
            let accounts = self.state.fiat_accounts.read().unwrap();
            accounts.values()
                .filter(|account| {
                    account.user_id.to_string() == req.user_id &&
                    !account.is_deleted() &&
                    (!req.verified_only || account.is_verified)
                })
                .cloned()
                .collect::<Vec<_>>()
        };

        let proto_accounts = accounts.iter()
            .map(Self::convert_bank_account_to_proto)
            .collect();

        let response = GetBankAccountsResponse {
            accounts: proto_accounts,
        };

        Ok(Response::new(response))
    }

    /// Remove a bank account
    async fn remove_bank_account(
        &self,
        request: Request<RemoveBankAccountRequest>,
    ) -> Result<Response<RemoveBankAccountResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID format"))?;

        // Validate bank account access
        self.fiat_guard.validate_bank_account_access(auth_context, account_id).await?;

        // Remove account (soft delete)
        {
            let mut accounts = self.state.fiat_accounts.write().unwrap();
            if let Some(account) = accounts.get_mut(&req.account_id) {
                account.soft_delete();
            } else {
                return Err(Status::not_found("Bank account not found"));
            }
        }

        // Log audit event
        self.audit_logger.log_audit_event(
            auth_context,
            crate::middleware::audit::AuditEventType::FiatAccountRemoved,
            "remove_bank_account",
            true,
            Some(&req.account_id),
            None,
        ).await;

        let response = RemoveBankAccountResponse {
            success: true,
            message: "Bank account removed successfully".to_string(),
        };

        Ok(Response::new(response))
    }

    /// Verify a bank account
    async fn verify_bank_account(
        &self,
        request: Request<VerifyBankAccountRequest>,
    ) -> Result<Response<VerifyBankAccountResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID format"))?;

        // Validate bank account access
        self.fiat_guard.validate_bank_account_access(auth_context, account_id).await?;

        // Get account
        let mut account = {
            let accounts = self.state.fiat_accounts.read().unwrap();
            accounts.get(&req.account_id)
                .ok_or_else(|| Status::not_found("Bank account not found"))?
                .clone()
        };

        // Mock verification logic
        let verification_successful = if req.verification_amounts.len() == 2 {
            // Mock micro-deposit verification
            req.verification_amounts == vec!["0.12".to_string(), "0.34".to_string()]
        } else {
            false
        };

        if verification_successful {
            account.mark_verified("micro_deposits".to_string());

            // Update in state
            {
                let mut accounts = self.state.fiat_accounts.write().unwrap();
                accounts.insert(account.id.to_string(), account.clone());
            }
        }

        let response = VerifyBankAccountResponse {
            verified: verification_successful,
            message: if verification_successful {
                "Bank account verified successfully".to_string()
            } else {
                "Verification failed - incorrect amounts".to_string()
            },
            account: Some(Self::convert_bank_account_to_proto(&account)),
        };

        Ok(Response::new(response))
    }

    /// Submit a withdrawal request
    async fn submit_withdrawal(
        &self,
        request: Request<SubmitWithdrawalRequest>,
    ) -> Result<Response<SubmitWithdrawalResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check withdrawal permission
        self.fiat_guard.check_withdrawal_permission(auth_context)?;

        let req = request.into_inner();

        // Validate user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Users can only submit withdrawals for themselves
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Can only submit withdrawals for your own account"));
        }

        // Parse and validate amount
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Amount must be positive"));
        }

        // Parse bank account ID
        let bank_account_id = Uuid::parse_str(&req.bank_account_id)
            .map_err(|_| Status::invalid_argument("Invalid bank account ID format"))?;

        // Validate bank account access and get account
        self.fiat_guard.validate_bank_account_access(auth_context, bank_account_id).await?;

        let bank_account = {
            let accounts = self.state.fiat_accounts.read().unwrap();
            accounts.get(&req.bank_account_id)
                .ok_or_else(|| Status::not_found("Bank account not found"))?
                .clone()
        };

        if !bank_account.is_usable() {
            return Err(Status::failed_precondition("Bank account is not verified or has been deleted"));
        }

        // Validate transaction against limits and compliance
        let requires_approval = self.fiat_guard.validate_transaction(
            auth_context,
            amount,
            &req.currency,
            TransactionType::Withdrawal,
        ).await?;

        // Check AML compliance
        self.fiat_guard.check_aml_compliance(
            auth_context,
            amount,
            &req.currency,
            TransactionType::Withdrawal,
        ).await?;

        // Check velocity limits
        self.fiat_guard.check_velocity_limits(
            auth_context,
            amount,
            TransactionType::Withdrawal,
        ).await?;

        // Create transaction
        let mut transaction = FiatTransaction::new(
            user_id,
            Some(bank_account_id),
            TransactionType::Withdrawal,
            amount,
            req.currency.clone(),
            bank_account.provider,
            if req.description.is_empty() { None } else { Some(req.description) },
        );

        // Set status based on approval requirement
        if requires_approval {
            transaction.status = TransactionStatus::RequiresApproval;
        }

        // Store transaction
        {
            let mut transactions = self.state.fiat_transactions.write().unwrap();
            transactions.insert(transaction.id.to_string(), transaction.clone());
        }

        // Log audit event
        self.audit_logger.log_audit_event(
            auth_context,
            crate::middleware::audit::AuditEventType::FiatWithdrawalSubmitted,
            "submit_withdrawal",
            true,
            Some(&transaction.id.to_string()),
            None,
        ).await;

        let response = SubmitWithdrawalResponse {
            transaction: Some(Self::convert_transaction_to_proto(&transaction)),
        };

        Ok(Response::new(response))
    }

    /// Get withdrawal status
    async fn get_withdrawal_status(
        &self,
        request: Request<GetWithdrawalStatusRequest>,
    ) -> Result<Response<GetWithdrawalStatusResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only view their own transactions
        if auth_context.user_id != req.user_id {
            self.fiat_guard.check_admin_permission(auth_context)?;
        }

        // Get transaction
        let transaction = {
            let transactions = self.state.fiat_transactions.read().unwrap();
            transactions.get(&req.transaction_id)
                .ok_or_else(|| Status::not_found("Transaction not found"))?
                .clone()
        };

        // Verify user owns the transaction
        if transaction.user_id.to_string() != req.user_id {
            return Err(Status::permission_denied("Access denied to this transaction"));
        }

        let response = GetWithdrawalStatusResponse {
            transaction: Some(Self::convert_transaction_to_proto(&transaction)),
        };

        Ok(Response::new(response))
    }
}
