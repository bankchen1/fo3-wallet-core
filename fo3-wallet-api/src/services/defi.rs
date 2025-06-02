//! DeFi service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};

use fo3_wallet::{
    crypto::keys::KeyType as WalletKeyType,
    defi::{self, SwapRequest, LendingRequest, StakingRequest, Protocol as WalletProtocol, LendingAction as WalletLendingAction, StakingAction as WalletStakingAction},
};

use crate::proto::fo3::wallet::v1::{
    defi_service_server::DefiService,
    *,
};
use crate::state::AppState;
use crate::error::{wallet_error_to_status, invalid_argument_error};

pub struct DefiServiceImpl {
    state: Arc<AppState>,
}

impl DefiServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl DefiService for DefiServiceImpl {
    async fn get_supported_tokens(
        &self,
        request: Request<GetSupportedTokensRequest>,
    ) -> Result<Response<GetSupportedTokensResponse>, Status> {
        let req = request.into_inner();
        
        let key_type = proto_to_wallet_key_type(req.key_type());
        let provider_config = match key_type {
            WalletKeyType::Ethereum => self.state.provider_config.clone(),
            WalletKeyType::Bitcoin => self.state.get_bitcoin_config(),
            #[cfg(feature = "solana")]
            WalletKeyType::Solana => self.state.get_solana_config(),
            #[cfg(not(feature = "solana"))]
            WalletKeyType::Solana => return Err(invalid_argument_error("Solana support not enabled")),
        };

        let tokens = defi::get_supported_tokens(key_type, &provider_config)
            .map_err(wallet_error_to_status)?;

        let proto_tokens = tokens.into_iter().map(|token| Token {
            address: token.address,
            symbol: token.symbol,
            name: token.name,
            decimals: token.decimals as i32,
            logo_uri: token.logo_uri.unwrap_or_default(),
            chain: wallet_key_type_to_proto(key_type) as i32,
        }).collect();

        let response = GetSupportedTokensResponse {
            tokens: proto_tokens,
        };

        Ok(Response::new(response))
    }

    async fn get_token_balance(
        &self,
        _request: Request<GetTokenBalanceRequest>,
    ) -> Result<Response<GetTokenBalanceResponse>, Status> {
        // Simple implementation - would query blockchain in real implementation
        let response = GetTokenBalanceResponse {
            balance: "0".to_string(),
            token: None,
        };

        Ok(Response::new(response))
    }

    async fn get_swap_quote(
        &self,
        request: Request<GetSwapQuoteRequest>,
    ) -> Result<Response<GetSwapQuoteResponse>, Status> {
        let req = request.into_inner();
        
        let input = req.input.ok_or_else(|| invalid_argument_error("Input token amount required"))?;
        let output_token = req.output_token.ok_or_else(|| invalid_argument_error("Output token required"))?;

        // Convert proto to wallet types
        let input_token = proto_to_wallet_token(input.token.ok_or_else(|| invalid_argument_error("Input token required"))?);
        let output_wallet_token = proto_to_wallet_token(output_token);

        let swap_request = SwapRequest {
            from: defi::TokenAmount {
                token: input_token,
                amount: input.amount,
            },
            to: output_wallet_token,
            slippage: req.slippage,
            protocol: proto_to_wallet_protocol(req.protocol()),
            deadline: Some(1800), // 30 minutes default
        };

        let provider_config = self.state.provider_config.clone();
        let quote = defi::get_swap_quote(&swap_request, &provider_config)
            .map_err(wallet_error_to_status)?;

        let proto_quote = SwapQuote {
            input: Some(TokenAmount {
                token: Some(wallet_token_to_proto(&quote.from.token)),
                amount: quote.from.amount,
            }),
            output: Some(TokenAmount {
                token: Some(wallet_token_to_proto(&quote.to)),
                amount: quote.minimum_output.unwrap_or_default(),
            }),
            price: quote.price.unwrap_or_default(),
            price_impact: quote.price_impact.unwrap_or_default(),
            minimum_output: quote.minimum_output.unwrap_or_default(),
            protocol: wallet_protocol_to_proto(quote.protocol) as i32,
            route: quote.route.unwrap_or_default(),
            deadline: quote.deadline.unwrap_or(1800),
        };

        let response = GetSwapQuoteResponse {
            quote: Some(proto_quote),
        };

        Ok(Response::new(response))
    }

    async fn execute_swap(
        &self,
        request: Request<ExecuteSwapRequest>,
    ) -> Result<Response<ExecuteSwapResponse>, Status> {
        let req = request.into_inner();
        
        let quote = req.quote.ok_or_else(|| invalid_argument_error("Quote required"))?;
        
        // Convert proto quote back to wallet swap request
        let input = quote.input.ok_or_else(|| invalid_argument_error("Input required"))?;
        let output = quote.output.ok_or_else(|| invalid_argument_error("Output required"))?;

        let swap_request = SwapRequest {
            from: defi::TokenAmount {
                token: proto_to_wallet_token(input.token.ok_or_else(|| invalid_argument_error("Input token required"))?),
                amount: input.amount,
            },
            to: proto_to_wallet_token(output.token.ok_or_else(|| invalid_argument_error("Output token required"))?),
            slippage: 0.5, // Default slippage
            protocol: proto_to_wallet_protocol(Protocol::try_from(quote.protocol).unwrap_or(Protocol::ProtocolUniswap)),
            deadline: Some(quote.deadline),
        };

        let provider_config = self.state.provider_config.clone();
        let result = defi::swap_tokens(&swap_request, &provider_config)
            .map_err(wallet_error_to_status)?;

        let response = ExecuteSwapResponse {
            transaction_hash: result.transaction_hash,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn get_lending_markets(
        &self,
        _request: Request<GetLendingMarketsRequest>,
    ) -> Result<Response<GetLendingMarketsResponse>, Status> {
        // Simple implementation - would query DeFi protocols in real implementation
        let response = GetLendingMarketsResponse {
            markets: vec![],
        };

        Ok(Response::new(response))
    }

    async fn execute_lending(
        &self,
        request: Request<ExecuteLendingRequest>,
    ) -> Result<Response<ExecuteLendingResponse>, Status> {
        let req = request.into_inner();
        
        let amount = req.amount.ok_or_else(|| invalid_argument_error("Amount required"))?;
        let token = proto_to_wallet_token(amount.token.ok_or_else(|| invalid_argument_error("Token required"))?);

        let lending_request = LendingRequest {
            action: proto_to_wallet_lending_action(LendingAction::try_from(req.action).unwrap_or(LendingAction::LendingActionSupply)),
            amount: defi::TokenAmount {
                token,
                amount: amount.amount,
            },
            protocol: proto_to_wallet_protocol(Protocol::try_from(req.protocol).unwrap_or(Protocol::ProtocolAave)),
        };

        let provider_config = self.state.provider_config.clone();
        let result = defi::execute_lending(&lending_request, &provider_config)
            .map_err(wallet_error_to_status)?;

        let response = ExecuteLendingResponse {
            transaction_hash: result.transaction_hash,
            success: true,
        };

        Ok(Response::new(response))
    }

    async fn get_staking_pools(
        &self,
        _request: Request<GetStakingPoolsRequest>,
    ) -> Result<Response<GetStakingPoolsResponse>, Status> {
        // Simple implementation - would query staking protocols in real implementation
        let response = GetStakingPoolsResponse {
            pools: vec![],
        };

        Ok(Response::new(response))
    }

    async fn execute_staking(
        &self,
        request: Request<ExecuteStakingRequest>,
    ) -> Result<Response<ExecuteStakingResponse>, Status> {
        let req = request.into_inner();
        
        let amount = req.amount.ok_or_else(|| invalid_argument_error("Amount required"))?;
        let token = proto_to_wallet_token(amount.token.ok_or_else(|| invalid_argument_error("Token required"))?);

        let staking_request = StakingRequest {
            action: proto_to_wallet_staking_action(StakingAction::try_from(req.action).unwrap_or(StakingAction::StakingActionStake), defi::TokenAmount {
                token,
                amount: amount.amount,
            }),
            protocol: proto_to_wallet_protocol(Protocol::try_from(req.protocol).unwrap_or(Protocol::ProtocolLido)),
        };

        let provider_config = self.state.provider_config.clone();
        let result = defi::execute_staking(&staking_request, &provider_config)
            .map_err(wallet_error_to_status)?;

        let response = ExecuteStakingResponse {
            transaction_hash: result.transaction_hash,
            success: true,
        };

        Ok(Response::new(response))
    }
}

// Helper functions for type conversion
fn proto_to_wallet_key_type(key_type: KeyType) -> WalletKeyType {
    match key_type {
        KeyType::KeyTypeEthereum => WalletKeyType::Ethereum,
        KeyType::KeyTypeBitcoin => WalletKeyType::Bitcoin,
        KeyType::KeyTypeSolana => WalletKeyType::Solana,
        _ => WalletKeyType::Ethereum,
    }
}

fn wallet_key_type_to_proto(key_type: WalletKeyType) -> KeyType {
    match key_type {
        WalletKeyType::Ethereum => KeyType::KeyTypeEthereum,
        WalletKeyType::Bitcoin => KeyType::KeyTypeBitcoin,
        WalletKeyType::Solana => KeyType::KeyTypeSolana,
    }
}

fn proto_to_wallet_token(token: Token) -> defi::Token {
    defi::Token {
        address: token.address,
        symbol: token.symbol,
        name: token.name,
        decimals: token.decimals as u8,
        logo_uri: if token.logo_uri.is_empty() { None } else { Some(token.logo_uri) },
    }
}

fn wallet_token_to_proto(token: &defi::Token) -> Token {
    Token {
        address: token.address.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        decimals: token.decimals as i32,
        logo_uri: token.logo_uri.clone().unwrap_or_default(),
        chain: 0, // Would be set based on context
    }
}

fn proto_to_wallet_protocol(protocol: Protocol) -> WalletProtocol {
    match protocol {
        Protocol::ProtocolUniswap => WalletProtocol::Uniswap,
        Protocol::ProtocolSushiswap => WalletProtocol::SushiSwap,
        Protocol::ProtocolAave => WalletProtocol::Aave,
        Protocol::ProtocolCompound => WalletProtocol::Compound,
        Protocol::ProtocolLido => WalletProtocol::Lido,
        Protocol::ProtocolRaydium => WalletProtocol::Raydium,
        Protocol::ProtocolOrca => WalletProtocol::Orca,
        Protocol::ProtocolMarinade => WalletProtocol::Marinade,
        _ => WalletProtocol::Uniswap,
    }
}

fn wallet_protocol_to_proto(protocol: WalletProtocol) -> Protocol {
    match protocol {
        WalletProtocol::Uniswap => Protocol::ProtocolUniswap,
        WalletProtocol::SushiSwap => Protocol::ProtocolSushiswap,
        WalletProtocol::Aave => Protocol::ProtocolAave,
        WalletProtocol::Compound => Protocol::ProtocolCompound,
        WalletProtocol::Lido => Protocol::ProtocolLido,
        WalletProtocol::Raydium => Protocol::ProtocolRaydium,
        WalletProtocol::Orca => Protocol::ProtocolOrca,
        WalletProtocol::Marinade => Protocol::ProtocolMarinade,
    }
}

fn proto_to_wallet_lending_action(action: LendingAction) -> WalletLendingAction {
    match action {
        LendingAction::LendingActionSupply => WalletLendingAction::Supply,
        LendingAction::LendingActionWithdraw => WalletLendingAction::Withdraw,
        LendingAction::LendingActionBorrow => WalletLendingAction::Borrow,
        LendingAction::LendingActionRepay => WalletLendingAction::Repay,
        _ => WalletLendingAction::Supply,
    }
}

fn proto_to_wallet_staking_action(action: StakingAction, amount: defi::TokenAmount) -> WalletStakingAction {
    match action {
        StakingAction::StakingActionStake => WalletStakingAction::Stake(amount),
        StakingAction::StakingActionUnstake => WalletStakingAction::Unstake(amount),
        StakingAction::StakingActionClaimRewards => WalletStakingAction::ClaimRewards,
        _ => WalletStakingAction::Stake(amount),
    }
}
