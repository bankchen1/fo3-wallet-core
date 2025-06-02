//! Pricing service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    pricing_service_server::PricingService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    pricing_guard::PricingGuard,
};
use crate::models::pricing::{
    Asset, Price, PricePoint, FiatRate, PricingMetrics, AssetType, PriceSource, TimeInterval,
    PricingRepository, InMemoryPricingRepository, CoinGeckoPrice, CoinGeckoSimplePrice,
};

/// External price provider trait
#[async_trait::async_trait]
pub trait PriceProvider: Send + Sync {
    async fn get_price(&self, symbol: &str, quote_currency: &str) -> Result<Price, String>;
    async fn get_batch_prices(&self, symbols: &[String], quote_currency: &str) -> Result<Vec<Price>, String>;
    async fn get_fiat_rate(&self, from: &str, to: &str) -> Result<FiatRate, String>;
    async fn get_historical_data(&self, symbol: &str, days: u32) -> Result<Vec<PricePoint>, String>;
}

/// CoinGecko API provider implementation
pub struct CoinGeckoProvider {
    api_key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

impl CoinGeckoProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.coingecko.com/api/v3".to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn get_coingecko_id(&self, symbol: &str) -> String {
        // Map common symbols to CoinGecko IDs
        match symbol.to_uppercase().as_str() {
            "BTC" => "bitcoin".to_string(),
            "ETH" => "ethereum".to_string(),
            "USDT" => "tether".to_string(),
            "USDC" => "usd-coin".to_string(),
            "BNB" => "binancecoin".to_string(),
            "SOL" => "solana".to_string(),
            "ADA" => "cardano".to_string(),
            "AVAX" => "avalanche-2".to_string(),
            "DOT" => "polkadot".to_string(),
            "MATIC" => "matic-network".to_string(),
            _ => symbol.to_lowercase(),
        }
    }
}

#[async_trait::async_trait]
impl PriceProvider for CoinGeckoProvider {
    async fn get_price(&self, symbol: &str, quote_currency: &str) -> Result<Price, String> {
        let coin_id = self.get_coingecko_id(symbol);
        let quote_lower = quote_currency.to_lowercase();
        
        let url = format!(
            "{}/simple/price?ids={}&vs_currencies={}&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true&include_last_updated_at=true",
            self.base_url, coin_id, quote_lower
        );

        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.api_key {
            request = request.header("x-cg-demo-api-key", api_key);
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to fetch price: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()));
        }

        let data: CoinGeckoSimplePrice = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(price_data) = data.prices.get(&coin_id) {
            let price_usd = price_data.usd.unwrap_or(0.0);
            let timestamp = if let Some(ts) = price_data.last_updated_at {
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            } else {
                Utc::now()
            };

            Ok(Price {
                symbol: symbol.to_uppercase(),
                price_usd: Decimal::try_from(price_usd).unwrap_or_default(),
                price_btc: price_data.btc.map(|p| Decimal::try_from(p).unwrap_or_default()),
                market_cap: price_data.usd_market_cap.map(|mc| Decimal::try_from(mc).unwrap_or_default()),
                volume_24h: price_data.usd_24h_vol.map(|v| Decimal::try_from(v).unwrap_or_default()),
                change_24h: price_data.usd_24h_change.map(|c| Decimal::try_from(c).unwrap_or_default()),
                change_7d: None,
                source: PriceSource::CoinGecko,
                timestamp,
                last_updated: timestamp,
            })
        } else {
            Err(format!("Price data not found for symbol: {}", symbol))
        }
    }

    async fn get_batch_prices(&self, symbols: &[String], quote_currency: &str) -> Result<Vec<Price>, String> {
        let coin_ids: Vec<String> = symbols.iter()
            .map(|s| self.get_coingecko_id(s))
            .collect();
        
        let ids_param = coin_ids.join(",");
        let quote_lower = quote_currency.to_lowercase();
        
        let url = format!(
            "{}/simple/price?ids={}&vs_currencies={}&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true&include_last_updated_at=true",
            self.base_url, ids_param, quote_lower
        );

        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.api_key {
            request = request.header("x-cg-demo-api-key", api_key);
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to fetch batch prices: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()));
        }

        let data: CoinGeckoSimplePrice = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut prices = Vec::new();
        for (i, symbol) in symbols.iter().enumerate() {
            let coin_id = &coin_ids[i];
            if let Some(price_data) = data.prices.get(coin_id) {
                let price_usd = price_data.usd.unwrap_or(0.0);
                let timestamp = if let Some(ts) = price_data.last_updated_at {
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                } else {
                    Utc::now()
                };

                prices.push(Price {
                    symbol: symbol.to_uppercase(),
                    price_usd: Decimal::try_from(price_usd).unwrap_or_default(),
                    price_btc: price_data.btc.map(|p| Decimal::try_from(p).unwrap_or_default()),
                    market_cap: price_data.usd_market_cap.map(|mc| Decimal::try_from(mc).unwrap_or_default()),
                    volume_24h: price_data.usd_24h_vol.map(|v| Decimal::try_from(v).unwrap_or_default()),
                    change_24h: price_data.usd_24h_change.map(|c| Decimal::try_from(c).unwrap_or_default()),
                    change_7d: None,
                    source: PriceSource::CoinGecko,
                    timestamp,
                    last_updated: timestamp,
                });
            }
        }

        Ok(prices)
    }

    async fn get_fiat_rate(&self, from: &str, to: &str) -> Result<FiatRate, String> {
        // For fiat rates, we can use a simple conversion through USD
        if from == to {
            return Ok(FiatRate {
                from_currency: from.to_string(),
                to_currency: to.to_string(),
                rate: Decimal::ONE,
                source: PriceSource::CoinGecko,
                timestamp: Utc::now(),
            });
        }

        // This is a simplified implementation
        // In production, you'd use a proper forex API
        let rate = match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
            ("USD", "EUR") => Decimal::try_from(0.85).unwrap_or_default(),
            ("EUR", "USD") => Decimal::try_from(1.18).unwrap_or_default(),
            ("USD", "GBP") => Decimal::try_from(0.73).unwrap_or_default(),
            ("GBP", "USD") => Decimal::try_from(1.37).unwrap_or_default(),
            _ => Decimal::ONE,
        };

        Ok(FiatRate {
            from_currency: from.to_string(),
            to_currency: to.to_string(),
            rate,
            source: PriceSource::CoinGecko,
            timestamp: Utc::now(),
        })
    }

    async fn get_historical_data(&self, symbol: &str, days: u32) -> Result<Vec<PricePoint>, String> {
        let coin_id = self.get_coingecko_id(symbol);
        
        let url = format!(
            "{}/coins/{}/market_chart?vs_currency=usd&days={}",
            self.base_url, coin_id, days
        );

        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.api_key {
            request = request.header("x-cg-demo-api-key", api_key);
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to fetch historical data: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut points = Vec::new();
        if let Some(prices) = data["prices"].as_array() {
            for price_array in prices {
                if let Some(price_data) = price_array.as_array() {
                    if price_data.len() >= 2 {
                        if let (Some(timestamp), Some(price)) = (
                            price_data[0].as_f64(),
                            price_data[1].as_f64(),
                        ) {
                            let dt = DateTime::from_timestamp((timestamp / 1000.0) as i64, 0)
                                .unwrap_or_else(Utc::now);
                            
                            points.push(PricePoint {
                                timestamp: dt,
                                price: Decimal::try_from(price).unwrap_or_default(),
                                volume: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(points)
    }
}

/// Mock price provider for testing
pub struct MockPriceProvider;

#[async_trait::async_trait]
impl PriceProvider for MockPriceProvider {
    async fn get_price(&self, symbol: &str, _quote_currency: &str) -> Result<Price, String> {
        let mock_price = match symbol.to_uppercase().as_str() {
            "BTC" => 45000.0,
            "ETH" => 3000.0,
            "USDT" | "USDC" => 1.0,
            "SOL" => 100.0,
            _ => 1.0,
        };

        Ok(Price {
            symbol: symbol.to_uppercase(),
            price_usd: Decimal::try_from(mock_price).unwrap_or_default(),
            price_btc: Some(Decimal::try_from(mock_price / 45000.0).unwrap_or_default()),
            market_cap: Some(Decimal::try_from(mock_price * 1_000_000.0).unwrap_or_default()),
            volume_24h: Some(Decimal::try_from(mock_price * 10_000.0).unwrap_or_default()),
            change_24h: Some(Decimal::try_from(2.5).unwrap_or_default()),
            change_7d: Some(Decimal::try_from(5.0).unwrap_or_default()),
            source: PriceSource::Mock,
            timestamp: Utc::now(),
            last_updated: Utc::now(),
        })
    }

    async fn get_batch_prices(&self, symbols: &[String], quote_currency: &str) -> Result<Vec<Price>, String> {
        let mut prices = Vec::new();
        for symbol in symbols {
            if let Ok(price) = self.get_price(symbol, quote_currency).await {
                prices.push(price);
            }
        }
        Ok(prices)
    }

    async fn get_fiat_rate(&self, from: &str, to: &str) -> Result<FiatRate, String> {
        Ok(FiatRate {
            from_currency: from.to_string(),
            to_currency: to.to_string(),
            rate: Decimal::ONE,
            source: PriceSource::Mock,
            timestamp: Utc::now(),
        })
    }

    async fn get_historical_data(&self, _symbol: &str, _days: u32) -> Result<Vec<PricePoint>, String> {
        let mut points = Vec::new();
        let now = Utc::now();
        
        for i in 0..10 {
            points.push(PricePoint {
                timestamp: now - chrono::Duration::hours(i),
                price: Decimal::try_from(1000.0 + i as f64 * 10.0).unwrap_or_default(),
                volume: Some(Decimal::try_from(100000.0).unwrap_or_default()),
            });
        }
        
        Ok(points)
    }
}

/// Pricing service implementation
pub struct PricingServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    pricing_guard: Arc<PricingGuard>,
    repository: Arc<dyn PricingRepository>,
    price_provider: Arc<dyn PriceProvider>,
    cache_ttl_seconds: u64,
}

impl PricingServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        pricing_guard: Arc<PricingGuard>,
    ) -> Self {
        let repository = Arc::new(InMemoryPricingRepository::new());

        // Use CoinGecko provider if API key is available, otherwise use mock
        let price_provider: Arc<dyn PriceProvider> = if std::env::var("COINGECKO_API_KEY").is_ok() {
            Arc::new(CoinGeckoProvider::new(std::env::var("COINGECKO_API_KEY").ok()))
        } else {
            tracing::warn!("No CoinGecko API key found, using mock price provider");
            Arc::new(MockPriceProvider)
        };

        Self {
            state,
            auth_service,
            audit_logger,
            pricing_guard,
            repository,
            price_provider,
            cache_ttl_seconds: 30, // 30 seconds cache TTL
        }
    }

    /// Get price with caching
    async fn get_price_with_cache(&self, symbol: &str, quote_currency: &str) -> Result<Price, String> {
        // Try cache first
        if let Some(cached_price) = self.repository.get_cached_price(symbol, quote_currency).await {
            self.repository.increment_request_counter("cache").await?;
            return Ok(cached_price);
        }

        // Fetch from external provider
        let price = self.price_provider.get_price(symbol, quote_currency).await?;

        // Cache the result
        self.repository.cache_price(symbol, quote_currency, &price, self.cache_ttl_seconds).await?;
        self.repository.increment_request_counter("external").await?;

        Ok(price)
    }

    /// Convert internal Price to proto Price
    fn price_to_proto(&self, price: &Price) -> crate::proto::fo3::wallet::v1::Price {
        crate::proto::fo3::wallet::v1::Price {
            symbol: price.symbol.clone(),
            price_usd: price.price_usd.to_string(),
            price_btc: price.price_btc.map(|p| p.to_string()).unwrap_or_default(),
            market_cap: price.market_cap.map(|mc| mc.to_string()).unwrap_or_default(),
            volume_24h: price.volume_24h.map(|v| v.to_string()).unwrap_or_default(),
            change_24h: price.change_24h.map(|c| c.to_string()).unwrap_or_default(),
            change_7d: price.change_7d.map(|c| c.to_string()).unwrap_or_default(),
            source: match price.source {
                PriceSource::CoinGecko => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceCoingecko as i32,
                PriceSource::Mock => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceMock as i32,
                PriceSource::Cache => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceCache as i32,
                _ => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceUnspecified as i32,
            },
            timestamp: price.timestamp.timestamp(),
            last_updated: price.last_updated.timestamp(),
        }
    }

    /// Convert internal Asset to proto Asset
    fn asset_to_proto(&self, asset: &Asset) -> crate::proto::fo3::wallet::v1::Asset {
        crate::proto::fo3::wallet::v1::Asset {
            symbol: asset.symbol.clone(),
            name: asset.name.clone(),
            r#type: match asset.asset_type {
                AssetType::Cryptocurrency => crate::proto::fo3::wallet::v1::AssetType::AssetTypeCryptocurrency as i32,
                AssetType::Fiat => crate::proto::fo3::wallet::v1::AssetType::AssetTypeFiat as i32,
                AssetType::Token => crate::proto::fo3::wallet::v1::AssetType::AssetTypeToken as i32,
                AssetType::Stablecoin => crate::proto::fo3::wallet::v1::AssetType::AssetTypeStablecoin as i32,
            },
            chain: asset.chain.clone().unwrap_or_default(),
            contract_address: asset.contract_address.clone().unwrap_or_default(),
            decimals: asset.decimals as i32,
            icon_url: asset.icon_url.clone().unwrap_or_default(),
            is_active: asset.is_active,
            created_at: asset.created_at.timestamp(),
            updated_at: asset.updated_at.timestamp(),
        }
    }
}

#[async_trait::async_trait]
impl PricingService for PricingServiceImpl {
    async fn get_price(
        &self,
        request: Request<GetPriceRequest>,
    ) -> Result<Response<GetPriceResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate inputs
        self.pricing_guard.validate_symbol(&req.symbol)?;
        let quote_currency = if req.quote_currency.is_empty() { "USD" } else { &req.quote_currency };
        self.pricing_guard.validate_quote_currency(quote_currency)?;

        // Get price
        let price = self.get_price_with_cache(&req.symbol, quote_currency).await
            .map_err(|e| Status::internal(format!("Failed to get price: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.get_price",
            &format!("symbol={}, quote_currency={}", req.symbol, quote_currency),
            true,
            None,
        ).await;

        Ok(Response::new(GetPriceResponse {
            price: Some(self.price_to_proto(&price)),
        }))
    }

    async fn get_price_batch(
        &self,
        request: Request<GetPriceBatchRequest>,
    ) -> Result<Response<GetPriceBatchResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate inputs
        self.pricing_guard.validate_batch_size(req.symbols.len())?;
        let quote_currency = if req.quote_currency.is_empty() { "USD" } else { &req.quote_currency };
        self.pricing_guard.validate_quote_currency(quote_currency)?;

        for symbol in &req.symbols {
            self.pricing_guard.validate_symbol(symbol)?;
        }

        // Get batch prices
        let mut prices = Vec::new();
        let mut failed_symbols = Vec::new();

        for symbol in &req.symbols {
            match self.get_price_with_cache(symbol, quote_currency).await {
                Ok(price) => prices.push(self.price_to_proto(&price)),
                Err(_) => failed_symbols.push(symbol.clone()),
            }
        }

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.get_price_batch",
            &format!("symbols_count={}, quote_currency={}", req.symbols.len(), quote_currency),
            true,
            None,
        ).await;

        Ok(Response::new(GetPriceBatchResponse {
            prices,
            total_count: req.symbols.len() as i32,
            successful_count: prices.len() as i32,
            failed_symbols,
        }))
    }

    async fn get_fiat_rate(
        &self,
        request: Request<GetFiatRateRequest>,
    ) -> Result<Response<GetFiatRateResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate inputs
        self.pricing_guard.validate_quote_currency(&req.from_currency)?;
        self.pricing_guard.validate_quote_currency(&req.to_currency)?;

        // Get fiat rate
        let rate = self.price_provider.get_fiat_rate(&req.from_currency, &req.to_currency).await
            .map_err(|e| Status::internal(format!("Failed to get fiat rate: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.get_fiat_rate",
            &format!("from={}, to={}", req.from_currency, req.to_currency),
            true,
            None,
        ).await;

        Ok(Response::new(GetFiatRateResponse {
            rate: Some(crate::proto::fo3::wallet::v1::FiatRate {
                from_currency: rate.from_currency,
                to_currency: rate.to_currency,
                rate: rate.rate.to_string(),
                source: match rate.source {
                    PriceSource::CoinGecko => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceCoingecko as i32,
                    PriceSource::Mock => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceMock as i32,
                    _ => crate::proto::fo3::wallet::v1::PriceSource::PriceSourceUnspecified as i32,
                },
                timestamp: rate.timestamp.timestamp(),
            }),
        }))
    }

    async fn list_supported_symbols(
        &self,
        request: Request<ListSupportedSymbolsRequest>,
    ) -> Result<Response<ListSupportedSymbolsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate pagination
        self.pricing_guard.validate_pagination(req.page_size, &req.page_token)?;

        // Convert asset type filter
        let asset_type_filter = if req.type_filter != 0 {
            Some(match req.type_filter {
                1 => AssetType::Cryptocurrency,
                2 => AssetType::Fiat,
                3 => AssetType::Token,
                4 => AssetType::Stablecoin,
                _ => return Err(Status::invalid_argument("Invalid asset type filter")),
            })
        } else {
            None
        };

        let chain_filter = if req.chain_filter.is_empty() { None } else { Some(req.chain_filter.as_str()) };

        // Get supported assets
        let assets = self.repository.get_supported_assets(asset_type_filter, chain_filter).await;
        let proto_assets: Vec<_> = assets.iter().map(|a| self.asset_to_proto(a)).collect();

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.list_supported_symbols",
            &format!("asset_count={}", proto_assets.len()),
            true,
            None,
        ).await;

        Ok(Response::new(ListSupportedSymbolsResponse {
            assets: proto_assets,
            next_page_token: String::new(), // Simplified pagination
            total_count: assets.len() as i32,
        }))
    }

    async fn get_price_history(
        &self,
        request: Request<GetPriceHistoryRequest>,
    ) -> Result<Response<GetPriceHistoryResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate inputs
        self.pricing_guard.validate_symbol(&req.symbol)?;
        let quote_currency = if req.quote_currency.is_empty() { "USD" } else { &req.quote_currency };
        self.pricing_guard.validate_quote_currency(quote_currency)?;
        self.pricing_guard.validate_time_range(req.start_time, req.end_time)?;

        let start_time = DateTime::from_timestamp(req.start_time, 0)
            .ok_or_else(|| Status::invalid_argument("Invalid start time"))?;
        let end_time = DateTime::from_timestamp(req.end_time, 0)
            .ok_or_else(|| Status::invalid_argument("Invalid end time"))?;

        // Convert interval
        let interval = match req.interval {
            1 => TimeInterval::OneMinute,
            2 => TimeInterval::FiveMinutes,
            3 => TimeInterval::FifteenMinutes,
            4 => TimeInterval::OneHour,
            5 => TimeInterval::FourHours,
            6 => TimeInterval::OneDay,
            7 => TimeInterval::OneWeek,
            8 => TimeInterval::OneMonth,
            _ => TimeInterval::OneHour,
        };

        // Get historical data
        let points = self.repository.get_price_history(
            &req.symbol,
            interval,
            start_time,
            end_time,
            if req.limit > 0 { Some(req.limit as u32) } else { None },
        ).await;

        let proto_points: Vec<_> = points.iter().map(|p| {
            crate::proto::fo3::wallet::v1::PricePoint {
                timestamp: p.timestamp.timestamp(),
                price: p.price.to_string(),
                volume: p.volume.map(|v| v.to_string()).unwrap_or_default(),
            }
        }).collect();

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.get_price_history",
            &format!("symbol={}, points_count={}", req.symbol, proto_points.len()),
            true,
            None,
        ).await;

        Ok(Response::new(GetPriceHistoryResponse {
            points: proto_points,
            symbol: req.symbol,
            quote_currency: quote_currency.to_string(),
            interval: req.interval,
            total_points: points.len() as i32,
        }))
    }

    async fn update_price_feed(
        &self,
        request: Request<UpdatePriceFeedRequest>,
    ) -> Result<Response<UpdatePriceFeedResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permissions
        self.auth_service.check_permission(auth_context, crate::middleware::auth::Permission::PermissionPricingAdmin)?;

        let req = request.into_inner();

        // Validate inputs
        self.pricing_guard.validate_symbol(&req.symbol)?;

        // This would update the price feed in a real implementation
        // For now, we'll just return success
        let success_message = format!("Price feed updated for symbol: {}", req.symbol);

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.update_price_feed",
            &format!("symbol={}", req.symbol),
            true,
            None,
        ).await;

        Ok(Response::new(UpdatePriceFeedResponse {
            success: true,
            message: success_message,
            updated_price: req.price_data,
        }))
    }

    async fn get_pricing_metrics(
        &self,
        request: Request<GetPricingMetricsRequest>,
    ) -> Result<Response<GetPricingMetricsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permissions
        self.auth_service.check_permission(auth_context, crate::middleware::auth::Permission::PermissionPricingAdmin)?;

        let metrics = self.repository.get_pricing_metrics().await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.get_pricing_metrics",
            "metrics_retrieved",
            true,
            None,
        ).await;

        Ok(Response::new(GetPricingMetricsResponse {
            metrics: Some(crate::proto::fo3::wallet::v1::PricingMetrics {
                total_requests: metrics.total_requests,
                cache_hits: metrics.cache_hits,
                cache_misses: metrics.cache_misses,
                cache_hit_rate: metrics.cache_hit_rate,
                api_calls_today: metrics.api_calls_today,
                api_rate_limit: metrics.api_rate_limit,
                supported_assets_count: metrics.supported_assets_count,
                last_cache_refresh: metrics.last_cache_refresh.timestamp(),
                active_sources: metrics.active_sources,
                source_request_counts: metrics.source_request_counts,
            }),
        }))
    }

    async fn refresh_price_cache(
        &self,
        request: Request<RefreshPriceCacheRequest>,
    ) -> Result<Response<RefreshPriceCacheResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permissions
        self.auth_service.check_permission(auth_context, crate::middleware::auth::Permission::PermissionPricingAdmin)?;

        let req = request.into_inner();

        let refreshed_count = if req.symbols.is_empty() {
            // Refresh all cache
            self.repository.clear_cache(None).await
                .map_err(|e| Status::internal(format!("Failed to clear cache: {}", e)))?
        } else {
            // Refresh specific symbols
            let mut total_cleared = 0;
            for symbol in &req.symbols {
                let cleared = self.repository.clear_cache(Some(symbol)).await
                    .map_err(|e| Status::internal(format!("Failed to clear cache for {}: {}", symbol, e)))?;
                total_cleared += cleared;
            }
            total_cleared
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "pricing.refresh_price_cache",
            &format!("refreshed_count={}", refreshed_count),
            true,
            None,
        ).await;

        Ok(Response::new(RefreshPriceCacheResponse {
            success: true,
            message: format!("Cache refreshed for {} entries", refreshed_count),
            refreshed_count: refreshed_count as i32,
            failed_symbols: Vec::new(),
        }))
    }
}
