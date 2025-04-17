//! Solana NFT functionality
//!
//! This module provides functionality for interacting with NFTs on Solana,
//! including querying NFTs owned by a wallet and fetching NFT metadata.

use std::str::FromStr;
use serde::{Serialize, Deserialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::{
    pubkey::Pubkey,
    program_pack::Pack,
    instruction::Instruction,
    transaction::Transaction,
    signer::{Signer, keypair::Keypair},
    system_instruction,

};
use spl_token::{state::Account as TokenAccount, instruction as token_instruction};
use spl_associated_token_account::{get_associated_token_address, instruction as associated_token_instruction};
use borsh::{BorshDeserialize, BorshSerialize};

use fo3_wallet::error::{Error, Result};

/// Metaplex program ID
pub const METADATA_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/// Metaplex token metadata account prefix
pub const METADATA_PREFIX: &str = "metadata";

/// Metaplex token metadata instruction discriminator for create metadata accounts v3
pub const CREATE_METADATA_ACCOUNTS_V3: u8 = 33;

/// NFT mint parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMintParams {
    /// NFT name
    pub name: String,
    /// NFT symbol
    pub symbol: String,
    /// NFT URI (usually points to JSON metadata)
    pub uri: String,
    /// NFT seller fee basis points (e.g., 500 = 5%)
    pub seller_fee_basis_points: Option<u16>,
    /// NFT creators
    pub creators: Option<Vec<NftCreator>>,
    /// Whether the NFT metadata is mutable
    pub is_mutable: Option<bool>,
}

/// NFT mint result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMintResult {
    /// NFT mint address
    pub mint: String,
    /// NFT token account address
    pub token_account: String,
    /// NFT metadata account address
    pub metadata_account: String,
    /// Transaction signature
    pub signature: String,
}

/// NFT metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadata {
    /// NFT mint address
    pub mint: String,
    /// NFT name
    pub name: String,
    /// NFT symbol
    pub symbol: String,
    /// NFT URI (usually points to JSON metadata)
    pub uri: String,
    /// NFT image URL (if available)
    pub image: Option<String>,
    /// NFT description (if available)
    pub description: Option<String>,
    /// NFT attributes (if available)
    pub attributes: Option<Vec<NftAttribute>>,
    /// NFT creators (if available)
    pub creators: Option<Vec<NftCreator>>,
    /// NFT royalty basis points (if available)
    pub seller_fee_basis_points: Option<u16>,
    /// NFT collection (if available)
    pub collection: Option<NftCollection>,
    /// NFT uses (if available)
    pub uses: Option<NftUses>,
}

/// NFT attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftAttribute {
    /// Attribute trait type
    pub trait_type: String,
    /// Attribute value
    pub value: String,
}

/// NFT creator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftCreator {
    /// Creator address
    pub address: String,
    /// Creator share (percentage)
    pub share: u8,
    /// Whether the creator has verified the NFT
    pub verified: bool,
}

/// NFT collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftCollection {
    /// Collection name
    pub name: String,
    /// Collection family
    pub family: Option<String>,
    /// Collection verified
    pub verified: bool,
}

/// NFT uses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftUses {
    /// Use method
    pub use_method: String,
    /// Remaining uses
    pub remaining: u64,
    /// Total uses
    pub total: u64,
}

/// NFT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftToken {
    /// NFT mint address
    pub mint: String,
    /// NFT owner address
    pub owner: String,
    /// NFT metadata (if available)
    pub metadata: Option<NftMetadata>,
}

/// Metaplex metadata account data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MetadataAccount {
    /// Account key
    pub key: u8,
    /// Update authority
    pub update_authority: Pubkey,
    /// Mint address
    pub mint: Pubkey,
    /// Metadata data
    pub data: MetadataData,
    /// Primary sale happened
    pub primary_sale_happened: bool,
    /// Is mutable
    pub is_mutable: bool,
    /// Edition nonce
    pub edition_nonce: Option<u8>,
    /// Token standard
    pub token_standard: Option<u8>,
    /// Collection
    pub collection: Option<Collection>,
    /// Uses
    pub uses: Option<Uses>,
}

/// Metaplex metadata data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MetadataData {
    /// Name
    pub name: String,
    /// Symbol
    pub symbol: String,
    /// URI
    pub uri: String,
    /// Seller fee basis points
    pub seller_fee_basis_points: u16,
    /// Creators
    pub creators: Option<Vec<Creator>>,
}

/// Metaplex creator
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Creator {
    /// Creator address
    pub address: Pubkey,
    /// Creator verified
    pub verified: bool,
    /// Creator share
    pub share: u8,
}

/// Metaplex collection
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Collection {
    /// Collection verified
    pub verified: bool,
    /// Collection key
    pub key: Pubkey,
}

/// Metaplex uses
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Uses {
    /// Use method
    pub use_method: u8,
    /// Remaining uses
    pub remaining: u64,
    /// Total uses
    pub total: u64,
}

/// External metadata from URI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMetadata {
    /// Name
    pub name: Option<String>,
    /// Symbol
    pub symbol: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Image URL
    pub image: Option<String>,
    /// Animation URL
    pub animation_url: Option<String>,
    /// External URL
    pub external_url: Option<String>,
    /// Attributes
    pub attributes: Option<Vec<ExternalAttribute>>,
    /// Properties
    pub properties: Option<Properties>,
}

/// External attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAttribute {
    /// Trait type
    pub trait_type: Option<String>,
    /// Value
    pub value: serde_json::Value,
}

/// Properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Properties {
    /// Files
    pub files: Option<Vec<File>>,
    /// Creators
    pub creators: Option<Vec<ExternalCreator>>,
}

/// File
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// URI
    pub uri: Option<String>,
    /// Type
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}

/// External creator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCreator {
    /// Address
    pub address: Option<String>,
    /// Share
    pub share: Option<u8>,
}

/// NFT client for interacting with NFTs on Solana
pub struct NftClient {
    /// RPC client
    client: RpcClient,
}

impl NftClient {
    /// Create a new NFT client
    pub fn new(client: RpcClient) -> Self {
        Self {
            client,
        }
    }

    /// Get NFTs owned by a wallet
    pub async fn get_nfts_by_owner(&self, owner: &str) -> Result<Vec<NftToken>> {
        // Parse owner address
        let owner_pubkey = Pubkey::from_str(owner)
            .map_err(|e| Error::Transaction(format!("Invalid owner address: {}", e)))?;

        // Get token accounts by owner
        let token_accounts = self.client.get_token_accounts_by_owner(
            &owner_pubkey,
            TokenAccountsFilter::ProgramId(spl_token::id()),
        ).map_err(|e| Error::Transaction(format!("Failed to get token accounts: {}", e)))?;

        let mut nfts = Vec::new();

        // Filter for NFTs (tokens with amount = 1)
        for account in token_accounts {
            // Parse pubkey
            let pubkey = Pubkey::from_str(&account.pubkey)
                .map_err(|e| Error::Transaction(format!("Invalid token account pubkey: {}", e)))?;

            // Get account data
            let account_data = self.client.get_account(&pubkey)
                .map_err(|e| Error::Transaction(format!("Failed to get token account: {}", e)))?;

            // Parse token account
            let token_account = TokenAccount::unpack(&account_data.data)
                .map_err(|e| Error::Transaction(format!("Failed to parse token account: {}", e)))?;

            // Check if this is an NFT (amount = 1)
            if token_account.amount == 1 {
                let mint = token_account.mint.to_string();
                let nft = NftToken {
                    mint: mint.clone(),
                    owner: owner.to_string(),
                    metadata: None, // We'll fetch metadata separately
                };
                nfts.push(nft);
            }
        }

        Ok(nfts)
    }

    /// Get NFT metadata
    pub async fn get_nft_metadata(&self, mint: &str) -> Result<NftMetadata> {
        // Parse mint address
        let mint_pubkey = Pubkey::from_str(mint)
            .map_err(|e| Error::Transaction(format!("Invalid mint address: {}", e)))?;

        // Derive metadata account address
        let metadata_program_id = Pubkey::from_str(METADATA_PROGRAM_ID)
            .map_err(|e| Error::Transaction(format!("Invalid metadata program ID: {}", e)))?;

        let metadata_seeds = &[
            METADATA_PREFIX.as_bytes(),
            metadata_program_id.as_ref(),
            mint_pubkey.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &metadata_program_id);

        // Get metadata account
        let metadata_account = match self.client.get_account_data(&metadata_pubkey) {
            Ok(data) => data,
            Err(e) => return Err(Error::Transaction(format!("Failed to get metadata account: {}", e))),
        };

        // Parse metadata account
        let metadata = match MetadataAccount::try_from_slice(&metadata_account) {
            Ok(metadata) => metadata,
            Err(e) => return Err(Error::Transaction(format!("Failed to parse metadata account: {}", e))),
        };

        // Create NFT metadata
        let mut nft_metadata = NftMetadata {
            mint: mint.to_string(),
            name: metadata.data.name.trim_end_matches('\0').to_string(),
            symbol: metadata.data.symbol.trim_end_matches('\0').to_string(),
            uri: metadata.data.uri.trim_end_matches('\0').to_string(),
            image: None,
            description: None,
            attributes: None,
            creators: metadata.data.creators.as_ref().map(|creators| {
                creators.iter().map(|creator| {
                    NftCreator {
                        address: creator.address.to_string(),
                        share: creator.share,
                        verified: creator.verified,
                    }
                }).collect()
            }),
            seller_fee_basis_points: Some(metadata.data.seller_fee_basis_points),
            collection: metadata.collection.as_ref().map(|collection| {
                NftCollection {
                    name: "".to_string(), // We don't have the name from on-chain data
                    family: None,
                    verified: collection.verified,
                }
            }),
            uses: metadata.uses.as_ref().map(|uses| {
                let use_method = match uses.use_method {
                    0 => "Burn".to_string(),
                    1 => "Multiple".to_string(),
                    2 => "Single".to_string(),
                    _ => "Unknown".to_string(),
                };
                NftUses {
                    use_method,
                    remaining: uses.remaining,
                    total: uses.total,
                }
            }),
        };

        // Try to fetch external metadata if URI is an HTTPS URL
        if nft_metadata.uri.starts_with("https://") {
            if let Ok(external_metadata) = self.fetch_external_metadata(&nft_metadata.uri).await {
                // Update metadata with external data
                if let Some(image) = external_metadata.image {
                    nft_metadata.image = Some(image);
                }
                if let Some(description) = external_metadata.description {
                    nft_metadata.description = Some(description);
                }
                if let Some(attributes) = external_metadata.attributes {
                    nft_metadata.attributes = Some(attributes.iter().filter_map(|attr| {
                        if let (Some(trait_type), Some(value)) = (&attr.trait_type, attr.value.as_str()) {
                            Some(NftAttribute {
                                trait_type: trait_type.clone(),
                                value: value.to_string(),
                            })
                        } else {
                            None
                        }
                    }).collect());
                }
            }
        }

        Ok(nft_metadata)
    }

    /// Fetch external metadata from URI
    async fn fetch_external_metadata(&self, _uri: &str) -> Result<ExternalMetadata> {
        // This would normally be an async HTTP request
        // For simplicity, we'll just return an error
        // In a real implementation, you would use reqwest or another HTTP client
        Err(Error::Transaction("External metadata fetching not implemented".to_string()))
    }

    /// Transfer an NFT from one wallet to another
    pub async fn transfer_nft(
        &self,
        from_wallet: &str,
        to_wallet: &str,
        mint: &str,
        keypair: &Keypair,
    ) -> Result<String> {
        // Parse addresses
        let from_pubkey = Pubkey::from_str(from_wallet)
            .map_err(|e| Error::Transaction(format!("Invalid from address: {}", e)))?;

        let to_pubkey = Pubkey::from_str(to_wallet)
            .map_err(|e| Error::Transaction(format!("Invalid to address: {}", e)))?;

        let mint_pubkey = Pubkey::from_str(mint)
            .map_err(|e| Error::Transaction(format!("Invalid mint address: {}", e)))?;

        // Verify that the keypair matches the from_wallet
        if keypair.pubkey() != from_pubkey {
            return Err(Error::Transaction("Keypair does not match from wallet address".to_string()));
        }

        // Get source token account
        let source_token_account = get_associated_token_address(&from_pubkey, &mint_pubkey);

        // Check if source token account exists and has the NFT
        let source_account = match self.client.get_account(&source_token_account) {
            Ok(account) => account,
            Err(e) => return Err(Error::Transaction(format!("Failed to get source token account: {}", e))),
        };

        // Parse token account data
        let token_account_data = TokenAccount::unpack(&source_account.data)
            .map_err(|e| Error::Transaction(format!("Failed to parse source token account: {}", e)))?;

        // Verify that the token account has the NFT
        if token_account_data.amount != 1 {
            return Err(Error::Transaction("Source account does not have the NFT".to_string()));
        }

        // Get destination token account
        let destination_token_account = get_associated_token_address(&to_pubkey, &mint_pubkey);

        // Check if destination token account exists
        let destination_account_exists = self.client.get_account_with_commitment(&destination_token_account, self.client.commitment())
            .map_err(|e| Error::Transaction(format!("Failed to check destination account: {}", e)))?;

        let mut instructions = Vec::new();

        // If destination token account doesn't exist, create it
        if destination_account_exists.value.is_none() {
            let create_account_ix = associated_token_instruction::create_associated_token_account(
                &from_pubkey,
                &to_pubkey,
                &mint_pubkey,
                &spl_token::id(),
            );
            instructions.push(create_account_ix);
        }

        // Create transfer instruction
        let transfer_ix = token_instruction::transfer(
            &spl_token::id(),
            &source_token_account,
            &destination_token_account,
            &from_pubkey,
            &[&from_pubkey],
            1, // NFTs have amount 1
        ).map_err(|e| Error::Transaction(format!("Failed to create transfer instruction: {}", e)))?;

        instructions.push(transfer_ix);

        // Get recent blockhash
        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| Error::Transaction(format!("Failed to get recent blockhash: {}", e)))?;

        // Create transaction
        let mut transaction = Transaction::new_with_payer(
            &instructions,
            Some(&from_pubkey),
        );

        // Set recent blockhash
        transaction.message.recent_blockhash = recent_blockhash;

        // Sign transaction
        transaction.sign(&[keypair], recent_blockhash);

        // Send transaction
        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| Error::Transaction(format!("Failed to send transaction: {}", e)))?;

        Ok(signature.to_string())
    }

    /// Mint a new NFT
    pub async fn mint_nft(
        &self,
        wallet: &str,
        keypair: &Keypair,
        params: &NftMintParams,
    ) -> Result<NftMintResult> {
        // Parse wallet address
        let wallet_pubkey = Pubkey::from_str(wallet)
            .map_err(|e| Error::Transaction(format!("Invalid wallet address: {}", e)))?;

        // Verify that the keypair matches the wallet
        if keypair.pubkey() != wallet_pubkey {
            return Err(Error::Transaction("Keypair does not match wallet address".to_string()));
        }

        // Create a new keypair for the mint account
        let mint_keypair = Keypair::new();
        let mint_pubkey = mint_keypair.pubkey();

        // Get recent blockhash
        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| Error::Transaction(format!("Failed to get recent blockhash: {}", e)))?;

        // Get rent-exempt minimum balance for mint account
        let rent = self.client.get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)
            .map_err(|e| Error::Transaction(format!("Failed to get rent exemption: {}", e)))?;

        let mut instructions = Vec::new();

        // 1. Create mint account
        instructions.push(system_instruction::create_account(
            &wallet_pubkey,
            &mint_pubkey,
            rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ));

        // 2. Initialize mint account
        instructions.push(token_instruction::initialize_mint(
            &spl_token::id(),
            &mint_pubkey,
            &wallet_pubkey,
            None, // Freeze authority
            0,     // Decimals (0 for NFTs)
        ).map_err(|e| Error::Transaction(format!("Failed to create initialize mint instruction: {}", e)))?);

        // 3. Create associated token account for the owner
        let token_account = get_associated_token_address(&wallet_pubkey, &mint_pubkey);
        instructions.push(associated_token_instruction::create_associated_token_account(
            &wallet_pubkey,
            &wallet_pubkey,
            &mint_pubkey,
            &spl_token::id(),
        ));

        // 4. Mint 1 token to the owner's token account
        instructions.push(token_instruction::mint_to(
            &spl_token::id(),
            &mint_pubkey,
            &token_account,
            &wallet_pubkey,
            &[&wallet_pubkey],
            1, // Amount (1 for NFTs)
        ).map_err(|e| Error::Transaction(format!("Failed to create mint to instruction: {}", e)))?);

        // 5. Create metadata account
        let metadata_program_id = Pubkey::from_str(METADATA_PROGRAM_ID)
            .map_err(|e| Error::Transaction(format!("Invalid metadata program ID: {}", e)))?;

        let metadata_seeds = &[
            METADATA_PREFIX.as_bytes(),
            metadata_program_id.as_ref(),
            mint_pubkey.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &metadata_program_id);

        // Create metadata instruction data
        let creators = params.creators.clone().unwrap_or_default();
        let creators_vec: Vec<Creator> = creators.iter().map(|c| {
            Creator {
                address: Pubkey::from_str(&c.address).unwrap_or(wallet_pubkey),
                verified: c.verified,
                share: c.share,
            }
        }).collect();

        // If no creators are specified, add the wallet as the creator with 100% share
        let creators_data = if creators_vec.is_empty() {
            Some(vec![Creator {
                address: wallet_pubkey,
                verified: true,
                share: 100,
            }])
        } else {
            Some(creators_vec)
        };

        let data = MetadataData {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            seller_fee_basis_points: params.seller_fee_basis_points.unwrap_or(0),
            creators: creators_data,
        };

        // Create metadata instruction
        let create_metadata_ix = create_metadata_instruction(
            metadata_program_id,
            metadata_pubkey,
            mint_pubkey,
            wallet_pubkey,
            wallet_pubkey,
            wallet_pubkey,
            data,
            params.is_mutable.unwrap_or(true),
        );

        instructions.push(create_metadata_ix);

        // Create transaction
        let mut transaction = Transaction::new_with_payer(
            &instructions,
            Some(&wallet_pubkey),
        );

        // Set recent blockhash
        transaction.message.recent_blockhash = recent_blockhash;

        // Sign transaction with wallet keypair and mint keypair
        transaction.sign(&[keypair, &mint_keypair], recent_blockhash);

        // Send transaction
        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| Error::Transaction(format!("Failed to send transaction: {}", e)))?;

        // Return result
        Ok(NftMintResult {
            mint: mint_pubkey.to_string(),
            token_account: token_account.to_string(),
            metadata_account: metadata_pubkey.to_string(),
            signature: signature.to_string(),
        })
    }
}

/// Create metadata instruction
fn create_metadata_instruction(
    program_id: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    data: MetadataData,
    is_mutable: bool,
) -> Instruction {
    // Create instruction data
    let mut instruction_data = vec![CREATE_METADATA_ACCOUNTS_V3];

    // Serialize metadata
    let mut metadata = vec![];
    data.serialize(&mut metadata).unwrap();

    // Add metadata to instruction data
    instruction_data.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(&metadata);

    // Add is_mutable flag
    instruction_data.extend_from_slice(&[is_mutable as u8]);

    // Add collection details (none for now)
    instruction_data.extend_from_slice(&[0]); // No collection

    // Add uses details (none for now)
    instruction_data.extend_from_slice(&[0]); // No uses

    // Create accounts
    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(metadata_account, false),
        solana_sdk::instruction::AccountMeta::new(mint, false),
        solana_sdk::instruction::AccountMeta::new_readonly(mint_authority, true),
        solana_sdk::instruction::AccountMeta::new_readonly(payer, true),
        solana_sdk::instruction::AccountMeta::new_readonly(update_authority, true),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
    ];

    // Create instruction
    Instruction {
        program_id,
        accounts,
        data: instruction_data,
    }
}