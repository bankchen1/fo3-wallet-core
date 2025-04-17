# FO3 Wallet Solana Integration

This crate provides Solana blockchain integration for the FO3 Wallet Core library.

## Features

- **Wallet Management**: Create and manage Solana wallets
- **Transaction Handling**: Create, sign, and broadcast Solana transactions
- **Token Support**: Transfer SPL tokens and manage token accounts
- **Staking**: Stake SOL to validators and manage stake accounts
- **DeFi**: Swap tokens on Raydium DEX
- **NFT Support**: Query NFTs and metadata

## Usage Examples

### Initialize Provider

```rust
use fo3_wallet::transaction::provider::ProviderConfig;
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

### Transfer SOL

```rust
use fo3_wallet::transaction::{TransactionRequest, KeyType};

// Create transaction request
let request = TransactionRequest {
    key_type: KeyType::Solana,
    from: "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b".to_string(),
    to: "83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri".to_string(),
    value: "0.1".to_string(), // 0.1 SOL
    gas_price: None,
    gas_limit: None,
    nonce: None,
    data: Some(serde_json::json!({
        "private_key": "your_private_key_here"
    })),
};

// Sign and send transaction
let signed_tx = provider.sign_transaction(&request).unwrap();
let signature = provider.broadcast_transaction(&signed_tx).unwrap();
println!("Transaction sent: {}", signature);
```

### Transfer SPL Tokens

```rust
use fo3_wallet_solana::{TokenTransferParams, SolanaProvider};
use solana_sdk::signature::{Keypair, Signer};

// Create keypair from private key
let keypair = provider.private_key_to_keypair("your_private_key_here").unwrap();
let payer = keypair.pubkey();

// Create token transfer parameters
let params = TokenTransferParams {
    token_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
    from: payer.to_string(),
    to: "recipient_address_here".to_string(),
    amount: 1000000, // 1 USDC (assuming 6 decimals)
    decimals: 6,
};

// Create token transfer transaction
let transaction = provider.create_token_transfer_transaction(&params, &payer).unwrap();

// Sign and send transaction
let signed_transaction = transaction.sign(&[&keypair], transaction.message.recent_blockhash);
let serialized = bincode::serialize(&signed_transaction).unwrap();
let signature = provider.broadcast_transaction(&serialized).unwrap();
println!("Token transfer sent: {}", signature);
```

### Stake SOL

```rust
use fo3_wallet_solana::{StakingParams, SolanaProvider};
use solana_sdk::signature::{Keypair, Signer};

// Create keypair from private key
let keypair = provider.private_key_to_keypair("your_private_key_here").unwrap();
let payer = keypair.pubkey();

// Create stake account keypair
let stake_account = Keypair::new();
let stake_account_pubkey = stake_account.pubkey();

// Create staking parameters
let params = StakingParams {
    from: payer.to_string(),
    validator: "validator_vote_account_here".to_string(),
    amount: 1000000000, // 1 SOL
};

// Create staking transaction
let transaction = provider.create_stake_transaction(&params, &payer).unwrap();

// Sign and send transaction
let signed_transaction = transaction.sign(&[&keypair, &stake_account], transaction.message.recent_blockhash);
let serialized = bincode::serialize(&signed_transaction).unwrap();
let signature = provider.broadcast_transaction(&serialized).unwrap();
println!("Staking transaction sent: {}", signature);
println!("Stake account: {}", stake_account_pubkey);
```

## API Documentation

For detailed API documentation, run:

```
cargo doc --open
```

## Building and Testing

To build the crate:

```
cargo build
```

To run tests:

```
cargo test
```

To run Solana-specific tests that require a network connection:

```
RUST_LOG=info RUN_SOLANA_TESTS=1 cargo test
```

### Raydium DEX Integration

```rust
// Get available token pairs on Raydium
let pairs = provider.get_raydium_token_pairs().unwrap();

// Get swap quote
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

// Execute swap
let min_amount_out = quote.min_out_amount;
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
```

See [Raydium Documentation](docs/raydium.md) for more details.

### NFT Support

```rust
// Get NFTs owned by a wallet
let wallet_address = "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b";
let nfts = provider.get_nfts_by_owner(wallet_address).await.unwrap();

// Get NFT metadata
let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
let metadata = provider.get_nft_metadata(mint).await.unwrap();

// Transfer an NFT
let from_wallet = "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b";
let to_wallet = "83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri";
let private_key = "your_private_key_here";
let signature = provider.transfer_nft(from_wallet, to_wallet, mint, private_key).await.unwrap();
```

See [NFT Documentation](docs/nft.md) for more details.

## License

MIT
