# VexFS Production Docker Image
# Builds ONLY the unified server - no architectural chaos

FROM rust:1.82-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libfuse3-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source files
COPY Cargo.toml Cargo.lock ./
COPY rust/ ./rust/

# Build the unified server with static file serving
RUN cargo build --release --features="server" --bin vexfs_unified_server

# Node.js stage for dashboard
FROM node:18-bookworm as dashboard-builder

WORKDIR /app/dashboard
COPY vexfs-dashboard/package*.json ./
RUN npm ci

COPY vexfs-dashboard/ ./
RUN npm run build

# Production runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false vexfs

# Create directories
RUN mkdir -p /app/data /app/dashboard && \
    chown -R vexfs:vexfs /app

# Copy the unified server binary with static file serving
COPY --from=builder /app/target/release/vexfs_unified_server /usr/local/bin/vexfs

# Copy dashboard
COPY --from=dashboard-builder /app/dashboard/dist /app/dashboard/

# Copy entrypoint
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Set ownership
RUN chown -R vexfs:vexfs /app /usr/local/bin/vexfs /usr/local/bin/docker-entrypoint.sh

# Switch to non-root user
USER vexfs

# Expose configurable port (default 7680)
EXPOSE 7680

# Environment variables
ENV RUST_LOG=info
ENV VEXFS_PORT=7680
ENV VEXFS_DATA_DIR=/app/data

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:${VEXFS_PORT}/api/v1/version || exit 1

# Run the unified server
CMD ["/usr/local/bin/docker-entrypoint.sh"]
