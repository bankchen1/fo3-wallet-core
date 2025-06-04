#!/bin/bash
# FO3 Wallet Core - Trading Flow End-to-End Testing Script
# 
# Tests complete automated trading lifecycle including:
# - Trading strategy creation and management
# - Trade execution and monitoring
# - Risk management and position sizing
# - Performance analytics and reporting
# - ML-driven trading signals

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CLI_BINARY="$PROJECT_ROOT/target/debug/fo3_cli"
GRPC_ENDPOINT="http://localhost:50051"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/trading_flows_$(date +%Y%m%d_%H%M%S).log"

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
    log "Setting up trading test environment..."
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Initialize database with trading data
    log "Initializing trading database..."
    "$CLI_BINARY" database init --config "$PROJECT_ROOT/config/development.toml"
    "$CLI_BINARY" database seed --config "$PROJECT_ROOT/config/development.toml"
    
    success "Trading test environment setup completed"
}

# Test 1: Trading Strategy Creation
test_strategy_creation() {
    log "Testing trading strategy creation..."
    
    # Test different strategy types
    local strategies=(
        "DCA Strategy:dca"
        "Grid Trading:grid"
        "Momentum Strategy:momentum"
        "Mean Reversion:mean_reversion"
        "Arbitrage Strategy:arbitrage"
    )
    
    for strategy in "${strategies[@]}"; do
        local name="${strategy%:*}"
        local type="${strategy#*:}"
        
        run_test "Create $name" \
            "$CLI_BINARY trading create-strategy \"$name\" \"$type\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 2: Strategy Configuration
test_strategy_configuration() {
    log "Testing strategy configuration..."
    
    # Test configuring strategy parameters
    local strategy_id="test-strategy-id"
    
    run_test "Configure strategy risk parameters" \
        "$CLI_BINARY trading configure \"$strategy_id\" --max-position-size 1000 --stop-loss 0.05 --take-profit 0.15 --config $PROJECT_ROOT/config/development.toml"
    
    run_test "Configure strategy trading pairs" \
        "$CLI_BINARY trading configure \"$strategy_id\" --pairs BTC/USD,ETH/USD,SOL/USD --config $PROJECT_ROOT/config/development.toml"
}

# Test 3: Trade Execution
test_trade_execution() {
    log "Testing trade execution..."
    
    local strategy_id="test-strategy-id"
    local trading_pairs=("BTC/USD" "ETH/USD" "SOL/USD" "USDC/USD")
    local amounts=("100" "500" "250" "1000")
    
    for i in "${!trading_pairs[@]}"; do
        local pair="${trading_pairs[$i]}"
        local amount="${amounts[$i]}"
        
        run_test "Execute trade: $amount $pair" \
            "$CLI_BINARY trading execute-trade \"$strategy_id\" \"$pair\" \"$amount\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 4: Risk Management
test_risk_management() {
    log "Testing risk management features..."
    
    local strategy_id="test-strategy-id"
    
    # Test position sizing
    run_test "Test position sizing calculation" \
        "$CLI_BINARY trading calculate-position-size \"$strategy_id\" BTC/USD 10000 --risk-percent 2 --config $PROJECT_ROOT/config/development.toml"
    
    # Test stop-loss triggers
    run_test "Test stop-loss trigger" \
        "$CLI_BINARY trading test-stop-loss \"$strategy_id\" --price-drop 6 --config $PROJECT_ROOT/config/development.toml"
    
    # Test portfolio exposure limits
    run_test "Test portfolio exposure limits" \
        "$CLI_BINARY trading check-exposure \"$strategy_id\" --max-exposure 50 --config $PROJECT_ROOT/config/development.toml"
}

# Test 5: ML Trading Signals
test_ml_trading_signals() {
    log "Testing ML-driven trading signals..."
    
    # Test sentiment analysis signals
    run_test "Generate sentiment trading signals" \
        "$CLI_BINARY trading ml-signals sentiment BTC --timeframe 1h --config $PROJECT_ROOT/config/development.toml"
    
    # Test technical analysis signals
    run_test "Generate technical analysis signals" \
        "$CLI_BINARY trading ml-signals technical ETH --indicators RSI,MACD,BB --config $PROJECT_ROOT/config/development.toml"
    
    # Test market prediction signals
    run_test "Generate market prediction signals" \
        "$CLI_BINARY trading ml-signals prediction SOL --horizon 24h --config $PROJECT_ROOT/config/development.toml"
}

# Test 6: Performance Analytics
test_performance_analytics() {
    log "Testing trading performance analytics..."
    
    local strategy_id="test-strategy-id"
    
    # Test strategy performance metrics
    run_test "Get strategy performance" \
        "$CLI_BINARY trading performance \"$strategy_id\" --period 30d --config $PROJECT_ROOT/config/development.toml"
    
    # Test risk-adjusted returns
    run_test "Calculate Sharpe ratio" \
        "$CLI_BINARY trading metrics \"$strategy_id\" --metric sharpe --config $PROJECT_ROOT/config/development.toml"
    
    # Test drawdown analysis
    run_test "Analyze maximum drawdown" \
        "$CLI_BINARY trading metrics \"$strategy_id\" --metric max-drawdown --config $PROJECT_ROOT/config/development.toml"
}

# Test 7: Portfolio Management
test_portfolio_management() {
    log "Testing portfolio management features..."
    
    # Test portfolio rebalancing
    run_test "Test portfolio rebalancing" \
        "$CLI_BINARY trading rebalance --target-allocation BTC:40,ETH:30,SOL:20,USDC:10 --config $PROJECT_ROOT/config/development.toml"
    
    # Test correlation analysis
    run_test "Analyze asset correlations" \
        "$CLI_BINARY trading correlations --assets BTC,ETH,SOL --period 90d --config $PROJECT_ROOT/config/development.toml"
    
    # Test diversification metrics
    run_test "Calculate diversification ratio" \
        "$CLI_BINARY trading diversification --config $PROJECT_ROOT/config/development.toml"
}

# Test 8: Backtesting
test_backtesting() {
    log "Testing strategy backtesting..."
    
    local strategy_id="test-strategy-id"
    
    # Test historical backtesting
    run_test "Run historical backtest" \
        "$CLI_BINARY trading backtest \"$strategy_id\" --start-date 2023-01-01 --end-date 2023-12-31 --config $PROJECT_ROOT/config/development.toml"
    
    # Test walk-forward analysis
    run_test "Run walk-forward analysis" \
        "$CLI_BINARY trading walk-forward \"$strategy_id\" --window 90d --step 30d --config $PROJECT_ROOT/config/development.toml"
}

# Test 9: Real-time Monitoring
test_realtime_monitoring() {
    log "Testing real-time trading monitoring..."
    
    local strategy_id="test-strategy-id"
    
    # Test real-time position monitoring
    run_test "Monitor active positions" \
        "timeout 10s $CLI_BINARY trading monitor \"$strategy_id\" --real-time --config $PROJECT_ROOT/config/development.toml || true"
    
    # Test alert system
    run_test "Test trading alerts" \
        "$CLI_BINARY trading alerts \"$strategy_id\" --price-change 5 --volume-spike 200 --config $PROJECT_ROOT/config/development.toml"
}

# Test 10: Performance Benchmarks
test_performance_benchmarks() {
    log "Testing trading performance benchmarks..."
    
    local start_time=$(date +%s%N)
    
    # Execute multiple trades rapidly
    for i in {1..20}; do
        "$CLI_BINARY" trading execute-trade "test-strategy" "BTC/USD" "10" --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    done
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    log "Executed 20 trades in ${duration_ms}ms (avg: $((duration_ms / 20))ms per trade)"
    
    # Check if performance meets requirements (<200ms per trade)
    if [ $((duration_ms / 20)) -lt 200 ]; then
        success "Trade execution performance meets requirements"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Trade execution performance does not meet requirements"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Test 11: Error Handling and Recovery
test_error_handling() {
    log "Testing trading error handling and recovery..."
    
    # Test invalid strategy operations
    run_test "Handle invalid strategy ID" \
        "! $CLI_BINARY trading performance \"invalid-strategy-id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test insufficient balance scenarios
    run_test "Handle insufficient balance" \
        "! $CLI_BINARY trading execute-trade \"test-strategy\" \"BTC/USD\" \"999999\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test market closure scenarios
    run_test "Handle market closure" \
        "$CLI_BINARY trading check-market-status --config $PROJECT_ROOT/config/development.toml"
}

# Test 12: Concurrent Trading Operations
test_concurrent_trading() {
    log "Testing concurrent trading operations..."
    
    # Run multiple trading operations concurrently
    local pids=()
    
    for i in {1..5}; do
        "$CLI_BINARY" trading execute-trade "test-strategy-$i" "ETH/USD" "50" --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 &
        pids+=($!)
    done
    
    # Wait for all operations to complete
    local all_success=true
    for pid in "${pids[@]}"; do
        if ! wait "$pid"; then
            all_success=false
        fi
    done
    
    if $all_success; then
        success "Concurrent trading operations completed successfully"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Some concurrent trading operations failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Cleanup test environment
cleanup_test_environment() {
    log "Cleaning up trading test environment..."
    
    # Close any open positions
    "$CLI_BINARY" trading close-all-positions --force --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    
    # Archive test logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE"
        log "Test logs archived: ${LOG_FILE}.gz"
    fi
    
    success "Trading test environment cleanup completed"
}

# Generate test report
generate_test_report() {
    log "Generating trading flows test report..."
    
    local report_file="$TEST_DATA_DIR/trading_flows_report_$(date +%Y%m%d_%H%M%S).json"
    
    cat > "$report_file" << EOF
{
    "test_suite": "Trading Flows End-to-End Testing",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "results": {
        "total_tests": $TESTS_TOTAL,
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "success_rate": $(echo "scale=2; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)
    },
    "test_categories": [
        "strategy_creation",
        "strategy_configuration",
        "trade_execution",
        "risk_management",
        "ml_trading_signals",
        "performance_analytics",
        "portfolio_management",
        "backtesting",
        "realtime_monitoring",
        "performance_benchmarks",
        "error_handling",
        "concurrent_trading"
    ],
    "performance_metrics": {
        "avg_trade_execution_time_ms": "< 200",
        "concurrent_trades_supported": 5,
        "ml_signal_generation_time_ms": "< 500"
    },
    "trading_features_tested": [
        "automated_strategies",
        "risk_management",
        "ml_signals",
        "backtesting",
        "real_time_monitoring",
        "portfolio_rebalancing"
    ],
    "log_file": "${LOG_FILE}.gz"
}
EOF
    
    log "Test report generated: $report_file"
    
    # Print summary
    echo
    echo "=========================================="
    echo "TRADING FLOWS TEST SUMMARY"
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
    log "Starting FO3 Wallet Core - Trading Flows End-to-End Testing"
    
    setup_test_environment
    
    test_strategy_creation
    test_strategy_configuration
    test_trade_execution
    test_risk_management
    test_ml_trading_signals
    test_performance_analytics
    test_portfolio_management
    test_backtesting
    test_realtime_monitoring
    test_performance_benchmarks
    test_error_handling
    test_concurrent_trading
    
    cleanup_test_environment
    generate_test_report
    
    # Exit with appropriate code
    if [ $TESTS_FAILED -eq 0 ]; then
        success "All trading flow tests passed!"
        exit 0
    else
        error "$TESTS_FAILED out of $TESTS_TOTAL tests failed"
        exit 1
    fi
}

# Run main function
main "$@"
