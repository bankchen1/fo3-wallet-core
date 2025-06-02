//! Wallet management tests

use super::{TestClient, TestResults, run_test, generate_test_wallet_name};
use fo3_wallet_api::proto::fo3::wallet::v1::{
    CreateWalletRequest, ImportWalletRequest, GetWalletRequest, ListWalletsRequest,
    DeriveAddressRequest, KeyType, BitcoinNetwork,
};

pub async fn run_wallet_tests(client: &mut TestClient) -> TestResults {
    let mut results = TestResults::default();
    
    println!("💼 Running Wallet Tests...");

    // Test 1: Create wallet
    run_test!(results, "Create Wallet", test_create_wallet(client));

    // Test 2: Import wallet
    run_test!(results, "Import Wallet", test_import_wallet(client));

    // Test 3: Get wallet
    run_test!(results, "Get Wallet", test_get_wallet(client));

    // Test 4: List wallets
    run_test!(results, "List Wallets", test_list_wallets(client));

    // Test 5: Derive Ethereum address
    run_test!(results, "Derive Ethereum Address", test_derive_ethereum_address(client));

    // Test 6: Derive Bitcoin address
    run_test!(results, "Derive Bitcoin Address", test_derive_bitcoin_address(client));

    // Test 7: Derive Solana address
    run_test!(results, "Derive Solana Address", test_derive_solana_address(client));

    // Test 8: Delete wallet
    run_test!(results, "Delete Wallet", test_delete_wallet(client));

    results
}

async fn test_create_wallet(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;
    let wallet_name = generate_test_wallet_name();

    let request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name.clone(),
    });
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.wallet.create_wallet(authenticated_request).await?;
    let wallet_response = response.into_inner();

    assert!(wallet_response.wallet.is_some(), "Wallet should be created");
    assert!(!wallet_response.mnemonic.is_empty(), "Mnemonic should be provided");

    let wallet = wallet_response.wallet.unwrap();
    assert_eq!(wallet.name, wallet_name, "Wallet name should match");
    assert!(!wallet.id.is_empty(), "Wallet ID should not be empty");
    assert!(wallet.created_at > 0, "Created timestamp should be set");

    // Validate mnemonic format (should be 12 or 24 words)
    let word_count = wallet_response.mnemonic.split_whitespace().count();
    assert!(word_count == 12 || word_count == 24, "Mnemonic should have 12 or 24 words");

    Ok(())
}

async fn test_import_wallet(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;
    let wallet_name = generate_test_wallet_name();
    
    // Use a test mnemonic (this is a well-known test mnemonic, never use in production)
    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    let request = tonic::Request::new(ImportWalletRequest {
        name: wallet_name.clone(),
        mnemonic: test_mnemonic.to_string(),
    });
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.wallet.import_wallet(authenticated_request).await?;
    let wallet_response = response.into_inner();

    assert!(wallet_response.wallet.is_some(), "Wallet should be imported");

    let wallet = wallet_response.wallet.unwrap();
    assert_eq!(wallet.name, wallet_name, "Wallet name should match");
    assert!(!wallet.id.is_empty(), "Wallet ID should not be empty");

    Ok(())
}

async fn test_get_wallet(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    // First create a wallet
    let wallet_name = generate_test_wallet_name();
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name.clone(),
    });
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await?;
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id.clone();

    // Now get the wallet
    let get_request = tonic::Request::new(GetWalletRequest {
        wallet_id: wallet_id.clone(),
    });
    let authenticated_get_request = client.add_auth_header(get_request, &token);

    let get_response = client.wallet.get_wallet(authenticated_get_request).await?;
    let retrieved_wallet = get_response.into_inner().wallet.unwrap();

    assert_eq!(retrieved_wallet.id, wallet_id, "Wallet ID should match");
    assert_eq!(retrieved_wallet.name, wallet_name, "Wallet name should match");

    Ok(())
}

async fn test_list_wallets(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    let request = tonic::Request::new(ListWalletsRequest {
        page_size: 10,
        page_token: String::new(),
    });
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.wallet.list_wallets(authenticated_request).await?;
    let wallets_response = response.into_inner();

    // Should not error, even if empty
    assert!(wallets_response.wallets.len() >= 0, "Should return wallet list");

    Ok(())
}

async fn test_derive_ethereum_address(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    // First create a wallet
    let wallet_name = generate_test_wallet_name();
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name,
    });
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await?;
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id;

    // Derive Ethereum address
    let derive_request = tonic::Request::new(DeriveAddressRequest {
        wallet_id,
        key_type: KeyType::KeyTypeEthereum as i32,
        derivation_path: "m/44'/60'/0'/0/0".to_string(),
        bitcoin_network: BitcoinNetwork::BitcoinNetworkUnspecified as i32,
    });
    let authenticated_derive_request = client.add_auth_header(derive_request, &token);

    let derive_response = client.wallet.derive_address(authenticated_derive_request).await?;
    let address_response = derive_response.into_inner();

    assert!(address_response.address.is_some(), "Address should be derived");
    
    let address = address_response.address.unwrap();
    assert!(!address.address.is_empty(), "Address should not be empty");
    assert!(address.address.starts_with("0x"), "Ethereum address should start with 0x");
    assert_eq!(address.address.len(), 42, "Ethereum address should be 42 characters");
    assert_eq!(address.key_type, KeyType::KeyTypeEthereum as i32, "Key type should match");

    Ok(())
}

async fn test_derive_bitcoin_address(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    // First create a wallet
    let wallet_name = generate_test_wallet_name();
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name,
    });
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await?;
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id;

    // Derive Bitcoin address
    let derive_request = tonic::Request::new(DeriveAddressRequest {
        wallet_id,
        key_type: KeyType::KeyTypeBitcoin as i32,
        derivation_path: "m/44'/0'/0'/0/0".to_string(),
        bitcoin_network: BitcoinNetwork::BitcoinNetworkMainnet as i32,
    });
    let authenticated_derive_request = client.add_auth_header(derive_request, &token);

    let derive_response = client.wallet.derive_address(authenticated_derive_request).await?;
    let address_response = derive_response.into_inner();

    assert!(address_response.address.is_some(), "Address should be derived");
    
    let address = address_response.address.unwrap();
    assert!(!address.address.is_empty(), "Address should not be empty");
    assert_eq!(address.key_type, KeyType::KeyTypeBitcoin as i32, "Key type should match");
    assert_eq!(address.bitcoin_network, BitcoinNetwork::BitcoinNetworkMainnet as i32, "Network should match");

    Ok(())
}

async fn test_derive_solana_address(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    // First create a wallet
    let wallet_name = generate_test_wallet_name();
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name,
    });
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await?;
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id;

    // Derive Solana address
    let derive_request = tonic::Request::new(DeriveAddressRequest {
        wallet_id,
        key_type: KeyType::KeyTypeSolana as i32,
        derivation_path: "m/44'/501'/0'/0'".to_string(),
        bitcoin_network: BitcoinNetwork::BitcoinNetworkUnspecified as i32,
    });
    let authenticated_derive_request = client.add_auth_header(derive_request, &token);

    let derive_response = client.wallet.derive_address(authenticated_derive_request).await?;
    let address_response = derive_response.into_inner();

    assert!(address_response.address.is_some(), "Address should be derived");
    
    let address = address_response.address.unwrap();
    assert!(!address.address.is_empty(), "Address should not be empty");
    assert_eq!(address.key_type, KeyType::KeyTypeSolana as i32, "Key type should match");
    // Solana addresses are base58 encoded and typically 32-44 characters
    assert!(address.address.len() >= 32 && address.address.len() <= 44, "Solana address should be 32-44 characters");

    Ok(())
}

async fn test_delete_wallet(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let token = client.authenticate().await?;

    // First create a wallet
    let wallet_name = generate_test_wallet_name();
    let create_request = tonic::Request::new(CreateWalletRequest {
        name: wallet_name,
    });
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await?;
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id.clone();

    // Delete the wallet
    let delete_request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::DeleteWalletRequest {
            wallet_id: wallet_id.clone(),
        }
    );
    let authenticated_delete_request = client.add_auth_header(delete_request, &token);

    let delete_response = client.wallet.delete_wallet(authenticated_delete_request).await?;
    let delete_result = delete_response.into_inner();

    assert!(delete_result.success, "Wallet deletion should be successful");

    // Verify wallet is deleted by trying to get it
    let get_request = tonic::Request::new(GetWalletRequest {
        wallet_id,
    });
    let authenticated_get_request = client.add_auth_header(get_request, &token);

    let get_result = client.wallet.get_wallet(authenticated_get_request).await;
    match get_result {
        Err(status) => {
            assert_eq!(status.code(), tonic::Code::NotFound);
            Ok(())
        }
        Ok(_) => Err("Wallet should not exist after deletion".into()),
    }
}
