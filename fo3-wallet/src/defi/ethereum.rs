//! Ethereum DeFi implementations

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
use super::types::{Protocol, Token, TokenAmount, SwapRequest, SwapResult};

// Generate type-safe bindings for Uniswap V2 Router
abigen!(
    IUniswapV2Router02,
    r#"[
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)
        function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
    ]"#,
);

// Generate type-safe bindings for ERC20 token
abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transfer(address recipient, uint256 amount) external returns (bool)
        function transferFrom(address sender, address recipient, uint256 amount) external returns (bool)
    ]"#,
);

/// Uniswap router addresses
pub struct UniswapRouters {
    /// Uniswap V2 router address
    pub v2: Address,
    /// Uniswap V3 router address
    pub v3: Address,
}

impl Default for UniswapRouters {
    fn default() -> Self {
        Self {
            // Uniswap V2 Router02 address on Ethereum mainnet
            v2: Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(),
            // Uniswap V3 Router address on Ethereum mainnet
            v3: Address::from_str("0xE592427A0AEce92De3Edee1F18E0157C05861564").unwrap(),
        }
    }
}

/// Ethereum DeFi service
pub struct EthereumDeFiService {
    /// Provider
    provider: Arc<Provider<Http>>,
    /// Uniswap routers
    uniswap: UniswapRouters,
}

impl EthereumDeFiService {
    /// Create a new Ethereum DeFi service
    pub fn new(config: &ProviderConfig) -> Result<Self> {
        // Create provider
        let provider = Provider::<Http>::try_from(config.url.clone())
            .map_err(|e| Error::DeFi(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            uniswap: UniswapRouters::default(),
        })
    }

    /// Get token balance
    pub async fn get_token_balance(&self, token: &Token, address: &str) -> Result<TokenAmount> {
        let address = Address::from_str(address)
            .map_err(|e| Error::DeFi(format!("Invalid address: {}", e)))?;

        // Check if token is ETH
        if token.symbol == "ETH" || token.address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" {
            // Get ETH balance
            let balance = self.provider.get_balance(address, None)
                .await
                .map_err(|e| Error::DeFi(format!("Failed to get ETH balance: {}", e)))?;

            return Ok(TokenAmount {
                token: token.clone(),
                amount: balance.to_string(),
            });
        }

        // Get ERC20 token balance
        let token_address = Address::from_str(&token.address)
            .map_err(|e| Error::DeFi(format!("Invalid token address: {}", e)))?;

        let token_contract = IERC20::new(token_address, self.provider.clone());

        let balance = token_contract.balance_of(address)
            .call()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get token balance: {}", e)))?;

        Ok(TokenAmount {
            token: token.clone(),
            amount: balance.to_string(),
        })
    }

    /// Get token price in USD
    pub async fn get_token_price(&self, token: &Token) -> Result<f64> {
        // In a real implementation, we would call a price oracle like Chainlink
        // For simplicity, we'll use hardcoded prices
        let price = match token.symbol.as_str() {
            "ETH" => 3000.0,
            "USDC" => 1.0,
            "USDT" => 1.0,
            "DAI" => 1.0,
            "WETH" => 3000.0,
            _ => 0.0,
        };

        Ok(price)
    }

    /// Get swap quote from Uniswap
    pub async fn get_uniswap_quote(&self, request: &SwapRequest) -> Result<TokenAmount> {
        // Get router contract
        let router = IUniswapV2Router02::new(self.uniswap.v2, self.provider.clone());

        // Parse amount
        let amount_in = U256::from_dec_str(&request.from.amount)
            .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

        // Create path
        let mut path = Vec::new();

        // Check if from token is ETH
        let from_address = if request.from.token.symbol == "ETH" || request.from.token.address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" {
            // Use WETH address for ETH
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
        } else {
            Address::from_str(&request.from.token.address)
                .map_err(|e| Error::DeFi(format!("Invalid from token address: {}", e)))?
        };

        // Check if to token is ETH
        let to_address = if request.to.symbol == "ETH" || request.to.address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" {
            // Use WETH address for ETH
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
        } else {
            Address::from_str(&request.to.address)
                .map_err(|e| Error::DeFi(format!("Invalid to token address: {}", e)))?
        };

        path.push(from_address);
        path.push(to_address);

        // Get amounts out
        let amounts = router.get_amounts_out(amount_in, path)
            .call()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to get swap quote: {}", e)))?;

        // Get amount out
        let amount_out = amounts[1];

        Ok(TokenAmount {
            token: request.to.clone(),
            amount: amount_out.to_string(),
        })
    }

    /// Execute swap on Uniswap
    pub async fn execute_uniswap_swap(&self, request: &SwapRequest, wallet: &Wallet<SigningKey>) -> Result<SwapResult> {
        // Get router contract
        let router = IUniswapV2Router02::new(self.uniswap.v2, self.provider.clone());

        // Parse amount
        let amount_in = U256::from_dec_str(&request.from.amount)
            .map_err(|e| Error::DeFi(format!("Invalid amount: {}", e)))?;

        // Calculate amount out min with slippage
        let quote = self.get_uniswap_quote(request).await?;
        let amount_out = U256::from_dec_str(&quote.amount)
            .map_err(|e| Error::DeFi(format!("Invalid quote amount: {}", e)))?;

        let slippage_factor = (10000.0 - request.slippage * 100.0) / 10000.0;
        let amount_out_min = (amount_out.as_u128() as f64 * slippage_factor) as u128;
        let amount_out_min = U256::from(amount_out_min);

        // Create path
        let mut path = Vec::new();

        // Check if from token is ETH
        let is_from_eth = request.from.token.symbol == "ETH" || request.from.token.address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let from_address = if is_from_eth {
            // Use WETH address for ETH
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
        } else {
            Address::from_str(&request.from.token.address)
                .map_err(|e| Error::DeFi(format!("Invalid from token address: {}", e)))?
        };

        // Check if to token is ETH
        let is_to_eth = request.to.symbol == "ETH" || request.to.address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let to_address = if is_to_eth {
            // Use WETH address for ETH
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
        } else {
            Address::from_str(&request.to.address)
                .map_err(|e| Error::DeFi(format!("Invalid to token address: {}", e)))?
        };

        path.push(from_address);
        path.push(to_address);

        // Get deadline
        let deadline = match request.deadline {
            Some(deadline) => U256::from(deadline),
            None => {
                let block = self.provider.get_block(BlockNumber::Latest)
                    .await
                    .map_err(|e| Error::DeFi(format!("Failed to get latest block: {}", e)))?
                    .ok_or_else(|| Error::DeFi("Failed to get latest block".to_string()))?;

                let timestamp = block.timestamp;
                timestamp + U256::from(1800) // 30 minutes
            }
        };

        // Execute swap
        let tx_hash = if is_from_eth && !is_to_eth {
            // ETH to Token
            let tx = router.swap_exact_eth_for_tokens(
                amount_out_min,
                path,
                wallet.address(),
                deadline,
            )
            .value(amount_in)
            .send()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to execute swap: {}", e)))?;

            tx.tx_hash()
        } else if !is_from_eth && is_to_eth {
            // Token to ETH
            // First approve router to spend tokens
            let token_contract = IERC20::new(from_address, self.provider.clone());

            let approve_tx = token_contract.approve(self.uniswap.v2, amount_in)
                .send()
                .await
                .map_err(|e| Error::DeFi(format!("Failed to approve token: {}", e)))?;

            // Wait for approval to be mined
            let _receipt = approve_tx.await
                .map_err(|e| Error::DeFi(format!("Failed to get approval receipt: {}", e)))?;

            // Execute swap
            let tx = router.swap_exact_tokens_for_eth(
                amount_in,
                amount_out_min,
                path,
                wallet.address(),
                deadline,
            )
            .send()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to execute swap: {}", e)))?;

            tx.tx_hash()
        } else if !is_from_eth && !is_to_eth {
            // Token to Token
            // First approve router to spend tokens
            let token_contract = IERC20::new(from_address, self.provider.clone());

            let approve_tx = token_contract.approve(self.uniswap.v2, amount_in)
                .send()
                .await
                .map_err(|e| Error::DeFi(format!("Failed to approve token: {}", e)))?;

            // Wait for approval to be mined
            let _receipt = approve_tx.await
                .map_err(|e| Error::DeFi(format!("Failed to get approval receipt: {}", e)))?;

            // Execute swap
            let tx = router.swap_exact_tokens_for_tokens(
                amount_in,
                amount_out_min,
                path,
                wallet.address(),
                deadline,
            )
            .send()
            .await
            .map_err(|e| Error::DeFi(format!("Failed to execute swap: {}", e)))?;

            tx.tx_hash()
        } else {
            return Err(Error::DeFi("Invalid swap type".to_string()));
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

        Ok(SwapResult {
            from: request.from.clone(),
            to: quote,
            transaction_hash: format!("{:?}", tx_hash),
            protocol: Protocol::Uniswap,
            fee: fee.to_string(),
        })
    }
}
