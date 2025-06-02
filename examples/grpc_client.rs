//! gRPC Client Example for FO3 Wallet Core
//!
//! This example demonstrates how to interact with the FO3 Wallet Core gRPC API.

use tonic::transport::Channel;

// Include the generated gRPC code
pub mod proto {
    pub mod fo3 {
        pub mod wallet {
            pub mod v1 {
                tonic::include_proto!("fo3.wallet.v1");
            }
        }
    }
}

use proto::fo3::wallet::v1::{
    wallet_service_client::WalletServiceClient,
    transaction_service_client::TransactionServiceClient,
    defi_service_client::DefiServiceClient,
    health_service_client::HealthServiceClient,
    *,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the gRPC server
    let channel = Channel::from_static("http://localhost:50051")
        .connect()
        .await?;

    println!("Connected to FO3 Wallet Core gRPC API");

    // Test health check
    test_health_check(channel.clone()).await?;

    // Test wallet operations
    test_wallet_operations(channel.clone()).await?;

    // Test DeFi operations
    test_defi_operations(channel.clone()).await?;

    println!("All tests completed successfully!");

    Ok(())
}

async fn test_health_check(channel: Channel) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Health Check ===");
    
    let mut client = HealthServiceClient::new(channel);
    
    let request = tonic::Request::new(HealthCheckRequest {
        service: "fo3.wallet.v1.WalletService".to_string(),
    });

    let response = client.check(request).await?;
    println!("Health check response: {:?}", response.into_inner());

    Ok(())
}

async fn test_wallet_operations(channel: Channel) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Wallet Operations ===");
    
    let mut client = WalletServiceClient::new(channel);

    // Create a new wallet
    println!("Creating a new wallet...");
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: "Test Wallet".to_string(),
    });

    let create_response = client.create_wallet(create_request).await?;
    let wallet_response = create_response.into_inner();
    
    println!("Created wallet: {:?}", wallet_response.wallet);
    println!("Mnemonic: {}", wallet_response.mnemonic);

    let wallet = wallet_response.wallet.unwrap();
    let wallet_id = wallet.id.clone();

    // Get the wallet
    println!("\nGetting wallet by ID...");
    let get_request = tonic::Request::new(GetWalletRequest {
        wallet_id: wallet_id.clone(),
    });

    let get_response = client.get_wallet(get_request).await?;
    println!("Retrieved wallet: {:?}", get_response.into_inner().wallet);

    // List all wallets
    println!("\nListing all wallets...");
    let list_request = tonic::Request::new(ListWalletsRequest {
        page_size: 10,
        page_token: String::new(),
    });

    let list_response = client.list_wallets(list_request).await?;
    println!("Wallets: {:?}", list_response.into_inner().wallets);

    // Derive an Ethereum address
    println!("\nDeriving Ethereum address...");
    let derive_request = tonic::Request::new(DeriveAddressRequest {
        wallet_id: wallet_id.clone(),
        key_type: KeyType::KeyTypeEthereum as i32,
        derivation_path: "m/44'/60'/0'/0/0".to_string(),
        bitcoin_network: BitcoinNetwork::BitcoinNetworkUnspecified as i32,
    });

    let derive_response = client.derive_address(derive_request).await?;
    println!("Derived address: {:?}", derive_response.into_inner().address);

    // Derive a Solana address
    println!("\nDeriving Solana address...");
    let derive_request = tonic::Request::new(DeriveAddressRequest {
        wallet_id: wallet_id.clone(),
        key_type: KeyType::KeyTypeSolana as i32,
        derivation_path: "m/44'/501'/0'/0'".to_string(),
        bitcoin_network: BitcoinNetwork::BitcoinNetworkUnspecified as i32,
    });

    let derive_response = client.derive_address(derive_request).await?;
    println!("Derived Solana address: {:?}", derive_response.into_inner().address);

    Ok(())
}

async fn test_defi_operations(channel: Channel) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing DeFi Operations ===");
    
    let mut client = DefiServiceClient::new(channel);

    // Get supported tokens for Ethereum
    println!("Getting supported tokens for Ethereum...");
    let tokens_request = tonic::Request::new(GetSupportedTokensRequest {
        key_type: KeyType::KeyTypeEthereum as i32,
    });

    let tokens_response = client.get_supported_tokens(tokens_request).await?;
    let tokens = tokens_response.into_inner().tokens;
    println!("Supported tokens: {:?}", tokens);

    if !tokens.is_empty() {
        // Get a swap quote
        println!("\nGetting swap quote...");
        let input_token = tokens[0].clone();
        let output_token = if tokens.len() > 1 {
            tokens[1].clone()
        } else {
            input_token.clone()
        };

        let quote_request = tonic::Request::new(GetSwapQuoteRequest {
            input: Some(TokenAmount {
                token: Some(input_token),
                amount: "1000000000000000000".to_string(), // 1 token
            }),
            output_token: Some(output_token),
            slippage: 0.5,
            protocol: Protocol::ProtocolUniswap as i32,
        });

        match client.get_swap_quote(quote_request).await {
            Ok(quote_response) => {
                println!("Swap quote: {:?}", quote_response.into_inner().quote);
            }
            Err(e) => {
                println!("Swap quote failed (expected in test environment): {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "solana")]
async fn test_solana_operations(channel: Channel) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Solana Operations ===");
    
    use proto::fo3::wallet::v1::solana_service_client::SolanaServiceClient;
    
    let mut client = SolanaServiceClient::new(channel);

    // Get Raydium token pairs
    println!("Getting Raydium token pairs...");
    let pairs_request = tonic::Request::new(GetRaydiumPairsRequest {});

    match client.get_raydium_pairs(pairs_request).await {
        Ok(pairs_response) => {
            let pairs = pairs_response.into_inner().pairs;
            println!("Raydium pairs: {:?}", pairs);
        }
        Err(e) => {
            println!("Raydium pairs failed (expected in test environment): {}", e);
        }
    }

    // Get NFTs by owner (example address)
    println!("\nGetting NFTs by owner...");
    let nfts_request = tonic::Request::new(GetNftsByOwnerRequest {
        wallet_address: "9ZNTfG4NyQgxy2SWjSiQoUyBPEvXT2xo7fKc5hPYYJ7b".to_string(),
    });

    match client.get_nfts_by_owner(nfts_request).await {
        Ok(nfts_response) => {
            let nfts = nfts_response.into_inner().nfts;
            println!("NFTs: {:?}", nfts);
        }
        Err(e) => {
            println!("NFTs query failed (expected in test environment): {}", e);
        }
    }

    Ok(())
}
