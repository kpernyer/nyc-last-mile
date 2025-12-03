//! MCP Server for Last-Mile Analytics
//!
//! Exposes lane clustering analytics via Model Context Protocol (MCP)
//! for integration with LLMs like Claude Desktop.
//!
//! Run: ./target/release/mcp_server
//!
//! Tools exposed:
//! - get_lane_clusters: Returns all behavioral clusters with statistics
//! - get_lanes_in_cluster: Lists lanes in a specific cluster
//! - get_lane_profile: Get metrics and cluster assignment for a specific lane
//! - get_cluster_playbook: Get recommended actions for a cluster
//! - find_similar_lanes: Find lanes with similar behavior patterns
//!
//! Configure in Claude Desktop's settings as a stdio MCP server.

use anyhow::Result;
use nyc_last_mile::{db, location_names::format_lane_short};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

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
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct LaneMetricsRaw {
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    avg_delay: f64,
    transit_variance: f64,
    early_count: i64,
    ontime_count: i64,
    late_count: i64,
}

// Note: Early delivery analysis uses cached lane data to avoid DB lock issues

#[derive(Debug, Clone, Serialize)]
struct LaneMetrics {
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    avg_delay: f64,
    transit_variance: f64,
    early_rate: f64,
    on_time_rate: f64,
    late_rate: f64,
    cluster_id: u8,
    cluster_name: String,
}

// ============================================================================
// Cluster Definitions
// ============================================================================

struct ClusterDef {
    id: u8,
    name: &'static str,
    description: &'static str,
    playbook: Vec<&'static str>,
}

fn get_cluster_definitions() -> Vec<ClusterDef> {
    vec![
        ClusterDef {
            id: 1,
            name: "Early & Stable",
            description: "Consistently arrive 0.5-2 days early with low variance",
            playbook: vec![
                "Implement hold-until policies at local depot",
                "Offer tight customer delivery windows",
                "Consider tightening SLA promises (reduce buffer)",
                "Use for premium time-slot offerings",
            ],
        },
        ClusterDef {
            id: 2,
            name: "On-Time & Reliable",
            description: "High on-time rate with predictable transit",
            playbook: vec![
                "Maintain current operations - these are your best lanes",
                "Use as benchmark for other lanes",
                "Suitable for guaranteed delivery promises",
                "Monitor for degradation, protect capacity",
            ],
        },
        ClusterDef {
            id: 3,
            name: "High-Jitter",
            description: "Average is OK but high variance - unpredictable",
            playbook: vec![
                "Add buffer days to customer promises",
                "Avoid 'guaranteed by noon' commitments",
                "Route to lockers/pickup points to handle timing uncertainty",
                "Investigate root cause: carrier issues? weather corridors?",
            ],
        },
        ClusterDef {
            id: 4,
            name: "Systematically Late",
            description: "Consistently miss SLA - structural problem",
            playbook: vec![
                "Downgrade promise (next-day to 2-day) for these lanes",
                "Negotiate with carriers or switch providers",
                "Consider pre-positioning inventory closer to destination",
                "Flag for carrier performance review",
            ],
        },
        ClusterDef {
            id: 5,
            name: "Low Volume / Mixed",
            description: "Insufficient data or mixed patterns",
            playbook: vec![
                "Apply conservative SLA buffers",
                "Monitor as volume grows",
                "Consider consolidating with similar lanes",
                "Default to standard operating procedures",
            ],
        },
    ]
}

fn assign_cluster(avg_delay: f64, transit_variance: f64, early_rate: f64, on_time_rate: f64, late_rate: f64, volume: i64) -> (u8, &'static str) {
    if volume < 20 {
        return (5, "Low Volume / Mixed");
    }
    if avg_delay < -0.3 && transit_variance < 2.0 && early_rate > 0.3 {
        return (1, "Early & Stable");
    }
    if late_rate > 0.45 {
        return (4, "Systematically Late");
    }
    if transit_variance > 3.5 {
        return (3, "High-Jitter");
    }
    if on_time_rate > 0.55 && transit_variance < 2.5 {
        return (2, "On-Time & Reliable");
    }
    (5, "Low Volume / Mixed")
}

// ============================================================================
// Analytics Service
// ============================================================================

struct AnalyticsService {
    db_path: String,
    cached_lanes: Arc<RwLock<Option<Vec<LaneMetrics>>>>,
}

impl AnalyticsService {
    fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
            cached_lanes: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_lanes(&self) -> Result<Vec<LaneMetrics>> {
        {
            let cache = self.cached_lanes.read().await;
            if let Some(lanes) = cache.as_ref() {
                return Ok(lanes.clone());
            }
        }

        let db = db::connect(&self.db_path).await?;

        let lanes_raw: Vec<LaneMetricsRaw> = db
            .query(r#"
                SELECT
                    origin_zip,
                    dest_zip,
                    count() as volume,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    math::variance(actual_transit_days) as transit_variance,
                    count(IF otd = "Early" THEN 1 END) as early_count,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count,
                    count(IF otd = "Late" THEN 1 END) as late_count
                FROM shipment
                GROUP BY origin_zip, dest_zip
            "#)
            .await?
            .take(0)?;

        let lanes: Vec<LaneMetrics> = lanes_raw
            .into_iter()
            .map(|raw| {
                let vol = raw.volume as f64;
                let early_rate = raw.early_count as f64 / vol;
                let on_time_rate = raw.ontime_count as f64 / vol;
                let late_rate = raw.late_count as f64 / vol;
                let (cluster_id, cluster_name) = assign_cluster(
                    raw.avg_delay, raw.transit_variance, early_rate, on_time_rate, late_rate, raw.volume
                );
                LaneMetrics {
                    origin_zip: raw.origin_zip,
                    dest_zip: raw.dest_zip,
                    volume: raw.volume,
                    avg_delay: raw.avg_delay,
                    transit_variance: raw.transit_variance,
                    early_rate,
                    on_time_rate,
                    late_rate,
                    cluster_id,
                    cluster_name: cluster_name.to_string(),
                }
            })
            .collect();

        {
            let mut cache = self.cached_lanes.write().await;
            *cache = Some(lanes.clone());
        }

        Ok(lanes)
    }

    async fn get_lane_clusters(&self) -> Result<Value> {
        let lanes = self.get_lanes().await?;
        let definitions = get_cluster_definitions();

        let clusters: Vec<Value> = definitions.iter().map(|def| {
            let cluster_lanes: Vec<&LaneMetrics> = lanes
                .iter()
                .filter(|l| l.cluster_id == def.id)
                .collect();

            let lane_count = cluster_lanes.len();
            let total_volume: i64 = cluster_lanes.iter().map(|l| l.volume).sum();
            let avg_delay = if lane_count > 0 {
                cluster_lanes.iter().map(|l| l.avg_delay).sum::<f64>() / lane_count as f64
            } else { 0.0 };
            let avg_late_rate = if lane_count > 0 {
                cluster_lanes.iter().map(|l| l.late_rate).sum::<f64>() / lane_count as f64
            } else { 0.0 };

            json!({
                "id": def.id,
                "name": def.name,
                "description": def.description,
                "lane_count": lane_count,
                "total_volume": total_volume,
                "avg_delay_days": (avg_delay * 100.0).round() / 100.0,
                "avg_late_rate_pct": (avg_late_rate * 1000.0).round() / 10.0
            })
        }).collect();

        Ok(json!(clusters))
    }

    async fn get_lanes_in_cluster(&self, cluster_id: u8, limit: usize) -> Result<Value> {
        let lanes = self.get_lanes().await?;

        let mut cluster_lanes: Vec<&LaneMetrics> = lanes
            .iter()
            .filter(|l| l.cluster_id == cluster_id)
            .collect();

        cluster_lanes.sort_by(|a, b| b.volume.cmp(&a.volume));

        let output: Vec<Value> = cluster_lanes.into_iter().take(limit).map(|l| {
            json!({
                "route": format_lane_short(&l.origin_zip, &l.dest_zip),
                "volume": l.volume,
                "avg_delay_days": (l.avg_delay * 100.0).round() / 100.0,
                "early_pct": (l.early_rate * 1000.0).round() / 10.0,
                "on_time_pct": (l.on_time_rate * 1000.0).round() / 10.0,
                "late_pct": (l.late_rate * 1000.0).round() / 10.0
            })
        }).collect();

        Ok(json!(output))
    }

    async fn get_lane_profile(&self, origin: &str, dest: &str) -> Result<Value> {
        let lanes = self.get_lanes().await?;
        let origin_lower = origin.to_lowercase();
        let dest_lower = dest.to_lowercase();

        let lane = lanes.iter().find(|l| {
            let route = format_lane_short(&l.origin_zip, &l.dest_zip).to_lowercase();
            (l.origin_zip.to_lowercase().contains(&origin_lower) || route.contains(&origin_lower)) &&
            (l.dest_zip.to_lowercase().contains(&dest_lower) || route.contains(&dest_lower))
        });

        match lane {
            Some(l) => Ok(json!({
                "route": format_lane_short(&l.origin_zip, &l.dest_zip),
                "origin_zip": l.origin_zip,
                "dest_zip": l.dest_zip,
                "cluster_id": l.cluster_id,
                "cluster_name": l.cluster_name,
                "volume": l.volume,
                "avg_delay_days": (l.avg_delay * 100.0).round() / 100.0,
                "transit_variance": (l.transit_variance * 100.0).round() / 100.0,
                "early_pct": (l.early_rate * 1000.0).round() / 10.0,
                "on_time_pct": (l.on_time_rate * 1000.0).round() / 10.0,
                "late_pct": (l.late_rate * 1000.0).round() / 10.0
            })),
            None => Ok(json!({
                "error": format!("Lane not found matching origin='{}' and dest='{}'", origin, dest)
            }))
        }
    }

    fn get_cluster_playbook(&self, cluster_id: u8) -> Value {
        let definitions = get_cluster_definitions();
        let cluster = definitions.into_iter().find(|d| d.id == cluster_id);

        match cluster {
            Some(def) => json!({
                "cluster_id": def.id,
                "cluster_name": def.name,
                "description": def.description,
                "recommended_actions": def.playbook
            }),
            None => json!({
                "error": format!("Cluster {} not found. Valid IDs: 1-5", cluster_id)
            })
        }
    }

    async fn find_similar_lanes(&self, pattern: &str, limit: usize) -> Result<Value> {
        let lanes = self.get_lanes().await?;
        let pattern_lower = pattern.to_lowercase();

        let target = lanes.iter().find(|l| {
            let route = format_lane_short(&l.origin_zip, &l.dest_zip).to_lowercase();
            route.contains(&pattern_lower) ||
            l.origin_zip.to_lowercase().contains(&pattern_lower) ||
            l.dest_zip.to_lowercase().contains(&pattern_lower)
        });

        match target {
            Some(target_lane) => {
                let mut similar: Vec<&LaneMetrics> = lanes
                    .iter()
                    .filter(|l| l.cluster_id == target_lane.cluster_id &&
                               !(l.origin_zip == target_lane.origin_zip && l.dest_zip == target_lane.dest_zip))
                    .collect();

                similar.sort_by(|a, b| b.volume.cmp(&a.volume));

                let similar_output: Vec<Value> = similar.into_iter().take(limit).map(|l| {
                    json!({
                        "route": format_lane_short(&l.origin_zip, &l.dest_zip),
                        "volume": l.volume,
                        "avg_delay_days": (l.avg_delay * 100.0).round() / 100.0,
                        "late_pct": (l.late_rate * 1000.0).round() / 10.0
                    })
                }).collect();

                Ok(json!({
                    "target_lane": {
                        "route": format_lane_short(&target_lane.origin_zip, &target_lane.dest_zip),
                        "cluster_name": target_lane.cluster_name,
                        "volume": target_lane.volume,
                        "late_pct": (target_lane.late_rate * 1000.0).round() / 10.0
                    },
                    "similar_lanes": similar_output,
                    "shared_playbook": target_lane.cluster_name
                }))
            }
            None => Ok(json!({
                "error": format!("No lane found matching '{}'. Try a ZIP3 code like '750' or location name like 'DFW'.", pattern)
            }))
        }
    }

    async fn get_early_delivery_analysis(&self) -> Result<Value> {
        // Get stats from cached lane data (avoids opening second DB connection)
        let lanes = self.get_lanes().await?;

        // Calculate overall totals
        let total: i64 = lanes.iter().map(|l| l.volume).sum();
        let early_count: i64 = lanes.iter().map(|l| (l.early_rate * l.volume as f64) as i64).sum();

        // Aggregate by destination ZIP
        let mut dest_stats: std::collections::HashMap<String, (i64, i64, f64)> = std::collections::HashMap::new();
        for lane in &lanes {
            let entry = dest_stats.entry(lane.dest_zip.clone()).or_insert((0, 0, 0.0));
            entry.0 += lane.volume;  // total volume
            entry.1 += (lane.early_rate * lane.volume as f64) as i64;  // early count
            if lane.avg_delay < 0.0 {
                entry.2 = lane.avg_delay.abs();  // avg days early (negative delay = early)
            }
        }

        // Sort destinations by early count descending
        let mut by_dest: Vec<(String, i64, i64, f64)> = dest_stats
            .into_iter()
            .map(|(zip, (vol, early, days))| (zip, vol, early, days))
            .collect();
        by_dest.sort_by(|a, b| b.2.cmp(&a.2));

        // Build destination output
        let dest_output: Vec<Value> = by_dest.iter().take(10).map(|(zip, vol, early, days)| {
            let early_rate = if *vol > 0 { *early as f64 / *vol as f64 * 100.0 } else { 0.0 };
            json!({
                "dest_zip": zip,
                "location": format_lane_short("", zip).trim_start_matches(" → "),
                "early_rate_pct": (early_rate * 10.0).round() / 10.0,
                "avg_days_early": (*days * 10.0).round() / 10.0,
                "early_shipments": early,
                "volume": vol
            })
        }).collect();

        // Calculate overall rates
        let early_rate = if total > 0 { early_count as f64 / total as f64 * 100.0 } else { 0.0 };

        Ok(json!({
            "summary": {
                "total_shipments": total,
                "early_shipments": early_count,
                "early_rate_pct": (early_rate * 10.0).round() / 10.0,
                "definition": "Early = delivered before goal transit days"
            },
            "top_early_destinations": dest_output,
            "early_lane_clusters": {
                "cluster_1_early_stable": {
                    "lane_count": lanes.iter().filter(|l| l.cluster_id == 1).count(),
                    "total_volume": lanes.iter().filter(|l| l.cluster_id == 1).map(|l| l.volume).sum::<i64>(),
                    "description": "These lanes consistently deliver early - ideal for hold-until policies"
                }
            },
            "recommendations": [
                "Consider hold-until policies for Early & Stable lanes to reduce storage costs",
                "Destinations with high early rates may benefit from tighter SLA windows",
                "Review carrier contracts - early deliveries may indicate over-provisioned transit times"
            ]
        }))
    }

    async fn get_regional_performance(&self, zip3: &str) -> Result<Value> {
        let lanes = self.get_lanes().await?;
        let zip3_lower = zip3.to_lowercase();

        // Find all lanes involving this region (as origin or destination)
        let regional_lanes: Vec<&LaneMetrics> = lanes
            .iter()
            .filter(|l| {
                let route = format_lane_short(&l.origin_zip, &l.dest_zip).to_lowercase();
                l.origin_zip.to_lowercase().contains(&zip3_lower) ||
                l.dest_zip.to_lowercase().contains(&zip3_lower) ||
                route.contains(&zip3_lower)
            })
            .collect();

        if regional_lanes.is_empty() {
            return Ok(json!({
                "error": format!("No lanes found for region '{}'. Try a ZIP3 like '750' or location like 'DFW'.", zip3)
            }));
        }

        // Calculate regional stats
        let total_volume: i64 = regional_lanes.iter().map(|l| l.volume).sum();
        let total_lanes = regional_lanes.len();
        let avg_late_rate: f64 = regional_lanes.iter().map(|l| l.late_rate).sum::<f64>() / total_lanes as f64;
        let avg_early_rate: f64 = regional_lanes.iter().map(|l| l.early_rate).sum::<f64>() / total_lanes as f64;
        let avg_delay: f64 = regional_lanes.iter().map(|l| l.avg_delay).sum::<f64>() / total_lanes as f64;

        // Count by cluster
        let cluster_breakdown: Vec<Value> = (1..=5).map(|cid| {
            let count = regional_lanes.iter().filter(|l| l.cluster_id == cid).count();
            let vol: i64 = regional_lanes.iter().filter(|l| l.cluster_id == cid).map(|l| l.volume).sum();
            let name = match cid {
                1 => "Early & Stable",
                2 => "On-Time & Reliable",
                3 => "High-Jitter",
                4 => "Systematically Late",
                _ => "Low Volume / Mixed"
            };
            json!({
                "cluster": name,
                "lane_count": count,
                "volume": vol
            })
        }).collect();

        // Top problem lanes (highest late rate with decent volume)
        let mut problem_lanes: Vec<&LaneMetrics> = regional_lanes
            .iter()
            .filter(|l| l.volume >= 10)
            .copied()
            .collect();
        problem_lanes.sort_by(|a, b| b.late_rate.partial_cmp(&a.late_rate).unwrap_or(std::cmp::Ordering::Equal));

        let problem_output: Vec<Value> = problem_lanes.iter().take(5).map(|l| {
            json!({
                "route": format_lane_short(&l.origin_zip, &l.dest_zip),
                "late_pct": (l.late_rate * 1000.0).round() / 10.0,
                "volume": l.volume,
                "cluster": &l.cluster_name
            })
        }).collect();

        Ok(json!({
            "region": zip3,
            "summary": {
                "total_lanes": total_lanes,
                "total_volume": total_volume,
                "avg_late_rate_pct": (avg_late_rate * 1000.0).round() / 10.0,
                "avg_early_rate_pct": (avg_early_rate * 1000.0).round() / 10.0,
                "avg_delay_days": (avg_delay * 100.0).round() / 100.0
            },
            "cluster_breakdown": cluster_breakdown,
            "highest_friction_lanes": problem_output
        }))
    }

    async fn get_friction_zones(&self, limit: usize) -> Result<Value> {
        let lanes = self.get_lanes().await?;

        // Aggregate by destination ZIP3
        let mut dest_stats: std::collections::HashMap<String, (i64, f64, f64, i64)> = std::collections::HashMap::new();
        for lane in &lanes {
            let entry = dest_stats.entry(lane.dest_zip.clone()).or_insert((0, 0.0, 0.0, 0));
            entry.0 += lane.volume;
            entry.1 += lane.late_rate * lane.volume as f64;  // weighted late
            entry.2 += lane.transit_variance * lane.volume as f64;  // weighted variance
            entry.3 += 1;  // lane count
        }

        // Calculate weighted averages and find friction zones
        let mut friction_zones: Vec<(String, i64, f64, f64, i64)> = dest_stats
            .into_iter()
            .filter(|(_, (vol, _, _, _))| *vol >= 100)  // minimum volume threshold
            .map(|(zip, (vol, late_sum, var_sum, count))| {
                let avg_late = late_sum / vol as f64;
                let avg_var = var_sum / vol as f64;
                (zip, vol, avg_late, avg_var, count)
            })
            .collect();

        // Sort by late rate descending (friction = high late rate)
        friction_zones.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let friction_output: Vec<Value> = friction_zones.iter().take(limit).map(|(zip, vol, late, var, lanes)| {
            let friction_score = (late * 100.0 + var * 10.0).round() / 10.0;  // Custom friction formula
            json!({
                "dest_zip": zip,
                "location": format_lane_short("", zip).trim_start_matches(" → "),
                "friction_score": friction_score,
                "late_rate_pct": (late * 1000.0).round() / 10.0,
                "transit_variance": (*var * 100.0).round() / 100.0,
                "volume": vol,
                "lane_count": lanes
            })
        }).collect();

        Ok(json!({
            "description": "Friction zones are destinations with high late rates and transit variance",
            "friction_score_formula": "late_rate% + (variance * 10)",
            "friction_zones": friction_output,
            "recommendations": [
                "High-friction zones may need carrier renegotiation",
                "Consider alternative routing or pre-positioning inventory",
                "Increase SLA buffer for these destinations"
            ]
        }))
    }

    async fn get_terminal_performance(&self, limit: usize) -> Result<Value> {
        let lanes = self.get_lanes().await?;

        // Aggregate by origin (terminal/DC)
        let mut origin_stats: std::collections::HashMap<String, (i64, f64, f64, f64, i64)> = std::collections::HashMap::new();
        for lane in &lanes {
            let entry = origin_stats.entry(lane.origin_zip.clone()).or_insert((0, 0.0, 0.0, 0.0, 0));
            entry.0 += lane.volume;  // total volume
            entry.1 += lane.late_rate * lane.volume as f64;  // weighted late
            entry.2 += lane.early_rate * lane.volume as f64;  // weighted early
            entry.3 += lane.on_time_rate * lane.volume as f64;  // weighted on-time
            entry.4 += 1;  // lane count
        }

        // Calculate weighted averages and performance scores
        let mut terminals: Vec<(String, i64, f64, f64, f64, i64, f64)> = origin_stats
            .into_iter()
            .filter(|(_, (vol, _, _, _, _))| *vol >= 50)  // minimum volume
            .map(|(zip, (vol, late_sum, early_sum, ontime_sum, count))| {
                let late_rate = late_sum / vol as f64;
                let early_rate = early_sum / vol as f64;
                let ontime_rate = ontime_sum / vol as f64;
                // Performance score: higher is better (100 = perfect on-time, 0 = all late)
                let score = ((1.0 - late_rate) * 100.0).round();
                (zip, vol, late_rate, early_rate, ontime_rate, count, score)
            })
            .collect();

        // Sort by performance score descending (best terminals first)
        terminals.sort_by(|a, b| b.6.partial_cmp(&a.6).unwrap_or(std::cmp::Ordering::Equal));

        // Best performers
        let best_output: Vec<Value> = terminals.iter().take(limit).map(|(zip, vol, late, early, ontime, lanes, score)| {
            json!({
                "origin_zip": zip,
                "terminal": format_lane_short(zip, "").trim_end_matches(" → "),
                "performance_score": score,
                "on_time_rate_pct": (*ontime * 1000.0).round() / 10.0,
                "late_rate_pct": (*late * 1000.0).round() / 10.0,
                "early_rate_pct": (*early * 1000.0).round() / 10.0,
                "volume": vol,
                "lane_count": lanes
            })
        }).collect();

        // Worst performers (reverse order)
        let mut worst_terminals = terminals.clone();
        worst_terminals.sort_by(|a, b| a.6.partial_cmp(&b.6).unwrap_or(std::cmp::Ordering::Equal));

        let worst_output: Vec<Value> = worst_terminals.iter().take(limit).map(|(zip, vol, late, early, ontime, lanes, score)| {
            json!({
                "origin_zip": zip,
                "terminal": format_lane_short(zip, "").trim_end_matches(" → "),
                "performance_score": score,
                "on_time_rate_pct": (*ontime * 1000.0).round() / 10.0,
                "late_rate_pct": (*late * 1000.0).round() / 10.0,
                "volume": vol,
                "lane_count": lanes
            })
        }).collect();

        // Calculate overall network stats
        let total_volume: i64 = terminals.iter().map(|(_, vol, _, _, _, _, _)| *vol).sum();
        let avg_score: f64 = terminals.iter().map(|(_, _, _, _, _, _, score)| *score).sum::<f64>() / terminals.len() as f64;

        Ok(json!({
            "network_summary": {
                "total_terminals": terminals.len(),
                "total_volume": total_volume,
                "average_performance_score": (avg_score * 10.0).round() / 10.0,
                "score_definition": "100 = all on-time/early, 0 = all late"
            },
            "top_performers": best_output,
            "needs_improvement": worst_output,
            "recommendations": [
                "Terminals scoring below 70 may need capacity review",
                "Consider load balancing from low-performers to high-performers",
                "Review carrier mix at underperforming terminals"
            ]
        }))
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
            "version": "1.0.0"
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

async fn handle_tool_call(service: &AnalyticsService, name: &str, args: &Value) -> Result<Value> {
    match name {
        "get_lane_clusters" => service.get_lane_clusters().await,
        "get_lanes_in_cluster" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            service.get_lanes_in_cluster(cluster_id, limit).await
        }
        "get_lane_profile" => {
            let origin = args.get("origin").and_then(|v| v.as_str()).unwrap_or("");
            let dest = args.get("dest").and_then(|v| v.as_str()).unwrap_or("");
            service.get_lane_profile(origin, dest).await
        }
        "get_cluster_playbook" => {
            let cluster_id = args.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            Ok(service.get_cluster_playbook(cluster_id))
        }
        "find_similar_lanes" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            service.find_similar_lanes(pattern, limit).await
        }
        "get_early_delivery_analysis" => service.get_early_delivery_analysis().await,
        "get_regional_performance" => {
            let zip3 = args.get("zip3").and_then(|v| v.as_str()).unwrap_or("");
            service.get_regional_performance(zip3).await
        }
        "get_friction_zones" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            service.get_friction_zones(limit).await
        }
        "get_terminal_performance" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            service.get_terminal_performance(limit).await
        }
        _ => Ok(json!({"error": format!("Unknown tool: {}", name)}))
    }
}

async fn handle_request(service: &AnalyticsService, request: JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        "initialize" => Ok(get_server_info()),
        "tools/list" => Ok(get_tools_list()),
        "tools/call" => {
            if let Some(params) = request.params {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let empty_args = json!({});
                let args = params.get("arguments").unwrap_or(&empty_args);
                match handle_tool_call(service, name, args).await {
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
        "notifications/initialized" => return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: None,
        },
        _ => Err("Method not found")
    };

    match result {
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
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Use environment variable or default to Application Support (sandbox-friendly)
    let db_path = std::env::var("LASTMILE_DB_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/Library/Application Support/LastMileAnalytics/lastmile.db", home)
    });
    let service = AnalyticsService::new(&db_path);

    // MCP servers should be silent on startup - no stderr output
    // Debug info only when LASTMILE_DEBUG is set
    if std::env::var("LASTMILE_DEBUG").is_ok() {
        eprintln!("Last-Mile Analytics MCP Server started");
        eprintln!("Database: {}", db_path);
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
                let response = handle_request(&service, request).await;
                let response_json = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
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
