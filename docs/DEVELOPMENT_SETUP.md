# FO3 Wallet Core - Development Setup Guide

This guide provides comprehensive instructions for setting up the FO3 Wallet Core development environment.

## Prerequisites

### Required Software
- **Rust 1.75+** - Install via [rustup](https://rustup.rs/)
- **Docker 20.10+** - [Install Docker](https://docs.docker.com/get-docker/)
- **Docker Compose 2.0+** - Usually included with Docker Desktop
- **PostgreSQL 15+** - For local development (optional if using Docker)
- **Redis 7+** - For caching (optional if using Docker)
- **Protocol Buffers Compiler** - `protoc` for gRPC code generation

### System Requirements
- **RAM:** Minimum 8GB, recommended 16GB
- **Storage:** At least 20GB free space
- **OS:** Linux, macOS, or Windows with WSL2

## Installation Steps

### 1. Install Rust and Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required tools
cargo install cargo-watch
cargo install grpcurl

# Install protobuf compiler (Ubuntu/Debian)
sudo apt-get install protobuf-compiler

# Install protobuf compiler (macOS)
brew install protobuf

# Install protobuf compiler (Windows)
# Download from https://github.com/protocolbuffers/protobuf/releases
```

### 2. Clone and Setup Repository

```bash
# Clone the repository
git clone https://github.com/bankchen1/fo3-wallet-core.git
cd fo3-wallet-core

# Copy environment configuration
cp .env.example .env

# Edit .env file with your configuration
nano .env
```

### 3. Database Setup

#### Option A: Using Docker (Recommended)
```bash
# Start PostgreSQL and Redis with Docker Compose
docker-compose up -d postgres redis

# Wait for services to be ready
sleep 10

# Run database migrations
cargo run --bin fo3-wallet-api -- migrate
```

#### Option B: Local Installation
```bash
# Install PostgreSQL (Ubuntu/Debian)
sudo apt-get install postgresql postgresql-contrib

# Install PostgreSQL (macOS)
brew install postgresql

# Create database and user
sudo -u postgres psql
CREATE DATABASE fo3_wallet;
CREATE USER fo3_user WITH PASSWORD 'fo3_secure_password_change_me';
GRANT ALL PRIVILEGES ON DATABASE fo3_wallet TO fo3_user;
\q

# Install Redis (Ubuntu/Debian)
sudo apt-get install redis-server

# Install Redis (macOS)
brew install redis

# Start Redis
redis-server
```

### 4. Build and Run

```bash
# Build the project
cargo build --release --features solana

# Run the gRPC API server
cargo run --bin fo3-wallet-api

# Or run with file watching for development
cargo watch -x "run --bin fo3-wallet-api"
```

## Service Endpoints and Ports

### Core Services
| Service | Port | Protocol | Description |
|---------|------|----------|-------------|
| **gRPC API** | 50051 | gRPC | Main API server |
| **WebSocket** | 8080 | WebSocket | Real-time notifications |
| **Metrics** | 9090 | HTTP | Prometheus metrics |
| **Health Check** | 8080/health | HTTP | Service health status |

### Infrastructure Services
| Service | Port | Protocol | Description |
|---------|------|----------|-------------|
| **PostgreSQL** | 5432 | TCP | Primary database |
| **Redis** | 6379 | TCP | Cache and sessions |
| **Prometheus** | 9091 | HTTP | Metrics collection |
| **Grafana** | 3000 | HTTP | Monitoring dashboard |
| **Jaeger** | 16686 | HTTP | Distributed tracing |

## Testing the Setup

### 1. Health Check
```bash
# Check if the gRPC server is running
grpcurl -plaintext localhost:50051 fo3.wallet.v1.HealthService/Check

# Expected response:
{
  "status": "SERVING"
}
```

### 2. Authentication Test
```bash
# Test authentication service
grpcurl -plaintext \
  -d '{"email": "test@example.com", "password": "password123"}' \
  localhost:50051 fo3.wallet.v1.AuthService/Login
```

### 3. Service Discovery
```bash
# List all available services
grpcurl -plaintext localhost:50051 list

# List methods for a specific service
grpcurl -plaintext localhost:50051 list fo3.wallet.v1.WalletService
```

## Development Workflow

### 1. Code Changes
```bash
# Watch for changes and rebuild automatically
cargo watch -x "run --bin fo3-wallet-api"

# Run tests on file changes
cargo watch -x test

# Run specific test suite
cargo test --test integration_tests
```

### 2. Database Migrations
```bash
# Create new migration
cargo run --bin fo3-wallet-api -- create-migration add_new_table

# Run pending migrations
cargo run --bin fo3-wallet-api -- migrate

# Rollback last migration
cargo run --bin fo3-wallet-api -- rollback
```

### 3. Protocol Buffer Changes
```bash
# Regenerate gRPC code after proto changes
cargo build

# The build.rs script automatically handles proto compilation
```

## Testing Procedures

### 1. Unit Tests
```bash
# Run all unit tests
cargo test

# Run tests for specific service
cargo test services::wallet

# Run tests with output
cargo test -- --nocapture
```

### 2. Integration Tests
```bash
# Run integration tests
cargo test --test integration_tests

# Run E2E tests
cargo test --test e2e_tests

# Run performance tests
cargo test --test performance_tests
```

### 3. Load Testing
```bash
# Start the server
cargo run --bin fo3-wallet-api

# Run load tests (in another terminal)
cargo test --test load_tests --release
```

## Troubleshooting

### Common Issues

#### 1. Port Already in Use
```bash
# Find process using port 50051
lsof -i :50051

# Kill the process
kill -9 <PID>
```

#### 2. Database Connection Issues
```bash
# Check PostgreSQL status
sudo systemctl status postgresql

# Check database connectivity
psql -h localhost -U fo3_user -d fo3_wallet -c "SELECT 1;"
```

#### 3. Redis Connection Issues
```bash
# Check Redis status
redis-cli ping

# Should return: PONG
```

#### 4. Build Errors
```bash
# Clean build cache
cargo clean

# Update dependencies
cargo update

# Rebuild from scratch
cargo build --release
```

### Environment Variables

#### Required Variables
```bash
# Authentication
JWT_SECRET=your_jwt_secret_key_change_me
ENCRYPTION_KEY=your_32_byte_encryption_key_change_me

# Database
DATABASE_URL=postgresql://fo3_user:password@localhost:5432/fo3_wallet

# Redis
REDIS_URL=redis://localhost:6379

# Logging
RUST_LOG=info
```

#### Optional Variables
```bash
# TLS Configuration
ENABLE_TLS=false
TLS_CERT_PATH=./certs/server.crt
TLS_KEY_PATH=./certs/server.key

# Blockchain RPC URLs
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_API_KEY
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Monitoring
JAEGER_ENDPOINT=http://localhost:14268/api/traces
PROMETHEUS_ENABLED=true
```

## IDE Configuration

### Visual Studio Code
```json
// .vscode/settings.json
{
    "rust-analyzer.cargo.features": ["solana"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.loadOutDirsFromCheck": true
}
```

### IntelliJ IDEA / CLion
- Install Rust plugin
- Configure Cargo features: `solana`
- Enable Clippy for code analysis

## Performance Optimization

### Development Mode
```bash
# Fast compilation for development
export CARGO_PROFILE_DEV_DEBUG=1
cargo build
```

### Release Mode
```bash
# Optimized build for testing
cargo build --release --features solana
```

### Memory Usage
```bash
# Monitor memory usage during development
cargo run --bin fo3-wallet-api &
top -p $!
```

## Next Steps

After successful setup:

1. **Explore the API** - Use grpcurl to test different endpoints
2. **Run the test suite** - Ensure all tests pass
3. **Review the documentation** - Check `docs/` directory for service guides
4. **Start development** - Begin implementing new features or fixes

## Support

For additional help:
- Check the [API Documentation](./API_REFERENCE.md)
- Review [Service Implementation Guides](./services/)
- Open an issue on GitHub for bugs or questions
