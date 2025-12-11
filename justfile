# Last-Mile MCP Server - Development and Deployment Commands
# See: https://github.com/casey/just

set dotenv-load

# Default recipe - show available commands
default:
    @just --list

# =============================================================================
# Development
# =============================================================================

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests in release mode
test-release:
    cargo test --release

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run all checks (lint, format, test)
check: fmt-check lint test

# =============================================================================
# Local Services
# =============================================================================

# Run the MCP HTTP server locally
run-mcp:
    cargo run --release --bin mcp_server_http -- --port 8080 --db data/synthetic.db

# Run the API server locally
run-api:
    cargo run --release --bin api_server -- --port 8080 --db data/synthetic.db

# =============================================================================
# Docker
# =============================================================================

# Build Docker image locally
docker-build:
    docker build -t lastmile-mcp:local .

# Run Docker container locally
docker-run: docker-build
    docker run -p 8080:8080 -v $(pwd)/data:/app/data lastmile-mcp:local

# Test the Docker health endpoint
docker-health:
    curl -s http://localhost:8080/health | jq .

# =============================================================================
# Deployment
# =============================================================================

# Initialize Terraform (run once)
tf-init:
    cd terraform && terraform init

# Plan Terraform changes
tf-plan:
    cd terraform && terraform plan

# Apply Terraform changes
tf-apply:
    cd terraform && terraform apply

# Destroy Terraform resources (careful!)
tf-destroy:
    cd terraform && terraform destroy

# Deploy using gcloud directly (quick deploy)
deploy project_id region="us-central1":
    ./deploy/deploy.sh {{project_id}} {{region}}

# =============================================================================
# Release
# =============================================================================

# Create a new release tag and push (triggers CI/CD deployment)
release version:
    @echo "Creating release {{version}}..."
    git tag -a {{version}} -m "Release {{version}}"
    git push origin {{version}}
    @echo "Release {{version}} created and pushed!"
    @echo "CI/CD will now build and deploy automatically."

# List recent tags
tags:
    git tag -l --sort=-v:refname | head -10

# =============================================================================
# Database
# =============================================================================

# Ingest data into the database
ingest:
    cargo run --release --bin ingest

# Ingest synthetic data
ingest-synthetic:
    cargo run --release --bin ingest_synthetic

# Run analytics - descriptive
analytics-descriptive:
    cargo run --release --bin analytics_descriptive -- --db data/synthetic.db

# Run analytics - diagnostic
analytics-diagnostic:
    cargo run --release --bin analytics_diagnostic -- --db data/synthetic.db

# Run analytics - predictive
analytics-predictive:
    cargo run --release --bin analytics_predictive -- --db data/synthetic.db

# Run analytics - prescriptive
analytics-prescriptive:
    cargo run --release --bin analytics_prescriptive -- --db data/synthetic.db

# Run clustering analysis
analytics-clustering:
    cargo run --release --bin analytics_clustering -- --db data/synthetic.db

# =============================================================================
# MCP Testing
# =============================================================================

# Test MCP health endpoint
mcp-health url="http://localhost:8080":
    curl -s {{url}}/health | jq .

# Test MCP initialize
mcp-init url="http://localhost:8080":
    curl -s -X POST {{url}}/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | jq .

# Test MCP list tools
mcp-tools url="http://localhost:8080":
    curl -s -X POST {{url}}/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | jq .

# Test MCP get clusters
mcp-clusters url="http://localhost:8080":
    curl -s -X POST {{url}}/mcp \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_lane_clusters","arguments":{}}}' | jq .

# Full MCP test suite
mcp-test url="http://localhost:8080":
    @echo "Testing MCP at {{url}}..."
    @echo "\n=== Health Check ==="
    @just mcp-health {{url}}
    @echo "\n=== Initialize ==="
    @just mcp-init {{url}}
    @echo "\n=== List Tools ==="
    @just mcp-tools {{url}}
    @echo "\n=== Get Clusters ==="
    @just mcp-clusters {{url}}
    @echo "\n=== All tests passed! ==="

# Test production MCP endpoint
mcp-test-prod:
    just mcp-test "https://logistic.hey.sh"

# =============================================================================
# Utilities
# =============================================================================

# Clean build artifacts
clean:
    cargo clean

# Show project stats
stats:
    @echo "Lines of code:"
    @tokei src/ -e "*.json" 2>/dev/null || wc -l src/**/*.rs
    @echo "\nBinary sizes:"
    @ls -lh target/release/mcp_server_http 2>/dev/null || echo "Not built yet"

# Open documentation
docs:
    cargo doc --open
