//! Lido staking implementations

use std::sync::Arc;
use std::str::FromStr;
use std::ops::Mul;
use ethers::prelude::*;
use ethers::core::types::{Address, U256};
use ethers::providers::{Provider, Http};
use ethers::contract::abigen;
use ethers::signers::{LocalWallet, Signer, Wallet};
use k256::ecdsa::SigningKey;

use crate::error::{Error, Result};
use crate::crypto::keys::KeyType;
use crate::transaction::provider::ProviderConfig;
use super::types::{Protocol, Token, TokenAmount, StakingAction, StakingRequest, StakingResult};

// Generate type-safe bindings for Lido stETH token
abigen!(
    ILidoStETH,
    r#"[
        function submit(address _referral) external payable returns (uint256)
        function balanceOf(address account) external view returns (uint256)
        function transfer(address recipient, uint256 amount) external returns (bool)
        function transferFrom(address sender, address recipient, uint256 amount) external returns (bool)
        function approve(address spender, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function getTotalShares() external view returns (uint256)
        function getTotalPooledEther() external view returns (uint256)
        function getSharesByPooledEth(uint256 _ethAmount) external view returns (uint256)
        function getPooledEthByShares(uint256 _sharesAmount) external view returns (uint256)
    ]"#,
);

// Generate type-safe bindings for Lido Withdrawal Queue
abigen!(
    ILidoWithdrawalQueue,
    r#"[
        function requestWithdrawals(uint256[] calldata _amounts, address _owner) external returns (uint256[] memory)
        function claimWithdrawals(uint256[] calldata _requestIds, uint256[] calldata _hints) external
        function getWithdrawalStatus(uint256 _requestId) external view returns (uint256 amount, address owner, uint256 timestamp, bool isFinalized, bool isClaimed)
    ]"#,
);

/// Lido addresses
pub struct LidoAddresses {
    /// stETH token address
    pub steth: Address,
    /// Withdrawal queue address
    pub withdrawal_queue: Address,
}

impl Default for LidoAddresses {
    fn default() -> Self {
        Self {
            // Lido stETH token address on Ethereum mainnet
            steth: Address::from_str("0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84").unwrap(),
            // Lido Withdrawal Queue address on Ethereum mainnet
            withdrawal_queue: Address::from_str("0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1").unwrap(),
        }
    }
}

/// Lido staking service
pub struct LidoStakingService {
    /// Provider
    provider: Arc<Provider<Http>>,
    /// Lido addresses
    lido: LidoAddresses,
}

impl LidoStakingService {
    /// Create a new Lido staking service
    pub fn new(config: &ProviderConfig) -> Result<Self> {
        // Create provider
        let provider = Provider::<Http>::try_from(config.url.clone())
            .map_err(|e| Error::DeFi(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            lido: LidoAddresses::default(),
        })
    }

    /// Get stETH balance
    pub async fn get_steth_balance(&self, address: &str) -> Result<TokenAmount> {
        let address = Address::from_str(address)
            .map_err(|e| Error::DeFi(format!("Invalid address: {}", e)))?;

        let steth = ILidoStETH::new(self.lido.steth, self.provider.clone());

        let balance = steth.balance_of(address)
            .call()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get stETH balance: {}", e)))?;

        Ok(TokenAmount {
            token: Token {
                name: "Lido Staked ETH".to_string(),
                symbol: "stETH".to_string(),
                decimals: 18,
                address: format!("{:?}", self.lido.steth),
                key_type: KeyType::Ethereum,
                logo_url: Some("https://cryptologos.cc/logos/lido-dao-ldo-logo.png".to_string()),
            },
            amount: balance.to_string(),
        })
    }

    /// Get staking APR
    pub async fn get_staking_apr(&self) -> Result<f64> {
        // In a real implementation, we would call the Lido API or calculate from on-chain data
        // This is a simplified implementation

        Ok(4.0) // 4% APR
    }

    /// Execute staking action
    pub async fn execute_staking_action(&self, request: &StakingRequest, wallet: &Wallet<SigningKey>) -> Result<StakingResult> {
        // Execute action
        let (tx_hash, rewards) = match &request.action {
            StakingAction::Stake(token_amount) => {
                // Check that the token is ETH
                if token_amount.token.symbol != "ETH" {
                    return Err(Error::DeFi("Only ETH can be staked in Lido".to_string()));
                }

                // Parse amount
                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                // Get stETH contract
                let steth = ILidoStETH::new(self.lido.steth, self.provider.clone());

                // Execute stake
                let tx = steth.submit(Address::zero())
                    .value(amount)
                    .send()
                    .await
                    .map_err(|e| Error::DeFi(format!("Failed to execute stake: {}", e)))?;

                (tx.tx_hash(), None)
            },
            StakingAction::Unstake(token_amount) => {
                // Check that the token is stETH
                if token_amount.token.symbol != "stETH" {
                    return Err(Error::DeFi("Only stETH can be unstaked from Lido".to_string()));
                }

                // Parse amount
                let amount = U256::from_dec_str(&token_amount.amount)
                    .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

                // Get withdrawal queue contract
                let withdrawal_queue = ILidoWithdrawalQueue::new(self.lido.withdrawal_queue, self.provider.clone());

                // First approve withdrawal queue to spend stETH
                let steth = ILidoStETH::new(self.lido.steth, self.provider.clone());

                let approve_tx = steth.approve(self.lido.withdrawal_queue, amount)
                    .send()
                    .await
                    .map_err(|e| Error::DeFi(format!("Failed to approve stETH: {}", e)))?;

                // Wait for approval to be mined
                let _receipt = approve_tx.await
                    .map_err(|e| Error::DeFi(format!("Failed to get approval receipt: {}", e)))?;

                // Execute unstake
                let amounts = vec![amount];
                let tx = withdrawal_queue.request_withdrawals(amounts, wallet.address())
                    .send()
                    .await
                    .map_err(|e| Error::DeFi(format!("Failed to execute unstake: {}", e)))?;

                (tx.tx_hash(), None)
            },
            StakingAction::ClaimRewards => {
                // In Lido, rewards are automatically accrued in the stETH balance
                // So we don't need to claim them explicitly
                // But we can return the current stETH balance as the rewards

                let steth_balance = self.get_steth_balance(&format!("{:?}", wallet.address())).await?;

                // Return a dummy transaction hash
                (H256::zero(), Some(steth_balance))
            },
        };

        // Get transaction receipt
        let receipt = if tx_hash != H256::zero() {
            self.provider.get_transaction_receipt(tx_hash)
                .await
                .map_err(|e| Error::DeFi(format!("Failed to get transaction receipt: {}", e)))?
                .ok_or_else(|| Error::DeFi("Failed to get transaction receipt".to_string()))?
        } else {
            // For ClaimRewards, we don't have a real transaction
            return Ok(StakingResult {
                action: request.action.clone(),
                transaction_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                protocol: Protocol::Lido,
                fee: "0".to_string(),
                rewards,
            });
        };

        // Calculate fee
        let gas_used = receipt.gas_used.unwrap_or_default();
        let gas_price = receipt.effective_gas_price.unwrap_or_default();
        let fee = gas_used.mul(gas_price);

        Ok(StakingResult {
            action: request.action.clone(),
            transaction_hash: format!("{:?}", tx_hash),
            protocol: Protocol::Lido,
            fee: fee.to_string(),
            rewards,
        })
    }
}
