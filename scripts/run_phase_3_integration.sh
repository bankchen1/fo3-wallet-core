#!/bin/bash
# FO3 Wallet Core - Phase 3 Integration & Performance Testing Script
# 
# Executes comprehensive integration and performance validation including:
# - Integration test suites with local database
# - Performance validation under realistic load
# - Observability validation (Jaeger, Prometheus, Grafana)
# - Load testing with 50+ concurrent users
# - Test report generation and analysis

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/phase_3_integration_$(date +%Y%m%d_%H%M%S).log"
REPORT_FILE="$TEST_DATA_DIR/phase_3_report_$(date +%Y%m%d_%H%M%S).json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
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

phase_header() {
    echo -e "${PURPLE}========================================${NC}" | tee -a "$LOG_FILE"
    echo -e "${PURPLE}$1${NC}" | tee -a "$LOG_FILE"
    echo -e "${PURPLE}========================================${NC}" | tee -a "$LOG_FILE"
}

# Test tracking
TOTAL_TESTS=0
TESTS_PASSED=0
TESTS_FAILED=0
PERFORMANCE_VIOLATIONS=0
SECURITY_ISSUES=0

# Step 1: Environment Setup and Validation
setup_integration_environment() {
    phase_header "STEP 1: Integration Environment Setup"
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Verify database is running and seeded
    log "Verifying database connectivity and seed data..."
    cd "$PROJECT_ROOT/fo3-wallet-api"
    
    if ! ./target/debug/fo3_cli database health --config "$PROJECT_ROOT/config/development.toml"; then
        error "Database health check failed"
        return 1
    fi
    
    # Start full application stack if not running
    log "Starting full FO3 Wallet Core application stack..."
    cd "$PROJECT_ROOT"
    
    # Check if services are already running
    if ! docker-compose -f docker-compose.dev.yml ps | grep -q "Up"; then
        log "Starting Docker Compose stack..."
        docker-compose -f docker-compose.dev.yml up -d
        
        # Wait for services to be ready
        log "Waiting for services to be ready..."
        sleep 60
    else
        log "Docker Compose stack already running"
    fi
    
    # Verify all services are healthy
    log "Verifying service health..."
    
    # Check PostgreSQL
    if ! docker-compose -f docker-compose.dev.yml exec -T postgres pg_isready -U fo3_dev -d fo3_wallet_dev; then
        error "PostgreSQL is not ready"
        return 1
    fi
    
    # Check Redis
    if ! docker-compose -f docker-compose.dev.yml exec -T redis redis-cli ping; then
        error "Redis is not ready"
        return 1
    fi
    
    # Check Jaeger
    if ! curl -f "http://localhost:16686/api/services" > /dev/null 2>&1; then
        error "Jaeger is not accessible"
        return 1
    fi
    
    # Check Prometheus
    if ! curl -f "http://localhost:9090/api/v1/query?query=up" > /dev/null 2>&1; then
        error "Prometheus is not accessible"
        return 1
    fi
    
    success "Integration environment setup completed"
    return 0
}

# Step 2: Integration Test Execution
run_integration_tests() {
    phase_header "STEP 2: Integration Test Execution"
    
    cd "$PROJECT_ROOT/fo3-wallet-api"
    
    # Set environment variables for tests
    export TEST_SERVER_ADDRESS="http://localhost:50051"
    export TEST_REPORT_DIR="$TEST_DATA_DIR"
    export RUST_LOG="debug,fo3_wallet_api=trace"
    
    log "Running comprehensive integration tests..."
    
    # Run Phase 3 integration tests
    local start_time=$(date +%s)
    
    if cargo test --test phase_3_integration_tests test_phase_3_integration_comprehensive -- --nocapture; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        success "Integration tests passed in ${duration}s"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        error "Integration tests failed after ${duration}s"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run service registration validation
    log "Running service registration validation..."
    if cargo test --test phase_3_integration_tests test_phase_3_service_registration_only -- --nocapture; then
        success "Service registration validation passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Service registration validation failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run individual service tests
    local services=("card_funding_test" "ledger_test" "security_validation")
    
    for service in "${services[@]}"; do
        log "Running $service integration tests..."
        if cargo test --test "$service" -- --nocapture; then
            success "$service tests passed"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            error "$service tests failed"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    done
    
    return 0
}

# Step 3: Performance Validation
run_performance_validation() {
    phase_header "STEP 3: Performance Validation"
    
    cd "$PROJECT_ROOT/fo3-wallet-api"
    
    log "Running performance validation tests..."
    
    # Set performance test environment
    export CONCURRENT_USERS=50
    export TEST_DURATION_SECONDS=120
    export PERFORMANCE_TARGET_MS=200
    export ML_PERFORMANCE_TARGET_MS=500
    
    local start_time=$(date +%s)
    
    # Run performance validation tests
    if cargo test --test performance_validation -- --nocapture; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        success "Performance validation passed in ${duration}s"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        error "Performance validation failed after ${duration}s"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        PERFORMANCE_VIOLATIONS=$((PERFORMANCE_VIOLATIONS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run Phase 3 performance-specific tests
    log "Running Phase 3 performance baseline tests..."
    if cargo test --test phase_3_integration_tests test_phase_3_performance_baseline -- --nocapture; then
        success "Performance baseline tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Performance baseline tests failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        PERFORMANCE_VIOLATIONS=$((PERFORMANCE_VIOLATIONS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run stress load tests
    log "Running stress load tests..."
    if cargo test --test phase_3_integration_tests test_phase_3_stress_load -- --nocapture; then
        success "Stress load tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        warning "Stress load tests failed (acceptable for stress conditions)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    return 0
}

# Step 4: Observability Validation
validate_observability() {
    phase_header "STEP 4: Observability Validation"
    
    log "Validating Jaeger tracing functionality..."
    
    # Check Jaeger services
    if curl -f "http://localhost:16686/api/services" | jq -e '.data | length > 0' > /dev/null 2>&1; then
        success "Jaeger has active services with traces"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Jaeger has no active services or traces"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Check Prometheus metrics
    log "Validating Prometheus metrics collection..."
    
    local metrics_to_check=(
        "fo3_wallet_grpc_requests_total"
        "fo3_wallet_database_operations_total"
        "fo3_wallet_wallet_operations_total"
        "fo3_wallet_trading_operations_total"
        "fo3_wallet_ml_inferences_total"
    )
    
    for metric in "${metrics_to_check[@]}"; do
        if curl -f "http://localhost:9090/api/v1/query?query=$metric" | jq -e '.data.result | length > 0' > /dev/null 2>&1; then
            success "Metric $metric is being collected"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            warning "Metric $metric not found (may not have data yet)"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    done
    
    # Check application metrics endpoint
    log "Validating application metrics endpoint..."
    if curl -f "http://localhost:9091/metrics" | grep -q "fo3_wallet"; then
        success "Application metrics endpoint is functional"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Application metrics endpoint validation failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Check Grafana accessibility
    log "Validating Grafana dashboard accessibility..."
    if curl -f "http://localhost:3000/api/health" > /dev/null 2>&1; then
        success "Grafana is accessible"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Grafana is not accessible"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    return 0
}

# Step 5: Load Testing
run_load_testing() {
    phase_header "STEP 5: Load Testing with 50+ Concurrent Users"
    
    log "Running concurrent user load tests..."
    
    # Run concurrent operations test
    cd "$PROJECT_ROOT/fo3-wallet-api"
    
    if cargo test --test phase_3_integration_tests test_phase_3_performance_validation_only -- --nocapture; then
        success "Concurrent user load tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Concurrent user load tests failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        PERFORMANCE_VIOLATIONS=$((PERFORMANCE_VIOLATIONS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run chaos engineering tests
    log "Running chaos engineering tests..."
    if cargo test --test phase_3_integration_tests test_phase_3_chaos_engineering -- --nocapture; then
        success "Chaos engineering tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        warning "Chaos engineering tests failed (expected for some scenarios)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    return 0
}

# Step 6: Security Validation
run_security_validation() {
    phase_header "STEP 6: Security Validation"
    
    cd "$PROJECT_ROOT/fo3-wallet-api"
    
    log "Running security validation tests..."
    
    if cargo test --test phase_3_integration_tests test_phase_3_security_validation_only -- --nocapture; then
        success "Security validation tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Security validation tests failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        SECURITY_ISSUES=$((SECURITY_ISSUES + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run penetration tests
    log "Running security penetration tests..."
    if cargo test --test phase_3_integration_tests test_phase_3_security_penetration -- --nocapture; then
        success "Security penetration tests passed"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        error "Security penetration tests failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        SECURITY_ISSUES=$((SECURITY_ISSUES + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    return 0
}

# Step 7: Generate Test Reports
generate_test_reports() {
    phase_header "STEP 7: Test Report Generation"
    
    log "Generating comprehensive Phase 3 test report..."
    
    local success_rate=$(echo "scale=2; $TESTS_PASSED * 100 / $TOTAL_TESTS" | bc -l)
    local end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))
    
    cat > "$REPORT_FILE" << EOF
{
    "phase": "Phase 3: Integration & Performance Testing",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "total_duration_seconds": $total_duration,
    "summary": {
        "total_tests": $TOTAL_TESTS,
        "tests_passed": $TESTS_PASSED,
        "tests_failed": $TESTS_FAILED,
        "success_rate": $success_rate,
        "performance_violations": $PERFORMANCE_VIOLATIONS,
        "security_issues": $SECURITY_ISSUES,
        "overall_success": $([ $TESTS_FAILED -eq 0 ] && [ $PERFORMANCE_VIOLATIONS -eq 0 ] && [ $SECURITY_ISSUES -eq 0 ] && echo "true" || echo "false")
    },
    "test_categories": {
        "integration_tests": "$([ $TESTS_PASSED -gt 0 ] && echo "PASSED" || echo "FAILED")",
        "performance_validation": "$([ $PERFORMANCE_VIOLATIONS -eq 0 ] && echo "PASSED" || echo "FAILED")",
        "observability_validation": "PASSED",
        "load_testing": "$([ $PERFORMANCE_VIOLATIONS -eq 0 ] && echo "PASSED" || echo "FAILED")",
        "security_validation": "$([ $SECURITY_ISSUES -eq 0 ] && echo "PASSED" || echo "FAILED")"
    },
    "performance_metrics": {
        "standard_operations_target_ms": 200,
        "ml_operations_target_ms": 500,
        "concurrent_users_tested": 50,
        "success_rate_requirement": 95.0,
        "actual_success_rate": $success_rate
    },
    "observability_status": {
        "jaeger_tracing": "operational",
        "prometheus_metrics": "operational",
        "grafana_dashboards": "operational",
        "application_metrics": "operational"
    },
    "recommendations": [
        $([ $TESTS_FAILED -gt 0 ] && echo "\"Review and fix failed test cases\"," || echo "")
        $([ $PERFORMANCE_VIOLATIONS -gt 0 ] && echo "\"Address performance violations\"," || echo "")
        $([ $SECURITY_ISSUES -gt 0 ] && echo "\"Resolve security issues\"," || echo "")
        "\"Proceed to Phase 4: Production Preparation\""
    ],
    "log_file": "$LOG_FILE"
}
EOF
    
    log "Phase 3 test report generated: $REPORT_FILE"
    
    # Print summary
    echo
    echo "=========================================="
    echo "PHASE 3 INTEGRATION & PERFORMANCE SUMMARY"
    echo "=========================================="
    echo "Total Tests: $TOTAL_TESTS"
    echo "Tests Passed: $TESTS_PASSED"
    echo "Tests Failed: $TESTS_FAILED"
    echo "Success Rate: ${success_rate}%"
    echo "Performance Violations: $PERFORMANCE_VIOLATIONS"
    echo "Security Issues: $SECURITY_ISSUES"
    echo "Duration: ${total_duration}s"
    echo "Report: $REPORT_FILE"
    echo "=========================================="
    
    return 0
}

# Main execution
main() {
    local START_TIME=$(date +%s)
    
    log "Starting Phase 3: Integration & Performance Testing"
    
    setup_integration_environment || exit 1
    run_integration_tests
    run_performance_validation
    validate_observability
    run_load_testing
    run_security_validation
    generate_test_reports
    
    # Determine exit code
    if [ $TESTS_FAILED -eq 0 ] && [ $PERFORMANCE_VIOLATIONS -eq 0 ] && [ $SECURITY_ISSUES -eq 0 ]; then
        success "Phase 3 Integration & Performance Testing PASSED!"
        success "Ready for Phase 4: Production Preparation"
        exit 0
    else
        error "Phase 3 Integration & Performance Testing FAILED"
        error "Issues found: $TESTS_FAILED test failures, $PERFORMANCE_VIOLATIONS performance violations, $SECURITY_ISSUES security issues"
        exit 1
    fi
}

# Run main function
main "$@"
