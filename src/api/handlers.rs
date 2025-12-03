//! REST API handlers for Last-Mile Analytics
//!
//! These handlers use the shared AnalyticsService.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::service::{AnalyticsService, LaneMetrics};

// ============================================================================
// Response Types (JSON-serializable versions)
// ============================================================================

#[derive(Serialize)]
pub struct LaneResponse {
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

impl From<LaneMetrics> for LaneResponse {
    fn from(l: LaneMetrics) -> Self {
        Self {
            origin_zip: l.origin_zip,
            dest_zip: l.dest_zip,
            route: l.route,
            volume: l.volume,
            avg_delay: (l.avg_delay * 100.0).round() / 100.0,
            transit_variance: (l.transit_variance * 100.0).round() / 100.0,
            early_rate: (l.early_rate * 1000.0).round() / 10.0,
            on_time_rate: (l.on_time_rate * 1000.0).round() / 10.0,
            late_rate: (l.late_rate * 1000.0).round() / 10.0,
            cluster_id: l.cluster_id,
            cluster_name: l.cluster_name,
        }
    }
}

#[derive(Serialize)]
pub struct ClusterResponse {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub lane_count: usize,
    pub total_volume: i64,
    pub avg_delay: f64,
    pub avg_late_rate: f64,
}

#[derive(Serialize)]
pub struct PlaybookResponse {
    pub cluster_id: u8,
    pub cluster_name: String,
    pub description: String,
    pub actions: Vec<String>,
}

#[derive(Serialize)]
pub struct FrictionZoneResponse {
    pub dest_zip: String,
    pub location: String,
    pub friction_score: f64,
    pub late_rate: f64,
    pub transit_variance: f64,
    pub volume: i64,
    pub lane_count: i64,
}

#[derive(Serialize)]
pub struct TerminalResponse {
    pub origin_zip: String,
    pub terminal: String,
    pub performance_score: f64,
    pub on_time_rate: f64,
    pub late_rate: f64,
    pub early_rate: f64,
    pub volume: i64,
    pub lane_count: i64,
}

#[derive(Serialize)]
pub struct TerminalsResponse {
    pub total_terminals: i64,
    pub total_volume: i64,
    pub average_score: f64,
    pub top_performers: Vec<TerminalResponse>,
    pub needs_improvement: Vec<TerminalResponse>,
    pub recommendations: Vec<String>,
}

#[derive(Serialize)]
pub struct EarlyDestinationResponse {
    pub dest_zip: String,
    pub location: String,
    pub early_rate: f64,
    pub avg_days_early: f64,
    pub early_shipments: i64,
    pub volume: i64,
}

#[derive(Serialize)]
pub struct EarlyAnalysisResponse {
    pub total_shipments: i64,
    pub early_shipments: i64,
    pub early_rate: f64,
    pub top_destinations: Vec<EarlyDestinationResponse>,
    pub recommendations: Vec<String>,
}

#[derive(Serialize)]
pub struct SimilarLanesResponse {
    pub target_lane: Option<LaneResponse>,
    pub similar_lanes: Vec<LaneResponse>,
    pub shared_playbook: String,
}

#[derive(Serialize)]
pub struct ClusterBreakdownResponse {
    pub cluster: String,
    pub lane_count: usize,
    pub volume: i64,
}

#[derive(Serialize)]
pub struct RegionalResponse {
    pub region: String,
    pub total_lanes: usize,
    pub total_volume: i64,
    pub avg_late_rate: f64,
    pub avg_early_rate: f64,
    pub avg_delay: f64,
    pub cluster_breakdown: Vec<ClusterBreakdownResponse>,
    pub highest_friction_lanes: Vec<LaneResponse>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_shipments: i64,
    pub total_lanes: i64,
    pub total_carriers: i64,
    pub total_locations: i64,
    pub overall_on_time_rate: f64,
    pub overall_late_rate: f64,
    pub overall_early_rate: f64,
}

#[derive(Serialize)]
pub struct FrictionZonesResponse {
    pub zones: Vec<FrictionZoneResponse>,
    pub recommendations: Vec<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Deserialize)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct SimilarQuery {
    pub lane: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ClusterLanesQuery {
    pub limit: Option<usize>,
}

// ============================================================================
// Handlers
// ============================================================================

pub type AppState = Arc<AnalyticsService>;

/// GET /api/v1/health
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /api/v1/stats
pub async fn get_stats(
    State(service): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_stats().await {
        Ok(stats) => Ok(Json(StatsResponse {
            total_shipments: stats.total_shipments,
            total_lanes: stats.total_lanes,
            total_carriers: stats.total_carriers,
            total_locations: stats.total_locations,
            overall_on_time_rate: stats.overall_on_time_rate,
            overall_late_rate: stats.overall_late_rate,
            overall_early_rate: stats.overall_early_rate,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/lanes
pub async fn get_lanes(
    State(service): State<AppState>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<LaneResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(100);
    match service.get_lanes().await {
        Ok(lanes) => {
            let response: Vec<LaneResponse> = lanes.into_iter().take(limit).map(LaneResponse::from).collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/lanes/:origin/:dest
pub async fn get_lane(
    State(service): State<AppState>,
    Path((origin, dest)): Path<(String, String)>,
) -> Result<Json<LaneResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_lane_profile(&origin, &dest).await {
        Ok(Some(lane)) => Ok(Json(LaneResponse::from(lane))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: format!("Lane not found: {} -> {}", origin, dest)
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/clusters
pub async fn get_clusters(
    State(service): State<AppState>,
) -> Result<Json<Vec<ClusterResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_clusters().await {
        Ok(clusters) => {
            let response: Vec<ClusterResponse> = clusters.into_iter().map(|c| ClusterResponse {
                id: c.id,
                name: c.name,
                description: c.description,
                lane_count: c.lane_count,
                total_volume: c.total_volume,
                avg_delay: c.avg_delay,
                avg_late_rate: c.avg_late_rate,
            }).collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/clusters/:id/lanes
pub async fn get_cluster_lanes(
    State(service): State<AppState>,
    Path(id): Path<u8>,
    Query(params): Query<ClusterLanesQuery>,
) -> Result<Json<Vec<LaneResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(20);
    match service.get_lanes_in_cluster(id, limit).await {
        Ok(lanes) => {
            let response: Vec<LaneResponse> = lanes.into_iter().map(LaneResponse::from).collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/clusters/:id/playbook
pub async fn get_playbook(
    State(service): State<AppState>,
    Path(id): Path<u8>,
) -> Result<Json<PlaybookResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_playbook(id) {
        Some(playbook) => Ok(Json(PlaybookResponse {
            cluster_id: playbook.cluster_id,
            cluster_name: playbook.cluster_name,
            description: playbook.description,
            actions: playbook.actions,
        })),
        None => Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: format!("Cluster {} not found. Valid IDs: 1-5", id)
        }))),
    }
}

/// GET /api/v1/regions/:zip3
pub async fn get_region(
    State(service): State<AppState>,
    Path(zip3): Path<String>,
) -> Result<Json<RegionalResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_regional_performance(&zip3).await {
        Ok(Some(perf)) => Ok(Json(RegionalResponse {
            region: perf.region,
            total_lanes: perf.total_lanes,
            total_volume: perf.total_volume,
            avg_late_rate: perf.avg_late_rate,
            avg_early_rate: perf.avg_early_rate,
            avg_delay: perf.avg_delay,
            cluster_breakdown: perf.cluster_breakdown.into_iter().map(|c| ClusterBreakdownResponse {
                cluster: c.cluster,
                lane_count: c.lane_count,
                volume: c.volume,
            }).collect(),
            highest_friction_lanes: perf.highest_friction_lanes.into_iter().map(LaneResponse::from).collect(),
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: format!("No lanes found for region '{}'. Try a ZIP3 like '750' or location like 'DFW'.", zip3)
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/analysis/friction
pub async fn get_friction_zones(
    State(service): State<AppState>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<FrictionZonesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(10);
    match service.get_friction_zones(limit).await {
        Ok(zones) => Ok(Json(FrictionZonesResponse {
            zones: zones.into_iter().map(|z| FrictionZoneResponse {
                dest_zip: z.dest_zip,
                location: z.location,
                friction_score: z.friction_score,
                late_rate: z.late_rate,
                transit_variance: z.transit_variance,
                volume: z.volume,
                lane_count: z.lane_count,
            }).collect(),
            recommendations: vec![
                "High-friction zones may need carrier renegotiation".to_string(),
                "Consider alternative routing or pre-positioning inventory".to_string(),
                "Increase SLA buffer for these destinations".to_string(),
            ],
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/analysis/terminals
pub async fn get_terminals(
    State(service): State<AppState>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<TerminalsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(5);
    match service.get_terminal_performance(limit).await {
        Ok((best, worst, avg_score, total_volume, total_terminals)) => Ok(Json(TerminalsResponse {
            total_terminals,
            total_volume,
            average_score: avg_score,
            top_performers: best.into_iter().map(|t| TerminalResponse {
                origin_zip: t.origin_zip,
                terminal: t.terminal,
                performance_score: t.performance_score,
                on_time_rate: t.on_time_rate,
                late_rate: t.late_rate,
                early_rate: t.early_rate,
                volume: t.volume,
                lane_count: t.lane_count,
            }).collect(),
            needs_improvement: worst.into_iter().map(|t| TerminalResponse {
                origin_zip: t.origin_zip,
                terminal: t.terminal,
                performance_score: t.performance_score,
                on_time_rate: t.on_time_rate,
                late_rate: t.late_rate,
                early_rate: t.early_rate,
                volume: t.volume,
                lane_count: t.lane_count,
            }).collect(),
            recommendations: vec![
                "Terminals scoring below 70 may need capacity review".to_string(),
                "Consider load balancing from low-performers to high-performers".to_string(),
                "Review carrier mix at underperforming terminals".to_string(),
            ],
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/analysis/early
pub async fn get_early_analysis(
    State(service): State<AppState>,
) -> Result<Json<EarlyAnalysisResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_early_analysis().await {
        Ok(analysis) => Ok(Json(EarlyAnalysisResponse {
            total_shipments: analysis.total_shipments,
            early_shipments: analysis.early_shipments,
            early_rate: analysis.early_rate,
            top_destinations: analysis.top_destinations.into_iter().map(|d| EarlyDestinationResponse {
                dest_zip: d.dest_zip,
                location: d.location,
                early_rate: d.early_rate,
                avg_days_early: d.avg_days_early,
                early_shipments: d.early_shipments,
                volume: d.volume,
            }).collect(),
            recommendations: analysis.recommendations,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

/// GET /api/v1/search/similar?lane=X
pub async fn find_similar(
    State(service): State<AppState>,
    Query(params): Query<SimilarQuery>,
) -> Result<Json<SimilarLanesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(10);
    match service.find_similar_lanes(&params.lane, limit).await {
        Ok(result) => Ok(Json(SimilarLanesResponse {
            target_lane: result.target_lane.map(LaneResponse::from),
            similar_lanes: result.similar_lanes.into_iter().map(LaneResponse::from).collect(),
            shared_playbook: result.shared_playbook,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}
