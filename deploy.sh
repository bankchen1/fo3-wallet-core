#!/bin/bash

# FO3 Wallet Core Deployment Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Docker is installed
check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        print_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    print_success "Docker and Docker Compose are installed"
}

# Check if .env file exists
check_env_file() {
    if [ ! -f ".env" ]; then
        print_warning ".env file not found. Creating from .env.example..."
        cp .env.example .env
        print_warning "Please edit .env file with your configuration before running again."
        exit 1
    fi
    print_success ".env file found"
}

# Build the Docker images
build_images() {
    print_status "Building Docker images..."
    docker-compose build --no-cache
    print_success "Docker images built successfully"
}

# Start the services
start_services() {
    print_status "Starting services..."
    docker-compose up -d
    print_success "Services started successfully"
}

# Stop the services
stop_services() {
    print_status "Stopping services..."
    docker-compose down
    print_success "Services stopped successfully"
}

# Show service status
show_status() {
    print_status "Service status:"
    docker-compose ps
}

# Show logs
show_logs() {
    local service=${1:-fo3-wallet-api}
    print_status "Showing logs for $service..."
    docker-compose logs -f "$service"
}

# Health check
health_check() {
    print_status "Performing health check..."
    
    # Check if gRPC service is responding
    if command -v grpc_health_probe &> /dev/null; then
        if grpc_health_probe -addr=localhost:50051; then
            print_success "gRPC service is healthy"
        else
            print_error "gRPC service health check failed"
            return 1
        fi
    else
        print_warning "grpc_health_probe not found. Skipping gRPC health check."
    fi
    
    # Check if database is responding
    if docker-compose exec postgres pg_isready -U fo3_user -d fo3_wallet; then
        print_success "Database is healthy"
    else
        print_error "Database health check failed"
        return 1
    fi
    
    # Check if Redis is responding
    if docker-compose exec redis redis-cli ping | grep -q PONG; then
        print_success "Redis is healthy"
    else
        print_error "Redis health check failed"
        return 1
    fi
}

# Run tests
run_tests() {
    print_status "Running tests..."
    
    # Build test image
    docker build -t fo3-wallet-test --target builder .
    
    # Run tests
    docker run --rm fo3-wallet-test cargo test --all-features
    
    print_success "Tests completed"
}

# Backup database
backup_database() {
    local backup_file="backup_$(date +%Y%m%d_%H%M%S).sql"
    print_status "Creating database backup: $backup_file"
    
    docker-compose exec postgres pg_dump -U fo3_user fo3_wallet > "$backup_file"
    
    print_success "Database backup created: $backup_file"
}

# Restore database
restore_database() {
    local backup_file=$1
    if [ -z "$backup_file" ]; then
        print_error "Please specify backup file to restore"
        exit 1
    fi
    
    if [ ! -f "$backup_file" ]; then
        print_error "Backup file not found: $backup_file"
        exit 1
    fi
    
    print_status "Restoring database from: $backup_file"
    
    docker-compose exec -T postgres psql -U fo3_user fo3_wallet < "$backup_file"
    
    print_success "Database restored from: $backup_file"
}

# Generate TLS certificates
generate_certificates() {
    print_status "Generating TLS certificates..."

    # Create certs directory
    mkdir -p certs

    # Generate self-signed certificate if it doesn't exist
    if [ ! -f "certs/server.crt" ] || [ ! -f "certs/server.key" ]; then
        openssl req -x509 -newkey rsa:4096 -keyout certs/server.key -out certs/server.crt \
            -days 365 -nodes -subj "/C=US/ST=CA/L=San Francisco/O=FO3/CN=localhost" \
            -addext "subjectAltName=DNS:localhost,DNS:fo3-wallet-api,IP:127.0.0.1"

        print_success "TLS certificates generated"
    else
        print_success "TLS certificates already exist"
    fi
}

# Security check
security_check() {
    print_status "Performing security checks..."

    # Check if TLS is enabled
    if [ -f "certs/server.crt" ]; then
        print_success "TLS certificate found"
    else
        print_warning "TLS certificate not found"
    fi

    # Check if JWT secret is set
    if grep -q "JWT_SECRET=your_jwt_secret" .env 2>/dev/null; then
        print_warning "Default JWT secret detected - please change in production"
    else
        print_success "JWT secret configured"
    fi

    # Check if strong passwords are set
    if grep -q "fo3_secure_password" .env 2>/dev/null; then
        print_warning "Default database password detected - please change in production"
    else
        print_success "Database password configured"
    fi

    # Test gRPC with TLS if enabled
    if command -v grpcurl &> /dev/null; then
        if grpcurl -insecure localhost:50051 list > /dev/null 2>&1; then
            print_success "gRPC service accessible"
        else
            print_warning "gRPC service not accessible"
        fi
    fi

    print_success "Security check completed"
}

# Clean up
cleanup() {
    print_status "Cleaning up..."
    docker-compose down -v
    docker system prune -f
    print_success "Cleanup completed"
}

# Main script logic
case "$1" in
    "build")
        check_docker
        check_env_file
        build_images
        ;;
    "start")
        check_docker
        check_env_file
        start_services
        ;;
    "stop")
        stop_services
        ;;
    "restart")
        stop_services
        start_services
        ;;
    "status")
        show_status
        ;;
    "logs")
        show_logs "$2"
        ;;
    "health")
        health_check
        ;;
    "test")
        check_docker
        run_tests
        ;;
    "backup")
        backup_database
        ;;
    "restore")
        restore_database "$2"
        ;;
    "cleanup")
        cleanup
        ;;
    "deploy")
        check_docker
        check_env_file
        build_images
        start_services
        sleep 10
        health_check
        print_success "Deployment completed successfully!"
        ;;
    "deploy-secure")
        check_docker
        check_env_file
        generate_certificates
        build_images
        start_services
        sleep 15
        health_check
        security_check
        print_success "Secure deployment completed successfully!"
        ;;
    *)
        echo "Usage: $0 {build|start|stop|restart|status|logs|health|test|backup|restore|cleanup|deploy|deploy-secure}"
        echo ""
        echo "Commands:"
        echo "  build         - Build Docker images"
        echo "  start         - Start all services"
        echo "  stop          - Stop all services"
        echo "  restart       - Restart all services"
        echo "  status        - Show service status"
        echo "  logs          - Show logs (optionally specify service name)"
        echo "  health        - Perform health check"
        echo "  test          - Run tests"
        echo "  backup        - Backup database"
        echo "  restore       - Restore database (specify backup file)"
        echo "  cleanup       - Clean up containers and volumes"
        echo "  deploy        - Full deployment (build, start, health check)"
        echo "  deploy-secure - Secure deployment with TLS and security checks"
        echo ""
        echo "Security Features:"
        echo "  - JWT-based authentication"
        echo "  - API key management with RBAC"
        echo "  - Rate limiting per user/API key"
        echo "  - TLS encryption for gRPC"
        echo "  - Real-time WebSocket notifications"
        echo "  - Distributed tracing with Jaeger"
        echo "  - Prometheus metrics and Grafana dashboards"
        echo "  - Comprehensive audit logging"
        exit 1
        ;;
esac
