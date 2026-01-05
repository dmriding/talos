# Talos License Server - Multi-stage Docker Build
#
# Build with all features for production:
#   docker build -t talos-server .
#
# Build with specific features:
#   docker build --build-arg FEATURES="server,postgres,jwt-auth,admin-api" -t talos-server .

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src/server src/client && \
    echo "fn main() {}" > src/server/main.rs && \
    echo "fn main() {}" > src/client/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build argument for features (default: all production features)
ARG FEATURES="server,postgres,sqlite,jwt-auth,admin-api,rate-limiting,background-jobs,openapi"

# Build dependencies only (this layer gets cached)
RUN cargo build --release --features "$FEATURES" --bin talos_server || true

# Remove dummy files
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Build the actual application
RUN cargo build --release --features "$FEATURES" --bin talos_server

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash talos

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/talos_server /app/talos_server

# Copy migrations for database setup
COPY --from=builder /app/migrations /app/migrations

# Copy example config (users should mount their own config.toml)
COPY config.toml.example /app/config.toml.example

# Set ownership
RUN chown -R talos:talos /app

# Switch to non-root user
USER talos

# Expose the default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default environment variables
ENV TALOS_SERVER_HOST=0.0.0.0
ENV TALOS_SERVER_PORT=8080
ENV TALOS_LOGGING_ENABLED=true
ENV TALOS_LOG_LEVEL=info

# Run the server
CMD ["/app/talos_server"]
