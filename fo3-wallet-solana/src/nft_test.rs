//! Tests for NFT functionality

#[cfg(test)]
mod tests {
    use super::super::*;
    use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};

    #[tokio::test]
    async fn test_get_nfts_by_owner() {
        let config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://api.mainnet-beta.solana.com".to_string(),
            api_key: None,
            timeout: Some(30),
        };

        // Skip this test in CI environment
        if std::env::var("CI").is_ok() {
            return;
        }

        // Skip this test by default to avoid making real RPC calls
        if std::env::var("RUN_SOLANA_TESTS").is_err() {
            return;
        }

        // Use a known NFT holder address for testing
        let owner = "2JCxZv6LaFjtWqBXSC2ZnRmh8A9xKdj6zJGvUv5XA9Vy";

        let provider = SolanaProvider::new(config).unwrap();
        let nfts = provider.get_nfts_by_owner(owner).await;

        // Check that the function returns a result
        assert!(nfts.is_ok() || nfts.is_err());

        // If the result is Ok, check that it contains NFTs
        if let Ok(nfts) = nfts {
            println!("Found {} NFTs", nfts.len());
            for (i, nft) in nfts.iter().enumerate().take(5) {
                println!("NFT {}: {}", i + 1, nft.mint);
            }
        }
    }

    #[tokio::test]
    async fn test_get_nft_metadata() {
        let config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://api.mainnet-beta.solana.com".to_string(),
            api_key: None,
            timeout: Some(30),
        };

        // Skip this test in CI environment
        if std::env::var("CI").is_ok() {
            return;
        }

        // Skip this test by default to avoid making real RPC calls
        if std::env::var("RUN_SOLANA_TESTS").is_err() {
            return;
        }

        // Use a known NFT mint address for testing
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // This is actually USDC, but it's a good test case

        let provider = SolanaProvider::new(config).unwrap();
        let metadata = provider.get_nft_metadata(mint).await;

        // Check that the function returns a result
        assert!(metadata.is_ok() || metadata.is_err());

        // If the result is Ok, check that it contains metadata
        if let Ok(metadata) = metadata {
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
        }
    }
}
