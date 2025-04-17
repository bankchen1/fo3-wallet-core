//! Solana NFT functionality
//!
//! This module provides functionality for interacting with NFTs on Solana,
//! including querying NFTs owned by a wallet and fetching NFT metadata.

use std::str::FromStr;
use serde::{Serialize, Deserialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    program_pack::Pack,
};
use spl_token::state::Account as TokenAccount;
use borsh::{BorshDeserialize, BorshSerialize};

use fo3_wallet::error::{Error, Result};

/// Metaplex program ID
pub const METADATA_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/// Metaplex token metadata account prefix
pub const METADATA_PREFIX: &str = "metadata";

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
            solana_client::rpc_config::TokenAccountsFilter::ProgramId(spl_token::id()),
        ).map_err(|e| Error::Transaction(format!("Failed to get token accounts: {}", e)))?;

        let mut nfts = Vec::new();

        // Filter for NFTs (tokens with amount = 1)
        for account in token_accounts {
            let data = account.account.data.clone();
            let token_account = TokenAccount::unpack(&data)
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
    async fn fetch_external_metadata(&self, uri: &str) -> Result<ExternalMetadata> {
        // This would normally be an async HTTP request
        // For simplicity, we'll just return an error
        // In a real implementation, you would use reqwest or another HTTP client
        Err(Error::Transaction("External metadata fetching not implemented".to_string()))
    }
}
