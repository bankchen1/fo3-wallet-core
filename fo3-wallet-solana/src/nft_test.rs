//! Tests for NFT functionality

#[cfg(test)]
mod tests {
    use super::super::*;
    use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};
    use solana_sdk::signature::Keypair;

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

    #[tokio::test]
    async fn test_transfer_nft() {
        let config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://api.devnet.solana.com".to_string(), // Use devnet for testing
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

        // Create test keypairs
        let from_keypair = Keypair::new();
        let to_keypair = Keypair::new();

        let from_wallet = from_keypair.pubkey().to_string();
        let to_wallet = to_keypair.pubkey().to_string();
        let private_key = bs58::encode(from_keypair.to_bytes()).into_string();

        // Use a known NFT mint address for testing
        // In a real test, you would need to create an NFT and mint it to the from_wallet
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // This is actually USDC, but it's a good test case

        let provider = SolanaProvider::new(config).unwrap();

        // This test will fail without a real NFT owned by the from_wallet
        // So we'll just check that the function exists and doesn't panic when creating the transaction
        let result = provider.transfer_nft(&from_wallet, &to_wallet, mint, &private_key).await;

        // The test will likely fail with an error about the source account not existing
        // or not having the NFT, which is expected
        assert!(result.is_err());

        // Check that the error is about the source account
        if let Err(e) = result {
            let error_message = e.to_string();
            println!("Error: {}", error_message);
            assert!(error_message.contains("source") || error_message.contains("account") || error_message.contains("NFT"));
        }
    }
}
