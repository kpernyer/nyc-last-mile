//! Shared business logic for the analytics API
//!
//! This service layer is used by both REST and gRPC handlers.

use anyhow::Result;
use crate::{db, location_names::format_lane_short};
use crate::carrier_names::get_carrier_name;
use crate::location_names::get_location_long;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// Re-export graph response types
pub use super::graph_handlers::{
    CarrierNetworkResponse, CarrierLane, LocationConnectionsResponse, ConnectionStats, Connection,
    NetworkTopologyResponse, NodeCounts, EdgeCounts, NetworkDensity,
    ShipmentTraceResponse, ShipmentInfo, CarrierInfo, LocationInfo, LaneInfo,
    ReachableDestinationsResponse, ReachableDestination,
};

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

#[derive(Debug, Clone)]
pub struct LaneMetrics {
    pub origin_zip: String,
    pub dest_zip: String,
    pub route: String,
    pub volume: i64,
    pub avg_delay: f64,
    pub transit_variance: f64,
    pub early_rate: f64,
    pub on_time_rate: f64,
    pub late_rate: f64,
    pub cluster_id: u8,
    pub cluster_name: String,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub lane_count: usize,
    pub total_volume: i64,
    pub avg_delay: f64,
    pub avg_late_rate: f64,
}

#[derive(Debug, Clone)]
pub struct Playbook {
    pub cluster_id: u8,
    pub cluster_name: String,
    pub description: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FrictionZone {
    pub dest_zip: String,
    pub location: String,
    pub friction_score: f64,
    pub late_rate: f64,
    pub transit_variance: f64,
    pub volume: i64,
    pub lane_count: i64,
}

#[derive(Debug, Clone)]
pub struct TerminalPerformance {
    pub origin_zip: String,
    pub terminal: String,
    pub performance_score: f64,
    pub on_time_rate: f64,
    pub late_rate: f64,
    pub early_rate: f64,
    pub volume: i64,
    pub lane_count: i64,
}

#[derive(Debug, Clone)]
pub struct RegionalPerformance {
    pub region: String,
    pub total_lanes: usize,
    pub total_volume: i64,
    pub avg_late_rate: f64,
    pub avg_early_rate: f64,
    pub avg_delay: f64,
    pub cluster_breakdown: Vec<ClusterBreakdown>,
    pub highest_friction_lanes: Vec<LaneMetrics>,
}

#[derive(Debug, Clone)]
pub struct ClusterBreakdown {
    pub cluster: String,
    pub lane_count: usize,
    pub volume: i64,
}

#[derive(Debug, Clone)]
pub struct EarlyAnalysis {
    pub total_shipments: i64,
    pub early_shipments: i64,
    pub early_rate: f64,
    pub top_destinations: Vec<EarlyDestination>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EarlyDestination {
    pub dest_zip: String,
    pub location: String,
    pub early_rate: f64,
    pub avg_days_early: f64,
    pub early_shipments: i64,
    pub volume: i64,
}

#[derive(Debug, Clone)]
pub struct SimilarLanesResult {
    pub target_lane: Option<LaneMetrics>,
    pub similar_lanes: Vec<LaneMetrics>,
    pub shared_playbook: String,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_shipments: i64,
    pub total_lanes: i64,
    pub total_carriers: i64,
    pub total_locations: i64,
    pub overall_on_time_rate: f64,
    pub overall_late_rate: f64,
    pub overall_early_rate: f64,
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

pub struct AnalyticsService {
    db_path: String,
    cached_lanes: Arc<RwLock<Option<Vec<LaneMetrics>>>>,
}

impl AnalyticsService {
    pub fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
            cached_lanes: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_lanes(&self) -> Result<Vec<LaneMetrics>> {
        // Check cache first
        {
            let cache = self.cached_lanes.read().await;
            if let Some(lanes) = cache.as_ref() {
                return Ok(lanes.clone());
            }
        }

        // Query database
        let db = db::connect(&self.db_path).await?;

        let lanes_raw: Vec<LaneMetricsRaw> = db
            .query(r#"
                SELECT
                    ->origin_at->location.zip3 as origin_zip,
                    ->dest_at->location.zip3 as dest_zip,
                    count() as volume,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    math::variance(actual_transit_days) as transit_variance,
                    count(IF otd = "Early" THEN 1 END) as early_count,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count,
                    count(IF otd = "Late" THEN 1 END) as late_count
                FROM shipment
                GROUP BY ->origin_at->location.zip3, ->dest_at->location.zip3
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
                    origin_zip: raw.origin_zip.clone(),
                    dest_zip: raw.dest_zip.clone(),
                    route: format_lane_short(&raw.origin_zip, &raw.dest_zip),
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

        // Update cache
        {
            let mut cache = self.cached_lanes.write().await;
            *cache = Some(lanes.clone());
        }

        Ok(lanes)
    }

    pub async fn get_clusters(&self) -> Result<Vec<Cluster>> {
        let lanes = self.get_lanes().await?;
        let definitions = get_cluster_definitions();

        let clusters: Vec<Cluster> = definitions.iter().map(|def| {
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

            Cluster {
                id: def.id,
                name: def.name.to_string(),
                description: def.description.to_string(),
                lane_count,
                total_volume,
                avg_delay: (avg_delay * 100.0).round() / 100.0,
                avg_late_rate: (avg_late_rate * 1000.0).round() / 10.0,
            }
        }).collect();

        Ok(clusters)
    }

    pub async fn get_lanes_in_cluster(&self, cluster_id: u8, limit: usize) -> Result<Vec<LaneMetrics>> {
        let lanes = self.get_lanes().await?;

        let mut cluster_lanes: Vec<LaneMetrics> = lanes
            .into_iter()
            .filter(|l| l.cluster_id == cluster_id)
            .collect();

        cluster_lanes.sort_by(|a, b| b.volume.cmp(&a.volume));
        cluster_lanes.truncate(limit);

        Ok(cluster_lanes)
    }

    pub async fn get_lane_profile(&self, origin: &str, dest: &str) -> Result<Option<LaneMetrics>> {
        let lanes = self.get_lanes().await?;
        let origin_lower = origin.to_lowercase();
        let dest_lower = dest.to_lowercase();

        let lane = lanes.into_iter().find(|l| {
            let route = l.route.to_lowercase();
            (l.origin_zip.to_lowercase().contains(&origin_lower) || route.contains(&origin_lower)) &&
            (l.dest_zip.to_lowercase().contains(&dest_lower) || route.contains(&dest_lower))
        });

        Ok(lane)
    }

    pub fn get_playbook(&self, cluster_id: u8) -> Option<Playbook> {
        let definitions = get_cluster_definitions();
        definitions.into_iter().find(|d| d.id == cluster_id).map(|def| {
            Playbook {
                cluster_id: def.id,
                cluster_name: def.name.to_string(),
                description: def.description.to_string(),
                actions: def.playbook.iter().map(|s| s.to_string()).collect(),
            }
        })
    }

    pub async fn find_similar_lanes(&self, pattern: &str, limit: usize) -> Result<SimilarLanesResult> {
        let lanes = self.get_lanes().await?;
        let pattern_lower = pattern.to_lowercase();

        // Find the target lane and clone it immediately
        let target_lane = lanes.iter().find(|l| {
            let route = l.route.to_lowercase();
            route.contains(&pattern_lower) ||
            l.origin_zip.to_lowercase().contains(&pattern_lower) ||
            l.dest_zip.to_lowercase().contains(&pattern_lower)
        }).cloned();

        match target_lane {
            Some(target) => {
                let target_cluster = target.cluster_id;
                let target_origin = target.origin_zip.clone();
                let target_dest = target.dest_zip.clone();

                let mut similar: Vec<LaneMetrics> = lanes
                    .into_iter()
                    .filter(|l| l.cluster_id == target_cluster &&
                               !(l.origin_zip == target_origin && l.dest_zip == target_dest))
                    .collect();

                similar.sort_by(|a, b| b.volume.cmp(&a.volume));
                similar.truncate(limit);

                let playbook = target.cluster_name.clone();
                Ok(SimilarLanesResult {
                    target_lane: Some(target),
                    similar_lanes: similar,
                    shared_playbook: playbook,
                })
            }
            None => Ok(SimilarLanesResult {
                target_lane: None,
                similar_lanes: vec![],
                shared_playbook: String::new(),
            })
        }
    }

    pub async fn get_early_analysis(&self) -> Result<EarlyAnalysis> {
        let lanes = self.get_lanes().await?;

        let total: i64 = lanes.iter().map(|l| l.volume).sum();
        let early_count: i64 = lanes.iter().map(|l| (l.early_rate * l.volume as f64) as i64).sum();

        // Aggregate by destination ZIP
        let mut dest_stats: HashMap<String, (i64, i64, f64)> = HashMap::new();
        for lane in &lanes {
            let entry = dest_stats.entry(lane.dest_zip.clone()).or_insert((0, 0, 0.0));
            entry.0 += lane.volume;
            entry.1 += (lane.early_rate * lane.volume as f64) as i64;
            if lane.avg_delay < 0.0 {
                entry.2 = lane.avg_delay.abs();
            }
        }

        let mut by_dest: Vec<(String, i64, i64, f64)> = dest_stats
            .into_iter()
            .map(|(zip, (vol, early, days))| (zip, vol, early, days))
            .collect();
        by_dest.sort_by(|a, b| b.2.cmp(&a.2));

        let top_destinations: Vec<EarlyDestination> = by_dest.iter().take(10).map(|(zip, vol, early, days)| {
            let early_rate = if *vol > 0 { *early as f64 / *vol as f64 * 100.0 } else { 0.0 };
            EarlyDestination {
                dest_zip: zip.clone(),
                location: format_lane_short("", zip).trim_start_matches(" → ").to_string(),
                early_rate: (early_rate * 10.0).round() / 10.0,
                avg_days_early: (*days * 10.0).round() / 10.0,
                early_shipments: *early,
                volume: *vol,
            }
        }).collect();

        let early_rate = if total > 0 { early_count as f64 / total as f64 * 100.0 } else { 0.0 };

        Ok(EarlyAnalysis {
            total_shipments: total,
            early_shipments: early_count,
            early_rate: (early_rate * 10.0).round() / 10.0,
            top_destinations,
            recommendations: vec![
                "Consider hold-until policies for Early & Stable lanes to reduce storage costs".to_string(),
                "Destinations with high early rates may benefit from tighter SLA windows".to_string(),
                "Review carrier contracts - early deliveries may indicate over-provisioned transit times".to_string(),
            ],
        })
    }

    pub async fn get_regional_performance(&self, zip3: &str) -> Result<Option<RegionalPerformance>> {
        let lanes = self.get_lanes().await?;
        let zip3_lower = zip3.to_lowercase();

        let regional_lanes: Vec<&LaneMetrics> = lanes
            .iter()
            .filter(|l| {
                let route = l.route.to_lowercase();
                l.origin_zip.to_lowercase().contains(&zip3_lower) ||
                l.dest_zip.to_lowercase().contains(&zip3_lower) ||
                route.contains(&zip3_lower)
            })
            .collect();

        if regional_lanes.is_empty() {
            return Ok(None);
        }

        let total_volume: i64 = regional_lanes.iter().map(|l| l.volume).sum();
        let total_lanes = regional_lanes.len();
        let avg_late_rate: f64 = regional_lanes.iter().map(|l| l.late_rate).sum::<f64>() / total_lanes as f64;
        let avg_early_rate: f64 = regional_lanes.iter().map(|l| l.early_rate).sum::<f64>() / total_lanes as f64;
        let avg_delay: f64 = regional_lanes.iter().map(|l| l.avg_delay).sum::<f64>() / total_lanes as f64;

        let cluster_breakdown: Vec<ClusterBreakdown> = (1..=5).map(|cid| {
            let count = regional_lanes.iter().filter(|l| l.cluster_id == cid).count();
            let vol: i64 = regional_lanes.iter().filter(|l| l.cluster_id == cid).map(|l| l.volume).sum();
            let name = match cid {
                1 => "Early & Stable",
                2 => "On-Time & Reliable",
                3 => "High-Jitter",
                4 => "Systematically Late",
                _ => "Low Volume / Mixed"
            };
            ClusterBreakdown {
                cluster: name.to_string(),
                lane_count: count,
                volume: vol,
            }
        }).collect();

        let mut problem_lanes: Vec<&LaneMetrics> = regional_lanes
            .iter()
            .filter(|l| l.volume >= 10)
            .copied()
            .collect();
        problem_lanes.sort_by(|a, b| b.late_rate.partial_cmp(&a.late_rate).unwrap_or(std::cmp::Ordering::Equal));

        let highest_friction_lanes: Vec<LaneMetrics> = problem_lanes.iter().take(5).map(|l| (*l).clone()).collect();

        Ok(Some(RegionalPerformance {
            region: zip3.to_string(),
            total_lanes,
            total_volume,
            avg_late_rate: (avg_late_rate * 1000.0).round() / 10.0,
            avg_early_rate: (avg_early_rate * 1000.0).round() / 10.0,
            avg_delay: (avg_delay * 100.0).round() / 100.0,
            cluster_breakdown,
            highest_friction_lanes,
        }))
    }

    pub async fn get_friction_zones(&self, limit: usize) -> Result<Vec<FrictionZone>> {
        let lanes = self.get_lanes().await?;

        let mut dest_stats: HashMap<String, (i64, f64, f64, i64)> = HashMap::new();
        for lane in &lanes {
            let entry = dest_stats.entry(lane.dest_zip.clone()).or_insert((0, 0.0, 0.0, 0));
            entry.0 += lane.volume;
            entry.1 += lane.late_rate * lane.volume as f64;
            entry.2 += lane.transit_variance * lane.volume as f64;
            entry.3 += 1;
        }

        let mut friction_zones: Vec<FrictionZone> = dest_stats
            .into_iter()
            .filter(|(_, (vol, _, _, _))| *vol >= 100)
            .map(|(zip, (vol, late_sum, var_sum, count))| {
                let avg_late = late_sum / vol as f64;
                let avg_var = var_sum / vol as f64;
                let friction_score = (avg_late * 100.0 + avg_var * 10.0).round() / 10.0;
                FrictionZone {
                    dest_zip: zip.clone(),
                    location: format_lane_short("", &zip).trim_start_matches(" → ").to_string(),
                    friction_score,
                    late_rate: (avg_late * 1000.0).round() / 10.0,
                    transit_variance: (avg_var * 100.0).round() / 100.0,
                    volume: vol,
                    lane_count: count,
                }
            })
            .collect();

        friction_zones.sort_by(|a, b| b.friction_score.partial_cmp(&a.friction_score).unwrap_or(std::cmp::Ordering::Equal));
        friction_zones.truncate(limit);

        Ok(friction_zones)
    }

    pub async fn get_terminal_performance(&self, limit: usize) -> Result<(Vec<TerminalPerformance>, Vec<TerminalPerformance>, f64, i64, i64)> {
        let lanes = self.get_lanes().await?;

        let mut origin_stats: HashMap<String, (i64, f64, f64, f64, i64)> = HashMap::new();
        for lane in &lanes {
            let entry = origin_stats.entry(lane.origin_zip.clone()).or_insert((0, 0.0, 0.0, 0.0, 0));
            entry.0 += lane.volume;
            entry.1 += lane.late_rate * lane.volume as f64;
            entry.2 += lane.early_rate * lane.volume as f64;
            entry.3 += lane.on_time_rate * lane.volume as f64;
            entry.4 += 1;
        }

        let terminals: Vec<TerminalPerformance> = origin_stats
            .into_iter()
            .filter(|(_, (vol, _, _, _, _))| *vol >= 50)
            .map(|(zip, (vol, late_sum, early_sum, ontime_sum, count))| {
                let late_rate = late_sum / vol as f64;
                let early_rate = early_sum / vol as f64;
                let ontime_rate = ontime_sum / vol as f64;
                let score = ((1.0 - late_rate) * 100.0).round();
                TerminalPerformance {
                    origin_zip: zip.clone(),
                    terminal: format_lane_short(&zip, "").trim_end_matches(" → ").to_string(),
                    performance_score: score,
                    on_time_rate: (ontime_rate * 1000.0).round() / 10.0,
                    late_rate: (late_rate * 1000.0).round() / 10.0,
                    early_rate: (early_rate * 1000.0).round() / 10.0,
                    volume: vol,
                    lane_count: count,
                }
            })
            .collect();

        let total_volume: i64 = terminals.iter().map(|t| t.volume).sum();
        let avg_score: f64 = if !terminals.is_empty() {
            terminals.iter().map(|t| t.performance_score).sum::<f64>() / terminals.len() as f64
        } else { 0.0 };
        let total_terminals = terminals.len() as i64;

        let mut best = terminals.clone();
        best.sort_by(|a, b| b.performance_score.partial_cmp(&a.performance_score).unwrap_or(std::cmp::Ordering::Equal));
        best.truncate(limit);

        let mut worst = terminals;
        worst.sort_by(|a, b| a.performance_score.partial_cmp(&b.performance_score).unwrap_or(std::cmp::Ordering::Equal));
        worst.truncate(limit);

        Ok((best, worst, (avg_score * 10.0).round() / 10.0, total_volume, total_terminals))
    }

    pub async fn get_stats(&self) -> Result<Stats> {
        let lanes = self.get_lanes().await?;

        let total_volume: i64 = lanes.iter().map(|l| l.volume).sum();
        let total_lanes = lanes.len() as i64;

        let early_count: f64 = lanes.iter().map(|l| l.early_rate * l.volume as f64).sum();
        let ontime_count: f64 = lanes.iter().map(|l| l.on_time_rate * l.volume as f64).sum();
        let late_count: f64 = lanes.iter().map(|l| l.late_rate * l.volume as f64).sum();

        let overall_early_rate = if total_volume > 0 { early_count / total_volume as f64 * 100.0 } else { 0.0 };
        let overall_on_time_rate = if total_volume > 0 { ontime_count / total_volume as f64 * 100.0 } else { 0.0 };
        let overall_late_rate = if total_volume > 0 { late_count / total_volume as f64 * 100.0 } else { 0.0 };

        Ok(Stats {
            total_shipments: total_volume,
            total_lanes,
            total_carriers: 117, // From demo stats
            total_locations: 806, // From demo stats
            overall_on_time_rate: (overall_on_time_rate * 10.0).round() / 10.0,
            overall_late_rate: (overall_late_rate * 10.0).round() / 10.0,
            overall_early_rate: (overall_early_rate * 10.0).round() / 10.0,
        })
    }

    // ========================================================================
    // Graph-Oriented Methods
    // ========================================================================

    /// Get a carrier's operational network - lanes served, volume, and performance
    pub async fn get_carrier_network(&self, carrier_id: &str, limit: usize) -> Result<CarrierNetworkResponse> {
        let db = db::connect(&self.db_path).await?;
        let carrier_id_owned = carrier_id.to_string();

        #[derive(Debug, Deserialize)]
        struct LaneData {
            lane_zip5_pair: String,
            origin_zip5: String,
            dest_zip5: String,
            volume: i64,
            ontime_count: i64,
            avg_transit: f64,
        }

        let lanes: Vec<LaneData> = db
            .query(r#"
                SELECT
                    lane_zip5_pair,
                    origin_zip5,
                    dest_zip5,
                    count() as volume,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count,
                    math::mean(actual_transit_days) as avg_transit
                FROM shipment
                WHERE carrier_ref = $carrier_id
                GROUP BY lane_zip5_pair, origin_zip5, dest_zip5
                ORDER BY volume DESC
                LIMIT $limit
            "#)
            .bind(("carrier_id", carrier_id_owned.clone()))
            .bind(("limit", limit))
            .await?
            .take(0)?;

        let total_shipments: Option<i64> = db
            .query("SELECT count() FROM shipment WHERE carrier_ref = $carrier_id GROUP ALL")
            .bind(("carrier_id", carrier_id_owned))
            .await?
            .take("count")?;

        let mut origins: Vec<String> = lanes.iter().map(|l| l.origin_zip5.clone()).collect();
        let mut destinations: Vec<String> = lanes.iter().map(|l| l.dest_zip5.clone()).collect();
        origins.sort();
        origins.dedup();
        destinations.sort();
        destinations.dedup();

        let top_lanes: Vec<CarrierLane> = lanes
            .iter()
            .map(|l| {
                let otd_rate = if l.volume > 0 {
                    l.ontime_count as f64 / l.volume as f64
                } else {
                    0.0
                };
                CarrierLane {
                    lane: l.lane_zip5_pair.clone(),
                    origin: get_location_long(&l.origin_zip5),
                    destination: get_location_long(&l.dest_zip5),
                    volume: l.volume,
                    otd_rate: (otd_rate * 1000.0).round() / 10.0,
                    avg_transit: (l.avg_transit * 10.0).round() / 10.0,
                }
            })
            .collect();

        Ok(CarrierNetworkResponse {
            carrier_id: carrier_id.to_string(),
            display_name: get_carrier_name(carrier_id),
            total_shipments: total_shipments.unwrap_or(0),
            total_lanes: lanes.len(),
            origins,
            destinations,
            top_lanes,
        })
    }

    /// Get location connections - what ZIP5s are connected inbound/outbound
    pub async fn get_location_connections(&self, zip5: &str, direction: &str, limit: usize) -> Result<LocationConnectionsResponse> {
        let db = db::connect(&self.db_path).await?;
        let zip5_owned = zip5.to_string();

        #[derive(Debug, Deserialize)]
        struct ConnectionData {
            zip5: String,
            volume: i64,
            ontime_count: i64,
        }

        // Outbound connections (this ZIP5 as origin)
        let outbound_data: Vec<ConnectionData> = if direction == "both" || direction == "outbound" {
            db.query(r#"
                SELECT
                    dest_zip5 as zip5,
                    count() as volume,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count
                FROM shipment
                WHERE origin_zip5 = $zip5
                GROUP BY dest_zip5
                ORDER BY volume DESC
                LIMIT $limit
            "#)
            .bind(("zip5", zip5_owned.clone()))
            .bind(("limit", limit))
            .await?
            .take(0)?
        } else {
            vec![]
        };

        // Inbound connections (this ZIP5 as destination)
        let inbound_data: Vec<ConnectionData> = if direction == "both" || direction == "inbound" {
            db.query(r#"
                SELECT
                    origin_zip5 as zip5,
                    count() as volume,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count
                FROM shipment
                WHERE dest_zip5 = $zip5
                GROUP BY origin_zip5
                ORDER BY volume DESC
                LIMIT $limit
            "#)
            .bind(("zip5", zip5_owned.clone()))
            .bind(("limit", limit))
            .await?
            .take(0)?
        } else {
            vec![]
        };

        let outbound = ConnectionStats {
            total_destinations: outbound_data.len(),
            total_volume: outbound_data.iter().map(|c| c.volume).sum(),
            top_connections: outbound_data
                .into_iter()
                .map(|c| {
                    let otd_rate = if c.volume > 0 {
                        c.ontime_count as f64 / c.volume as f64
                    } else {
                        0.0
                    };
                    Connection {
                        zip5: c.zip5.clone(),
                        location: get_location_long(&c.zip5),
                        volume: c.volume,
                        otd_rate: (otd_rate * 1000.0).round() / 10.0,
                    }
                })
                .collect(),
        };

        let inbound = ConnectionStats {
            total_destinations: inbound_data.len(),
            total_volume: inbound_data.iter().map(|c| c.volume).sum(),
            top_connections: inbound_data
                .into_iter()
                .map(|c| {
                    let otd_rate = if c.volume > 0 {
                        c.ontime_count as f64 / c.volume as f64
                    } else {
                        0.0
                    };
                    Connection {
                        zip5: c.zip5.clone(),
                        location: get_location_long(&c.zip5),
                        volume: c.volume,
                        otd_rate: (otd_rate * 1000.0).round() / 10.0,
                    }
                })
                .collect(),
        };

        Ok(LocationConnectionsResponse {
            zip5: zip5.to_string(),
            location: get_location_long(zip5),
            outbound,
            inbound,
        })
    }

    /// Get network topology statistics - counts of nodes and edges
    pub async fn get_network_topology(&self) -> Result<NetworkTopologyResponse> {
        let db = db::connect(&self.db_path).await?;

        // Node counts
        let shipments: Option<i64> = db.query("SELECT count() FROM shipment GROUP ALL").await?.take("count")?;
        let carriers: Option<i64> = db.query("SELECT count() FROM carrier GROUP ALL").await?.take("count")?;
        let locations_zip3: Option<i64> = db.query("SELECT count() FROM location GROUP ALL").await?.take("count")?;
        let locations_zip5: Option<i64> = db.query("SELECT count() FROM location5 GROUP ALL").await?.take("count")?;
        let lanes_zip3: Option<i64> = db.query("SELECT count() FROM lane GROUP ALL").await?.take("count")?;
        let lanes_zip5: Option<i64> = db.query("SELECT count() FROM lane5 GROUP ALL").await?.take("count")?;

        // Edge counts (if graph edges exist)
        let shipped_by: Option<i64> = db.query("SELECT count() FROM shipped_by GROUP ALL").await?.take("count").unwrap_or(Some(0));
        let origin5_at: Option<i64> = db.query("SELECT count() FROM origin5_at GROUP ALL").await?.take("count").unwrap_or(Some(0));
        let dest5_at: Option<i64> = db.query("SELECT count() FROM dest5_at GROUP ALL").await?.take("count").unwrap_or(Some(0));
        let on_lane5: Option<i64> = db.query("SELECT count() FROM on_lane5 GROUP ALL").await?.take("count").unwrap_or(Some(0));
        let connects5: Option<i64> = db.query("SELECT count() FROM connects5 GROUP ALL").await?.take("count").unwrap_or(Some(0));

        let shipment_count = shipments.unwrap_or(0);
        let carrier_count = carriers.unwrap_or(0);
        let lane5_count = lanes_zip5.unwrap_or(0);

        // Calculate density metrics
        let avg_shipments_per_carrier = if carrier_count > 0 {
            shipment_count as f64 / carrier_count as f64
        } else {
            0.0
        };
        let avg_shipments_per_lane = if lane5_count > 0 {
            shipment_count as f64 / lane5_count as f64
        } else {
            0.0
        };

        // Estimate avg destinations per origin
        #[derive(Debug, Deserialize)]
        struct OriginDestCount {
            origin_count: i64,
            dest_count: i64,
        }
        let origin_dest: Option<OriginDestCount> = db
            .query(r#"
                SELECT
                    array::len(array::distinct(origin_zip5)) as origin_count,
                    array::len(array::distinct(dest_zip5)) as dest_count
                FROM shipment
                GROUP ALL
            "#)
            .await?
            .take(0)?;

        let avg_destinations_per_origin = match origin_dest {
            Some(od) if od.origin_count > 0 => od.dest_count as f64 / od.origin_count as f64,
            _ => 0.0,
        };

        Ok(NetworkTopologyResponse {
            nodes: NodeCounts {
                shipments: shipment_count,
                carriers: carrier_count,
                locations_zip3: locations_zip3.unwrap_or(0),
                locations_zip5: locations_zip5.unwrap_or(0),
                lanes_zip3: lanes_zip3.unwrap_or(0),
                lanes_zip5: lane5_count,
            },
            edges: EdgeCounts {
                shipped_by: shipped_by.unwrap_or(0),
                origin5_at: origin5_at.unwrap_or(0),
                dest5_at: dest5_at.unwrap_or(0),
                on_lane5: on_lane5.unwrap_or(0),
                connects5: connects5.unwrap_or(0),
            },
            density: NetworkDensity {
                avg_shipments_per_carrier: (avg_shipments_per_carrier * 10.0).round() / 10.0,
                avg_shipments_per_lane: (avg_shipments_per_lane * 100.0).round() / 100.0,
                avg_destinations_per_origin: (avg_destinations_per_origin * 10.0).round() / 10.0,
            },
        })
    }

    /// Trace a shipment through the graph - carrier, origin, destination, lane
    pub async fn trace_shipment(&self, load_id: &str) -> Result<Option<ShipmentTraceResponse>> {
        let db = db::connect(&self.db_path).await?;
        let load_id_owned = load_id.to_string();

        #[derive(Debug, Deserialize)]
        struct ShipmentData {
            load_id: String,
            carrier_mode: String,
            otd: String,
            actual_transit_days: i64,
            goal_transit_days: i64,
            actual_ship: String,
            actual_delivery: String,
            is_synthetic: Option<bool>,
            carrier_ref: String,
            origin_zip5: String,
            dest_zip5: String,
            lane_zip5_pair: String,
            origin_zip: String,
            dest_zip: String,
        }

        let shipment: Option<ShipmentData> = db
            .query(r#"
                SELECT
                    load_id, carrier_mode, otd, actual_transit_days, goal_transit_days,
                    actual_ship, actual_delivery, is_synthetic, carrier_ref,
                    origin_zip5, dest_zip5, lane_zip5_pair,
                    origin_zip, dest_zip
                FROM shipment
                WHERE load_id = $load_id
                LIMIT 1
            "#)
            .bind(("load_id", load_id_owned))
            .await?
            .take(0)?;

        match shipment {
            Some(s) => {
                let origin_zip3 = format!("{}xx", &s.origin_zip5[..3]);
                let dest_zip3 = format!("{}xx", &s.dest_zip5[..3]);
                let zip3_pair = format!("{}→{}", origin_zip3, dest_zip3);

                Ok(Some(ShipmentTraceResponse {
                    shipment: ShipmentInfo {
                        load_id: s.load_id,
                        carrier_mode: s.carrier_mode,
                        otd: s.otd,
                        actual_transit_days: s.actual_transit_days,
                        goal_transit_days: s.goal_transit_days,
                        ship_date: s.actual_ship,
                        delivery_date: s.actual_delivery,
                        is_synthetic: s.is_synthetic.unwrap_or(false),
                    },
                    carrier: CarrierInfo {
                        carrier_id: s.carrier_ref.clone(),
                        display_name: get_carrier_name(&s.carrier_ref),
                    },
                    origin: LocationInfo {
                        zip5: s.origin_zip5.clone(),
                        zip3: origin_zip3,
                        location: get_location_long(&s.origin_zip5),
                    },
                    destination: LocationInfo {
                        zip5: s.dest_zip5.clone(),
                        zip3: dest_zip3,
                        location: get_location_long(&s.dest_zip5),
                    },
                    lane: LaneInfo {
                        zip5_pair: s.lane_zip5_pair,
                        zip3_pair,
                    },
                }))
            }
            None => Ok(None),
        }
    }

    /// Get reachable destinations from a ZIP5 with carrier and performance info
    pub async fn get_reachable_destinations(&self, zip5: &str, min_volume: i64, limit: usize) -> Result<ReachableDestinationsResponse> {
        let db = db::connect(&self.db_path).await?;
        let zip5_owned = zip5.to_string();

        #[derive(Debug, Deserialize)]
        struct DestData {
            dest_zip5: String,
            volume: i64,
            carriers: Vec<String>,
            avg_transit: f64,
            ontime_count: i64,
        }

        let destinations: Vec<DestData> = db
            .query(r#"
                SELECT
                    dest_zip5,
                    count() as volume,
                    array::distinct(carrier_ref) as carriers,
                    math::mean(actual_transit_days) as avg_transit,
                    count(IF otd = "OnTime" THEN 1 END) as ontime_count
                FROM shipment
                WHERE origin_zip5 = $zip5
                GROUP BY dest_zip5
                HAVING volume >= $min_volume
                ORDER BY volume DESC
                LIMIT $limit
            "#)
            .bind(("zip5", zip5_owned))
            .bind(("min_volume", min_volume))
            .bind(("limit", limit))
            .await?
            .take(0)?;

        let mut all_carriers: Vec<String> = destinations
            .iter()
            .flat_map(|d| d.carriers.clone())
            .collect();
        all_carriers.sort();
        all_carriers.dedup();

        let reachable: Vec<ReachableDestination> = destinations
            .into_iter()
            .map(|d| {
                let otd_rate = if d.volume > 0 {
                    d.ontime_count as f64 / d.volume as f64
                } else {
                    0.0
                };
                ReachableDestination {
                    zip5: d.dest_zip5.clone(),
                    location: get_location_long(&d.dest_zip5),
                    volume: d.volume,
                    carriers: d.carriers.iter().map(|c| get_carrier_name(c)).collect(),
                    avg_transit: (d.avg_transit * 10.0).round() / 10.0,
                    otd_rate: (otd_rate * 1000.0).round() / 10.0,
                }
            })
            .collect();

        let total_destinations = reachable.len();
        let total_carriers = all_carriers.len();

        Ok(ReachableDestinationsResponse {
            origin: zip5.to_string(),
            origin_location: get_location_long(zip5),
            total_destinations,
            total_carriers,
            destinations: reachable,
        })
    }
}
