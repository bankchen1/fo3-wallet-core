//! Aave lending implementations

use std::sync::Arc;
use std::str::FromStr;
use std::ops::Mul;
use ethers::prelude::*;
use ethers::core::types::{Address, U256};
use ethers::providers::{Provider, Http};
use ethers::contract::abigen;
use ethers::signers::{Signer, Wallet};
use k256::ecdsa::SigningKey;

use crate::error::{Error, Result};
use crate::transaction::provider::ProviderConfig;
use super::types::{Protocol, LendingAction, LendingRequest, LendingResult};

// Generate type-safe bindings for Aave V2 LendingPool
abigen!(
    IAaveV2LendingPool,
    r#"[
        function deposit(address asset, uint256 amount, address onBehalfOf, uint16 referralCode) external
        function withdraw(address asset, uint256 amount, address to) external returns (uint256)
        function borrow(address asset, uint256 amount, uint256 interestRateMode, uint16 referralCode, address onBehalfOf) external
        function repay(address asset, uint256 amount, uint256 rateMode, address onBehalfOf) external returns (uint256)
        function getUserAccountData(address user) external view returns (uint256 totalCollateralETH, uint256 totalDebtETH, uint256 availableBorrowsETH, uint256 currentLiquidationThreshold, uint256 ltv, uint256 healthFactor)
    ]"#,
);

// Generate type-safe bindings for Aave V2 Protocol Data Provider
abigen!(
    IAaveV2ProtocolDataProvider,
    r#"[
        function getUserReserveData(address asset, address user) external view returns (uint256 currentATokenBalance, uint256 currentStableDebt, uint256 currentVariableDebt, uint256 principalStableDebt, uint256 scaledVariableDebt, uint256 stableBorrowRate, uint256 liquidityRate, uint40 stableRateLastUpdated, bool usageAsCollateralEnabled)
        function getReserveConfigurationData(address asset) external view returns (uint256 decimals, uint256 ltv, uint256 liquidationThreshold, uint256 liquidationBonus, uint256 reserveFactor, bool usageAsCollateralEnabled, bool borrowingEnabled, bool stableBorrowRateEnabled, bool isActive, bool isFrozen)
        function getReserveTokensAddresses(address asset) external view returns (address aTokenAddress, address stableDebtTokenAddress, address variableDebtTokenAddress)
    ]"#,
);

/// Aave V2 addresses
pub struct AaveV2Addresses {
    /// Lending pool address
    pub lending_pool: Address,
    /// Protocol data provider address
    pub data_provider: Address,
}

impl Default for AaveV2Addresses {
    fn default() -> Self {
        Self {
            // Aave V2 LendingPool address on Ethereum mainnet
            lending_pool: Address::from_str("0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9").unwrap(),
            // Aave V2 Protocol Data Provider address on Ethereum mainnet
            data_provider: Address::from_str("0x057835Ad21a177dbdd3090bB1CAE03EaCF78Fc6d").unwrap(),
        }
    }
}

/// Aave lending service
pub struct AaveLendingService {
    /// Provider
    provider: Arc<Provider<Http>>,
    /// Aave V2 addresses
    aave_v2: AaveV2Addresses,
}

impl AaveLendingService {
    /// Create a new Aave lending service
    pub fn new(config: &ProviderConfig) -> Result<Self> {
        // Create provider
        let provider = Provider::<Http>::try_from(config.url.clone())
            .map_err(|e| Error::DeFi(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            aave_v2: AaveV2Addresses::default(),
        })
    }

    /// Get user account data
    pub async fn get_user_account_data(&self, address: &str) -> Result<AaveUserAccountData> {
        let address = Address::from_str(address)
            .map_err(|e| Error::DeFi(format!("Invalid address: {}", e)))?;

        let lending_pool = IAaveV2LendingPool::new(self.aave_v2.lending_pool, self.provider.clone());

        let account_data = lending_pool.get_user_account_data(address)
            .call()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get user account data: {}", e)))?;

        Ok(AaveUserAccountData {
            total_collateral_eth: account_data.0.to_string(),
            total_debt_eth: account_data.1.to_string(),
            available_borrows_eth: account_data.2.to_string(),
            current_liquidation_threshold: account_data.3.to_string(),
            ltv: account_data.4.to_string(),
            health_factor: account_data.5.to_string(),
        })
    }

    /// Get user reserve data
    pub async fn get_user_reserve_data(&self, asset: &str, address: &str) -> Result<AaveUserReserveData> {
        let asset = Address::from_str(asset)
            .map_err(|e| Error::DeFi(format!("Invalid asset address: {}", e)))?;

        let address = Address::from_str(address)
            .map_err(|e| Error::DeFi(format!("Invalid address: {}", e)))?;

        let data_provider = IAaveV2ProtocolDataProvider::new(self.aave_v2.data_provider, self.provider.clone());

        let reserve_data = data_provider.get_user_reserve_data(asset, address)
            .call()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get user reserve data: {}", e)))?;

        Ok(AaveUserReserveData {
            current_a_token_balance: reserve_data.0.to_string(),
            current_stable_debt: reserve_data.1.to_string(),
            current_variable_debt: reserve_data.2.to_string(),
            principal_stable_debt: reserve_data.3.to_string(),
            scaled_variable_debt: reserve_data.4.to_string(),
            stable_borrow_rate: reserve_data.5.to_string(),
            liquidity_rate: reserve_data.6.to_string(),
            stable_rate_last_updated: reserve_data.7.to_string(),
            usage_as_collateral_enabled: reserve_data.8,
        })
    }

    /// Execute lending action
    pub async fn execute_lending_action(&self, request: &LendingRequest, wallet: &Wallet<SigningKey>) -> Result<LendingResult> {
        // Get asset and amount from the request
        let (asset, amount) = match &request.action {
            LendingAction::Supply(token_amount) => {
                let asset = Address::from_str(&token_amount.token.address)
                    .map_err(|e| Error::DeFi(format!("Invalid asset address: {}", e)))?;

                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                (asset, amount)
            },
            LendingAction::Withdraw(token_amount) => {
                let asset = Address::from_str(&token_amount.token.address)
                    .map_err(|e| Error::DeFi(format!("Invalid asset address: {}", e)))?;

                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                (asset, amount)
            },
            LendingAction::Borrow(token_amount) => {
                let asset = Address::from_str(&token_amount.token.address)
                    .map_err(|e| Error::DeFi(format!("Invalid asset address: {}", e)))?;

                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                (asset, amount)
            },
            LendingAction::Repay(token_amount) => {
                let asset = Address::from_str(&token_amount.token.address)
                    .map_err(|e| Error::DeFi(format!("Invalid asset address: {}", e)))?;

                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                (asset, amount)
            },
        };

        // Get lending pool contract
        let lending_pool = IAaveV2LendingPool::new(self.aave_v2.lending_pool, self.provider.clone());

        // Execute action
        let tx_hash = match &request.action {
            LendingAction::Supply(_) => {
                // First approve lending pool to spend tokens
                let token_contract = super::ethereum::IERC20::new(asset, self.provider.clone());

                // Create the approve call
                let approve_call = token_contract.approve(self.aave_v2.lending_pool, amount);

                // Send the approve transaction
                let pending_approve = approve_call.send();

                // Wait for the approve transaction
                let approve_tx = pending_approve.await
                    .map_err(|e| Error::DeFi(format!("Failed to approve token: {}", e)))?;

                // Wait for approval to be mined
                let _receipt = approve_tx.await
                    .map_err(|e| Error::DeFi(format!("Failed to get approval receipt: {}", e)))?;

                // Execute deposit
                // Create the deposit call
                let deposit_call = lending_pool.deposit(
                    asset,
                    amount,
                    wallet.address(),
                    0, // referral code
                );

                // Send the deposit transaction
                let pending_deposit = deposit_call.send();

                // Wait for the deposit transaction
                let tx = pending_deposit.await
                    .map_err(|e| Error::DeFi(format!("Failed to execute deposit: {}", e)))?;

                tx.tx_hash()
            },
            LendingAction::Withdraw(_) => {
                // Execute withdraw
                // Create the withdraw call
                let withdraw_call = lending_pool.withdraw(
                    asset,
                    amount,
                    wallet.address(),
                );

                // Send the withdraw transaction
                let pending_withdraw = withdraw_call.send();

                // Wait for the withdraw transaction
                let tx = pending_withdraw.await
                    .map_err(|e| Error::DeFi(format!("Failed to execute withdraw: {}", e)))?;

                tx.tx_hash()
            },
            LendingAction::Borrow(_) => {
                // Execute borrow
                // Create the borrow call
                let borrow_call = lending_pool.borrow(
                    asset,
                    amount,
                    U256::from(2), // variable interest rate mode
                    0u16, // referral code
                    wallet.address(),
                );

                // Send the borrow transaction
                let pending_borrow = borrow_call.send();

                // Wait for the borrow transaction
                let tx = pending_borrow.await
                    .map_err(|e| Error::DeFi(format!("Failed to execute borrow: {}", e)))?;

                tx.tx_hash()
            },
            LendingAction::Repay(_) => {
                // First approve lending pool to spend tokens
                let token_contract = super::ethereum::IERC20::new(asset, self.provider.clone());

                // Create the approve call
                let approve_call = token_contract.approve(self.aave_v2.lending_pool, amount);

                // Send the approve transaction
                let pending_approve = approve_call.send();

                // Wait for the approve transaction
                let approve_tx = pending_approve.await
                    .map_err(|e| Error::DeFi(format!("Failed to approve token: {}", e)))?;

                // Wait for approval to be mined
                let _receipt = approve_tx.await
                    .map_err(|e| Error::DeFi(format!("Failed to get approval receipt: {}", e)))?;

                // Execute repay
                // Create the repay call
                let repay_call = lending_pool.repay(
                    asset,
                    amount,
                    U256::from(2), // variable interest rate mode
                    wallet.address(),
                );

                // Send the repay transaction
                let pending_repay = repay_call.send();

                // Wait for the repay transaction
                let tx = pending_repay.await
                    .map_err(|e| Error::DeFi(format!("Failed to execute repay: {}", e)))?;

                tx.tx_hash()
            },
        };

        // Get transaction receipt
        let receipt = self.provider.get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get transaction receipt: {}", e)))?
            .ok_or_else(|| Error::DeFi("Failed to get transaction receipt".to_string()))?;

        // Calculate fee
        let gas_used = receipt.gas_used.unwrap_or_default();
        let gas_price = receipt.effective_gas_price.unwrap_or_default();
        let fee = gas_used.mul(gas_price);

        Ok(LendingResult {
            action: request.action.clone(),
            transaction_hash: format!("{:?}", tx_hash),
            protocol: Protocol::Aave,
            fee: fee.to_string(),
        })
    }
}

/// Aave user account data
#[derive(Debug, Clone)]
pub struct AaveUserAccountData {
    /// Total collateral in ETH
    pub total_collateral_eth: String,
    /// Total debt in ETH
    pub total_debt_eth: String,
    /// Available borrows in ETH
    pub available_borrows_eth: String,
    /// Current liquidation threshold
    pub current_liquidation_threshold: String,
    /// Loan to value ratio
    pub ltv: String,
    /// Health factor
    pub health_factor: String,
}

/// Aave user reserve data
#[derive(Debug, Clone)]
pub struct AaveUserReserveData {
    /// Current aToken balance
    pub current_a_token_balance: String,
    /// Current stable debt
    pub current_stable_debt: String,
    /// Current variable debt
    pub current_variable_debt: String,
    /// Principal stable debt
    pub principal_stable_debt: String,
    /// Scaled variable debt
    pub scaled_variable_debt: String,
    /// Stable borrow rate
    pub stable_borrow_rate: String,
    /// Liquidity rate
    pub liquidity_rate: String,
    /// Stable rate last updated
    pub stable_rate_last_updated: String,
    /// Usage as collateral enabled
    pub usage_as_collateral_enabled: bool,
}
