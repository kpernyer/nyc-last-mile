//! Shared business logic for the analytics API
//!
//! This service layer is used by both REST and gRPC handlers.

use anyhow::Result;
use crate::{db, location_names::format_lane_short};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

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
}
