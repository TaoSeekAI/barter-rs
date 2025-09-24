# Multi-stage build for Barter
FROM rust:1.82-slim AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy workspace files
COPY Cargo.toml ./
COPY rust-toolchain.toml ./
COPY rustfmt.toml ./

# Copy all workspace members
COPY barter/ ./barter/
COPY barter-data/ ./barter-data/
COPY barter-execution/ ./barter-execution/
COPY barter-instrument/ ./barter-instrument/
COPY barter-integration/ ./barter-integration/
COPY barter-macro/ ./barter-macro/

# Build release version
RUN cargo build --release --package barter --example engine_sync_with_live_market_data_and_mock_execution_and_audit

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 appuser

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/examples/engine_sync_with_live_market_data_and_mock_execution_and_audit /usr/local/bin/barter

# Copy config files
RUN mkdir -p /etc/barter
COPY barter/examples/config /etc/barter/config

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /home/appuser

# Expose port for monitoring (if needed)
EXPOSE 8080

# Run the application
CMD ["barter"]