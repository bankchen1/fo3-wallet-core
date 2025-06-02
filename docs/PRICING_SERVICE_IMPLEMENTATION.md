# Pricing Service Implementation Guide

## Overview

The FO3 Wallet Core Pricing Service provides comprehensive real-time cryptocurrency and fiat currency pricing with enterprise-grade caching, rate limiting, and external API integration. The implementation follows the established FO3 Wallet Core patterns with JWT authentication, RBAC permissions, and comprehensive audit logging.

## 🏗️ Architecture

### Core Components

1. **Pricing Service** (`fo3-wallet-api/src/services/pricing.rs`)
   - gRPC service implementation for all pricing operations
   - Real-time price fetching with caching
   - Batch pricing operations
   - Historical data retrieval
   - Admin price feed management

2. **Data Models** (`fo3-wallet-api/src/models/pricing.rs`)
   - Price and asset entities
   - External API response structures
   - Repository trait for data access
   - In-memory repository implementation

3. **Pricing Guard** (`fo3-wallet-api/src/middleware/pricing_guard.rs`)
   - Rate limiting (1000 requests/minute for users, 5000 for admins)
   - Input validation and sanitization
   - Batch size limits (max 100 symbols per request)
   - Time range validation for historical data

4. **External Providers**
   - CoinGecko API integration with fallback to mock data
   - Configurable API key support
   - Automatic retry and error handling

## 🔧 Configuration

### Environment Variables

```bash
# CoinGecko API configuration
COINGECKO_API_KEY=your_api_key_here

# Cache configuration (optional)
PRICING_CACHE_TTL_SECONDS=30
PRICING_MAX_BATCH_SIZE=100
PRICING_USER_RATE_LIMIT=1000
PRICING_ADMIN_RATE_LIMIT=5000
```

### Supported Assets

The service comes pre-configured with support for:

**Cryptocurrencies:**
- BTC (Bitcoin)
- ETH (Ethereum)
- USDT (Tether)
- USDC (USD Coin)
- BNB (Binance Coin)
- SOL (Solana)
- ADA (Cardano)
- AVAX (Avalanche)
- DOT (Polkadot)
- MATIC (Polygon)

**Fiat Currencies:**
- USD (US Dollar)
- EUR (Euro)
- GBP (British Pound)
- JPY (Japanese Yen)
- CNY (Chinese Yuan)
- CAD (Canadian Dollar)
- AUD (Australian Dollar)

## 🚀 API Reference

### Core Pricing Operations

#### Get Single Price
```protobuf
rpc GetPrice(GetPriceRequest) returns (GetPriceResponse);
```

**Request:**
```json
{
  "symbol": "BTC",
  "quote_currency": "USD",
  "chain": "",
  "contract_address": ""
}
```

**Response:**
```json
{
  "price": {
    "symbol": "BTC",
    "price_usd": "45000.00",
    "price_btc": "1.00",
    "market_cap": "850000000000.00",
    "volume_24h": "25000000000.00",
    "change_24h": "2.5",
    "change_7d": "5.0",
    "source": 1,
    "timestamp": 1640995200,
    "last_updated": 1640995200
  }
}
```

#### Get Batch Prices
```protobuf
rpc GetPriceBatch(GetPriceBatchRequest) returns (GetPriceBatchResponse);
```

**Request:**
```json
{
  "symbols": ["BTC", "ETH", "USDT"],
  "quote_currency": "USD",
  "include_metadata": true
}
```

**Response:**
```json
{
  "prices": [...],
  "total_count": 3,
  "successful_count": 3,
  "failed_symbols": []
}
```

#### Get Fiat Exchange Rate
```protobuf
rpc GetFiatRate(GetFiatRateRequest) returns (GetFiatRateResponse);
```

#### List Supported Symbols
```protobuf
rpc ListSupportedSymbols(ListSupportedSymbolsRequest) returns (ListSupportedSymbolsResponse);
```

#### Get Historical Data
```protobuf
rpc GetPriceHistory(GetPriceHistoryRequest) returns (GetPriceHistoryResponse);
```

### Admin Operations

#### Update Price Feed (Admin Only)
```protobuf
rpc UpdatePriceFeed(UpdatePriceFeedRequest) returns (UpdatePriceFeedResponse);
```

#### Get Pricing Metrics (Admin Only)
```protobuf
rpc GetPricingMetrics(GetPricingMetricsRequest) returns (GetPricingMetricsResponse);
```

#### Refresh Price Cache (Admin Only)
```protobuf
rpc RefreshPriceCache(RefreshPriceCacheRequest) returns (RefreshPriceCacheResponse);
```

## 🔒 Security & Authentication

### Required Permissions

- **PERMISSION_PRICING_READ**: Read price data and supported symbols
- **PERMISSION_PRICING_ADMIN**: Manage price feeds and access metrics

### Rate Limiting

- **Regular Users**: 1000 requests per minute
- **Admin Users**: 5000 requests per minute
- **Batch Operations**: Maximum 100 symbols per request
- **Historical Data**: Maximum 1 year time range

### Input Validation

- Symbol format validation (alphanumeric, dash, underscore only)
- Quote currency validation (supported currencies only)
- Time range validation for historical data
- Pagination parameter validation

## ⚡ Performance Features

### Caching Strategy

- **Cache TTL**: 30 seconds for real-time prices
- **Cache Hit Rate Target**: >90%
- **Cache Keys**: `{SYMBOL}_{QUOTE_CURRENCY}`
- **Cache Invalidation**: Manual refresh via admin API

### Response Times

- **Single Price Query**: <50ms
- **Batch Pricing (100 assets)**: <100ms
- **Cache Refresh**: <200ms
- **Historical Data**: <500ms

### External API Integration

- **Primary Source**: CoinGecko API
- **Fallback**: Mock data provider
- **Rate Limiting**: Respects external API limits
- **Error Handling**: Graceful degradation

## 📊 Monitoring & Observability

### Metrics Tracked

- Total requests processed
- Cache hit/miss ratios
- External API call counts
- Response times by operation
- Error rates by type
- Active data sources

### Audit Logging

All pricing operations are logged with:
- User ID and authentication context
- Operation type and parameters
- Success/failure status
- Response metadata
- Timestamp and duration

### Health Checks

- External API connectivity
- Cache performance
- Repository availability
- Rate limit status

## 🧪 Testing

### Test Coverage

The implementation includes comprehensive tests:

- **Unit Tests**: Core logic and validation
- **Integration Tests**: End-to-end service operations
- **Performance Tests**: Cache and rate limiting
- **Security Tests**: Authentication and authorization

### Running Tests

```bash
# Run all pricing tests
cargo test pricing

# Run integration tests
cargo test --test pricing_integration_tests

# Run with coverage
cargo tarpaulin --include-tests --out html
```

## 🔄 Integration with Existing Services

### KYC Integration

- Pricing data available to all authenticated users
- No KYC verification required for basic price queries
- Admin operations require appropriate permissions

### Fiat Gateway Integration

- Real-time exchange rates for fiat conversions
- Currency validation for withdrawal/deposit limits
- Price data for transaction fee calculations

### WebSocket Notifications

- Real-time price updates via WebSocket
- Configurable price change thresholds
- Subscription management per user

## 🚀 Deployment

### Docker Configuration

The pricing service is included in the main FO3 Wallet Core container:

```dockerfile
# Environment variables for pricing service
ENV COINGECKO_API_KEY=""
ENV PRICING_CACHE_TTL_SECONDS=30
ENV PRICING_USER_RATE_LIMIT=1000
```

### Production Considerations

1. **API Key Management**: Store CoinGecko API key securely
2. **Cache Strategy**: Consider Redis for distributed caching
3. **Rate Limiting**: Monitor and adjust based on usage patterns
4. **Monitoring**: Set up alerts for API failures and high latency
5. **Backup Providers**: Configure multiple price data sources

## 📈 Future Enhancements

### Planned Features

1. **Multiple Price Sources**: Binance, CoinMarketCap integration
2. **Advanced Caching**: Redis cluster support
3. **Price Alerts**: User-configurable price notifications
4. **Historical Analytics**: Advanced charting and analysis
5. **DeFi Integration**: DEX price aggregation
6. **Custom Assets**: User-defined token support

### Scalability Improvements

1. **Database Integration**: PostgreSQL for persistent storage
2. **Microservice Architecture**: Separate pricing service
3. **Load Balancing**: Multiple pricing service instances
4. **CDN Integration**: Global price data distribution

This implementation provides a solid foundation for cryptocurrency and fiat pricing within the FO3 Wallet Core ecosystem, with enterprise-grade security, performance, and monitoring capabilities.
