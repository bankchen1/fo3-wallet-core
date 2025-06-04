#!/bin/bash

# FO3 Wallet Core Production Deployment Script
# 
# This script automates the deployment of FO3 Wallet Core to production Kubernetes cluster
# with comprehensive validation, monitoring, and rollback capabilities.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
K8S_DIR="${PROJECT_ROOT}/k8s"
VERSION="${1:-5.0.0}"
ENVIRONMENT="${2:-production}"
CLUSTER_NAME="${3:-fo3-wallet-cluster-prod}"
NAMESPACE="fo3-wallet"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Error handling
cleanup() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        log_error "Deployment failed with exit code $exit_code"
        log_info "Check logs above for details"
        log_info "To rollback, run: kubectl rollout undo deployment/fo3-wallet-core -n $NAMESPACE"
    fi
    exit $exit_code
}

trap cleanup EXIT

# Validation functions
validate_prerequisites() {
    log_info "Validating prerequisites..."
    
    # Check required tools
    local required_tools=("kubectl" "kustomize" "helm" "docker" "jq" "yq")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' is not installed"
            exit 1
        fi
    done
    
    # Check kubectl context
    local current_context=$(kubectl config current-context)
    if [[ "$current_context" != *"$CLUSTER_NAME"* ]]; then
        log_error "kubectl context '$current_context' does not match expected cluster '$CLUSTER_NAME'"
        log_info "Switch context with: kubectl config use-context <correct-context>"
        exit 1
    fi
    
    # Check cluster connectivity
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    
    # Check namespace exists
    if ! kubectl get namespace "$NAMESPACE" &> /dev/null; then
        log_warning "Namespace '$NAMESPACE' does not exist, creating it..."
        kubectl apply -f "${K8S_DIR}/namespace.yaml"
    fi
    
    log_success "Prerequisites validated"
}

validate_configuration() {
    log_info "Validating configuration..."
    
    # Check required files
    local required_files=(
        "${K8S_DIR}/namespace.yaml"
        "${K8S_DIR}/configmap.yaml"
        "${K8S_DIR}/secrets.yaml"
        "${K8S_DIR}/fo3-wallet-core.yaml"
        "${K8S_DIR}/kustomization.yaml"
    )
    
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            log_error "Required file '$file' not found"
            exit 1
        fi
    done
    
    # Validate Kubernetes manifests
    log_info "Validating Kubernetes manifests..."
    if ! kustomize build "$K8S_DIR" | kubectl apply --dry-run=client -f - &> /dev/null; then
        log_error "Kubernetes manifest validation failed"
        exit 1
    fi
    
    # Check secrets are configured
    if grep -q "CHANGE_ME" "${K8S_DIR}/secrets.yaml"; then
        log_error "Secrets contain placeholder values. Please update secrets.yaml with actual values."
        exit 1
    fi
    
    log_success "Configuration validated"
}

validate_image() {
    log_info "Validating Docker image..."
    
    local image="fo3wallet/fo3-wallet-core:${VERSION}"
    
    # Check if image exists locally or can be pulled
    if ! docker image inspect "$image" &> /dev/null; then
        log_info "Image not found locally, attempting to pull..."
        if ! docker pull "$image"; then
            log_error "Failed to pull image '$image'"
            exit 1
        fi
    fi
    
    # Scan image for vulnerabilities (if trivy is available)
    if command -v trivy &> /dev/null; then
        log_info "Scanning image for vulnerabilities..."
        if ! trivy image --exit-code 1 --severity HIGH,CRITICAL "$image"; then
            log_error "Image contains high or critical vulnerabilities"
            exit 1
        fi
    fi
    
    log_success "Image validated"
}

# Deployment functions
deploy_infrastructure() {
    log_info "Deploying infrastructure components..."
    
    # Deploy in order: namespace, RBAC, secrets, configmaps
    kubectl apply -f "${K8S_DIR}/namespace.yaml"
    kubectl apply -f "${K8S_DIR}/rbac.yaml"
    kubectl apply -f "${K8S_DIR}/secrets.yaml"
    kubectl apply -f "${K8S_DIR}/configmap.yaml"
    
    # Deploy storage components
    kubectl apply -f "${K8S_DIR}/postgres.yaml"
    kubectl apply -f "${K8S_DIR}/redis.yaml"
    
    # Wait for storage to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=fo3-postgres -n "$NAMESPACE" --timeout=300s
    
    log_info "Waiting for Redis to be ready..."
    kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=fo3-redis -n "$NAMESPACE" --timeout=300s
    
    log_success "Infrastructure deployed"
}

deploy_application() {
    log_info "Deploying FO3 Wallet Core application..."
    
    # Update image tag in kustomization
    cd "$K8S_DIR"
    kustomize edit set image "fo3wallet/fo3-wallet-core:${VERSION}"
    
    # Apply application manifests
    kustomize build . | kubectl apply -f -
    
    # Wait for deployment to be ready
    log_info "Waiting for application deployment to be ready..."
    kubectl wait --for=condition=available deployment/fo3-wallet-core -n "$NAMESPACE" --timeout=600s
    
    # Check rollout status
    kubectl rollout status deployment/fo3-wallet-core -n "$NAMESPACE" --timeout=600s
    
    log_success "Application deployed"
}

deploy_monitoring() {
    log_info "Deploying monitoring stack..."
    
    # Deploy monitoring components
    kubectl apply -f "${K8S_DIR}/monitoring.yaml"
    kubectl apply -f "${K8S_DIR}/grafana.yaml"
    
    # Wait for monitoring to be ready
    log_info "Waiting for Prometheus to be ready..."
    kubectl wait --for=condition=available deployment/fo3-prometheus -n "$NAMESPACE" --timeout=300s
    
    log_info "Waiting for Grafana to be ready..."
    kubectl wait --for=condition=available deployment/fo3-grafana -n "$NAMESPACE" --timeout=300s
    
    log_success "Monitoring deployed"
}

deploy_ingress() {
    log_info "Deploying ingress and load balancer..."
    
    kubectl apply -f "${K8S_DIR}/ingress.yaml"
    
    # Wait for ingress to get external IP
    log_info "Waiting for load balancer to be provisioned..."
    local timeout=300
    local count=0
    while [ $count -lt $timeout ]; do
        local external_ip=$(kubectl get ingress fo3-wallet-ingress -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "")
        if [ -n "$external_ip" ] && [ "$external_ip" != "null" ]; then
            log_success "Load balancer provisioned with IP: $external_ip"
            break
        fi
        sleep 5
        count=$((count + 5))
    done
    
    if [ $count -ge $timeout ]; then
        log_warning "Load balancer provisioning timed out, but deployment continues"
    fi
    
    log_success "Ingress deployed"
}

# Validation functions
validate_deployment() {
    log_info "Validating deployment..."
    
    # Check pod status
    local pods_ready=$(kubectl get pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" -o jsonpath='{.items[*].status.conditions[?(@.type=="Ready")].status}')
    if [[ "$pods_ready" != *"True"* ]]; then
        log_error "Not all pods are ready"
        kubectl get pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE"
        exit 1
    fi
    
    # Check service endpoints
    local endpoints=$(kubectl get endpoints fo3-wallet-api -n "$NAMESPACE" -o jsonpath='{.subsets[*].addresses[*].ip}')
    if [ -z "$endpoints" ]; then
        log_error "Service has no endpoints"
        exit 1
    fi
    
    # Health check
    log_info "Performing health checks..."
    local pod_name=$(kubectl get pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    
    if ! kubectl exec "$pod_name" -n "$NAMESPACE" -- grpc_health_probe -addr=localhost:50051 -tls -tls-no-verify; then
        log_error "Health check failed"
        exit 1
    fi
    
    # Check metrics endpoint
    if ! kubectl exec "$pod_name" -n "$NAMESPACE" -- curl -f http://localhost:9090/metrics > /dev/null; then
        log_error "Metrics endpoint check failed"
        exit 1
    fi
    
    log_success "Deployment validation passed"
}

run_smoke_tests() {
    log_info "Running smoke tests..."
    
    # Get service endpoint
    local service_ip=$(kubectl get service fo3-wallet-api -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
    if [ -z "$service_ip" ]; then
        service_ip=$(kubectl get service fo3-wallet-api -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}')
    fi
    
    # Test gRPC health check
    if command -v grpc_health_probe &> /dev/null; then
        log_info "Testing gRPC health endpoint..."
        if ! grpc_health_probe -addr="${service_ip}:50051" -tls -tls-no-verify; then
            log_error "gRPC health check failed"
            exit 1
        fi
    fi
    
    # Test metrics endpoint
    log_info "Testing metrics endpoint..."
    local pod_name=$(kubectl get pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    if ! kubectl exec "$pod_name" -n "$NAMESPACE" -- curl -f http://localhost:9090/metrics > /dev/null; then
        log_error "Metrics endpoint test failed"
        exit 1
    fi
    
    log_success "Smoke tests passed"
}

# Performance validation
validate_performance() {
    log_info "Validating performance metrics..."
    
    # Check resource usage
    local cpu_usage=$(kubectl top pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" --no-headers | awk '{sum+=$2} END {print sum}' | sed 's/m//')
    local memory_usage=$(kubectl top pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" --no-headers | awk '{sum+=$3} END {print sum}' | sed 's/Mi//')
    
    log_info "Current resource usage: ${cpu_usage}m CPU, ${memory_usage}Mi memory"
    
    # Check if usage is within expected ranges
    if [ "$cpu_usage" -gt 2000 ]; then
        log_warning "High CPU usage detected: ${cpu_usage}m"
    fi
    
    if [ "$memory_usage" -gt 4096 ]; then
        log_warning "High memory usage detected: ${memory_usage}Mi"
    fi
    
    # Check response times (if available)
    local pod_name=$(kubectl get pods -l app.kubernetes.io/name=fo3-wallet-core -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    local response_time=$(kubectl exec "$pod_name" -n "$NAMESPACE" -- curl -w "%{time_total}" -s -o /dev/null http://localhost:9090/metrics)
    
    log_info "Metrics endpoint response time: ${response_time}s"
    
    log_success "Performance validation completed"
}

# Main deployment function
main() {
    log_info "Starting FO3 Wallet Core production deployment"
    log_info "Version: $VERSION"
    log_info "Environment: $ENVIRONMENT"
    log_info "Cluster: $CLUSTER_NAME"
    log_info "Namespace: $NAMESPACE"
    
    # Pre-deployment validation
    validate_prerequisites
    validate_configuration
    validate_image
    
    # Deployment phases
    deploy_infrastructure
    deploy_application
    deploy_monitoring
    deploy_ingress
    
    # Post-deployment validation
    validate_deployment
    run_smoke_tests
    validate_performance
    
    # Success message
    log_success "🎉 FO3 Wallet Core deployment completed successfully!"
    log_info "Application is available at:"
    log_info "  gRPC API: https://api.fo3wallet.com:50051"
    log_info "  WebSocket: wss://ws.fo3wallet.com:8080"
    log_info "  Monitoring: https://grafana.fo3wallet.com"
    
    # Display useful commands
    log_info "Useful commands:"
    log_info "  Check status: kubectl get pods -n $NAMESPACE"
    log_info "  View logs: kubectl logs -f deployment/fo3-wallet-core -n $NAMESPACE"
    log_info "  Scale up: kubectl scale deployment fo3-wallet-core --replicas=5 -n $NAMESPACE"
    log_info "  Rollback: kubectl rollout undo deployment/fo3-wallet-core -n $NAMESPACE"
}

# Run main function
main "$@"
