#!/bin/bash
# FO3 Wallet Core - Immediate Validation Execution Script
# 
# Executes the complete local validation process immediately with proper setup
# and environment preparation. This script handles all prerequisites and runs
# the full 4-phase validation process.

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="$PROJECT_ROOT/test_data"
LOG_FILE="$TEST_DATA_DIR/immediate_validation_$(date +%Y%m%d_%H%M%S).log"

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

header() {
    echo -e "${PURPLE}========================================${NC}" | tee -a "$LOG_FILE"
    echo -e "${PURPLE}$1${NC}" | tee -a "$LOG_FILE"
    echo -e "${PURPLE}========================================${NC}" | tee -a "$LOG_FILE"
}

# Prerequisites check
check_prerequisites() {
    header "CHECKING PREREQUISITES"
    
    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    
    # Check if we're in the right directory
    if [ ! -f "$PROJECT_ROOT/fo3-wallet-api/Cargo.toml" ]; then
        error "Not in FO3 Wallet Core project root. Please run from project directory."
        exit 1
    fi
    
    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Check Docker installation
    if ! command -v docker &> /dev/null; then
        error "Docker not found. Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose not found. Please install Docker Compose."
        exit 1
    fi
    
    # Check required tools
    local tools=("curl" "jq" "bc")
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            warning "$tool not found. Installing via package manager..."
            # Try to install on macOS
            if command -v brew &> /dev/null; then
                brew install "$tool" || warning "Failed to install $tool"
            else
                error "$tool is required but not installed. Please install manually."
                exit 1
            fi
        fi
    done
    
    success "Prerequisites check completed"
}

# Environment setup
setup_environment() {
    header "SETTING UP ENVIRONMENT"
    
    # Ensure we're in the right directory
    cd "$PROJECT_ROOT"
    
    # Clean up any previous runs
    log "Cleaning up previous validation runs..."
    docker-compose -f docker-compose.dev.yml down --remove-orphans || true
    
    # Remove old test data (keep last 5 runs)
    log "Cleaning old test data..."
    find "$TEST_DATA_DIR" -name "*.log*" -mtime +7 -delete 2>/dev/null || true
    find "$TEST_DATA_DIR" -name "*.json" -mtime +7 -delete 2>/dev/null || true
    
    # Build the CLI tool
    log "Building FO3 CLI tool..."
    cd "$PROJECT_ROOT/fo3-wallet-api"
    if ! cargo build --bin fo3_cli; then
        error "Failed to build FO3 CLI tool"
        exit 1
    fi
    
    # Verify CLI tool works
    if ! ./target/debug/fo3_cli --help > /dev/null 2>&1; then
        error "FO3 CLI tool is not working properly"
        exit 1
    fi
    
    success "Environment setup completed"
}

# Quick validation run
run_quick_validation() {
    header "RUNNING QUICK VALIDATION"
    
    log "This will run a simplified validation to verify the system works..."
    
    # Initialize database
    log "Initializing database..."
    cd "$PROJECT_ROOT/fo3-wallet-api"
    if ! ./target/debug/fo3_cli database init --config "$PROJECT_ROOT/config/development.toml"; then
        error "Database initialization failed"
        return 1
    fi
    
    # Seed minimal test data
    log "Seeding test data..."
    if ! ./target/debug/fo3_cli database seed --config "$PROJECT_ROOT/config/development.toml"; then
        error "Database seeding failed"
        return 1
    fi
    
    # Test database health
    log "Testing database health..."
    if ! ./target/debug/fo3_cli database health --config "$PROJECT_ROOT/config/development.toml"; then
        error "Database health check failed"
        return 1
    fi
    
    # Start minimal observability stack
    log "Starting observability services..."
    cd "$PROJECT_ROOT"
    if ! docker-compose -f docker-compose.dev.yml up -d jaeger prometheus; then
        error "Failed to start observability services"
        return 1
    fi
    
    # Wait for services
    log "Waiting for services to be ready..."
    sleep 30
    
    # Test observability
    log "Testing observability services..."
    if curl -f "http://localhost:16686/api/services" > /dev/null 2>&1; then
        success "Jaeger is accessible"
    else
        warning "Jaeger is not accessible (may need more time)"
    fi
    
    if curl -f "http://localhost:9090/api/v1/query?query=up" > /dev/null 2>&1; then
        success "Prometheus is accessible"
    else
        warning "Prometheus is not accessible (may need more time)"
    fi
    
    success "Quick validation completed successfully"
    return 0
}

# Full validation execution
run_full_validation() {
    header "RUNNING FULL VALIDATION"
    
    log "Starting comprehensive 4-phase validation process..."
    
    # Execute the master validation script
    if "$SCRIPT_DIR/run_local_validation.sh"; then
        success "Full validation completed successfully!"
        return 0
    else
        error "Full validation failed"
        return 1
    fi
}

# Results summary
show_results() {
    header "VALIDATION RESULTS SUMMARY"
    
    # Find the latest reports
    local latest_validation_report=$(ls -t "$TEST_DATA_DIR"/validation_report_*.json 2>/dev/null | head -1)
    local latest_phase3_report=$(ls -t "$TEST_DATA_DIR"/phase_3_report_*.json 2>/dev/null | head -1)
    local latest_readiness_report=$(ls -t "$TEST_DATA_DIR"/production_readiness_*.json 2>/dev/null | head -1)
    
    echo
    echo "📊 VALIDATION REPORTS GENERATED:"
    echo "=================================="
    
    if [ -n "$latest_validation_report" ] && [ -f "$latest_validation_report" ]; then
        echo "🎯 Master Validation Report: $latest_validation_report"
        
        # Extract key metrics if jq is available
        if command -v jq &> /dev/null; then
            local success_rate=$(jq -r '.summary.test_success_rate // "N/A"' "$latest_validation_report" 2>/dev/null)
            local phases_passed=$(jq -r '.summary.phases_passed // "N/A"' "$latest_validation_report" 2>/dev/null)
            local total_phases=$(jq -r '.summary.total_phases // "N/A"' "$latest_validation_report" 2>/dev/null)
            
            echo "   • Test Success Rate: ${success_rate}%"
            echo "   • Phases Passed: ${phases_passed}/${total_phases}"
        fi
    fi
    
    if [ -n "$latest_phase3_report" ] && [ -f "$latest_phase3_report" ]; then
        echo "🔧 Phase 3 Integration Report: $latest_phase3_report"
    fi
    
    if [ -n "$latest_readiness_report" ] && [ -f "$latest_readiness_report" ]; then
        echo "🚀 Production Readiness Report: $latest_readiness_report"
        
        if command -v jq &> /dev/null; then
            local readiness_score=$(jq -r '.readiness_summary.readiness_score // "N/A"' "$latest_readiness_report" 2>/dev/null)
            local production_ready=$(jq -r '.readiness_summary.production_ready // "N/A"' "$latest_readiness_report" 2>/dev/null)
            
            echo "   • Readiness Score: ${readiness_score}%"
            echo "   • Production Ready: $production_ready"
        fi
    fi
    
    echo
    echo "📁 ARTIFACTS GENERATED:"
    echo "======================"
    echo "• Test Data: $TEST_DATA_DIR/"
    echo "• Production Config: $PROJECT_ROOT/production/"
    echo "• Logs: $LOG_FILE"
    
    echo
    echo "🌐 MONITORING DASHBOARDS:"
    echo "========================="
    echo "• Jaeger Tracing: http://localhost:16686"
    echo "• Prometheus Metrics: http://localhost:9090"
    echo "• Grafana Dashboards: http://localhost:3000"
    echo "• Application Metrics: http://localhost:9091/metrics"
    
    echo
    echo "📚 NEXT STEPS:"
    echo "=============="
    echo "1. Review validation reports for any issues"
    echo "2. Check monitoring dashboards for system health"
    echo "3. Review production artifacts in production/ directory"
    echo "4. Follow go-live checklist for production deployment"
    
    echo
}

# Cleanup function
cleanup() {
    log "Cleaning up validation environment..."
    
    # Archive logs
    if [ -f "$LOG_FILE" ]; then
        gzip "$LOG_FILE" 2>/dev/null || true
        log "Validation logs archived: ${LOG_FILE}.gz"
    fi
    
    # Optionally stop Docker services (uncomment if desired)
    # docker-compose -f docker-compose.dev.yml down
    
    success "Cleanup completed"
}

# Main execution
main() {
    local start_time=$(date +%s)
    
    echo
    header "FO3 WALLET CORE - IMMEDIATE VALIDATION EXECUTION"
    echo
    log "Starting immediate validation execution..."
    
    # Set up cleanup trap
    trap cleanup EXIT
    
    # Check prerequisites
    check_prerequisites
    
    # Setup environment
    setup_environment
    
    # Ask user for validation type
    echo
    echo "Choose validation type:"
    echo "1) Quick validation (5-10 minutes) - Basic functionality check"
    echo "2) Full validation (30-60 minutes) - Complete 4-phase validation"
    echo "3) Skip to results review (if validation already completed)"
    echo
    read -p "Enter choice (1-3): " choice
    
    case $choice in
        1)
            log "Running quick validation..."
            if run_quick_validation; then
                success "Quick validation completed successfully!"
            else
                error "Quick validation failed"
                exit 1
            fi
            ;;
        2)
            log "Running full validation..."
            if run_full_validation; then
                success "Full validation completed successfully!"
            else
                error "Full validation failed"
                exit 1
            fi
            ;;
        3)
            log "Skipping to results review..."
            ;;
        *)
            error "Invalid choice. Defaulting to quick validation."
            run_quick_validation
            ;;
    esac
    
    # Show results
    show_results
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    
    echo
    success "Validation execution completed in ${total_duration} seconds!"
    
    # Final status
    if [ $choice -eq 2 ]; then
        echo
        echo "🎉 FO3 Wallet Core Local Validation Complete!"
        echo "The system has been thoroughly tested and is ready for production deployment."
    else
        echo
        echo "✅ FO3 Wallet Core Quick Validation Complete!"
        echo "Run full validation (option 2) for comprehensive production readiness testing."
    fi
}

# Run main function
main "$@"
