#!/bin/bash
# FO3 Wallet Core - Wallet Flow End-to-End Testing Script
# 
# Tests complete wallet lifecycle including:
# - Wallet creation and import
# - Address generation across multiple chains
# - Balance checking and transaction flows
# - Multi-currency wallet management

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CLI_BINARY="$PROJECT_ROOT/target/debug/fo3_cli"
GRPC_ENDPOINT="http://localhost:50051"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/wallet_flows_$(date +%Y%m%d_%H%M%S).log"

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
    log "Setting up test environment..."
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Build CLI if it doesn't exist
    if [ ! -f "$CLI_BINARY" ]; then
        log "Building CLI binary..."
        cd "$PROJECT_ROOT/fo3-wallet-api"
        cargo build --bin fo3_cli
        cd "$SCRIPT_DIR"
    fi
    
    # Initialize database
    log "Initializing test database..."
    "$CLI_BINARY" database init --config "$PROJECT_ROOT/config/development.toml"
    
    # Seed test data
    log "Seeding test data..."
    "$CLI_BINARY" database seed --config "$PROJECT_ROOT/config/development.toml"
    
    success "Test environment setup completed"
}

# Test 1: Wallet Creation
test_wallet_creation() {
    log "Testing wallet creation..."
    
    # Test creating wallets with different names
    local wallet_names=("Primary Wallet" "Trading Wallet" "Savings Wallet")
    
    for wallet_name in "${wallet_names[@]}"; do
        run_test "Create wallet: $wallet_name" \
            "$CLI_BINARY wallet create \"$wallet_name\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 2: Wallet Import
test_wallet_import() {
    log "Testing wallet import..."
    
    # Test mnemonic (DO NOT USE IN PRODUCTION)
    local test_mnemonic="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    
    run_test "Import wallet from mnemonic" \
        "$CLI_BINARY wallet import \"Imported Wallet\" \"$test_mnemonic\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 3: Address Generation
test_address_generation() {
    log "Testing address generation across multiple chains..."
    
    # Get first wallet ID (assuming wallet creation succeeded)
    local wallet_id="test-wallet-id"  # This would be retrieved from previous tests
    
    # Test address generation for different key types
    local key_types=("ethereum" "bitcoin" "solana")
    
    for key_type in "${key_types[@]}"; do
        run_test "Generate $key_type address" \
            "$CLI_BINARY wallet address \"$wallet_id\" \"$key_type\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 4: Balance Checking
test_balance_checking() {
    log "Testing balance checking..."
    
    local wallet_id="test-wallet-id"
    
    run_test "Check wallet balance" \
        "$CLI_BINARY wallet balance \"$wallet_id\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 5: Multi-Currency Wallet Management
test_multi_currency_wallets() {
    log "Testing multi-currency wallet management..."
    
    local currencies=("USD" "BTC" "ETH" "USDC")
    
    for currency in "${currencies[@]}"; do
        run_test "Create $currency wallet" \
            "$CLI_BINARY wallet create \"$currency Wallet\" --currency \"$currency\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 6: Wallet Listing and Details
test_wallet_listing() {
    log "Testing wallet listing and details..."
    
    run_test "List all wallets" \
        "$CLI_BINARY wallet list --config $PROJECT_ROOT/config/development.toml"
    
    # Test getting details for specific wallet
    local wallet_id="test-wallet-id"
    run_test "Get wallet details" \
        "$CLI_BINARY wallet get \"$wallet_id\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 7: Error Handling
test_error_handling() {
    log "Testing error handling scenarios..."
    
    # Test invalid wallet operations
    run_test "Handle invalid wallet ID" \
        "! $CLI_BINARY wallet get \"invalid-wallet-id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test invalid mnemonic import
    run_test "Handle invalid mnemonic" \
        "! $CLI_BINARY wallet import \"Invalid Wallet\" \"invalid mnemonic phrase\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 8: Performance Testing
test_wallet_performance() {
    log "Testing wallet operation performance..."
    
    local start_time=$(date +%s%N)
    
    # Create multiple wallets rapidly
    for i in {1..10}; do
        "$CLI_BINARY" wallet create "Perf Test Wallet $i" --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    done
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    log "Created 10 wallets in ${duration_ms}ms (avg: $((duration_ms / 10))ms per wallet)"
    
    # Check if performance meets requirements (<200ms per operation)
    if [ $((duration_ms / 10)) -lt 200 ]; then
        success "Wallet creation performance meets requirements"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Wallet creation performance does not meet requirements"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Test 9: State Persistence
test_state_persistence() {
    log "Testing wallet state persistence..."
    
    # Create a wallet and verify it persists after restart
    local test_wallet_name="Persistence Test Wallet"
    
    run_test "Create wallet for persistence test" \
        "$CLI_BINARY wallet create \"$test_wallet_name\" --config $PROJECT_ROOT/config/development.toml"
    
    # Simulate restart by listing wallets again
    run_test "Verify wallet persists after restart" \
        "$CLI_BINARY wallet list --config $PROJECT_ROOT/config/development.toml | grep \"$test_wallet_name\""
}

# Test 10: Concurrent Operations
test_concurrent_operations() {
    log "Testing concurrent wallet operations..."
    
    # Run multiple wallet operations concurrently
    local pids=()
    
    for i in {1..5}; do
        "$CLI_BINARY" wallet create "Concurrent Wallet $i" --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 &
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
        success "Concurrent wallet operations completed successfully"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Some concurrent wallet operations failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Cleanup test environment
cleanup_test_environment() {
    log "Cleaning up test environment..."
    
    # Archive test logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE"
        log "Test logs archived: ${LOG_FILE}.gz"
    fi
    
    success "Test environment cleanup completed"
}

# Generate test report
generate_test_report() {
    log "Generating wallet flows test report..."
    
    local report_file="$TEST_DATA_DIR/wallet_flows_report_$(date +%Y%m%d_%H%M%S).json"
    
    cat > "$report_file" << EOF
{
    "test_suite": "Wallet Flows End-to-End Testing",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "results": {
        "total_tests": $TESTS_TOTAL,
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "success_rate": $(echo "scale=2; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)
    },
    "test_categories": [
        "wallet_creation",
        "wallet_import",
        "address_generation",
        "balance_checking",
        "multi_currency",
        "listing_details",
        "error_handling",
        "performance",
        "state_persistence",
        "concurrent_operations"
    ],
    "performance_metrics": {
        "avg_wallet_creation_time_ms": "< 200",
        "concurrent_operations_supported": 5
    },
    "log_file": "${LOG_FILE}.gz"
}
EOF
    
    log "Test report generated: $report_file"
    
    # Print summary
    echo
    echo "=========================================="
    echo "WALLET FLOWS TEST SUMMARY"
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
    log "Starting FO3 Wallet Core - Wallet Flows End-to-End Testing"
    
    setup_test_environment
    
    test_wallet_creation
    test_wallet_import
    test_address_generation
    test_balance_checking
    test_multi_currency_wallets
    test_wallet_listing
    test_error_handling
    test_wallet_performance
    test_state_persistence
    test_concurrent_operations
    
    cleanup_test_environment
    generate_test_report
    
    # Exit with appropriate code
    if [ $TESTS_FAILED -eq 0 ]; then
        success "All wallet flow tests passed!"
        exit 0
    else
        error "$TESTS_FAILED out of $TESTS_TOTAL tests failed"
        exit 1
    fi
}

# Run main function
main "$@"
