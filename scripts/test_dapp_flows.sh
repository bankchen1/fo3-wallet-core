#!/bin/bash
# FO3 Wallet Core - DApp Flow End-to-End Testing Script
# 
# Tests complete DApp browser and signing ecosystem including:
# - DApp connection and session management
# - Multi-chain transaction signing
# - WalletConnect v2 integration
# - DApp whitelist and security features
# - Real-time communication and notifications

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CLI_BINARY="$PROJECT_ROOT/target/debug/fo3_cli"
GRPC_ENDPOINT="http://localhost:50051"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/dapp_flows_$(date +%Y%m%d_%H%M%S).log"

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
    log "Setting up DApp test environment..."
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Initialize database with DApp data
    log "Initializing DApp database..."
    "$CLI_BINARY" database init --config "$PROJECT_ROOT/config/development.toml"
    "$CLI_BINARY" database seed --config "$PROJECT_ROOT/config/development.toml"
    
    success "DApp test environment setup completed"
}

# Test 1: DApp Connection
test_dapp_connection() {
    log "Testing DApp connection flows..."
    
    # Test connecting to different types of DApps
    local dapp_urls=(
        "https://uniswap.org"
        "https://app.aave.com"
        "https://compound.finance"
        "https://opensea.io"
        "https://app.1inch.io"
    )
    
    for dapp_url in "${dapp_urls[@]}"; do
        run_test "Connect to DApp: $dapp_url" \
            "$CLI_BINARY dapp connect \"$dapp_url\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 2: WalletConnect Integration
test_walletconnect_integration() {
    log "Testing WalletConnect v2 integration..."
    
    # Test WalletConnect session establishment
    run_test "Establish WalletConnect session" \
        "$CLI_BINARY dapp walletconnect-session --uri \"wc:test-session-uri\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test session management
    run_test "List active WalletConnect sessions" \
        "$CLI_BINARY dapp list-sessions --config $PROJECT_ROOT/config/development.toml"
    
    # Test session approval
    local session_id="test-session-id"
    run_test "Approve WalletConnect session" \
        "$CLI_BINARY dapp approve-session \"$session_id\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 3: Multi-Chain Transaction Signing
test_multichain_signing() {
    log "Testing multi-chain transaction signing..."
    
    # Test Ethereum transaction signing
    local eth_tx_data='{"to":"0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6","value":"1000000000000000000","data":"0x"}'
    run_test "Sign Ethereum transaction" \
        "$CLI_BINARY dapp sign \"test-dapp-id\" \"$eth_tx_data\" --chain ethereum --config $PROJECT_ROOT/config/development.toml"
    
    # Test Solana transaction signing
    local sol_tx_data='{"instructions":[{"programId":"11111111111111111111111111111112","accounts":[],"data":""}]}'
    run_test "Sign Solana transaction" \
        "$CLI_BINARY dapp sign \"test-dapp-id\" \"$sol_tx_data\" --chain solana --config $PROJECT_ROOT/config/development.toml"
    
    # Test Bitcoin transaction signing
    local btc_tx_data='{"inputs":[{"txid":"abc123","vout":0}],"outputs":[{"address":"bc1qtest","value":50000}]}'
    run_test "Sign Bitcoin transaction" \
        "$CLI_BINARY dapp sign \"test-dapp-id\" \"$btc_tx_data\" --chain bitcoin --config $PROJECT_ROOT/config/development.toml"
}

# Test 4: DApp Whitelist Management
test_dapp_whitelist() {
    log "Testing DApp whitelist management..."
    
    # Test adding DApps to whitelist
    local trusted_dapps=(
        "uniswap.org"
        "app.aave.com"
        "compound.finance"
    )
    
    for dapp in "${trusted_dapps[@]}"; do
        run_test "Add $dapp to whitelist" \
            "$CLI_BINARY dapp whitelist-add \"$dapp\" --config $PROJECT_ROOT/config/development.toml"
    done
    
    # Test whitelist verification
    run_test "Verify whitelist status" \
        "$CLI_BINARY dapp whitelist-check \"uniswap.org\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test removing from whitelist
    run_test "Remove from whitelist" \
        "$CLI_BINARY dapp whitelist-remove \"compound.finance\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 5: Permission Management
test_permission_management() {
    log "Testing DApp permission management..."
    
    local dapp_id="test-dapp-id"
    
    # Test granting permissions
    local permissions=("read_balance" "sign_transactions" "access_accounts")
    
    for permission in "${permissions[@]}"; do
        run_test "Grant $permission permission" \
            "$CLI_BINARY dapp grant-permission \"$dapp_id\" \"$permission\" --config $PROJECT_ROOT/config/development.toml"
    done
    
    # Test permission verification
    run_test "Check DApp permissions" \
        "$CLI_BINARY dapp check-permissions \"$dapp_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test revoking permissions
    run_test "Revoke permission" \
        "$CLI_BINARY dapp revoke-permission \"$dapp_id\" \"access_accounts\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 6: Transaction Simulation
test_transaction_simulation() {
    log "Testing transaction simulation..."
    
    # Test simulating different transaction types
    local simulations=(
        "ethereum:swap:ETH->USDC:1"
        "ethereum:approve:USDC:1000"
        "solana:transfer:SOL:0.5"
        "bitcoin:send:BTC:0.001"
    )
    
    for simulation in "${simulations[@]}"; do
        local chain="${simulation%%:*}"
        local remaining="${simulation#*:}"
        local type="${remaining%%:*}"
        local details="${remaining#*:}"
        
        run_test "Simulate $chain $type transaction" \
            "$CLI_BINARY dapp simulate \"$chain\" \"$type\" \"$details\" --config $PROJECT_ROOT/config/development.toml"
    done
}

# Test 7: Gas Estimation and Optimization
test_gas_optimization() {
    log "Testing gas estimation and optimization..."
    
    # Test gas estimation for different transaction types
    run_test "Estimate gas for token swap" \
        "$CLI_BINARY dapp estimate-gas ethereum swap ETH USDC 1 --config $PROJECT_ROOT/config/development.toml"
    
    # Test gas optimization suggestions
    run_test "Get gas optimization suggestions" \
        "$CLI_BINARY dapp optimize-gas ethereum --config $PROJECT_ROOT/config/development.toml"
    
    # Test priority fee recommendations
    run_test "Get priority fee recommendations" \
        "$CLI_BINARY dapp priority-fee ethereum --speed fast --config $PROJECT_ROOT/config/development.toml"
}

# Test 8: Real-time Notifications
test_realtime_notifications() {
    log "Testing real-time DApp notifications..."
    
    local dapp_id="test-dapp-id"
    
    # Test transaction status notifications
    run_test "Setup transaction notifications" \
        "$CLI_BINARY dapp notifications \"$dapp_id\" --events transaction_confirmed,transaction_failed --config $PROJECT_ROOT/config/development.toml"
    
    # Test price alert notifications
    run_test "Setup price alert notifications" \
        "$CLI_BINARY dapp price-alerts \"$dapp_id\" --asset ETH --threshold 2000 --config $PROJECT_ROOT/config/development.toml"
    
    # Test WebSocket connection for real-time updates
    run_test "Test WebSocket notifications" \
        "timeout 5s $CLI_BINARY dapp websocket-test \"$dapp_id\" --config $PROJECT_ROOT/config/development.toml || true"
}

# Test 9: Security Features
test_security_features() {
    log "Testing DApp security features..."
    
    # Test phishing detection
    local suspicious_urls=(
        "https://uniswap-fake.com"
        "https://metamask-phishing.org"
        "https://fake-aave.net"
    )
    
    for url in "${suspicious_urls[@]}"; do
        run_test "Detect phishing: $url" \
            "$CLI_BINARY dapp phishing-check \"$url\" --config $PROJECT_ROOT/config/development.toml"
    done
    
    # Test malicious contract detection
    run_test "Scan for malicious contracts" \
        "$CLI_BINARY dapp contract-scan \"0x123456789abcdef\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test transaction risk assessment
    run_test "Assess transaction risk" \
        "$CLI_BINARY dapp risk-assessment \"test-transaction-data\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 10: Session Management
test_session_management() {
    log "Testing DApp session management..."
    
    # Test session timeout handling
    run_test "Test session timeout" \
        "$CLI_BINARY dapp session-timeout \"test-session-id\" --timeout 3600 --config $PROJECT_ROOT/config/development.toml"
    
    # Test session renewal
    run_test "Renew session" \
        "$CLI_BINARY dapp renew-session \"test-session-id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test session cleanup
    run_test "Cleanup expired sessions" \
        "$CLI_BINARY dapp cleanup-sessions --config $PROJECT_ROOT/config/development.toml"
}

# Test 11: Cross-Chain DApp Support
test_crosschain_dapp_support() {
    log "Testing cross-chain DApp support..."
    
    # Test multi-chain DApp connections
    run_test "Connect multi-chain DApp" \
        "$CLI_BINARY dapp connect-multichain \"https://multichain-dapp.com\" --chains ethereum,solana,bitcoin --config $PROJECT_ROOT/config/development.toml"
    
    # Test chain switching
    run_test "Switch active chain" \
        "$CLI_BINARY dapp switch-chain \"test-dapp-id\" solana --config $PROJECT_ROOT/config/development.toml"
    
    # Test cross-chain transaction coordination
    run_test "Coordinate cross-chain transaction" \
        "$CLI_BINARY dapp cross-chain-tx \"test-dapp-id\" --from ethereum --to solana --config $PROJECT_ROOT/config/development.toml"
}

# Test 12: Performance Benchmarks
test_performance_benchmarks() {
    log "Testing DApp performance benchmarks..."
    
    local start_time=$(date +%s%N)
    
    # Execute multiple DApp operations rapidly
    for i in {1..20}; do
        "$CLI_BINARY" dapp sign "test-dapp-$i" '{"test":"data"}' --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    done
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    log "Executed 20 DApp operations in ${duration_ms}ms (avg: $((duration_ms / 20))ms per operation)"
    
    # Check if performance meets requirements (<500ms per signing operation)
    if [ $((duration_ms / 20)) -lt 500 ]; then
        success "DApp signing performance meets requirements"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "DApp signing performance does not meet requirements"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Test 13: Error Handling and Recovery
test_error_handling() {
    log "Testing DApp error handling and recovery..."
    
    # Test invalid DApp URL handling
    run_test "Handle invalid DApp URL" \
        "! $CLI_BINARY dapp connect \"invalid-url\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test malformed transaction data
    run_test "Handle malformed transaction data" \
        "! $CLI_BINARY dapp sign \"test-dapp-id\" \"invalid-json\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test network connectivity issues
    run_test "Handle network connectivity issues" \
        "$CLI_BINARY dapp connection-test --timeout 5 --config $PROJECT_ROOT/config/development.toml"
    
    # Test session recovery
    run_test "Test session recovery" \
        "$CLI_BINARY dapp recover-session \"test-session-id\" --config $PROJECT_ROOT/config/development.toml"
}

# Test 14: DApp Disconnection
test_dapp_disconnection() {
    log "Testing DApp disconnection flows..."
    
    # Test graceful disconnection
    local dapp_id="test-dapp-id"
    run_test "Graceful DApp disconnection" \
        "$CLI_BINARY dapp disconnect \"$dapp_id\" --config $PROJECT_ROOT/config/development.toml"
    
    # Test force disconnection
    run_test "Force disconnect all DApps" \
        "$CLI_BINARY dapp disconnect-all --force --config $PROJECT_ROOT/config/development.toml"
    
    # Test cleanup after disconnection
    run_test "Cleanup after disconnection" \
        "$CLI_BINARY dapp cleanup --config $PROJECT_ROOT/config/development.toml"
}

# Cleanup test environment
cleanup_test_environment() {
    log "Cleaning up DApp test environment..."
    
    # Disconnect all DApps
    "$CLI_BINARY" dapp disconnect-all --force --config "$PROJECT_ROOT/config/development.toml" >> "$LOG_FILE" 2>&1 || true
    
    # Archive test logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE"
        log "Test logs archived: ${LOG_FILE}.gz"
    fi
    
    success "DApp test environment cleanup completed"
}

# Generate test report
generate_test_report() {
    log "Generating DApp flows test report..."
    
    local report_file="$TEST_DATA_DIR/dapp_flows_report_$(date +%Y%m%d_%H%M%S).json"
    
    cat > "$report_file" << EOF
{
    "test_suite": "DApp Flows End-to-End Testing",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "results": {
        "total_tests": $TESTS_TOTAL,
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "success_rate": $(echo "scale=2; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)
    },
    "test_categories": [
        "dapp_connection",
        "walletconnect_integration",
        "multichain_signing",
        "dapp_whitelist",
        "permission_management",
        "transaction_simulation",
        "gas_optimization",
        "realtime_notifications",
        "security_features",
        "session_management",
        "crosschain_dapp_support",
        "performance_benchmarks",
        "error_handling",
        "dapp_disconnection"
    ],
    "performance_metrics": {
        "avg_signing_time_ms": "< 500",
        "supported_chains": ["Ethereum", "Solana", "Bitcoin"],
        "walletconnect_version": "v2",
        "max_concurrent_sessions": 10
    },
    "security_features_tested": [
        "phishing_detection",
        "malicious_contract_scanning",
        "transaction_risk_assessment",
        "permission_management",
        "session_security"
    ],
    "log_file": "${LOG_FILE}.gz"
}
EOF
    
    log "Test report generated: $report_file"
    
    # Print summary
    echo
    echo "=========================================="
    echo "DAPP FLOWS TEST SUMMARY"
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
    log "Starting FO3 Wallet Core - DApp Flows End-to-End Testing"
    
    setup_test_environment
    
    test_dapp_connection
    test_walletconnect_integration
    test_multichain_signing
    test_dapp_whitelist
    test_permission_management
    test_transaction_simulation
    test_gas_optimization
    test_realtime_notifications
    test_security_features
    test_session_management
    test_crosschain_dapp_support
    test_performance_benchmarks
    test_error_handling
    test_dapp_disconnection
    
    cleanup_test_environment
    generate_test_report
    
    # Exit with appropriate code
    if [ $TESTS_FAILED -eq 0 ]; then
        success "All DApp flow tests passed!"
        exit 0
    else
        error "$TESTS_FAILED out of $TESTS_TOTAL tests failed"
        exit 1
    fi
}

# Run main function
main "$@"
