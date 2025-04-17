# Solana NFT Support

This document provides information about the Solana NFT support in the FO3 Wallet Solana module.

## Overview

Solana NFTs are typically implemented using the SPL Token program and the Metaplex Token Metadata program. The FO3 Wallet Solana module provides functionality for interacting with NFTs on Solana, including:

- Querying NFTs owned by a wallet
- Fetching NFT metadata

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

### Get NFTs Owned by a Wallet

```rust
// Get NFTs owned by a wallet
let wallet_address = "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b";
let nfts = provider.get_nfts_by_owner(wallet_address).await.unwrap();

// Print NFTs
for nft in nfts {
    println!("NFT: {}", nft.mint);
}
```

### Get NFT Metadata

```rust
// Get NFT metadata
let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
let metadata = provider.get_nft_metadata(mint).await.unwrap();

println!("NFT Metadata:");
println!("  Name: {}", metadata.name);
println!("  Symbol: {}", metadata.symbol);
println!("  URI: {}", metadata.uri);
if let Some(image) = &metadata.image {
    println!("  Image: {}", image);
}
if let Some(description) = &metadata.description {
    println!("  Description: {}", description);
}
```

## API Endpoints

The FO3 Wallet API provides the following endpoints for NFT support:

### Get NFTs by Owner

```
GET /nft/:wallet_address
```

Returns a list of NFTs owned by the specified wallet address.

### Get NFT Metadata

```
GET /nft/:mint/metadata
```

Returns metadata for the specified NFT mint address.

## Implementation Details

The NFT support is implemented using the following components:

- `NftClient`: Core client for interacting with NFTs on Solana
- `NftToken`: Represents an NFT token
- `NftMetadata`: Represents NFT metadata

The implementation uses the SPL Token program and the Metaplex Token Metadata program to interact with NFTs on Solana.

## Metaplex Token Metadata

Metaplex Token Metadata is a program that extends the SPL Token program to add metadata to tokens. The metadata includes:

- Name
- Symbol
- URI (usually points to a JSON file with additional metadata)
- Creators
- Royalty information
- Collection information

The metadata is stored in a PDA (Program Derived Address) account that is derived from the token mint address.

## External Metadata

The URI in the on-chain metadata usually points to a JSON file with additional metadata, such as:

- Image URL
- Description
- Attributes
- Animation URL
- External URL

The implementation attempts to fetch this external metadata if the URI is an HTTPS URL.

## Known Limitations

- The current implementation does not fully implement external metadata fetching. In a production environment, this should be implemented using an HTTP client like reqwest.
- The implementation does not handle all possible NFT standards on Solana. It focuses on the most common standard (Metaplex Token Metadata).
- The implementation does not handle NFT collections or verifications.

## Future Improvements

- Implement external metadata fetching
- Add support for NFT collections
- Add support for NFT verifications
- Add support for NFT transfers
- Add support for NFT minting
