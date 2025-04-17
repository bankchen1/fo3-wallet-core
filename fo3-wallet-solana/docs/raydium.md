# Raydium DEX Integration

This document provides information about the Raydium DEX integration in the FO3 Wallet Solana module.

## Overview

Raydium is a decentralized exchange (DEX) built on the Solana blockchain. It provides automated market maker (AMM) functionality and is integrated with Serum DEX for order book functionality.

The FO3 Wallet Solana module provides integration with Raydium, allowing users to:

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
// Get available token pairs on Raydium
let pairs = provider.get_raydium_token_pairs().unwrap();

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

let quote = provider.get_raydium_swap_quote(
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

let signature = provider.execute_raydium_swap(
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

The FO3 Wallet API provides the following endpoints for Raydium integration:

### Get Token Pairs

```
GET /defi/swap/pairs
```

Returns a list of available token pairs on Raydium.

### Get Swap Preview

```
POST /defi/swap/preview
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
POST /defi/swap/execute
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

The Raydium integration includes comprehensive error handling for various scenarios:

- Pool not found
- Insufficient liquidity
- Slippage exceeded
- Transaction failure
- Invalid addresses or amounts

Errors are returned as appropriate error types with descriptive messages.

## Implementation Details

The Raydium integration uses the following components:

- `RaydiumClient`: Core client for interacting with Raydium pools
- `RaydiumPool`: Represents a Raydium liquidity pool
- `SwapParams`: Parameters for executing a swap
- `SwapQuote`: Quote information for a swap

The implementation uses the SPL Token Swap program to execute swaps on Raydium pools.

## Known Limitations

- The current implementation uses a hardcoded list of known pools. In a production environment, this should be replaced with a dynamic pool discovery mechanism.
- Token metadata (name, symbol) is not fetched from on-chain sources. In a production environment, this should be integrated with a token registry or metadata program.
- The price impact calculation is an approximation and may not exactly match the actual price impact on the Raydium UI.

## Future Improvements

- Dynamic pool discovery
- Integration with token metadata
- Support for more complex swap routes (e.g., multi-hop swaps)
- Support for providing liquidity and farming
- Better price impact calculation
