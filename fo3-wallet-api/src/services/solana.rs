//! Solana service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};

#[cfg(feature = "solana")]
use fo3_wallet_solana::{SolanaProvider, TokenTransferParams, StakingParams, NftMintParams, NftCreator as SolanaNftCreator};

use crate::proto::fo3::wallet::v1::{
    solana_service_server::SolanaService,
    *,
};
use crate::state::AppState;
use crate::error::{wallet_error_to_status, string_error_to_status, invalid_argument_error};

pub struct SolanaServiceImpl {
    state: Arc<AppState>,
}

impl SolanaServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[cfg(feature = "solana")]
#[tonic::async_trait]
impl SolanaService for SolanaServiceImpl {
    async fn get_nfts_by_owner(
        &self,
        request: Request<GetNftsByOwnerRequest>,
    ) -> Result<Response<GetNftsByOwnerResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let nfts = provider.get_nfts_by_owner(&req.wallet_address).await
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_nfts = nfts.into_iter().map(|nft| NftToken {
            mint: nft.mint,
            name: nft.name,
            symbol: nft.symbol,
            uri: nft.uri,
            owner: nft.owner,
            is_mutable: nft.is_mutable,
            creators: nft.creators.into_iter().map(|c| NftCreator {
                address: c.address,
                verified: c.verified,
                share: c.share as i32,
            }).collect(),
            seller_fee_basis_points: nft.seller_fee_basis_points as i32,
        }).collect();

        let response = GetNftsByOwnerResponse {
            nfts: proto_nfts,
        };

        Ok(Response::new(response))
    }

    async fn get_nft_metadata(
        &self,
        request: Request<GetNftMetadataRequest>,
    ) -> Result<Response<GetNftMetadataResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let metadata = provider.get_nft_metadata(&req.mint).await
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_metadata = NftMetadata {
            name: metadata.name,
            symbol: metadata.symbol,
            description: metadata.description.unwrap_or_default(),
            image: metadata.image.unwrap_or_default(),
            animation_url: metadata.animation_url.unwrap_or_default(),
            external_url: metadata.external_url.unwrap_or_default(),
            attributes: metadata.attributes.unwrap_or_default().into_iter().map(|attr| NftAttribute {
                trait_type: attr.trait_type,
                value: attr.value,
            }).collect(),
        };

        let response = GetNftMetadataResponse {
            metadata: Some(proto_metadata),
        };

        Ok(Response::new(response))
    }

    async fn transfer_nft(
        &self,
        request: Request<TransferNftRequest>,
    ) -> Result<Response<TransferNftResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let signature = provider.transfer_nft(
            &req.from_address,
            &req.to_address,
            &req.mint,
            &req.private_key,
        ).await.map_err(|e| string_error_to_status(e.to_string()))?;

        let response = TransferNftResponse {
            transaction_hash: signature,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn mint_nft(
        &self,
        request: Request<MintNftRequest>,
    ) -> Result<Response<MintNftResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let creators = req.creators.into_iter().map(|c| SolanaNftCreator {
            address: c.address,
            verified: c.verified,
            share: c.share as u8,
        }).collect();

        let params = NftMintParams {
            name: req.name,
            symbol: req.symbol,
            uri: req.uri,
            seller_fee_basis_points: Some(req.seller_fee_basis_points as u16),
            creators: Some(creators),
            is_mutable: Some(req.is_mutable),
        };

        let result = provider.mint_nft(&req.wallet_address, &req.private_key, &params).await
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let response = MintNftResponse {
            mint: result.mint,
            transaction_hash: result.signature,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn get_token_info(
        &self,
        request: Request<GetTokenInfoRequest>,
    ) -> Result<Response<GetTokenInfoResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let token_info = provider.get_token_info(&req.token_mint).await
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_token_info = TokenInfo {
            mint: token_info.mint,
            name: token_info.name,
            symbol: token_info.symbol,
            decimals: token_info.decimals as i32,
            supply: token_info.supply,
            is_initialized: token_info.is_initialized,
            freeze_authority: token_info.freeze_authority.unwrap_or_default(),
            mint_authority: token_info.mint_authority.unwrap_or_default(),
        };

        let response = GetTokenInfoResponse {
            token_info: Some(proto_token_info),
        };

        Ok(Response::new(response))
    }

    async fn transfer_tokens(
        &self,
        request: Request<TransferTokensRequest>,
    ) -> Result<Response<TransferTokensResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let keypair = provider.private_key_to_keypair(&req.private_key)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let params = TokenTransferParams {
            token_mint: req.token_mint,
            from: req.from_address,
            to: req.to_address,
            amount: req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?,
            decimals: req.decimals as u8,
        };

        let transaction = provider.create_token_transfer_transaction(&params, &keypair.pubkey())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let signed_transaction = transaction.sign(&[&keypair], transaction.message.recent_blockhash);
        let serialized = bincode::serialize(&signed_transaction)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let signature = provider.broadcast_transaction(&serialized)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let response = TransferTokensResponse {
            transaction_hash: signature,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn stake_sol(
        &self,
        request: Request<StakeSolRequest>,
    ) -> Result<Response<StakeSolResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let keypair = provider.private_key_to_keypair(&req.private_key)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let params = StakingParams {
            from: req.wallet_address,
            validator: req.validator_address,
            amount: req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?,
        };

        let transaction = provider.create_stake_transaction(&params, &keypair.pubkey())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let stake_account = solana_sdk::signature::Keypair::new();
        let signed_transaction = transaction.sign(&[&keypair, &stake_account], transaction.message.recent_blockhash);
        let serialized = bincode::serialize(&signed_transaction)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let signature = provider.broadcast_transaction(&serialized)
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let response = StakeSolResponse {
            stake_account: stake_account.pubkey().to_string(),
            transaction_hash: signature,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn get_staking_info(
        &self,
        _request: Request<GetStakingInfoRequest>,
    ) -> Result<Response<GetStakingInfoResponse>, Status> {
        // Simple implementation - would query staking info in real implementation
        let response = GetStakingInfoResponse {
            staking_info: None,
        };

        Ok(Response::new(response))
    }

    async fn get_raydium_pairs(
        &self,
        _request: Request<GetRaydiumPairsRequest>,
    ) -> Result<Response<GetRaydiumPairsResponse>, Status> {
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let pairs = provider.get_raydium_token_pairs()
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_pairs = pairs.into_iter().map(|(token_a, token_b)| TokenPair {
            token_a,
            token_b,
            pool_address: String::new(), // Would be filled in real implementation
        }).collect();

        let response = GetRaydiumPairsResponse {
            pairs: proto_pairs,
        };

        Ok(Response::new(response))
    }

    async fn get_raydium_quote(
        &self,
        request: Request<GetRaydiumQuoteRequest>,
    ) -> Result<Response<GetRaydiumQuoteResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let amount_in = req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?;

        let quote = provider.get_raydium_swap_quote(
            &req.input_mint,
            &req.output_mint,
            amount_in,
            req.slippage,
        ).map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_quote = SolanaSwapQuote {
            input_amount: quote.amount_in.to_string(),
            output_amount: quote.amount_out.to_string(),
            min_output_amount: quote.min_out_amount.to_string(),
            price_impact: quote.price_impact.to_string(),
            fee: quote.fee.to_string(),
            route: quote.route,
        };

        let response = GetRaydiumQuoteResponse {
            quote: Some(proto_quote),
        };

        Ok(Response::new(response))
    }

    async fn execute_raydium_swap(
        &self,
        request: Request<ExecuteRaydiumSwapRequest>,
    ) -> Result<Response<ExecuteRaydiumSwapResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let amount_in = req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?;
        let min_amount_out = req.min_amount_out.parse().map_err(|_| invalid_argument_error("Invalid min amount"))?;

        let signature = provider.execute_raydium_swap(
            &req.input_mint,
            &req.output_mint,
            amount_in,
            min_amount_out,
            &req.wallet_address,
            &req.private_key,
        ).map_err(|e| string_error_to_status(e.to_string()))?;

        let response = ExecuteRaydiumSwapResponse {
            transaction_hash: signature,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn get_orca_pairs(
        &self,
        _request: Request<GetOrcaPairsRequest>,
    ) -> Result<Response<GetOrcaPairsResponse>, Status> {
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let pairs = provider.get_orca_token_pairs()
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_pairs = pairs.into_iter().map(|(token_a, token_b)| TokenPair {
            token_a,
            token_b,
            pool_address: String::new(), // Would be filled in real implementation
        }).collect();

        let response = GetOrcaPairsResponse {
            pairs: proto_pairs,
        };

        Ok(Response::new(response))
    }

    async fn get_orca_quote(
        &self,
        request: Request<GetOrcaQuoteRequest>,
    ) -> Result<Response<GetOrcaQuoteResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let amount_in = req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?;

        let quote = provider.get_orca_swap_quote(
            &req.input_mint,
            &req.output_mint,
            amount_in,
            req.slippage,
        ).map_err(|e| string_error_to_status(e.to_string()))?;

        let proto_quote = SolanaSwapQuote {
            input_amount: quote.amount_in.to_string(),
            output_amount: quote.amount_out.to_string(),
            min_output_amount: quote.min_out_amount.to_string(),
            price_impact: quote.price_impact.to_string(),
            fee: quote.fee.to_string(),
            route: quote.route,
        };

        let response = GetOrcaQuoteResponse {
            quote: Some(proto_quote),
        };

        Ok(Response::new(response))
    }

    async fn execute_orca_swap(
        &self,
        request: Request<ExecuteOrcaSwapRequest>,
    ) -> Result<Response<ExecuteOrcaSwapResponse>, Status> {
        let req = request.into_inner();
        
        let provider = SolanaProvider::new(self.state.get_solana_config())
            .map_err(|e| string_error_to_status(e.to_string()))?;

        let amount_in = req.amount.parse().map_err(|_| invalid_argument_error("Invalid amount"))?;
        let min_amount_out = req.min_amount_out.parse().map_err(|_| invalid_argument_error("Invalid min amount"))?;

        let signature = provider.execute_orca_swap(
            &req.input_mint,
            &req.output_mint,
            amount_in,
            min_amount_out,
            &req.wallet_address,
            &req.private_key,
        ).map_err(|e| string_error_to_status(e.to_string()))?;

        let response = ExecuteOrcaSwapResponse {
            transaction_hash: signature,
            success: true,
        };

        Ok(Response::new(response))
    }
}

#[cfg(not(feature = "solana"))]
#[tonic::async_trait]
impl SolanaService for SolanaServiceImpl {
    async fn get_nfts_by_owner(&self, _request: Request<GetNftsByOwnerRequest>) -> Result<Response<GetNftsByOwnerResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_nft_metadata(&self, _request: Request<GetNftMetadataRequest>) -> Result<Response<GetNftMetadataResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn transfer_nft(&self, _request: Request<TransferNftRequest>) -> Result<Response<TransferNftResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn mint_nft(&self, _request: Request<MintNftRequest>) -> Result<Response<MintNftResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_token_info(&self, _request: Request<GetTokenInfoRequest>) -> Result<Response<GetTokenInfoResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn transfer_tokens(&self, _request: Request<TransferTokensRequest>) -> Result<Response<TransferTokensResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn stake_sol(&self, _request: Request<StakeSolRequest>) -> Result<Response<StakeSolResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_staking_info(&self, _request: Request<GetStakingInfoRequest>) -> Result<Response<GetStakingInfoResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_raydium_pairs(&self, _request: Request<GetRaydiumPairsRequest>) -> Result<Response<GetRaydiumPairsResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_raydium_quote(&self, _request: Request<GetRaydiumQuoteRequest>) -> Result<Response<GetRaydiumQuoteResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn execute_raydium_swap(&self, _request: Request<ExecuteRaydiumSwapRequest>) -> Result<Response<ExecuteRaydiumSwapResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_orca_pairs(&self, _request: Request<GetOrcaPairsRequest>) -> Result<Response<GetOrcaPairsResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn get_orca_quote(&self, _request: Request<GetOrcaQuoteRequest>) -> Result<Response<GetOrcaQuoteResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }

    async fn execute_orca_swap(&self, _request: Request<ExecuteOrcaSwapRequest>) -> Result<Response<ExecuteOrcaSwapResponse>, Status> {
        Err(Status::unimplemented("Solana support not enabled"))
    }
}
