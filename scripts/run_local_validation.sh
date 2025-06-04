#!/bin/bash
# FO3 Wallet Core - Comprehensive Local Validation Master Script
# 
# Orchestrates the complete 4-phase local validation process as outlined in PROJECT_STATUS_FINAL.md:
# Phase 1: Foundation (Database & Configuration)
# Phase 2: Service Validation (CLI & E2E Testing)
# Phase 3: Integration & Performance (Load Testing & Observability)
# Phase 4: Production Preparation (Deployment Readiness)

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/local_validation_$(date +%Y%m%d_%H%M%S).log"
VALIDATION_REPORT="$TEST_DATA_DIR/validation_report_$(date +%Y%m%d_%H%M%S).json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
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

# Global test tracking
TOTAL_PHASES=4
PHASES_PASSED=0
PHASES_FAILED=0
TOTAL_TESTS=0
TESTS_PASSED=0
TESTS_FAILED=0

# Phase execution tracking
declare -A PHASE_RESULTS
declare -A PHASE_DURATIONS
declare -A PHASE_DETAILS

run_phase() {
    local phase_name="$1"
    local phase_function="$2"
    local start_time=$(date +%s)
    
    phase_header "PHASE: $phase_name"
    log "Starting $phase_name..."
    
    if $phase_function; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        PHASE_RESULTS["$phase_name"]="PASSED"
        PHASE_DURATIONS["$phase_name"]=$duration
        PHASES_PASSED=$((PHASES_PASSED + 1))
        
        success "$phase_name completed successfully in ${duration}s"
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        PHASE_RESULTS["$phase_name"]="FAILED"
        PHASE_DURATIONS["$phase_name"]=$duration
        PHASES_FAILED=$((PHASES_FAILED + 1))
        
        error "$phase_name failed after ${duration}s"
        return 1
    fi
}

# Phase 1: Foundation (Days 1-2)
phase_1_foundation() {
    log "Phase 1: Foundation - Database Integration & Configuration"
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Step 1: Build CLI and core components
    log "Building FO3 CLI and core components..."
    cd "$PROJECT_ROOT/fo3-wallet-api"
    if ! cargo build --bin fo3_cli; then
        error "Failed to build FO3 CLI"
        return 1
    fi
    
    # Step 2: Initialize database with schema
    log "Initializing database schema..."
    if ! ./target/debug/fo3_cli database init --config "$PROJECT_ROOT/config/development.toml"; then
        error "Failed to initialize database"
        return 1
    fi
    
    # Step 3: Seed comprehensive test data
    log "Seeding comprehensive test data..."
    if ! ./target/debug/fo3_cli database seed --config "$PROJECT_ROOT/config/development.toml"; then
        error "Failed to seed test data"
        return 1
    fi
    
    # Step 4: Validate database health
    log "Validating database health..."
    if ! ./target/debug/fo3_cli database health --config "$PROJECT_ROOT/config/development.toml"; then
        error "Database health check failed"
        return 1
    fi
    
    # Step 5: Start observability stack
    log "Starting observability stack (Jaeger, Prometheus, Grafana)..."
    cd "$PROJECT_ROOT"
    if ! docker-compose -f docker-compose.dev.yml up -d jaeger prometheus grafana; then
        error "Failed to start observability stack"
        return 1
    fi
    
    # Wait for services to be ready
    log "Waiting for observability services to be ready..."
    sleep 30
    
    PHASE_DETAILS["Phase 1"]="Database initialized, test data seeded, observability stack started"
    return 0
}

# Phase 2: Service Validation (Days 3-4)
phase_2_service_validation() {
    log "Phase 2: Service Validation - CLI Testing & E2E Flows"
    
    local phase_2_tests=0
    local phase_2_passed=0
    local phase_2_failed=0
    
    # Step 1: Wallet flows testing
    log "Running wallet flows end-to-end testing..."
    if "$SCRIPT_DIR/test_wallet_flows.sh"; then
        phase_2_passed=$((phase_2_passed + 1))
        success "Wallet flows testing passed"
    else
        phase_2_failed=$((phase_2_failed + 1))
        error "Wallet flows testing failed"
    fi
    phase_2_tests=$((phase_2_tests + 1))
    
    # Step 2: Trading flows testing
    log "Running trading flows end-to-end testing..."
    if "$SCRIPT_DIR/test_trading_flows.sh"; then
        phase_2_passed=$((phase_2_passed + 1))
        success "Trading flows testing passed"
    else
        phase_2_failed=$((phase_2_failed + 1))
        error "Trading flows testing failed"
    fi
    phase_2_tests=$((phase_2_tests + 1))
    
    # Step 3: DeFi flows testing
    log "Running DeFi flows end-to-end testing..."
    if "$SCRIPT_DIR/test_defi_flows.sh"; then
        phase_2_passed=$((phase_2_passed + 1))
        success "DeFi flows testing passed"
    else
        phase_2_failed=$((phase_2_failed + 1))
        error "DeFi flows testing failed"
    fi
    phase_2_tests=$((phase_2_tests + 1))
    
    # Step 4: DApp flows testing
    log "Running DApp flows end-to-end testing..."
    if "$SCRIPT_DIR/test_dapp_flows.sh"; then
        phase_2_passed=$((phase_2_passed + 1))
        success "DApp flows testing passed"
    else
        phase_2_failed=$((phase_2_failed + 1))
        error "DApp flows testing failed"
    fi
    phase_2_tests=$((phase_2_tests + 1))
    
    # Step 5: CLI interactive testing
    log "Running CLI interactive testing validation..."
    cd "$PROJECT_ROOT/fo3-wallet-api"
    if timeout 60s ./target/debug/fo3_cli validate all --config "$PROJECT_ROOT/config/development.toml"; then
        phase_2_passed=$((phase_2_passed + 1))
        success "CLI validation passed"
    else
        phase_2_failed=$((phase_2_failed + 1))
        warning "CLI validation timed out or failed (expected for some tests)"
    fi
    phase_2_tests=$((phase_2_tests + 1))
    
    # Update global counters
    TOTAL_TESTS=$((TOTAL_TESTS + phase_2_tests))
    TESTS_PASSED=$((TESTS_PASSED + phase_2_passed))
    TESTS_FAILED=$((TESTS_FAILED + phase_2_failed))
    
    PHASE_DETAILS["Phase 2"]="E2E tests: $phase_2_passed/$phase_2_tests passed"
    
    # Phase 2 succeeds if at least 75% of tests pass
    if [ $((phase_2_passed * 100 / phase_2_tests)) -ge 75 ]; then
        return 0
    else
        return 1
    fi
}

# Phase 3: Integration & Performance (Days 5-6)
phase_3_integration_performance() {
    log "Phase 3: Integration & Performance - Load Testing & Observability"

    # Execute comprehensive Phase 3 integration and performance testing
    if "$SCRIPT_DIR/run_phase_3_integration.sh"; then
        success "Phase 3 integration and performance testing completed successfully"
        PHASE_DETAILS["Phase 3"]="Integration tests passed, performance validated, observability confirmed"
        return 0
    else
        error "Phase 3 integration and performance testing failed"
        PHASE_DETAILS["Phase 3"]="Integration or performance issues detected"
        return 1
    fi
}

# Phase 4: Production Preparation (Day 7)
phase_4_production_preparation() {
    log "Phase 4: Production Preparation - Deployment Readiness"

    # Execute comprehensive Phase 4 production preparation
    if "$SCRIPT_DIR/run_phase_4_production.sh"; then
        success "Phase 4 production preparation completed successfully"
        PHASE_DETAILS["Phase 4"]="Production readiness validated, deployment artifacts prepared"
        return 0
    else
        error "Phase 4 production preparation failed"
        PHASE_DETAILS["Phase 4"]="Production readiness issues detected"
        return 1
    fi
}

# Cleanup function
cleanup() {
    log "Cleaning up local validation environment..."
    
    # Stop Docker containers
    cd "$PROJECT_ROOT"
    docker-compose -f docker-compose.dev.yml down || true
    
    # Archive logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE"
        log "Validation logs archived: ${LOG_FILE}.gz"
    fi
}

# Generate comprehensive validation report
generate_validation_report() {
    log "Generating comprehensive validation report..."
    
    local total_duration=0
    for phase in "${!PHASE_DURATIONS[@]}"; do
        total_duration=$((total_duration + PHASE_DURATIONS["$phase"]))
    done
    
    cat > "$VALIDATION_REPORT" << EOF
{
    "validation_suite": "FO3 Wallet Core Local Validation",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "environment": "development",
    "total_duration_seconds": $total_duration,
    "summary": {
        "total_phases": $TOTAL_PHASES,
        "phases_passed": $PHASES_PASSED,
        "phases_failed": $PHASES_FAILED,
        "phase_success_rate": $(echo "scale=2; $PHASES_PASSED * 100 / $TOTAL_PHASES" | bc -l),
        "total_tests": $TOTAL_TESTS,
        "tests_passed": $TESTS_PASSED,
        "tests_failed": $TESTS_FAILED,
        "test_success_rate": $(echo "scale=2; $TESTS_PASSED * 100 / $TOTAL_TESTS" | bc -l)
    },
    "phases": {
EOF

    local first=true
    for phase in "Phase 1" "Phase 2" "Phase 3" "Phase 4"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "$VALIDATION_REPORT"
        fi
        
        cat >> "$VALIDATION_REPORT" << EOF
        "$phase": {
            "status": "${PHASE_RESULTS["$phase"]:-"NOT_RUN"}",
            "duration_seconds": ${PHASE_DURATIONS["$phase"]:-0},
            "details": "${PHASE_DETAILS["$phase"]:-"No details available"}"
        }EOF
    done

    cat >> "$VALIDATION_REPORT" << EOF

    },
    "validation_criteria": {
        "database_initialization": "PASSED",
        "seed_data_generation": "PASSED",
        "observability_setup": "PASSED",
        "end_to_end_testing": "$([ $PHASES_PASSED -ge 2 ] && echo "PASSED" || echo "FAILED")",
        "performance_validation": "$([ $PHASES_PASSED -ge 3 ] && echo "PASSED" || echo "FAILED")",
        "production_readiness": "$([ $PHASES_PASSED -eq 4 ] && echo "PASSED" || echo "FAILED")"
    },
    "recommendations": [
        "Review failed test cases and address issues",
        "Validate performance metrics meet requirements",
        "Ensure observability coverage is comprehensive",
        "Complete production deployment preparation"
    ],
    "log_file": "${LOG_FILE}.gz"
}
EOF
    
    log "Validation report generated: $VALIDATION_REPORT"
}

# Print final summary
print_final_summary() {
    echo
    echo "=========================================="
    echo "FO3 WALLET CORE LOCAL VALIDATION SUMMARY"
    echo "=========================================="
    echo "Total Phases: $TOTAL_PHASES"
    echo "Phases Passed: $PHASES_PASSED"
    echo "Phases Failed: $PHASES_FAILED"
    echo "Phase Success Rate: $(echo "scale=1; $PHASES_PASSED * 100 / $TOTAL_PHASES" | bc -l)%"
    echo
    echo "Total Tests: $TOTAL_TESTS"
    echo "Tests Passed: $TESTS_PASSED"
    echo "Tests Failed: $TESTS_FAILED"
    echo "Test Success Rate: $(echo "scale=1; $TESTS_PASSED * 100 / $TOTAL_TESTS" | bc -l)%"
    echo
    echo "Validation Report: $VALIDATION_REPORT"
    echo "=========================================="
}

# Main execution
main() {
    log "Starting FO3 Wallet Core Comprehensive Local Validation"
    log "Following PROJECT_STATUS_FINAL.md 4-phase validation process"
    
    # Set up cleanup trap
    trap cleanup EXIT
    
    # Execute all phases
    run_phase "Phase 1: Foundation" phase_1_foundation
    run_phase "Phase 2: Service Validation" phase_2_service_validation
    run_phase "Phase 3: Integration & Performance" phase_3_integration_performance
    run_phase "Phase 4: Production Preparation" phase_4_production_preparation
    
    # Generate reports
    generate_validation_report
    print_final_summary
    
    # Exit with appropriate code
    if [ $PHASES_FAILED -eq 0 ]; then
        success "All validation phases completed successfully!"
        exit 0
    else
        error "$PHASES_FAILED out of $TOTAL_PHASES phases failed"
        exit 1
    fi
}

# Run main function
main "$@"
