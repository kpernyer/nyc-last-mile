//! HTTP-based MCP Server for Last-Mile Analytics
//!
//! Exposes lane clustering analytics via Model Context Protocol (MCP)
//! over HTTP with Server-Sent Events (SSE) transport.
//!
//! This version can be deployed to cloud platforms (Cloud Run, etc.)
//! and accessed remotely by MCP clients.
//!
//! Run: ./target/release/mcp_server_http --port 8080
//!
//! Environment variables:
//!   LASTMILE_DB_PATH - Path to SurrealDB database
//!
//! Endpoints:
//!   POST /mcp    - JSON-RPC requests
//!   GET  /sse    - Server-Sent Events stream for notifications
//!   GET  /health - Health check

use anyhow::Result;
use axum::{
    extract::State,
    http::{header, Method},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "mcp_server_http")]
#[command(about = "HTTP-based MCP server for last-mile analytics")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Database path
    #[arg(long, default_value = "data/synthetic.db")]
    db: String,
}

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// ============================================================================
// Application State
// ============================================================================

struct AppState {
    db: surrealdb::Surreal<surrealdb::engine::local::Db>,
    sse_tx: broadcast::Sender<String>,
}

// ============================================================================
// Database Operations
// ============================================================================

async fn init_db(path: &str) -> Result<surrealdb::Surreal<surrealdb::engine::local::Db>> {
    use surrealdb::engine::local::RocksDb;
    use surrealdb::Surreal;

    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("lastmile").use_db("analytics").await?;
    Ok(db)
}

// ============================================================================
// MCP Tool Implementations
// ============================================================================

async fn get_clusters(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<Value> {
    #[derive(Debug, Deserialize)]
    struct ClusterStats {
        cluster_id: i64,
        lane_count: i64,
        avg_otd: f64,
        avg_transit: f64,
        avg_variance: f64,
    }

    let query = r#"
        SELECT
            cluster_id,
            count() as lane_count,
            math::mean(otd_rate) as avg_otd,
            math::mean(avg_transit) as avg_transit,
            math::mean(transit_variance) as avg_variance
        FROM lane_cluster
        GROUP BY cluster_id
        ORDER BY cluster_id
    "#;

    let mut result = db.query(query).await?;
    let stats: Vec<ClusterStats> = result.take(0)?;

    let cluster_names = [
        "Early & Stable",
        "On-Time & Reliable",
        "High-Jitter",
        "Systematically Late",
        "Low Volume/Mixed",
    ];

    let clusters: Vec<Value> = stats
        .iter()
        .map(|s| {
            let name = cluster_names
                .get((s.cluster_id - 1) as usize)
                .unwrap_or(&"Unknown");
            json!({
                "cluster_id": s.cluster_id,
                "name": name,
                "lane_count": s.lane_count,
                "avg_otd_rate": format!("{:.1}%", s.avg_otd),
                "avg_transit_days": format!("{:.1}", s.avg_transit),
                "avg_variance": format!("{:.2}", s.avg_variance)
            })
        })
        .collect();

    Ok(json!({ "clusters": clusters }))
}

async fn get_lanes_in_cluster(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    cluster_id: u8,
    limit: usize,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize)]
    struct LaneInfo {
        lane_ref: String,
        shipments: i64,
        otd_rate: f64,
        avg_transit: f64,
        transit_variance: f64,
    }

    let query = format!(
        r#"
        SELECT lane_ref, shipments, otd_rate, avg_transit, transit_variance
        FROM lane_cluster
        WHERE cluster_id = {}
        ORDER BY shipments DESC
        LIMIT {}
        "#,
        cluster_id, limit
    );

    let mut result = db.query(&query).await?;
    let lanes: Vec<LaneInfo> = result.take(0)?;

    Ok(json!({
        "cluster_id": cluster_id,
        "lanes": lanes
    }))
}

async fn get_lane_profile(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    origin: &str,
    dest: &str,
) -> Result<Value> {
    #[derive(Debug, Deserialize)]
    struct LaneProfile {
        lane_ref: String,
        cluster_id: i64,
        shipments: i64,
        otd_rate: f64,
        late_rate: f64,
        early_rate: f64,
        avg_transit: f64,
        transit_variance: f64,
    }

    let lane_pattern = format!("{}%{}%", origin, dest);
    let query = format!(
        r#"
        SELECT * FROM lane_cluster
        WHERE lane_ref LIKE '{}'
        LIMIT 1
        "#,
        lane_pattern
    );

    let mut result = db.query(&query).await?;
    let lanes: Vec<LaneProfile> = result.take(0)?;

    if let Some(lane) = lanes.first() {
        Ok(json!({
            "lane": lane.lane_ref,
            "cluster_id": lane.cluster_id,
            "metrics": {
                "shipments": lane.shipments,
                "otd_rate": format!("{:.1}%", lane.otd_rate),
                "late_rate": format!("{:.1}%", lane.late_rate),
                "early_rate": format!("{:.1}%", lane.early_rate),
                "avg_transit": format!("{:.1} days", lane.avg_transit),
                "variance": format!("{:.2}", lane.transit_variance)
            }
        }))
    } else {
        Ok(json!({ "error": "Lane not found" }))
    }
}

async fn get_playbook(cluster_id: u8) -> Value {
    let playbooks: HashMap<u8, Value> = [
        (1, json!({
            "cluster": "Early & Stable",
            "description": "Lanes consistently delivering 0.5-2 days early with low variance",
            "strategy": "Hold-Until Policy",
            "actions": [
                "Implement hold-until-date at destination DC",
                "Consider negotiating lower rates for non-urgent shipments",
                "Use as overflow capacity for time-sensitive lanes",
                "Monitor for over-provisioned transit time SLAs"
            ],
            "kpis_to_watch": ["Early delivery rate", "Storage costs", "SLA efficiency"]
        })),
        (2, json!({
            "cluster": "On-Time & Reliable",
            "description": "High OTD (>95%), low variance - benchmark performers",
            "strategy": "Protect & Replicate",
            "actions": [
                "Document carrier/route combinations for replication",
                "Prioritize these lanes for premium customers",
                "Use as baseline for SLA negotiations",
                "Avoid changes - if it works, don't fix it"
            ],
            "kpis_to_watch": ["OTD rate stability", "Volume trends", "Carrier capacity"]
        })),
        (3, json!({
            "cluster": "High-Jitter",
            "description": "Acceptable average transit but unpredictable variance",
            "strategy": "Buffer & Monitor",
            "actions": [
                "Add 1-2 buffer days to promised delivery dates",
                "Implement proactive tracking alerts",
                "Consider carrier diversification",
                "Root cause analysis on variance drivers"
            ],
            "kpis_to_watch": ["Transit variance", "Exception rate", "Customer complaints"]
        })),
        (4, json!({
            "cluster": "Systematically Late",
            "description": "Consistent SLA misses - structural issues",
            "strategy": "Carrier Switch or SLA Reset",
            "actions": [
                "Evaluate alternative carriers immediately",
                "If no alternatives, reset customer SLA expectations",
                "Implement penalty clauses with current carrier",
                "Consider mode change (LTL → Truckload)"
            ],
            "kpis_to_watch": ["Late rate", "Carrier response", "Customer churn risk"]
        })),
        (5, json!({
            "cluster": "Low Volume/Mixed",
            "description": "Insufficient data for reliable patterns",
            "strategy": "Conservative Approach",
            "actions": [
                "Use conservative SLA estimates",
                "Consolidate with similar lanes if possible",
                "Monitor until volume threshold reached",
                "Default to most reliable carrier in region"
            ],
            "kpis_to_watch": ["Volume growth", "Pattern emergence", "Consolidation opportunities"]
        })),
    ].into_iter().collect();

    playbooks.get(&cluster_id).cloned().unwrap_or(json!({
        "error": "Invalid cluster ID. Use 1-5."
    }))
}

async fn find_similar_lanes(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    pattern: &str,
    limit: usize,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize)]
    struct LaneData {
        lane_ref: String,
        cluster_id: i64,
        otd_rate: f64,
        avg_transit: f64,
    }

    // First find the reference lane
    let ref_query = format!(
        "SELECT * FROM lane_cluster WHERE lane_ref LIKE '%{}%' LIMIT 1",
        pattern
    );
    let mut result = db.query(&ref_query).await?;
    let ref_lanes: Vec<LaneData> = result.take(0)?;

    if let Some(ref_lane) = ref_lanes.first() {
        // Find lanes in the same cluster
        let similar_query = format!(
            r#"
            SELECT lane_ref, cluster_id, otd_rate, avg_transit
            FROM lane_cluster
            WHERE cluster_id = {} AND lane_ref != '{}'
            ORDER BY shipments DESC
            LIMIT {}
            "#,
            ref_lane.cluster_id, ref_lane.lane_ref, limit
        );

        let mut result = db.query(&similar_query).await?;
        let similar: Vec<LaneData> = result.take(0)?;

        Ok(json!({
            "reference_lane": ref_lane.lane_ref,
            "cluster_id": ref_lane.cluster_id,
            "similar_lanes": similar
        }))
    } else {
        Ok(json!({ "error": "Reference lane not found" }))
    }
}

async fn get_early_analysis(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize)]
    struct EarlyStats {
        cluster_id: i64,
        early_rate: f64,
        lane_count: i64,
    }

    let query = r#"
        SELECT
            cluster_id,
            math::mean(early_rate) as early_rate,
            count() as lane_count
        FROM lane_cluster
        WHERE early_rate > 10
        GROUP BY cluster_id
        ORDER BY early_rate DESC
    "#;

    let mut result = db.query(query).await?;
    let stats: Vec<EarlyStats> = result.take(0)?;

    Ok(json!({
        "early_delivery_summary": stats,
        "insight": "Early deliveries may indicate over-provisioned transit times or opportunities for cost optimization"
    }))
}

async fn get_regional_performance(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    zip3: &str,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize)]
    struct RegionLane {
        lane_ref: String,
        cluster_id: i64,
        shipments: i64,
        otd_rate: f64,
        late_rate: f64,
    }

    let query = format!(
        r#"
        SELECT lane_ref, cluster_id, shipments, otd_rate, late_rate
        FROM lane_cluster
        WHERE lane_ref LIKE '%{}%'
        ORDER BY shipments DESC
        LIMIT 20
        "#,
        zip3
    );

    let mut result = db.query(&query).await?;
    let lanes: Vec<RegionLane> = result.take(0)?;

    let total_shipments: i64 = lanes.iter().map(|l| l.shipments).sum();
    let avg_otd: f64 = if !lanes.is_empty() {
        lanes.iter().map(|l| l.otd_rate).sum::<f64>() / lanes.len() as f64
    } else {
        0.0
    };

    Ok(json!({
        "region": zip3,
        "total_lanes": lanes.len(),
        "total_shipments": total_shipments,
        "avg_otd_rate": format!("{:.1}%", avg_otd),
        "lanes": lanes
    }))
}

async fn get_friction_zones(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    limit: usize,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize)]
    struct FrictionZone {
        lane_ref: String,
        late_rate: f64,
        transit_variance: f64,
        shipments: i64,
    }

    let query = format!(
        r#"
        SELECT lane_ref, late_rate, transit_variance, shipments
        FROM lane_cluster
        WHERE late_rate > 20 AND shipments > 50
        ORDER BY late_rate DESC
        LIMIT {}
        "#,
        limit
    );

    let mut result = db.query(&query).await?;
    let zones: Vec<FrictionZone> = result.take(0)?;

    Ok(json!({
        "friction_zones": zones,
        "criteria": "Late rate > 20%, minimum 50 shipments"
    }))
}

async fn get_terminal_performance(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    limit: usize,
) -> Result<Value> {
    #[derive(Debug, Deserialize, Serialize, Clone)]
    struct TerminalStats {
        origin: String,
        lane_count: i64,
        avg_otd: f64,
        total_shipments: i64,
    }

    // Extract origin from lane_ref (format: "XXX→YYY")
    let query = format!(
        r#"
        SELECT
            string::split(lane_ref, '→')[0] as origin,
            count() as lane_count,
            math::mean(otd_rate) as avg_otd,
            math::sum(shipments) as total_shipments
        FROM lane_cluster
        GROUP BY string::split(lane_ref, '→')[0]
        ORDER BY avg_otd DESC
        LIMIT {}
        "#,
        limit * 2
    );

    let mut result = db.query(&query).await?;
    let terminals: Vec<TerminalStats> = result.take(0)?;

    let best: Vec<_> = terminals.iter().take(limit).cloned().collect();
    let worst: Vec<_> = terminals.iter().rev().take(limit).cloned().collect();

    Ok(json!({
        "best_performers": best,
        "worst_performers": worst
    }))
}

// ============================================================================
// MCP Protocol Handlers
// ============================================================================

fn get_server_info() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "last-mile-analytics",
            "version": "2.0.0-http"
        }
    })
}

fn get_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "get_lane_clusters",
                "description": "Get all lane behavioral clusters with summary statistics. Returns 5 clusters: Early & Stable, On-Time & Reliable, High-Jitter, Systematically Late, and Low Volume/Mixed.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "get_lanes_in_cluster",
                "description": "Get lanes in a specific cluster. Cluster IDs: 1=Early & Stable, 2=On-Time & Reliable, 3=High-Jitter, 4=Systematically Late, 5=Low Volume/Mixed",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cluster_id": {
                            "type": "integer",
                            "description": "Cluster ID (1-5)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of lanes to return (default 20)"
                        }
                    },
                    "required": ["cluster_id"]
                }
            },
            {
                "name": "get_lane_profile",
                "description": "Get metrics and cluster assignment for a specific lane.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "origin": {
                            "type": "string",
                            "description": "Origin ZIP3 code (e.g., '750')"
                        },
                        "dest": {
                            "type": "string",
                            "description": "Destination ZIP3 code (e.g., '857')"
                        }
                    },
                    "required": ["origin", "dest"]
                }
            },
            {
                "name": "get_cluster_playbook",
                "description": "Get recommended last-mile strategy and actions for a cluster.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cluster_id": {
                            "type": "integer",
                            "description": "Cluster ID (1-5)"
                        }
                    },
                    "required": ["cluster_id"]
                }
            },
            {
                "name": "find_similar_lanes",
                "description": "Find lanes with similar behavior patterns.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern - origin ZIP3, destination ZIP3, or partial lane name"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results (default 10)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "get_early_delivery_analysis",
                "description": "Analyze early delivery patterns across the network.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "get_regional_performance",
                "description": "Get performance metrics for a specific region (ZIP3).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "zip3": {
                            "type": "string",
                            "description": "ZIP3 code (e.g., '750', '441')"
                        }
                    },
                    "required": ["zip3"]
                }
            },
            {
                "name": "get_friction_zones",
                "description": "Identify high-friction destination zones with poor delivery performance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results (default 10)"
                        }
                    },
                    "required": []
                }
            },
            {
                "name": "get_terminal_performance",
                "description": "Score origin terminals/DCs on outbound delivery performance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of top/bottom performers (default 5)"
                        }
                    },
                    "required": []
                }
            }
        ]
    })
}

async fn handle_tool_call(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    name: &str,
    args: &Value,
) -> Result<Value> {
    match name {
        "get_lane_clusters" => get_clusters(db).await,
        "get_lanes_in_cluster" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            get_lanes_in_cluster(db, cluster_id, limit).await
        }
        "get_lane_profile" => {
            let origin = args.get("origin").and_then(|v| v.as_str()).unwrap_or("");
            let dest = args.get("dest").and_then(|v| v.as_str()).unwrap_or("");
            get_lane_profile(db, origin, dest).await
        }
        "get_cluster_playbook" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            Ok(get_playbook(cluster_id).await)
        }
        "find_similar_lanes" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            find_similar_lanes(db, pattern, limit).await
        }
        "get_early_delivery_analysis" => get_early_analysis(db).await,
        "get_regional_performance" => {
            let zip3 = args.get("zip3").and_then(|v| v.as_str()).unwrap_or("");
            get_regional_performance(db, zip3).await
        }
        "get_friction_zones" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            get_friction_zones(db, limit).await
        }
        "get_terminal_performance" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            get_terminal_performance(db, limit).await
        }
        _ => Ok(json!({"error": format!("Unknown tool: {}", name)})),
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "last-mile-mcp",
        "version": "2.0.0-http"
    }))
}

async fn handle_mcp_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = request.id.clone();

    let result = match request.method.as_str() {
        "initialize" => Ok(get_server_info()),
        "tools/list" => Ok(get_tools_list()),
        "tools/call" => {
            if let Some(params) = request.params {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let empty_args = json!({});
                let args = params.get("arguments").unwrap_or(&empty_args);
                match handle_tool_call(&state.db, name, args).await {
                    Ok(result) => Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })),
                    Err(e) => Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {}", e)
                        }],
                        "isError": true
                    })),
                }
            } else {
                Err("Missing params")
            }
        }
        _ => Err("Method not found"),
    };

    let response = match result {
        Ok(r) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(r),
            error: None,
        },
        Err(msg) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: msg.to_string(),
            }),
        },
    };

    Json(response)
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|msg| {
        let data = msg.unwrap_or_default();
        Ok(Event::default().data(data))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    info!("Last-Mile Analytics MCP Server (HTTP)");
    info!("Database: {}", args.db);
    info!("Port: {}", args.port);

    // Initialize database
    let db = init_db(&args.db).await?;
    info!("Database connected");

    // Create SSE broadcast channel
    let (sse_tx, _) = broadcast::channel::<String>(100);

    let state = Arc::new(AppState { db, sse_tx });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/mcp", post(handle_mcp_request))
        .route("/sse", get(sse_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
