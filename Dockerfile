# Build stage
FROM rust:1.75-bookworm as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libclang-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release --bin mcp_server_http

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 app
USER app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/mcp_server_http /usr/local/bin/

# Copy database (or mount as volume in production)
COPY --from=builder --chown=app:app /app/data /app/data

# Environment
ENV RUST_LOG=info

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run server
CMD ["mcp_server_http", "--port", "8080", "--db", "/app/data/synthetic.db"]
