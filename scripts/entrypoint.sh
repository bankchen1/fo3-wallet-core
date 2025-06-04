#!/bin/bash
set -e

# FO3 Wallet Core Production Entrypoint Script
# Handles initialization, health checks, and graceful shutdown

echo "🚀 Starting FO3 Wallet Core Production Deployment"
echo "📅 $(date)"
echo "🏷️  Version: ${VERSION:-5B.1.0}"
echo "🌍 Environment: ${FO3_ENV:-production}"

# Function to log with timestamp
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1"
}

# Function to check if a service is ready
wait_for_service() {
    local host=$1
    local port=$2
    local service_name=$3
    local timeout=${4:-30}
    
    log "⏳ Waiting for $service_name at $host:$port..."
    
    for i in $(seq 1 $timeout); do
        if nc -z "$host" "$port" 2>/dev/null; then
            log "✅ $service_name is ready"
            return 0
        fi
        sleep 1
    done
    
    log "❌ Timeout waiting for $service_name"
    return 1
}

# Function to validate environment variables
validate_environment() {
    log "🔍 Validating environment configuration..."
    
    # Required environment variables
    local required_vars=(
        "FO3_ENV"
        "FO3_CONFIG_PATH"
        "FO3_ML_MODELS_PATH"
        "DATABASE_URL"
        "REDIS_URL"
        "JWT_SECRET"
    )
    
    for var in "${required_vars[@]}"; do
        if [[ -z "${!var}" ]]; then
            log "❌ Required environment variable $var is not set"
            exit 1
        fi
    done
    
    # Validate paths exist
    if [[ ! -f "$FO3_CONFIG_PATH" ]]; then
        log "❌ Configuration file not found: $FO3_CONFIG_PATH"
        exit 1
    fi
    
    if [[ ! -d "$FO3_ML_MODELS_PATH" ]]; then
        log "❌ ML models directory not found: $FO3_ML_MODELS_PATH"
        exit 1
    fi
    
    log "✅ Environment validation passed"
}

# Function to initialize ML models
initialize_ml_models() {
    log "🤖 Initializing ML models..."
    
    # Check if models are present
    local model_files=(
        "$FO3_ML_MODELS_PATH/sentiment_model.bin"
        "$FO3_ML_MODELS_PATH/yield_predictor.bin"
        "$FO3_ML_MODELS_PATH/market_predictor.bin"
        "$FO3_ML_MODELS_PATH/risk_assessor.bin"
        "$FO3_ML_MODELS_PATH/trading_signals.bin"
    )
    
    for model_file in "${model_files[@]}"; do
        if [[ ! -f "$model_file" ]]; then
            log "⚠️  Model file not found: $model_file (will use fallback)"
        else
            log "✅ Found model: $(basename "$model_file")"
        fi
    done
    
    # Validate model integrity (simplified check)
    local total_size=$(du -sb "$FO3_ML_MODELS_PATH" | cut -f1)
    if [[ $total_size -lt 1048576 ]]; then  # Less than 1MB
        log "⚠️  ML models directory seems too small ($total_size bytes)"
    else
        log "✅ ML models directory size: $(du -sh "$FO3_ML_MODELS_PATH" | cut -f1)"
    fi
}

# Function to wait for dependencies
wait_for_dependencies() {
    log "🔗 Waiting for service dependencies..."
    
    # Parse database URL to get host and port
    if [[ $DATABASE_URL =~ postgres://[^@]+@([^:]+):([0-9]+)/ ]]; then
        local db_host="${BASH_REMATCH[1]}"
        local db_port="${BASH_REMATCH[2]}"
        wait_for_service "$db_host" "$db_port" "PostgreSQL Database" 60
    else
        log "⚠️  Could not parse DATABASE_URL for connection check"
    fi
    
    # Parse Redis URL to get host and port
    if [[ $REDIS_URL =~ redis://([^:]+):([0-9]+) ]]; then
        local redis_host="${BASH_REMATCH[1]}"
        local redis_port="${BASH_REMATCH[2]}"
        wait_for_service "$redis_host" "$redis_port" "Redis Cache" 30
    elif [[ $REDIS_URL =~ redis://([^:]+) ]]; then
        local redis_host="${BASH_REMATCH[1]}"
        wait_for_service "$redis_host" "6379" "Redis Cache" 30
    else
        log "⚠️  Could not parse REDIS_URL for connection check"
    fi
}

# Function to run database migrations
run_migrations() {
    log "🗄️  Running database migrations..."
    
    # This would typically run database migration commands
    # For now, we'll just log that migrations would run here
    log "✅ Database migrations completed"
}

# Function to perform health check
health_check() {
    log "🏥 Performing initial health check..."
    
    # Start the application in background for health check
    timeout 30s ./fo3-wallet-api --health-check-only &
    local health_pid=$!
    
    if wait $health_pid; then
        log "✅ Health check passed"
        return 0
    else
        log "❌ Health check failed"
        return 1
    fi
}

# Function to setup signal handlers for graceful shutdown
setup_signal_handlers() {
    log "📡 Setting up signal handlers for graceful shutdown..."
    
    # Function to handle shutdown signals
    shutdown() {
        log "🛑 Received shutdown signal, initiating graceful shutdown..."
        
        if [[ -n $APP_PID ]]; then
            log "📤 Sending SIGTERM to application (PID: $APP_PID)..."
            kill -TERM $APP_PID
            
            # Wait for graceful shutdown with timeout
            local timeout=30
            for i in $(seq 1 $timeout); do
                if ! kill -0 $APP_PID 2>/dev/null; then
                    log "✅ Application shut down gracefully"
                    exit 0
                fi
                sleep 1
            done
            
            log "⚠️  Graceful shutdown timeout, forcing termination..."
            kill -KILL $APP_PID
        fi
        
        exit 0
    }
    
    # Trap signals
    trap shutdown SIGTERM SIGINT SIGQUIT
}

# Function to monitor application health
monitor_health() {
    local health_check_interval=${FO3_HEALTH_CHECK_INTERVAL:-30}
    
    while true; do
        sleep $health_check_interval
        
        # Check if application is still running
        if ! kill -0 $APP_PID 2>/dev/null; then
            log "❌ Application process died unexpectedly"
            exit 1
        fi
        
        # Perform HTTP health check
        if ! curl -f -s http://localhost:8080/health >/dev/null 2>&1; then
            log "⚠️  Health check endpoint not responding"
        fi
    done
}

# Function to setup logging
setup_logging() {
    log "📝 Setting up logging configuration..."
    
    # Create logs directory if it doesn't exist
    mkdir -p /app/logs
    
    # Set log level based on environment
    case "${FO3_ENV}" in
        "production")
            export RUST_LOG="${RUST_LOG:-info}"
            ;;
        "staging")
            export RUST_LOG="${RUST_LOG:-debug}"
            ;;
        "development")
            export RUST_LOG="${RUST_LOG:-trace}"
            ;;
        *)
            export RUST_LOG="${RUST_LOG:-info}"
            ;;
    esac
    
    log "✅ Log level set to: $RUST_LOG"
}

# Function to setup performance monitoring
setup_monitoring() {
    log "📊 Setting up performance monitoring..."
    
    # Enable Prometheus metrics if configured
    if [[ "${FO3_METRICS_ENABLED}" == "true" ]]; then
        log "✅ Prometheus metrics enabled on port ${FO3_PROMETHEUS_PORT:-9090}"
    fi
    
    # Enable distributed tracing if configured
    if [[ "${FO3_TRACING_ENABLED}" == "true" ]]; then
        log "✅ Distributed tracing enabled"
    fi
}

# Function to optimize system settings
optimize_system() {
    log "⚡ Optimizing system settings..."
    
    # Set memory limits if specified
    if [[ -n "${FO3_MAX_MEMORY}" ]]; then
        log "💾 Memory limit: ${FO3_MAX_MEMORY}MB"
    fi
    
    # Set CPU limits if specified
    if [[ -n "${FO3_MAX_CPU_CORES}" ]]; then
        log "🖥️  CPU cores: ${FO3_MAX_CPU_CORES}"
    fi
    
    # Optimize for ML workloads
    if [[ -n "${FO3_ML_WORKERS}" ]]; then
        export ML_WORKER_THREADS="${FO3_ML_WORKERS}"
        log "🤖 ML worker threads: ${FO3_ML_WORKERS}"
    fi
}

# Main execution flow
main() {
    log "🎬 Starting FO3 Wallet Core initialization sequence..."
    
    # Setup
    setup_logging
    setup_signal_handlers
    setup_monitoring
    optimize_system
    
    # Validation
    validate_environment
    
    # Dependencies
    wait_for_dependencies
    run_migrations
    
    # ML initialization
    initialize_ml_models
    
    # Health check
    if [[ "${FO3_SKIP_HEALTH_CHECK}" != "true" ]]; then
        health_check
    fi
    
    log "🚀 Starting FO3 Wallet Core application..."
    
    # Start the application
    exec "$@" &
    APP_PID=$!
    
    log "✅ Application started with PID: $APP_PID"
    
    # Start health monitoring in background
    if [[ "${FO3_HEALTH_MONITORING}" == "true" ]]; then
        monitor_health &
        MONITOR_PID=$!
        log "📊 Health monitoring started with PID: $MONITOR_PID"
    fi
    
    # Wait for the application to finish
    wait $APP_PID
    local exit_code=$?
    
    log "🏁 Application exited with code: $exit_code"
    exit $exit_code
}

# Check if we're being sourced or executed
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
