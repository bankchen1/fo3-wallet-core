//! Pricing data models and entities

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Asset types supported by the pricing service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Cryptocurrency,
    Fiat,
    Token,
    Stablecoin,
}

/// Price data sources
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceSource {
    CoinGecko,
    CoinMarketCap,
    Binance,
    Mock,
    Cache,
}

/// Time intervals for historical data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInterval {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    OneMonth,
}

/// Asset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub chain: Option<String>,
    pub contract_address: Option<String>,
    pub decimals: u8,
    pub icon_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub symbol: String,
    pub price_usd: Decimal,
    pub price_btc: Option<Decimal>,
    pub market_cap: Option<Decimal>,
    pub volume_24h: Option<Decimal>,
    pub change_24h: Option<Decimal>,
    pub change_7d: Option<Decimal>,
    pub source: PriceSource,
    pub timestamp: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Historical price point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub volume: Option<Decimal>,
}

/// Fiat exchange rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatRate {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub source: PriceSource,
    pub timestamp: DateTime<Utc>,
}

/// Pricing metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingMetrics {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub api_calls_today: u64,
    pub api_rate_limit: u64,
    pub supported_assets_count: u32,
    pub last_cache_refresh: DateTime<Utc>,
    pub active_sources: Vec<String>,
    pub source_request_counts: HashMap<String, u64>,
}

/// Price cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceCacheEntry {
    pub price: Price,
    pub expires_at: DateTime<Utc>,
    pub cache_key: String,
}

/// External API response structures for CoinGecko
#[derive(Debug, Deserialize)]
pub struct CoinGeckoPrice {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub current_price: Option<f64>,
    pub market_cap: Option<f64>,
    pub total_volume: Option<f64>,
    pub price_change_percentage_24h: Option<f64>,
    pub price_change_percentage_7d_in_currency: Option<f64>,
    pub last_updated: String,
}

/// External API response for CoinGecko simple price
#[derive(Debug, Deserialize)]
pub struct CoinGeckoSimplePrice {
    #[serde(flatten)]
    pub prices: HashMap<String, CoinGeckoPriceData>,
}

#[derive(Debug, Deserialize)]
pub struct CoinGeckoPriceData {
    pub usd: Option<f64>,
    pub btc: Option<f64>,
    pub usd_market_cap: Option<f64>,
    pub usd_24h_vol: Option<f64>,
    pub usd_24h_change: Option<f64>,
    pub last_updated_at: Option<i64>,
}

/// Historical price data from CoinGecko
#[derive(Debug, Deserialize)]
pub struct CoinGeckoHistoricalData {
    pub prices: Vec<[f64; 2]>, // [timestamp, price]
    pub market_caps: Option<Vec<[f64; 2]>>,
    pub total_volumes: Option<Vec<[f64; 2]>>,
}

/// Pricing repository trait for data access
#[async_trait::async_trait]
pub trait PricingRepository: Send + Sync {
    /// Get cached price for a symbol
    async fn get_cached_price(&self, symbol: &str, quote_currency: &str) -> Option<Price>;
    
    /// Cache price data
    async fn cache_price(&self, symbol: &str, quote_currency: &str, price: &Price, ttl_seconds: u64) -> Result<(), String>;
    
    /// Get supported assets
    async fn get_supported_assets(&self, asset_type: Option<AssetType>, chain: Option<&str>) -> Vec<Asset>;
    
    /// Get asset by symbol
    async fn get_asset(&self, symbol: &str, chain: Option<&str>) -> Option<Asset>;
    
    /// Store historical price data
    async fn store_price_history(&self, symbol: &str, points: &[PricePoint]) -> Result<(), String>;
    
    /// Get historical price data
    async fn get_price_history(
        &self,
        symbol: &str,
        interval: TimeInterval,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<u32>,
    ) -> Vec<PricePoint>;
    
    /// Get fiat exchange rate
    async fn get_fiat_rate(&self, from: &str, to: &str) -> Option<FiatRate>;
    
    /// Cache fiat exchange rate
    async fn cache_fiat_rate(&self, rate: &FiatRate, ttl_seconds: u64) -> Result<(), String>;
    
    /// Get pricing metrics
    async fn get_pricing_metrics(&self) -> PricingMetrics;
    
    /// Update pricing metrics
    async fn update_pricing_metrics(&self, metrics: &PricingMetrics) -> Result<(), String>;
    
    /// Increment request counter
    async fn increment_request_counter(&self, source: &str) -> Result<(), String>;
    
    /// Clear cache for symbol
    async fn clear_cache(&self, symbol: Option<&str>) -> Result<u32, String>;
}

/// In-memory pricing repository implementation
pub struct InMemoryPricingRepository {
    price_cache: std::sync::RwLock<HashMap<String, PriceCacheEntry>>,
    assets: std::sync::RwLock<HashMap<String, Asset>>,
    price_history: std::sync::RwLock<HashMap<String, Vec<PricePoint>>>,
    fiat_rates: std::sync::RwLock<HashMap<String, FiatRate>>,
    metrics: std::sync::RwLock<PricingMetrics>,
}

impl InMemoryPricingRepository {
    pub fn new() -> Self {
        let mut assets = HashMap::new();
        
        // Initialize with common cryptocurrencies
        let common_assets = vec![
            ("BTC", "Bitcoin", AssetType::Cryptocurrency),
            ("ETH", "Ethereum", AssetType::Cryptocurrency),
            ("USDT", "Tether", AssetType::Stablecoin),
            ("USDC", "USD Coin", AssetType::Stablecoin),
            ("BNB", "Binance Coin", AssetType::Cryptocurrency),
            ("SOL", "Solana", AssetType::Cryptocurrency),
            ("ADA", "Cardano", AssetType::Cryptocurrency),
            ("AVAX", "Avalanche", AssetType::Cryptocurrency),
            ("DOT", "Polkadot", AssetType::Cryptocurrency),
            ("MATIC", "Polygon", AssetType::Cryptocurrency),
        ];
        
        for (symbol, name, asset_type) in common_assets {
            let asset = Asset {
                symbol: symbol.to_string(),
                name: name.to_string(),
                asset_type,
                chain: None,
                contract_address: None,
                decimals: 18,
                icon_url: None,
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            assets.insert(symbol.to_string(), asset);
        }
        
        // Add fiat currencies
        let fiat_currencies = vec![
            ("USD", "US Dollar"),
            ("EUR", "Euro"),
            ("GBP", "British Pound"),
            ("JPY", "Japanese Yen"),
            ("CNY", "Chinese Yuan"),
            ("CAD", "Canadian Dollar"),
            ("AUD", "Australian Dollar"),
        ];
        
        for (symbol, name) in fiat_currencies {
            let asset = Asset {
                symbol: symbol.to_string(),
                name: name.to_string(),
                asset_type: AssetType::Fiat,
                chain: None,
                contract_address: None,
                decimals: 2,
                icon_url: None,
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            assets.insert(symbol.to_string(), asset);
        }
        
        let metrics = PricingMetrics {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
            api_calls_today: 0,
            api_rate_limit: 1000,
            supported_assets_count: assets.len() as u32,
            last_cache_refresh: Utc::now(),
            active_sources: vec!["coingecko".to_string(), "mock".to_string()],
            source_request_counts: HashMap::new(),
        };
        
        Self {
            price_cache: std::sync::RwLock::new(HashMap::new()),
            assets: std::sync::RwLock::new(assets),
            price_history: std::sync::RwLock::new(HashMap::new()),
            fiat_rates: std::sync::RwLock::new(HashMap::new()),
            metrics: std::sync::RwLock::new(metrics),
        }
    }
    
    fn cache_key(symbol: &str, quote_currency: &str) -> String {
        format!("{}_{}", symbol.to_uppercase(), quote_currency.to_uppercase())
    }
    
    fn fiat_rate_key(from: &str, to: &str) -> String {
        format!("{}_{}", from.to_uppercase(), to.to_uppercase())
    }
}

impl Default for InMemoryPricingRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PricingRepository for InMemoryPricingRepository {
    async fn get_cached_price(&self, symbol: &str, quote_currency: &str) -> Option<Price> {
        let cache = self.price_cache.read().unwrap();
        let key = Self::cache_key(symbol, quote_currency);

        if let Some(entry) = cache.get(&key) {
            if entry.expires_at > Utc::now() {
                return Some(entry.price.clone());
            }
        }
        None
    }

    async fn cache_price(&self, symbol: &str, quote_currency: &str, price: &Price, ttl_seconds: u64) -> Result<(), String> {
        let mut cache = self.price_cache.write().unwrap();
        let key = Self::cache_key(symbol, quote_currency);
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);

        let entry = PriceCacheEntry {
            price: price.clone(),
            expires_at,
            cache_key: key.clone(),
        };

        cache.insert(key, entry);
        Ok(())
    }

    async fn get_supported_assets(&self, asset_type: Option<AssetType>, chain: Option<&str>) -> Vec<Asset> {
        let assets = self.assets.read().unwrap();
        assets.values()
            .filter(|asset| {
                if let Some(ref filter_type) = asset_type {
                    if &asset.asset_type != filter_type {
                        return false;
                    }
                }
                if let Some(filter_chain) = chain {
                    if let Some(ref asset_chain) = asset.chain {
                        if asset_chain != filter_chain {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                asset.is_active
            })
            .cloned()
            .collect()
    }

    async fn get_asset(&self, symbol: &str, chain: Option<&str>) -> Option<Asset> {
        let assets = self.assets.read().unwrap();
        let key = if let Some(chain) = chain {
            format!("{}_{}", symbol.to_uppercase(), chain)
        } else {
            symbol.to_uppercase()
        };

        assets.get(&key).cloned()
            .or_else(|| assets.get(&symbol.to_uppercase()).cloned())
    }

    async fn store_price_history(&self, symbol: &str, points: &[PricePoint]) -> Result<(), String> {
        let mut history = self.price_history.write().unwrap();
        let key = symbol.to_uppercase();

        let mut existing_points = history.get(&key).cloned().unwrap_or_default();
        existing_points.extend_from_slice(points);

        // Sort by timestamp and keep only the latest 1000 points
        existing_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        if existing_points.len() > 1000 {
            existing_points = existing_points.into_iter().rev().take(1000).rev().collect();
        }

        history.insert(key, existing_points);
        Ok(())
    }

    async fn get_price_history(
        &self,
        symbol: &str,
        _interval: TimeInterval,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<u32>,
    ) -> Vec<PricePoint> {
        let history = self.price_history.read().unwrap();
        let key = symbol.to_uppercase();

        if let Some(points) = history.get(&key) {
            let mut filtered: Vec<_> = points.iter()
                .filter(|point| point.timestamp >= start_time && point.timestamp <= end_time)
                .cloned()
                .collect();

            filtered.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            if let Some(limit) = limit {
                filtered.truncate(limit as usize);
            }

            filtered
        } else {
            Vec::new()
        }
    }

    async fn get_fiat_rate(&self, from: &str, to: &str) -> Option<FiatRate> {
        let rates = self.fiat_rates.read().unwrap();
        let key = Self::fiat_rate_key(from, to);
        rates.get(&key).cloned()
    }

    async fn cache_fiat_rate(&self, rate: &FiatRate, _ttl_seconds: u64) -> Result<(), String> {
        let mut rates = self.fiat_rates.write().unwrap();
        let key = Self::fiat_rate_key(&rate.from_currency, &rate.to_currency);
        rates.insert(key, rate.clone());
        Ok(())
    }

    async fn get_pricing_metrics(&self) -> PricingMetrics {
        let metrics = self.metrics.read().unwrap();
        let mut metrics = metrics.clone();

        // Calculate cache hit rate
        if metrics.total_requests > 0 {
            metrics.cache_hit_rate = metrics.cache_hits as f64 / metrics.total_requests as f64;
        }

        metrics
    }

    async fn update_pricing_metrics(&self, new_metrics: &PricingMetrics) -> Result<(), String> {
        let mut metrics = self.metrics.write().unwrap();
        *metrics = new_metrics.clone();
        Ok(())
    }

    async fn increment_request_counter(&self, source: &str) -> Result<(), String> {
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_requests += 1;

        let count = metrics.source_request_counts.get(source).unwrap_or(&0) + 1;
        metrics.source_request_counts.insert(source.to_string(), count);

        Ok(())
    }

    async fn clear_cache(&self, symbol: Option<&str>) -> Result<u32, String> {
        let mut cache = self.price_cache.write().unwrap();

        if let Some(symbol) = symbol {
            let keys_to_remove: Vec<_> = cache.keys()
                .filter(|key| key.starts_with(&symbol.to_uppercase()))
                .cloned()
                .collect();

            let count = keys_to_remove.len() as u32;
            for key in keys_to_remove {
                cache.remove(&key);
            }
            Ok(count)
        } else {
            let count = cache.len() as u32;
            cache.clear();
            Ok(count)
        }
    }
}
