# FO3 Wallet Core - Local Validation Guide

This guide provides comprehensive instructions for running the local validation system as outlined in `PROJECT_STATUS_FINAL.md`. The validation process ensures production readiness through systematic testing of all 15 core services and 8 ML components.

## 🎯 Overview

The local validation system implements a 4-phase approach:

1. **Phase 1: Foundation** - Database setup, configuration, and observability
2. **Phase 2: Service Validation** - CLI testing and end-to-end flows
3. **Phase 3: Integration & Performance** - Load testing and monitoring
4. **Phase 4: Production Preparation** - Deployment readiness validation

## 📋 Prerequisites

### System Requirements
- **OS**: macOS, Linux, or Windows with WSL2
- **RAM**: Minimum 8GB, recommended 16GB
- **Storage**: 10GB free space
- **Docker**: Version 20.10+ with Docker Compose
- **Rust**: Version 1.70+ with Cargo

### Required Tools
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Docker and Docker Compose
# Follow instructions at: https://docs.docker.com/get-docker/

# Install additional tools
brew install grpcurl  # For gRPC testing
brew install jq       # For JSON processing
brew install bc       # For calculations in scripts
```

## 🚀 Quick Start

### 1. Clone and Setup
```bash
git clone https://github.com/bankchen1/fo3-wallet-core.git
cd fo3-wallet-core
```

### 2. Run Complete Validation
```bash
# Execute the comprehensive 4-phase validation
./scripts/run_local_validation.sh
```

### 3. Monitor Progress
The validation process provides real-time feedback and generates comprehensive reports:
- **Live logs**: `test_data/local_validation_YYYYMMDD_HHMMSS.log`
- **Final report**: `test_data/validation_report_YYYYMMDD_HHMMSS.json`

## 📊 Observability Dashboard

During validation, access these monitoring interfaces:

- **Jaeger Tracing**: http://localhost:16686
- **Prometheus Metrics**: http://localhost:9090
- **Grafana Dashboards**: http://localhost:3000 (admin/fo3_dev_grafana_password)
- **Application Metrics**: http://localhost:9091/metrics

## 🔧 Manual Phase Execution

If you need to run phases individually:

### Phase 1: Foundation
```bash
# Build CLI
cd fo3-wallet-api
cargo build --bin fo3_cli

# Initialize database
./target/debug/fo3_cli database init --config ../config/development.toml

# Seed test data
./target/debug/fo3_cli database seed --config ../config/development.toml

# Start observability stack
cd ..
docker-compose -f docker-compose.dev.yml up -d jaeger prometheus grafana
```

### Phase 2: Service Validation
```bash
# Run individual flow tests
./scripts/test_wallet_flows.sh
./scripts/test_trading_flows.sh
./scripts/test_defi_flows.sh
./scripts/test_dapp_flows.sh

# Interactive CLI testing
cd fo3-wallet-api
./target/debug/fo3_cli interactive
```

### Phase 3: Integration & Performance
```bash
# Start full application stack
docker-compose -f docker-compose.dev.yml up -d

# Run integration tests
cd fo3-wallet-api
cargo test --test phase_3_integration_tests

# Run performance validation
cargo test --test performance_validation
```

### Phase 4: Production Preparation
```bash
# Generate deployment artifacts
./scripts/generate_production_config.sh

# Validate production readiness
./scripts/validate_production_readiness.sh
```

## 🧪 Individual Service Testing

### CLI Commands
```bash
# Wallet operations
./target/debug/fo3_cli wallet create "Test Wallet"
./target/debug/fo3_cli wallet list
./target/debug/fo3_cli wallet balance <wallet-id>

# KYC operations
./target/debug/fo3_cli kyc submit <user-id>
./target/debug/fo3_cli kyc status <submission-id>

# Card operations
./target/debug/fo3_cli card create <user-id> USD
./target/debug/fo3_cli card transaction <card-id> 100 "Test Merchant"

# Trading operations
./target/debug/fo3_cli trading create-strategy "DCA Strategy" dca
./target/debug/fo3_cli trading execute-trade <strategy-id> BTC/USD 100

# DeFi operations
./target/debug/fo3_cli defi list-products
./target/debug/fo3_cli defi stake <product-id> 1000

# DApp operations
./target/debug/fo3_cli dapp connect https://uniswap.org
./target/debug/fo3_cli dapp sign <dapp-id> <transaction-data>
```

## 📈 Performance Benchmarks

The validation system enforces these performance requirements:

| Operation Type | Threshold | Validation Method |
|---------------|-----------|-------------------|
| Standard Operations | <200ms | Automated testing |
| ML Inference | <500ms | Load testing |
| Notification Delivery | <100ms | Real-time testing |
| Concurrent Users | 50+ | Stress testing |
| Database Operations | <50ms | Performance testing |
| Success Rate | >95% | Error rate monitoring |

## 🔍 Troubleshooting

### Common Issues

**Database Connection Failed**
```bash
# Check PostgreSQL status
docker-compose -f docker-compose.dev.yml ps postgres

# Reset database
./target/debug/fo3_cli database reset
./target/debug/fo3_cli database init
```

**Observability Services Not Starting**
```bash
# Check Docker resources
docker system df
docker system prune  # If needed

# Restart observability stack
docker-compose -f docker-compose.dev.yml restart jaeger prometheus grafana
```

**Performance Tests Failing**
```bash
# Check system resources
top
df -h

# Reduce concurrent load
export CONCURRENT_USERS=25  # Default is 50
```

**CLI Build Failures**
```bash
# Clean and rebuild
cargo clean
cargo build --bin fo3_cli

# Check dependencies
cargo check
```

### Log Analysis
```bash
# View real-time logs
tail -f test_data/local_validation_*.log

# Search for errors
grep -i error test_data/local_validation_*.log

# Analyze performance metrics
grep -i "duration\|time\|ms" test_data/local_validation_*.log
```

## 📊 Validation Reports

### Report Structure
```json
{
  "validation_suite": "FO3 Wallet Core Local Validation",
  "timestamp": "2024-01-15T10:30:00Z",
  "summary": {
    "total_phases": 4,
    "phases_passed": 4,
    "phase_success_rate": 100.0,
    "total_tests": 150,
    "tests_passed": 147,
    "test_success_rate": 98.0
  },
  "phases": {
    "Phase 1": { "status": "PASSED", "duration_seconds": 120 },
    "Phase 2": { "status": "PASSED", "duration_seconds": 300 },
    "Phase 3": { "status": "PASSED", "duration_seconds": 180 },
    "Phase 4": { "status": "PASSED", "duration_seconds": 60 }
  },
  "validation_criteria": {
    "database_initialization": "PASSED",
    "end_to_end_testing": "PASSED",
    "performance_validation": "PASSED",
    "production_readiness": "PASSED"
  }
}
```

### Success Criteria
- ✅ All 4 phases complete successfully
- ✅ >95% test success rate
- ✅ Performance thresholds met
- ✅ Observability fully functional
- ✅ Security validation passed

## 🔄 Continuous Validation

### Automated Runs
```bash
# Schedule daily validation
crontab -e
# Add: 0 2 * * * /path/to/fo3-wallet-core/scripts/run_local_validation.sh

# CI/CD Integration
# Add to your CI pipeline for automated validation on commits
```

### Monitoring Integration
```bash
# Export metrics to external monitoring
curl http://localhost:9091/metrics | curl -X POST http://your-monitoring-system/metrics

# Set up alerts based on validation results
# Configure Grafana alerts for key metrics
```

## 📚 Additional Resources

- **Architecture Documentation**: `docs/architecture/`
- **API Documentation**: `docs/api/`
- **Deployment Guide**: `docs/deployment/`
- **Security Guide**: `docs/security/`

## 🆘 Support

For issues or questions:
1. Check the troubleshooting section above
2. Review logs in `test_data/` directory
3. Open an issue on GitHub with validation report attached
4. Contact the development team with specific error details

---

**Note**: This local validation system is designed for development and testing. For production deployment, follow the production deployment guide after successful local validation.
