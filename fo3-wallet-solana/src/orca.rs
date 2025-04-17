//! Orca DEX integration for Solana
//!
//! This module provides functionality for interacting with the Orca DEX on Solana,
//! including querying pools, getting quotes, and executing swaps.

use std::str::FromStr;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use solana_sdk::{
    pubkey::Pubkey,
    instruction::Instruction,
    transaction::Transaction,
    signer::{Signer, keypair::Keypair},
    system_instruction,
};
use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use spl_token::instruction as token_instruction;
use spl_token_swap::instruction as swap_instruction;
use spl_associated_token_account::get_associated_token_address;

use fo3_wallet::error::{Error, Result};

/// Orca program ID
pub const ORCA_SWAP_PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

/// Orca pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaPool {
    /// Pool ID (swap account address)
    pub id: String,
    /// Token A mint address
    pub token_a_mint: String,
    /// Token B mint address
    pub token_b_mint: String,
    /// Token A account address
    pub token_a_account: String,
    /// Token B account address
    pub token_b_account: String,
    /// Pool token mint address
    pub pool_token_mint: String,
    /// Pool authority
    pub authority: String,
    /// Pool fees account
    pub fees_account: String,
    /// Token A name
    pub token_a_name: String,
    /// Token B name
    pub token_b_name: String,
    /// Token A symbol
    pub token_a_symbol: String,
    /// Token B symbol
    pub token_b_symbol: String,
    /// Token A decimals
    pub token_a_decimals: u8,
    /// Token B decimals
    pub token_b_decimals: u8,
}

/// Swap direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapDirection {
    /// Swap from token A to token B
    AtoB,
    /// Swap from token B to token A
    BtoA,
}

/// Swap parameters
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Pool to use for the swap
    pub pool: OrcaPool,
    /// Amount to swap (in token's smallest unit)
    pub amount_in: u64,
    /// Minimum amount out (in token's smallest unit)
    pub min_amount_out: u64,
    /// Swap direction
    pub direction: SwapDirection,
    /// User's wallet address
    pub user_wallet: Pubkey,
}

/// Swap quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    /// Input token symbol
    pub in_token_symbol: String,
    /// Output token symbol
    pub out_token_symbol: String,
    /// Input amount (in token's smallest unit)
    pub in_amount: u64,
    /// Estimated output amount (in token's smallest unit)
    pub out_amount: u64,
    /// Price impact percentage
    pub price_impact: f64,
    /// Minimum output amount with slippage (in token's smallest unit)
    pub min_out_amount: u64,
    /// Fee amount (in SOL)
    pub fee: f64,
}

/// Orca client for interacting with the Orca DEX
pub struct OrcaClient {
    /// RPC client
    client: RpcClient,
    /// Pools cache
    pools: HashMap<String, OrcaPool>,
}

impl OrcaClient {
    /// Create a new Orca client
    pub fn new(client: RpcClient) -> Self {
        Self {
            client,
            pools: HashMap::new(),
        }
    }

    /// Initialize pools from a list of known pools
    pub fn init_pools(&mut self, pools: Vec<OrcaPool>) {
        for pool in pools {
            let key = format!("{}-{}", pool.token_a_mint, pool.token_b_mint);
            self.pools.insert(key, pool.clone());
            
            // Also add the reverse direction
            let reverse_key = format!("{}-{}", pool.token_b_mint, pool.token_a_mint);
            self.pools.insert(reverse_key, pool);
        }
    }

    /// Get all pools
    pub fn get_pools(&self) -> Vec<OrcaPool> {
        self.pools.values().cloned().collect()
    }

    /// Find a pool by token mints
    pub fn find_pool(&self, token_a_mint: &str, token_b_mint: &str) -> Option<OrcaPool> {
        let key = format!("{}-{}", token_a_mint, token_b_mint);
        self.pools.get(&key).cloned()
    }

    /// Get swap quote
    pub fn get_swap_quote(
        &self,
        token_in_mint: &str,
        token_out_mint: &str,
        amount_in: u64,
        slippage: f64,
    ) -> Result<SwapQuote> {
        // Find the pool
        let pool = self.find_pool(token_in_mint, token_out_mint)
            .ok_or_else(|| Error::DeFi(format!("Pool not found for {}-{}", token_in_mint, token_out_mint)))?;

        // Determine swap direction
        let direction = if token_in_mint == pool.token_a_mint {
            SwapDirection::AtoB
        } else {
            SwapDirection::BtoA
        };

        // Get token accounts to check balances
        let token_a_account = Pubkey::from_str(&pool.token_a_account)
            .map_err(|e| Error::DeFi(format!("Invalid token A account: {}", e)))?;
        
        let token_b_account = Pubkey::from_str(&pool.token_b_account)
            .map_err(|e| Error::DeFi(format!("Invalid token B account: {}", e)))?;

        // Get account info to check balances
        let token_a_info = self.client.get_account(&token_a_account)
            .map_err(|e| Error::DeFi(format!("Failed to get token A account: {}", e)))?;
        
        let token_b_info = self.client.get_account(&token_b_account)
            .map_err(|e| Error::DeFi(format!("Failed to get token B account: {}", e)))?;

        // Parse token accounts
        let token_a_data = spl_token::state::Account::unpack(&token_a_info.data)
            .map_err(|e| Error::DeFi(format!("Failed to parse token A account: {}", e)))?;
        
        let token_b_data = spl_token::state::Account::unpack(&token_b_info.data)
            .map_err(|e| Error::DeFi(format!("Failed to parse token B account: {}", e)))?;

        // Get reserves
        let reserve_a = token_a_data.amount;
        let reserve_b = token_b_data.amount;

        // Calculate output amount based on constant product formula (x * y = k)
        let (in_reserve, out_reserve, in_decimals, out_decimals, in_symbol, out_symbol) = match direction {
            SwapDirection::AtoB => (
                reserve_a,
                reserve_b,
                pool.token_a_decimals,
                pool.token_b_decimals,
                pool.token_a_symbol.clone(),
                pool.token_b_symbol.clone(),
            ),
            SwapDirection::BtoA => (
                reserve_b,
                reserve_a,
                pool.token_b_decimals,
                pool.token_a_decimals,
                pool.token_b_symbol.clone(),
                pool.token_a_symbol.clone(),
            ),
        };

        // Calculate output amount using constant product formula
        // out_amount = (out_reserve * amount_in) / (in_reserve + amount_in)
        // We apply a 0.3% fee by reducing the input amount by 0.3%
        let fee_numerator = 3;
        let fee_denominator = 1000;
        
        let amount_in_with_fee = amount_in
            .checked_mul(fee_denominator - fee_numerator)
            .ok_or_else(|| Error::DeFi("Overflow in fee calculation".to_string()))?
            .checked_div(fee_denominator)
            .ok_or_else(|| Error::DeFi("Division by zero in fee calculation".to_string()))?;
        
        let numerator = out_reserve
            .checked_mul(amount_in_with_fee)
            .ok_or_else(|| Error::DeFi("Overflow in output calculation".to_string()))?;
        
        let denominator = in_reserve
            .checked_add(amount_in_with_fee)
            .ok_or_else(|| Error::DeFi("Overflow in reserve calculation".to_string()))?;
        
        let out_amount = numerator
            .checked_div(denominator)
            .ok_or_else(|| Error::DeFi("Division by zero in output calculation".to_string()))?;

        // Calculate price impact
        // price_impact = 1 - (out_amount / amount_in * in_reserve / out_reserve)
        let in_amount_adjusted = if in_decimals > out_decimals {
            amount_in.checked_div(10u64.pow((in_decimals - out_decimals) as u32))
                .ok_or_else(|| Error::DeFi("Division by zero in decimal adjustment".to_string()))?
        } else {
            amount_in.checked_mul(10u64.pow((out_decimals - in_decimals) as u32))
                .ok_or_else(|| Error::DeFi("Overflow in decimal adjustment".to_string()))?
        };

        let ideal_out_amount = in_amount_adjusted
            .checked_mul(out_reserve)
            .ok_or_else(|| Error::DeFi("Overflow in ideal output calculation".to_string()))?
            .checked_div(in_reserve)
            .ok_or_else(|| Error::DeFi("Division by zero in ideal output calculation".to_string()))?;

        let price_impact = if ideal_out_amount > 0 {
            1.0 - (out_amount as f64 / ideal_out_amount as f64)
        } else {
            0.0
        };

        // Apply slippage to get minimum output amount
        let min_out_amount = (out_amount as f64 * (1.0 - slippage / 100.0)) as u64;

        // Create the quote
        let quote = SwapQuote {
            in_token_symbol: in_symbol,
            out_token_symbol: out_symbol,
            in_amount: amount_in,
            out_amount,
            price_impact,
            min_out_amount,
            fee: 0.003, // 0.3% fee
        };

        Ok(quote)
    }

    /// Create swap instructions
    pub fn create_swap_instructions(
        &self,
        params: &SwapParams,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();

        // Parse addresses
        let user_wallet = params.user_wallet;
        let pool_id = Pubkey::from_str(&params.pool.id)
            .map_err(|e| Error::DeFi(format!("Invalid pool ID: {}", e)))?;
        let authority = Pubkey::from_str(&params.pool.authority)
            .map_err(|e| Error::DeFi(format!("Invalid authority: {}", e)))?;
        let token_a_mint = Pubkey::from_str(&params.pool.token_a_mint)
            .map_err(|e| Error::DeFi(format!("Invalid token A mint: {}", e)))?;
        let token_b_mint = Pubkey::from_str(&params.pool.token_b_mint)
            .map_err(|e| Error::DeFi(format!("Invalid token B mint: {}", e)))?;
        let token_a_account = Pubkey::from_str(&params.pool.token_a_account)
            .map_err(|e| Error::DeFi(format!("Invalid token A account: {}", e)))?;
        let token_b_account = Pubkey::from_str(&params.pool.token_b_account)
            .map_err(|e| Error::DeFi(format!("Invalid token B account: {}", e)))?;
        let pool_token_mint = Pubkey::from_str(&params.pool.pool_token_mint)
            .map_err(|e| Error::DeFi(format!("Invalid pool token mint: {}", e)))?;
        let fees_account = Pubkey::from_str(&params.pool.fees_account)
            .map_err(|e| Error::DeFi(format!("Invalid fees account: {}", e)))?;
        let program_id = Pubkey::from_str(ORCA_SWAP_PROGRAM_ID)
            .map_err(|e| Error::DeFi(format!("Invalid program ID: {}", e)))?;

        // Get source token account
        let (source_mint, destination_mint) = match params.direction {
            SwapDirection::AtoB => (token_a_mint, token_b_mint),
            SwapDirection::BtoA => (token_b_mint, token_a_mint),
        };

        let user_source_account = get_associated_token_address(&user_wallet, &source_mint);
        let user_destination_account = get_associated_token_address(&user_wallet, &destination_mint);

        // Check if destination token account exists
        let destination_account_exists = self.client.get_account_with_commitment(&user_destination_account, self.client.commitment())
            .map_err(|e| Error::DeFi(format!("Failed to check destination account: {}", e)))?
            .value
            .is_some();

        // If destination token account doesn't exist, create it
        if !destination_account_exists {
            let create_account_ix = spl_associated_token_account::instruction::create_associated_token_account(
                &user_wallet,
                &user_wallet,
                &destination_mint,
                &spl_token::id(),
            );
            instructions.push(create_account_ix);
        }

        // Create the swap instruction
        let swap_ix = match params.direction {
            SwapDirection::AtoB => swap_instruction::swap(
                &program_id,
                &spl_token::id(),
                &pool_id,
                &authority,
                &user_wallet,
                &user_source_account,
                &token_a_account,
                &token_b_account,
                &user_destination_account,
                &pool_token_mint,
                &fees_account,
                None,
                swap_instruction::Swap {
                    amount_in: params.amount_in,
                    minimum_amount_out: params.min_amount_out,
                },
            ),
            SwapDirection::BtoA => swap_instruction::swap(
                &program_id,
                &spl_token::id(),
                &pool_id,
                &authority,
                &user_wallet,
                &user_source_account,
                &token_b_account,
                &token_a_account,
                &user_destination_account,
                &pool_token_mint,
                &fees_account,
                None,
                swap_instruction::Swap {
                    amount_in: params.amount_in,
                    minimum_amount_out: params.min_amount_out,
                },
            ),
        }.map_err(|e| Error::DeFi(format!("Failed to create swap instruction: {}", e)))?;

        instructions.push(swap_ix);

        Ok(instructions)
    }

    /// Create and sign a swap transaction
    pub fn create_swap_transaction(
        &self,
        params: &SwapParams,
        keypair: &Keypair,
    ) -> Result<Transaction> {
        // Get recent blockhash
        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| Error::DeFi(format!("Failed to get recent blockhash: {}", e)))?;

        // Create swap instructions
        let instructions = self.create_swap_instructions(params)?;

        // Create transaction
        let mut transaction = Transaction::new_with_payer(
            &instructions,
            Some(&keypair.pubkey()),
        );

        // Set recent blockhash
        transaction.message.recent_blockhash = recent_blockhash;

        // Sign transaction
        transaction.sign(&[keypair], recent_blockhash);

        Ok(transaction)
    }
}

/// Load known Orca pools from a JSON file
pub fn load_known_pools() -> Vec<OrcaPool> {
    // This is a simplified implementation with hardcoded pools
    // In a real implementation, we would load this from a JSON file or API
    
    vec![
        // SOL-USDC pool
        OrcaPool {
            id: "EGZ7tiLeH62TPV1gL8WwbXGzEPa9zmcpVnnkPKKnrE2U".to_string(),
            token_a_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
            token_b_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            token_a_account: "ANP74VNsHwSrq9uUSjiSNyNWvf6ZPrKTmE4gHoNd13Lg".to_string(),
            token_b_account: "75HgnSvXbWKZBpZHveX68ZzAhDqMzNDS29X6BGLtxMo1".to_string(),
            pool_token_mint: "APDFRM3HMr8CAGXwKHiu2f5ePSpaiEJhaURwhsRrUUt9".to_string(),
            authority: "8JUjWjAyXTMB4ZXcV7nk3p6Gg1fWAAoSck7xekuyADKL".to_string(),
            fees_account: "3XMrhbv989VxAMi3DErLV9eJht1pHppW5LbKxe9fkEFR".to_string(),
            token_a_name: "Solana".to_string(),
            token_b_name: "USD Coin".to_string(),
            token_a_symbol: "SOL".to_string(),
            token_b_symbol: "USDC".to_string(),
            token_a_decimals: 9,
            token_b_decimals: 6,
        },
        // Add more pools as needed
    ]
}
