# FO3 Wallet Core - Deployment Guide

This guide covers the deployment of the FO3 Wallet Core gRPC API using Docker and Docker Compose.

## Overview

The FO3 Wallet Core has been containerized and converted from REST API to gRPC for better performance, type safety, and inter-service communication. The deployment includes:

- **fo3-wallet-api**: Main gRPC service for wallet operations
- **PostgreSQL**: Database for persistent storage
- **Redis**: Caching layer
- **Prometheus**: Metrics collection
- **Grafana**: Monitoring dashboard
- **Nginx**: Reverse proxy (optional)

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- At least 4GB RAM
- 10GB free disk space

### Install grpc_health_probe (recommended)

```bash
# Linux
wget -qO- https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/v0.4.19/grpc_health_probe-linux-amd64.tar.gz | tar xvz
sudo mv grpc_health_probe /usr/local/bin/

# macOS
brew install grpc-health-probe

# Or download manually
wget https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/v0.4.19/grpc_health_probe-darwin-amd64
chmod +x grpc_health_probe-darwin-amd64
sudo mv grpc_health_probe-darwin-amd64 /usr/local/bin/grpc_health_probe
```

## Quick Start

1. **Clone and setup environment**:
   ```bash
   git clone https://github.com/fo3/fo3-wallet-core.git
   cd fo3-wallet-core
   cp .env.example .env
   ```

2. **Configure environment variables**:
   Edit `.env` file with your configuration:
   ```bash
   # Required: Set your blockchain API keys
   ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_INFURA_API_KEY
   ETHEREUM_API_KEY=YOUR_INFURA_API_KEY
   
   # Security: Change default passwords
   POSTGRES_PASSWORD=your_secure_password
   REDIS_PASSWORD=your_redis_password
   GRAFANA_PASSWORD=your_grafana_password
   ```

3. **Deploy**:
   ```bash
   ./deploy.sh deploy
   ```

## Manual Deployment Steps

### 1. Build and Start Services

```bash
# Build Docker images
./deploy.sh build

# Start all services
./deploy.sh start

# Check status
./deploy.sh status
```

### 2. Verify Deployment

```bash
# Health check
./deploy.sh health

# Check logs
./deploy.sh logs fo3-wallet-api
```

### 3. Test gRPC API

```bash
# Using grpc_health_probe
grpc_health_probe -addr=localhost:50051

# Using the example client
cd examples
cargo run --bin grpc_client
```

## Service Configuration

### gRPC API Service

- **Port**: 50051
- **Protocol**: gRPC/HTTP2
- **Health Check**: Built-in gRPC health service

### Database (PostgreSQL)

- **Port**: 5432 (internal)
- **Database**: fo3_wallet
- **User**: fo3_user
- **Schema**: Auto-initialized with tables for wallets, transactions, etc.

### Cache (Redis)

- **Port**: 6379 (internal)
- **Authentication**: Password-protected
- **Persistence**: AOF enabled

### Monitoring

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (admin/admin)

## gRPC Services

The API exposes the following gRPC services:

### WalletService
- `CreateWallet` - Create new wallet with mnemonic
- `ImportWallet` - Import wallet from mnemonic
- `GetWallet` - Retrieve wallet by ID
- `ListWallets` - List all wallets
- `DeleteWallet` - Delete wallet
- `DeriveAddress` - Derive blockchain address
- `GetAddresses` - Get wallet addresses

### TransactionService
- `SendTransaction` - Create and broadcast transaction
- `GetTransaction` - Get transaction details
- `SignTransaction` - Sign transaction
- `BroadcastTransaction` - Broadcast signed transaction
- `GetTransactionHistory` - Get transaction history

### DefiService
- `GetSupportedTokens` - Get supported tokens
- `GetTokenBalance` - Get token balance
- `GetSwapQuote` - Get DEX swap quote
- `ExecuteSwap` - Execute token swap
- `GetLendingMarkets` - Get lending markets
- `ExecuteLending` - Execute lending operation
- `GetStakingPools` - Get staking pools
- `ExecuteStaking` - Execute staking operation

### SolanaService (if enabled)
- `GetNftsByOwner` - Get NFTs owned by address
- `GetNftMetadata` - Get NFT metadata
- `TransferNft` - Transfer NFT
- `MintNft` - Mint new NFT
- `GetTokenInfo` - Get SPL token info
- `TransferTokens` - Transfer SPL tokens
- `StakeSol` - Stake SOL
- `GetRaydiumPairs` - Get Raydium trading pairs
- `ExecuteRaydiumSwap` - Execute Raydium swap
- `GetOrcaPairs` - Get Orca trading pairs
- `ExecuteOrcaSwap` - Execute Orca swap

### HealthService
- `Check` - Health check
- `Watch` - Health status streaming

## Environment Variables

### Required Configuration

```bash
# Blockchain RPC URLs
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_API_KEY
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
BITCOIN_RPC_URL=https://blockstream.info/api

# Database
POSTGRES_PASSWORD=secure_password
DATABASE_URL=postgresql://fo3_user:password@postgres:5432/fo3_wallet

# Redis
REDIS_PASSWORD=redis_password
REDIS_URL=redis://:password@redis:6379
```

### Optional Configuration

```bash
# Logging
RUST_LOG=info

# gRPC Server
GRPC_LISTEN_ADDR=0.0.0.0:50051

# Feature Flags
ENABLE_SOLANA=true
ENABLE_BITCOIN=true
ENABLE_ETHEREUM=true

# Security
JWT_SECRET=your_jwt_secret
ENCRYPTION_KEY=your_32_byte_key
```

## Management Commands

```bash
# Deployment
./deploy.sh deploy          # Full deployment
./deploy.sh build           # Build images only
./deploy.sh start           # Start services
./deploy.sh stop            # Stop services
./deploy.sh restart         # Restart services

# Monitoring
./deploy.sh status          # Show service status
./deploy.sh logs [service]  # Show logs
./deploy.sh health          # Health check

# Maintenance
./deploy.sh backup          # Backup database
./deploy.sh restore <file>  # Restore database
./deploy.sh cleanup         # Clean up containers

# Development
./deploy.sh test            # Run tests
```

## Client Integration

### Using gRPC Clients

```rust
use tonic::transport::Channel;
use fo3_wallet_grpc::proto::fo3::wallet::v1::wallet_service_client::WalletServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://localhost:50051")
        .connect()
        .await?;
    
    let mut client = WalletServiceClient::new(channel);
    
    // Create wallet
    let request = tonic::Request::new(CreateWalletRequest {
        name: "My Wallet".to_string(),
    });
    
    let response = client.create_wallet(request).await?;
    println!("Created wallet: {:?}", response.into_inner());
    
    Ok(())
}
```

### Using grpcurl

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Call health check
grpcurl -plaintext localhost:50051 fo3.wallet.v1.HealthService/Check

# Create wallet
grpcurl -plaintext -d '{"name": "Test Wallet"}' \
  localhost:50051 fo3.wallet.v1.WalletService/CreateWallet
```

## Performance Considerations

### Resource Requirements

- **Minimum**: 2 CPU cores, 4GB RAM, 10GB storage
- **Recommended**: 4 CPU cores, 8GB RAM, 50GB storage
- **Production**: 8+ CPU cores, 16GB+ RAM, 100GB+ storage

### Scaling

- **Horizontal**: Deploy multiple API instances behind load balancer
- **Database**: Use PostgreSQL read replicas for read-heavy workloads
- **Cache**: Use Redis Cluster for high availability

### Security

- Use TLS for gRPC in production
- Implement authentication/authorization
- Secure database connections
- Use secrets management for API keys
- Regular security updates

## Troubleshooting

### Common Issues

1. **gRPC connection failed**:
   ```bash
   # Check if service is running
   docker-compose ps fo3-wallet-api
   
   # Check logs
   docker-compose logs fo3-wallet-api
   ```

2. **Database connection failed**:
   ```bash
   # Check database status
   docker-compose exec postgres pg_isready -U fo3_user
   
   # Reset database
   docker-compose down -v
   docker-compose up -d postgres
   ```

3. **Build failures**:
   ```bash
   # Clean build
   docker-compose build --no-cache
   
   # Check Rust version
   docker run --rm rust:1.75 rustc --version
   ```

### Logs and Debugging

```bash
# View all logs
docker-compose logs

# Follow specific service logs
docker-compose logs -f fo3-wallet-api

# Debug mode
RUST_LOG=debug docker-compose up fo3-wallet-api
```

## Production Deployment

For production deployment, consider:

1. **Use external managed databases** (AWS RDS, Google Cloud SQL)
2. **Implement proper secrets management** (HashiCorp Vault, AWS Secrets Manager)
3. **Set up monitoring and alerting** (Prometheus + AlertManager)
4. **Use container orchestration** (Kubernetes, Docker Swarm)
5. **Implement CI/CD pipelines**
6. **Set up backup and disaster recovery**
7. **Use TLS/SSL certificates**
8. **Implement rate limiting and DDoS protection**

## Support

For issues and questions:
- Check the logs: `./deploy.sh logs`
- Run health check: `./deploy.sh health`
- Review configuration in `.env`
- Check Docker and Docker Compose versions
