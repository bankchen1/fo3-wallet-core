# Orca DEX Integration

This document provides information about the Orca DEX integration in the FO3 Wallet Solana module.

## Overview

Orca is a decentralized exchange (DEX) built on the Solana blockchain. It provides automated market maker (AMM) functionality with concentrated liquidity pools.

The FO3 Wallet Solana module provides integration with Orca, allowing users to:

- Get information about available token pairs
- Get swap quotes
- Execute token swaps

## Usage Examples

### Initialize Provider

```rust
use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};
use fo3_wallet_solana::SolanaProvider;

// Create provider configuration
let config = ProviderConfig {
    provider_type: ProviderType::Http,
    url: "https://api.mainnet-beta.solana.com".to_string(),
    api_key: None,
    timeout: Some(30),
};

// Create Solana provider
let provider = SolanaProvider::new(config).unwrap();
```

### Get Available Token Pairs

```rust
// Get available token pairs on Orca
let pairs = provider.get_orca_token_pairs().unwrap();

// Print pairs
for (token_a, token_b) in pairs {
    println!("{}-{}", token_a, token_b);
}
```

### Get Swap Quote

```rust
// SOL to USDC swap quote
let sol_mint = "So11111111111111111111111111111111111111112";
let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
let amount_in = 1_000_000_000; // 1 SOL
let slippage = 0.5; // 0.5%

let quote = provider.get_orca_swap_quote(
    sol_mint,
    usdc_mint,
    amount_in,
    slippage,
).unwrap();

println!("Swap quote:");
println!("  From: {} {}", quote.in_amount, quote.in_token_symbol);
println!("  To: {} {}", quote.out_amount, quote.out_token_symbol);
println!("  Price impact: {}%", quote.price_impact * 100.0);
println!("  Minimum output: {}", quote.min_out_amount);
println!("  Fee: {}%", quote.fee * 100.0);
```

### Execute Swap

```rust
// SOL to USDC swap
let sol_mint = "So11111111111111111111111111111111111111112";
let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
let amount_in = 1_000_000_000; // 1 SOL
let min_amount_out = 10_000_000; // 10 USDC
let wallet_address = "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b";
let private_key = "your_private_key_here";

let signature = provider.execute_orca_swap(
    sol_mint,
    usdc_mint,
    amount_in,
    min_amount_out,
    wallet_address,
    private_key,
).unwrap();

println!("Swap executed: {}", signature);
```

## API Endpoints

The FO3 Wallet API provides the following endpoints for Orca integration:

### Get Token Pairs

```
GET /defi/swap/orca/pairs
```

Returns a list of available token pairs on Orca.

### Get Swap Preview

```
POST /defi/swap/orca/preview
```

Request body:
```json
{
  "token_in_mint": "So11111111111111111111111111111111111111112",
  "token_out_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "amount_in": "1000000000",
  "slippage": 0.5
}
```

Returns a swap quote with estimated output amount, price impact, and minimum output amount.

### Execute Swap

```
POST /defi/swap/orca/execute
```

Request body:
```json
{
  "token_in_mint": "So11111111111111111111111111111111111111112",
  "token_out_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "amount_in": "1000000000",
  "min_amount_out": "10000000",
  "wallet_address": "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b",
  "private_key": "your_private_key_here"
}
```

Executes a swap and returns the transaction signature.

## Error Handling

The Orca integration includes comprehensive error handling for various scenarios:

- Pool not found
- Insufficient liquidity
- Slippage exceeded
- Transaction failure
- Invalid addresses or amounts

Errors are returned as appropriate error types with descriptive messages.

## Implementation Details

The Orca integration uses the following components:

- `OrcaClient`: Core client for interacting with Orca pools
- `OrcaPool`: Represents an Orca liquidity pool
- `SwapParams`: Parameters for executing a swap
- `SwapQuote`: Quote information for a swap

The implementation uses the SPL Token Swap program to execute swaps on Orca pools.

## Comparison with Raydium

Both Orca and Raydium are decentralized exchanges on Solana, but they have some differences:

- Orca focuses on concentrated liquidity pools, which can provide better pricing for certain token pairs
- Raydium integrates with Serum DEX for order book functionality
- Both use the SPL Token Swap program for AMM functionality

The FO3 Wallet Solana module provides integration with both DEXes, allowing users to choose the best option for their needs.

## Known Limitations

- The current implementation uses a hardcoded list of known pools. In a production environment, this should be replaced with a dynamic pool discovery mechanism.
- Token metadata (name, symbol) is not fetched from on-chain sources. In a production environment, this should be integrated with a token registry or metadata program.
- The price impact calculation is an approximation and may not exactly match the actual price impact on the Orca UI.

## Future Improvements

- Dynamic pool discovery
- Integration with token metadata
- Support for more complex swap routes (e.g., multi-hop swaps)
- Support for providing liquidity and farming
- Better price impact calculation
- Aggregation across multiple DEXes to find the best price
