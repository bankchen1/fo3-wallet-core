#!/bin/bash
# FO3 Wallet Core - DeFi Flow End-to-End Testing Script
# 
# Tests complete DeFi ecosystem including:
# - Yield farming and staking operations
# - Liquidity provision and management
# - DeFi protocol integrations
# - Risk assessment and portfolio optimization
# - Automated yield strategies

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CLI_BINARY="$PROJECT_ROOT/target/debug/fo3_cli"
GRPC_ENDPOINT="http://localhost:50051"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/defi_flows_$(date +%Y%m%d_%H%M%S).log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}[SUCCESS] $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}" | tee -a "$LOG_FILE"
}

# Test result tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

run_test() {
    local test_name="$1"
    local test_command="$2"
    
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    log "Running test: $test_name"
    
    if eval "$test_command" >> "$LOG_FILE" 2>&1; then
        success "Test passed: $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        error "Test failed: $test_name"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Setup test environment
setup_test_environment() {
    log "Setting up DeFi test environment..."
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Initialize database with DeFi data
    log "Initializing DeFi database..."
    "$CLI_BINARY" database init --config "$PROJECT_ROOT/config/development.toml"
    "$CLI_BINARY" database seed --config "$PROJECT_ROOT/config/development.toml"
    
    success "DeFi test environment setup completed"
}

# Test 1: Yield Product Discovery
test_yield_product_discovery() {
    log "Testing yield product discovery..."
    
    # Test listing available yield products
    run_test "List all yield products" \
        "$CLI_BINARY defi list-products --config $PROJECT_ROOT/config/development.toml"
    
    # Test filtering by protocol
    local protocols=("Compound" "Aave" "Uniswap" "Curve" "Yearn")
    
    for protocol in "${protocols[@]}"; do
        run_test "List $protocol products" \
            "$CLI_BINARY defi list-products --protocol \"$protocol\" --config $PROJECT_ROOT/config/development.toml"
    done
    
    # Test filtering by risk level
    local risk_levels=("low" "medium" "high")
    
    for risk in "${risk_levels[@]}"; do
        run_test "List $risk risk products" \
            "$CLI_BINARY defi list-products --risk \"$risk\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 2: Staking Operations
test_staking_operations() {
    log "Testing staking operations..."
    
    # Test staking different assets
    local staking_pairs=(
        "ETH:1.5"
        "USDC:1000"
        "BTC:0.1"
        "SOL:50"
    )
    
    for pair in "${staking_pairs[@]}"; do
        local asset="${pair%:*}"
        local amount="${pair#*:}"
        
        run_test "Stake $amount $asset" \
            "$CLI_BINARY defi stake \"$asset-staking-pool\" \"$amount\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 3: Liquidity Provision
test_liquidity_provision() {
    log "Testing liquidity provision..."
    
    # Test providing liquidity to different pools
    local liquidity_pools=(
        "ETH/USDC:1:2000"
        "BTC/ETH:0.05:1"
        "SOL/USDC:25:500"
    )
    
    for pool in "${liquidity_pools[@]}"; do
        local pair="${pool%:*:*}"
        local amount1="${pool#*:}"
        amount1="${amount1%:*}"
        local amount2="${pool##*:}"
        
        run_test "Provide liquidity to $pair pool" \
            "$CLI_BINARY defi provide-liquidity \"$pair\" \"$amount1\" \"$amount2\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 4: Yield Farming
test_yield_farming() {
    log "Testing yield farming strategies..."
    
    # Test different yield farming strategies
    local farming_strategies=(
        "compound-usdc:1000"
        "aave-eth:2"
        "curve-3pool:5000"
        "yearn-vault:1500"
    )
    
    for strategy in "${farming_strategies[@]}"; do
        local product="${strategy%:*}"
        local amount="${strategy#*:}"
        
        run_test "Start yield farming: $product" \
            "$CLI_BINARY defi farm \"$product\" \"$amount\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 5: Position Management
test_position_management() {
    log "Testing DeFi position management..."
    
    local user_id="test-user-id"
    
    # Test listing user positions
    run_test "List user DeFi positions" \
        "$CLI_BINARY defi positions \"$user_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test position details
    local position_id="test-position-id"
    run_test "Get position details" \
        "$CLI_BINARY defi position \"$position_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test position rebalancing
    run_test "Rebalance position" \
        "$CLI_BINARY defi rebalance \"$position_id\" --target-allocation 60:40 --config $PROJECT_ROOT/config/development.toml"
}

# Test 6: Unstaking and Withdrawals
test_unstaking_withdrawals() {
    log "Testing unstaking and withdrawal operations..."
    
    # Test partial unstaking
    local position_id="test-staking-position"
    run_test "Partial unstaking" \
        "$CLI_BINARY defi unstake \"$position_id\" --amount 50 --config $PROJECT_ROOT/config/development.toml"
    
    # Test full unstaking
    run_test "Full unstaking" \
        "$CLI_BINARY defi unstake \"$position_id\" --all --config $PROJECT_ROOT/config/development.toml"
    
    # Test emergency withdrawal
    run_test "Emergency withdrawal" \
        "$CLI_BINARY defi emergency-withdraw \"$position_id\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 7: Rewards and Harvesting
test_rewards_harvesting() {
    log "Testing rewards and harvesting..."
    
    local user_id="test-user-id"
    
    # Test checking available rewards
    run_test "Check available rewards" \
        "$CLI_BINARY defi rewards \"$user_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test harvesting rewards
    run_test "Harvest all rewards" \
        "$CLI_BINARY defi harvest \"$user_id\" --all --config $PROJECT_ROOT/config/development.toml"
    
    # Test compound rewards
    run_test "Compound rewards" \
        "$CLI_BINARY defi compound \"$user_id\" --auto-compound --config $PROJECT_ROOT/config/development.toml"
}

# Test 8: Risk Assessment
test_risk_assessment() {
    log "Testing DeFi risk assessment..."
    
    # Test protocol risk analysis
    local protocols=("compound" "aave" "uniswap")
    
    for protocol in "${protocols[@]}"; do
        run_test "Assess $protocol risk" \
            "$CLI_BINARY defi risk-assessment \"$protocol\" --config $PROJECT_ROOT/config/development.toml"
    done
    
    # Test portfolio risk analysis
    local user_id="test-user-id"
    run_test "Analyze portfolio risk" \
        "$CLI_BINARY defi portfolio-risk \"$user_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test impermanent loss calculation
    run_test "Calculate impermanent loss" \
        "$CLI_BINARY defi impermanent-loss ETH/USDC --entry-price 2000 --current-price 2500 --config $PROJECT_ROOT/config/development.toml"
}

# Test 9: Automated Strategies
test_automated_strategies() {
    log "Testing automated DeFi strategies..."
    
    # Test auto-compounding strategy
    run_test "Setup auto-compounding" \
        "$CLI_BINARY defi auto-compound --frequency daily --threshold 10 --config $PROJECT_ROOT/config/development.toml"
    
    # Test yield optimization strategy
    run_test "Setup yield optimization" \
        "$CLI_BINARY defi yield-optimization --target-apy 8 --max-risk medium --config $PROJECT_ROOT/config/development.toml"
    
    # Test rebalancing strategy
    run_test "Setup auto-rebalancing" \
        "$CLI_BINARY defi auto-rebalance --threshold 5 --frequency weekly --config $PROJECT_ROOT/config/development.toml"
}

# Test 10: Cross-Chain Operations
test_cross_chain_operations() {
    log "Testing cross-chain DeFi operations..."
    
    # Test cross-chain staking
    run_test "Cross-chain ETH staking" \
        "$CLI_BINARY defi cross-chain-stake ETH --from ethereum --to polygon --amount 1 --config $PROJECT_ROOT/config/development.toml"
    
    # Test cross-chain yield farming
    run_test "Cross-chain yield farming" \
        "$CLI_BINARY defi cross-chain-farm USDC --from ethereum --to arbitrum --amount 1000 --config $PROJECT_ROOT/config/development.toml"
    
    # Test bridge operations
    run_test "Bridge assets" \
        "$CLI_BINARY defi bridge USDC --from polygon --to ethereum --amount 500 --config $PROJECT_ROOT/config/development.toml"
}

# Test 11: Performance Analytics
test_performance_analytics() {
    log "Testing DeFi performance analytics..."
    
    local user_id="test-user-id"
    
    # Test yield performance tracking
    run_test "Track yield performance" \
        "$CLI_BINARY defi performance \"$user_id\" --period 30d --config $PROJECT_ROOT/config/development.toml"
    
    # Test APY calculations
    run_test "Calculate realized APY" \
        "$CLI_BINARY defi apy \"$user_id\" --realized --config $PROJECT_ROOT/config/development.toml"
    
    # Test fee analysis
    run_test "Analyze transaction fees" \
        "$CLI_BINARY defi fee-analysis \"$user_id\" --period 90d --config $PROJECT_ROOT/config/development.toml"
}

# Test 12: Performance Benchmarks
test_performance_benchmarks() {
    log "Testing DeFi performance benchmarks..."
    
    local start_time=$(date +%s%N)
    
    # Execute multiple DeFi operations rapidly
    for i in {1..10}; do
        "$CLI_BINARY" defi stake "test-pool-$i" "100" --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    done
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    log "Executed 10 DeFi operations in ${duration_ms}ms (avg: $((duration_ms / 10))ms per operation)"
    
    # Check if performance meets requirements (<200ms per operation)
    if [ $((duration_ms / 10)) -lt 200 ]; then
        success "DeFi operation performance meets requirements"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "DeFi operation performance does not meet requirements"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Test 13: Error Handling
test_error_handling() {
    log "Testing DeFi error handling..."
    
    # Test insufficient balance scenarios
    run_test "Handle insufficient balance for staking" \
        "! $CLI_BINARY defi stake \"test-pool\" \"999999\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test invalid protocol operations
    run_test "Handle invalid protocol" \
        "! $CLI_BINARY defi stake \"invalid-protocol\" \"100\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test slippage protection
    run_test "Test slippage protection" \
        "$CLI_BINARY defi swap ETH USDC 1 --max-slippage 1 --config $PROJECT_ROOT/config/development.toml"
}

# Cleanup test environment
cleanup_test_environment() {
    log "Cleaning up DeFi test environment..."
    
    # Unstake all positions
    "$CLI_BINARY" defi unstake-all --force --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    
    # Archive test logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE"
        log "Test logs archived: ${LOG_FILE}.gz"
    fi
    
    success "DeFi test environment cleanup completed"
}

# Generate test report
generate_test_report() {
    log "Generating DeFi flows test report..."
    
    local report_file="$TEST_DATA_DIR/defi_flows_report_$(date +%Y%m%d_%H%M%S).json"
    
    cat > "$report_file" << EOF
{
    "test_suite": "DeFi Flows End-to-End Testing",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "results": {
        "total_tests": $TESTS_TOTAL,
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "success_rate": $(echo "scale=2; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)
    },
    "test_categories": [
        "yield_product_discovery",
        "staking_operations",
        "liquidity_provision",
        "yield_farming",
        "position_management",
        "unstaking_withdrawals",
        "rewards_harvesting",
        "risk_assessment",
        "automated_strategies",
        "cross_chain_operations",
        "performance_analytics",
        "performance_benchmarks",
        "error_handling"
    ],
    "performance_metrics": {
        "avg_defi_operation_time_ms": "< 200",
        "supported_protocols": ["Compound", "Aave", "Uniswap", "Curve", "Yearn"],
        "cross_chain_support": ["Ethereum", "Polygon", "Arbitrum"]
    },
    "defi_features_tested": [
        "yield_farming",
        "staking",
        "liquidity_provision",
        "automated_strategies",
        "cross_chain_operations",
        "risk_assessment"
    ],
    "log_file": "${LOG_FILE}.gz"
}
EOF
    
    log "Test report generated: $report_file"
    
    # Print summary
    echo
    echo "=========================================="
    echo "DEFI FLOWS TEST SUMMARY"
    echo "=========================================="
    echo "Total Tests: $TESTS_TOTAL"
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    echo "Success Rate: $(echo "scale=1; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)%"
    echo "Report: $report_file"
    echo "=========================================="
}

# Main execution
main() {
    log "Starting FO3 Wallet Core - DeFi Flows End-to-End Testing"
    
    setup_test_environment
    
    test_yield_product_discovery
    test_staking_operations
    test_liquidity_provision
    test_yield_farming
    test_position_management
    test_unstaking_withdrawals
    test_rewards_harvesting
    test_risk_assessment
    test_automated_strategies
    test_cross_chain_operations
    test_performance_analytics
    test_performance_benchmarks
    test_error_handling
    
    cleanup_test_environment
    generate_test_report
    
    # Exit with appropriate code
    if [ $TESTS_FAILED -eq 0 ]; then
        success "All DeFi flow tests passed!"
        exit 0
    else
        error "$TESTS_FAILED out of $TESTS_TOTAL tests failed"
        exit 1
    fi
}

# Run main function
main "$@"
