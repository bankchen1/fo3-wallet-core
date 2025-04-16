//! Common DeFi types

use serde::{Serialize, Deserialize};
use crate::crypto::keys::KeyType;
use crate::error::{Error, Result};

/// DeFi protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    /// Uniswap
    Uniswap,
    /// SushiSwap
    SushiSwap,
    /// PancakeSwap
    PancakeSwap,
    /// Aave
    Aave,
    /// Compound
    Compound,
    /// Lido
    Lido,
    /// Raydium
    Raydium,
    /// Orca
    Orca,
    /// Marinade
    Marinade,
    /// Other
    Other(String),
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    /// Token decimals
    pub decimals: u8,
    /// Token address
    pub address: String,
    /// Blockchain type
    pub key_type: KeyType,
    /// Token logo URL
    pub logo_url: Option<String>,
}

/// Token amount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAmount {
    /// Token
    pub token: Token,
    /// Amount in the smallest unit
    pub amount: String,
}

/// Swap request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    /// From token amount
    pub from: TokenAmount,
    /// To token
    pub to: Token,
    /// Slippage tolerance in percentage (e.g., 0.5 for 0.5%)
    pub slippage: f64,
    /// Protocol to use
    pub protocol: Protocol,
    /// Deadline in seconds
    pub deadline: Option<u64>,
}

/// Swap result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    /// From token amount
    pub from: TokenAmount,
    /// To token amount
    pub to: TokenAmount,
    /// Transaction hash
    pub transaction_hash: String,
    /// Protocol used
    pub protocol: Protocol,
    /// Fee paid
    pub fee: String,
}

/// Lending request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LendingAction {
    /// Supply tokens
    Supply(TokenAmount),
    /// Withdraw tokens
    Withdraw(TokenAmount),
    /// Borrow tokens
    Borrow(TokenAmount),
    /// Repay tokens
    Repay(TokenAmount),
}

/// Lending request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingRequest {
    /// Action to perform
    pub action: LendingAction,
    /// Protocol to use
    pub protocol: Protocol,
}

/// Lending result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingResult {
    /// Action performed
    pub action: LendingAction,
    /// Transaction hash
    pub transaction_hash: String,
    /// Protocol used
    pub protocol: Protocol,
    /// Fee paid
    pub fee: String,
}

/// Staking request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakingAction {
    /// Stake tokens
    Stake(TokenAmount),
    /// Unstake tokens
    Unstake(TokenAmount),
    /// Claim rewards
    ClaimRewards,
}

/// Staking request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingRequest {
    /// Action to perform
    pub action: StakingAction,
    /// Protocol to use
    pub protocol: Protocol,
}

/// Staking result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingResult {
    /// Action performed
    pub action: StakingAction,
    /// Transaction hash
    pub transaction_hash: String,
    /// Protocol used
    pub protocol: Protocol,
    /// Fee paid
    pub fee: String,
    /// Rewards claimed
    pub rewards: Option<TokenAmount>,
}

/// DeFi provider
pub trait DeFiProvider {
    /// Get supported protocols
    fn get_supported_protocols(&self) -> Vec<Protocol>;
    
    /// Get supported tokens
    fn get_supported_tokens(&self) -> Result<Vec<Token>>;
    
    /// Get token balance
    fn get_token_balance(&self, token: &Token, address: &str) -> Result<TokenAmount>;
    
    /// Get token price
    fn get_token_price(&self, token: &Token) -> Result<f64>;
    
    /// Get swap quote
    fn get_swap_quote(&self, request: &SwapRequest) -> Result<TokenAmount>;
    
    /// Execute swap
    fn execute_swap(&self, request: &SwapRequest) -> Result<SwapResult>;
    
    /// Execute lending action
    fn execute_lending(&self, request: &LendingRequest) -> Result<LendingResult>;
    
    /// Execute staking action
    fn execute_staking(&self, request: &StakingRequest) -> Result<StakingResult>;
}
