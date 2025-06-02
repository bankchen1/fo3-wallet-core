# Multi-stage build for FO3 Wallet Core gRPC API
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY fo3-wallet/Cargo.toml ./fo3-wallet/
COPY fo3-wallet-api/Cargo.toml ./fo3-wallet-api/
COPY fo3-wallet-solana/Cargo.toml ./fo3-wallet-solana/

# Copy proto files
COPY proto/ ./proto/

# Copy source code
COPY fo3-wallet/src/ ./fo3-wallet/src/
COPY fo3-wallet-api/src/ ./fo3-wallet-api/src/
COPY fo3-wallet-api/build.rs ./fo3-wallet-api/
COPY fo3-wallet-solana/src/ ./fo3-wallet-solana/src/

# Build the application
RUN cargo build --release --bin fo3-wallet-api --features solana

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Create app directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/fo3-wallet-api /app/fo3-wallet-api

# Change ownership to app user
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose gRPC port
EXPOSE 50051

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD grpc_health_probe -addr=localhost:50051 || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV GRPC_LISTEN_ADDR=0.0.0.0:50051

# Run the application
CMD ["./fo3-wallet-api"]
