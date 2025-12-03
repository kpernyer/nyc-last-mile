//! MCP Server for Last-Mile Analytics
//!
//! Exposes lane clustering analytics via Model Context Protocol (MCP)
//! for integration with LLMs like Claude Desktop.
//!
//! This version uses the REST API server instead of direct database access.
//!
//! Run: ./target/release/mcp_server
//!
//! Environment variables:
//!   LASTMILE_API_URL - API server URL (default: http://localhost:8080)
//!   LASTMILE_DB_PATH - Fallback to direct DB if API unavailable
//!   LASTMILE_DEBUG - Enable debug output to stderr
//!
//! Tools exposed:
//! - get_lane_clusters: Returns all behavioral clusters with statistics
//! - get_lanes_in_cluster: Lists lanes in a specific cluster
//! - get_lane_profile: Get metrics and cluster assignment for a specific lane
//! - get_cluster_playbook: Get recommended actions for a cluster
//! - find_similar_lanes: Find lanes with similar behavior patterns
//! - get_early_delivery_analysis: Analyze early delivery patterns
//! - get_regional_performance: Get performance for a specific region
//! - get_friction_zones: Identify high-friction problem destinations
//! - get_terminal_performance: Score terminals/DCs on outbound performance
//!
//! Configure in Claude Desktop's settings as a stdio MCP server.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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
    id: Value,
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
// API Client
// ============================================================================

struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            Ok(json)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed: {} - {}", status, text)
        }
    }

    async fn get_clusters(&self) -> Result<Value> {
        self.get("/api/v1/clusters").await
    }

    async fn get_lanes_in_cluster(&self, cluster_id: u8, limit: usize) -> Result<Value> {
        self.get(&format!("/api/v1/clusters/{}/lanes?limit={}", cluster_id, limit)).await
    }

    async fn get_lane_profile(&self, origin: &str, dest: &str) -> Result<Value> {
        self.get(&format!("/api/v1/lanes/{}/{}", origin, dest)).await
    }

    async fn get_playbook(&self, cluster_id: u8) -> Result<Value> {
        self.get(&format!("/api/v1/clusters/{}/playbook", cluster_id)).await
    }

    async fn find_similar(&self, pattern: &str, limit: usize) -> Result<Value> {
        self.get(&format!("/api/v1/search/similar?lane={}&limit={}", pattern, limit)).await
    }

    async fn get_early_analysis(&self) -> Result<Value> {
        self.get("/api/v1/analysis/early").await
    }

    async fn get_regional_performance(&self, zip3: &str) -> Result<Value> {
        self.get(&format!("/api/v1/regions/{}", zip3)).await
    }

    async fn get_friction_zones(&self, limit: usize) -> Result<Value> {
        self.get(&format!("/api/v1/analysis/friction?limit={}", limit)).await
    }

    async fn get_terminal_performance(&self, limit: usize) -> Result<Value> {
        self.get(&format!("/api/v1/analysis/terminals?limit={}", limit)).await
    }
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
            "version": "2.0.0"
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
                "description": "Get metrics and cluster assignment for a specific lane. Provide origin and destination as ZIP3 codes or location names.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "origin": {
                            "type": "string",
                            "description": "Origin ZIP3 code or DC name (e.g., '750' or 'DFW')"
                        },
                        "dest": {
                            "type": "string",
                            "description": "Destination ZIP3 code or region name (e.g., '857' or 'TUS')"
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
                "description": "Find lanes that behave similarly to a target lane. Lanes in the same cluster share similar delivery patterns.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern - origin ZIP3, destination ZIP3, or location name"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of similar lanes to return (default 10)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "get_early_delivery_analysis",
                "description": "Analyze early delivery patterns across the network. Shows which destinations receive early shipments, timing patterns by day of week, and 'very early' (>1 day) deliveries that may indicate over-provisioned transit times.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "get_regional_performance",
                "description": "Get performance metrics for a specific region (ZIP3 or location code). Shows lane breakdown by cluster, volume, late rates, and identifies problem lanes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "zip3": {
                            "type": "string",
                            "description": "ZIP3 code or location name (e.g., '750', 'DFW', 'PHX', 'TUS')"
                        }
                    },
                    "required": ["zip3"]
                }
            },
            {
                "name": "get_friction_zones",
                "description": "Identify high-friction destination zones with poor delivery performance. Returns destinations ranked by friction score (combination of late rate and transit variance).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of friction zones to return (default 10)"
                        }
                    },
                    "required": []
                }
            },
            {
                "name": "get_terminal_performance",
                "description": "Score origin terminals/DCs on their outbound delivery performance. Returns a performance index (0-100) for each terminal, with best and worst performers highlighted.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of top/bottom performers to show (default 5)"
                        }
                    },
                    "required": []
                }
            }
        ]
    })
}

async fn handle_tool_call(client: &ApiClient, name: &str, args: &Value) -> Result<Value> {
    match name {
        "get_lane_clusters" => client.get_clusters().await,
        "get_lanes_in_cluster" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            client.get_lanes_in_cluster(cluster_id, limit).await
        }
        "get_lane_profile" => {
            let origin = args.get("origin").and_then(|v| v.as_str()).unwrap_or("");
            let dest = args.get("dest").and_then(|v| v.as_str()).unwrap_or("");
            client.get_lane_profile(origin, dest).await
        }
        "get_cluster_playbook" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            client.get_playbook(cluster_id).await
        }
        "find_similar_lanes" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            client.find_similar(pattern, limit).await
        }
        "get_early_delivery_analysis" => client.get_early_analysis().await,
        "get_regional_performance" => {
            let zip3 = args.get("zip3").and_then(|v| v.as_str()).unwrap_or("");
            client.get_regional_performance(zip3).await
        }
        "get_friction_zones" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            client.get_friction_zones(limit).await
        }
        "get_terminal_performance" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            client.get_terminal_performance(limit).await
        }
        _ => Ok(json!({"error": format!("Unknown tool: {}", name)}))
    }
}

async fn handle_request(client: &ApiClient, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
    // Notifications don't get responses
    if request.method.starts_with("notifications/") {
        return None;
    }

    let id = request.id.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        "initialize" => Ok(get_server_info()),
        "tools/list" => Ok(get_tools_list()),
        "tools/call" => {
            if let Some(params) = request.params {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let empty_args = json!({});
                let args = params.get("arguments").unwrap_or(&empty_args);
                match handle_tool_call(client, name, args).await {
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
                    }))
                }
            } else {
                Err("Missing params")
            }
        }
        _ => Err("Method not found")
    };

    Some(match result {
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
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Use environment variable or default to localhost
    let api_url = std::env::var("LASTMILE_API_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let client = ApiClient::new(&api_url);

    // MCP servers should be silent on startup - no stderr output
    // Debug info only when LASTMILE_DEBUG is set
    if std::env::var("LASTMILE_DEBUG").is_ok() {
        eprintln!("Last-Mile Analytics MCP Server v2.0.0");
        eprintln!("API URL: {}", api_url);
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                // Only send response if not a notification
                if let Some(response) = handle_request(&client, request).await {
                    let response_json = serde_json::to_string(&response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                // Don't write to stderr - return JSON-RPC error instead
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                let response_json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
