# NYC Last-Mile Delivery Analytics

A Rust-based analytics platform for analyzing last-mile delivery performance, with an MCP (Model Context Protocol) server for conversational AI queries via Claude Desktop.

## Features

- **Lane Clustering**: Automatically categorize shipping lanes into 5 behavioral clusters
- **Performance Analytics**: Descriptive, diagnostic, predictive, and prescriptive analytics
- **MCP Integration**: Query analytics conversationally through Claude Desktop
- **SurrealDB Backend**: Fast embedded database with RocksDB storage

## Quick Start

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs))
- Claude Desktop (for MCP features) - [Download](https://claude.ai/download)

### Installation

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/nyc-last-mile.git
cd nyc-last-mile

# Build all binaries
cargo build --release
```

### Ingest Sample Data

```bash
# Place your CSV data in raw-data/
./target/release/ingest raw-data/your-shipment-data.csv
```

### Run Analytics

```bash
# Descriptive analytics
./target/release/analytics_descriptive all

# Lane clustering
./target/release/analytics_clustering clusters
./target/release/analytics_clustering playbooks

# Find similar lanes
./target/release/analytics_clustering similar 750xx 857xx
```

## MCP Server for Claude Desktop

The MCP server enables conversational queries like:
- "Which lanes are systematically late?"
- "Where are we arriving too early?"
- "How is the Phoenix region performing?"

### Setup

1. **Build the MCP server:**
   ```bash
   cargo build --release --bin mcp_server
   ```

2. **Copy database to sandbox-friendly location:**
   ```bash
   mkdir -p ~/Library/Application\ Support/LastMileAnalytics
   cp -r data/lastmile.db ~/Library/Application\ Support/LastMileAnalytics/
   ```

3. **Configure Claude Desktop:**

   Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "last-mile-analytics": {
         "command": "/path/to/nyc-last-mile/target/release/mcp_server",
         "args": [],
         "cwd": "/path/to/nyc-last-mile"
       }
     }
   }
   ```

4. **Restart Claude Desktop**

See [docs/CLAUDE_DESKTOP_SETUP.md](docs/CLAUDE_DESKTOP_SETUP.md) for detailed instructions.

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `get_lane_clusters` | Get all 5 behavioral clusters with summary statistics |
| `get_lanes_in_cluster` | List lanes in a specific cluster |
| `get_lane_profile` | Get metrics and cluster assignment for a lane |
| `get_cluster_playbook` | Recommended last-mile strategy for a cluster |
| `find_similar_lanes` | Find lanes with similar delivery patterns |
| `get_early_delivery_analysis` | Analyze early delivery patterns |
| `get_regional_performance` | Performance metrics for a ZIP3 region |
| `get_friction_zones` | Identify high-friction problem destinations |
| `get_terminal_performance` | Score terminals/DCs on outbound performance |

## Lane Clusters

The system classifies shipping lanes into 5 behavioral clusters:

| Cluster | Description | Recommended Action |
|---------|-------------|-------------------|
| **Early & Stable** | Consistently 0.5-2 days early | Hold-until policies, tight delivery windows |
| **On-Time & Reliable** | High on-time rate, low variance | Maintain operations, use as benchmark |
| **High-Jitter** | OK average but unpredictable | Add buffer days, avoid guarantees |
| **Systematically Late** | Consistently miss SLA | Downgrade promises, negotiate with carriers |
| **Low Volume / Mixed** | Insufficient data | Conservative buffers, monitor growth |

## Project Structure

```
nyc-last-mile/
├── src/
│   ├── bin/
│   │   ├── ingest.rs              # Data ingestion
│   │   ├── mcp_server.rs          # MCP server for Claude
│   │   ├── analytics_descriptive.rs
│   │   ├── analytics_diagnostic.rs
│   │   ├── analytics_predictive.rs
│   │   ├── analytics_prescriptive.rs
│   │   ├── analytics_clustering.rs
│   │   └── demo_*.rs              # Demo/exploration tools
│   ├── lib.rs
│   ├── models.rs                  # Data models
│   ├── db.rs                      # Database connection
│   ├── carrier_names.rs           # Carrier code lookups
│   └── location_names.rs          # ZIP3 location mappings
├── data/
│   └── lastmile.db/               # SurrealDB database
├── docs/
│   ├── CLAUDE_DESKTOP_SETUP.md    # Claude Desktop setup guide
│   ├── mcp-integration-plan.md    # MCP development plan
│   └── claude-desktop-config.json # Sample config
└── raw-data/                      # Source CSV files
```

## Analytics Binaries

| Binary | Purpose |
|--------|---------|
| `ingest` | Load CSV shipment data into SurrealDB |
| `analytics_descriptive` | Summary statistics, volume analysis |
| `analytics_diagnostic` | Root cause analysis, variance decomposition |
| `analytics_predictive` | Transit time predictions, risk scoring |
| `analytics_prescriptive` | Recommendations, optimization suggestions |
| `analytics_clustering` | Lane behavioral clustering |
| `mcp_server` | MCP server for Claude Desktop integration |

## Data Model

### Shipment Record

```rust
struct Shipment {
    load_id: String,
    carrier_mode: CarrierMode,      // LTL, Truckload, TL Flatbed, TL Dry
    actual_ship: NaiveDateTime,
    actual_delivery: NaiveDateTime,
    goal_transit_days: i32,
    actual_transit_days: i32,
    otd: OtdDesignation,            // Early, OnTime, Late
    ship_dow: i32,                  // Day of week
    ship_week: i32,
    ship_month: i32,
    ship_year: i32,
    distance_bucket: String,
}
```

### Lane Metrics

Each lane (origin ZIP3 → destination ZIP3) is analyzed for:
- Volume
- Average delay (days)
- Transit variance
- Early/On-time/Late rates
- Cluster assignment

## Technology Stack

- **Language**: Rust
- **Database**: SurrealDB with RocksDB backend
- **Protocol**: MCP (Model Context Protocol) via JSON-RPC over stdio
- **AI Integration**: Claude Desktop

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

## Acknowledgments

Built with Claude Code and Claude Desktop for AI-powered analytics exploration.
