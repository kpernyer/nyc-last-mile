//! Graph-oriented REST API handlers
//!
//! These handlers leverage graph relationships in the synthetic database.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AnalyticsService;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct CarrierNetworkResponse {
    pub carrier_id: String,
    pub display_name: String,
    pub total_shipments: i64,
    pub total_lanes: usize,
    pub origins: Vec<String>,
    pub destinations: Vec<String>,
    pub top_lanes: Vec<CarrierLane>,
}

#[derive(Serialize)]
pub struct CarrierLane {
    pub lane: String,
    pub origin: String,
    pub destination: String,
    pub volume: i64,
    pub otd_rate: f64,
    pub avg_transit: f64,
}

#[derive(Serialize)]
pub struct LocationConnectionsResponse {
    pub zip5: String,
    pub location: String,
    pub outbound: ConnectionStats,
    pub inbound: ConnectionStats,
}

#[derive(Serialize)]
pub struct ConnectionStats {
    pub total_destinations: usize,
    pub total_volume: i64,
    pub top_connections: Vec<Connection>,
}

#[derive(Serialize)]
pub struct Connection {
    pub zip5: String,
    pub location: String,
    pub volume: i64,
    pub otd_rate: f64,
}

#[derive(Serialize)]
pub struct NetworkTopologyResponse {
    pub nodes: NodeCounts,
    pub edges: EdgeCounts,
    pub density: NetworkDensity,
}

#[derive(Serialize)]
pub struct NodeCounts {
    pub shipments: i64,
    pub carriers: i64,
    pub locations_zip3: i64,
    pub locations_zip5: i64,
    pub lanes_zip3: i64,
    pub lanes_zip5: i64,
}

#[derive(Serialize)]
pub struct EdgeCounts {
    pub shipped_by: i64,
    pub origin5_at: i64,
    pub dest5_at: i64,
    pub on_lane5: i64,
    pub connects5: i64,
}

#[derive(Serialize)]
pub struct NetworkDensity {
    pub avg_shipments_per_carrier: f64,
    pub avg_shipments_per_lane: f64,
    pub avg_destinations_per_origin: f64,
}

#[derive(Serialize)]
pub struct ShipmentTraceResponse {
    pub shipment: ShipmentInfo,
    pub carrier: CarrierInfo,
    pub origin: LocationInfo,
    pub destination: LocationInfo,
    pub lane: LaneInfo,
}

#[derive(Serialize)]
pub struct ShipmentInfo {
    pub load_id: String,
    pub carrier_mode: String,
    pub otd: String,
    pub actual_transit_days: i64,
    pub goal_transit_days: i64,
    pub ship_date: String,
    pub delivery_date: String,
    pub is_synthetic: bool,
}

#[derive(Serialize)]
pub struct CarrierInfo {
    pub carrier_id: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct LocationInfo {
    pub zip5: String,
    pub zip3: String,
    pub location: String,
}

#[derive(Serialize)]
pub struct LaneInfo {
    pub zip5_pair: String,
    pub zip3_pair: String,
}

#[derive(Serialize)]
pub struct ReachableDestinationsResponse {
    pub origin: String,
    pub origin_location: String,
    pub total_destinations: usize,
    pub total_carriers: usize,
    pub destinations: Vec<ReachableDestination>,
}

#[derive(Serialize)]
pub struct ReachableDestination {
    pub zip5: String,
    pub location: String,
    pub volume: i64,
    pub carriers: Vec<String>,
    pub avg_transit: f64,
    pub otd_rate: f64,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Deserialize)]
pub struct NetworkLimitQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ConnectionQuery {
    pub direction: Option<String>, // "inbound", "outbound", or "both"
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ReachableQuery {
    pub min_volume: Option<i64>,
    pub limit: Option<usize>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/v1/graph/carrier/{carrier_id}/network
pub async fn get_carrier_network(
    State(service): State<Arc<AnalyticsService>>,
    Path(carrier_id): Path<String>,
    Query(params): Query<NetworkLimitQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20);

    match service.get_carrier_network(&carrier_id, limit).await {
        Ok(network) => Json(network).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/graph/location/{zip5}/connections
pub async fn get_location_connections(
    State(service): State<Arc<AnalyticsService>>,
    Path(zip5): Path<String>,
    Query(params): Query<ConnectionQuery>,
) -> impl IntoResponse {
    let direction = params.direction.as_deref().unwrap_or("both");
    let limit = params.limit.unwrap_or(20);

    match service.get_location_connections(&zip5, direction, limit).await {
        Ok(connections) => Json(connections).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/graph/topology
pub async fn get_network_topology(
    State(service): State<Arc<AnalyticsService>>,
) -> impl IntoResponse {
    match service.get_network_topology().await {
        Ok(topology) => Json(topology).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/graph/shipment/{load_id}/trace
pub async fn trace_shipment(
    State(service): State<Arc<AnalyticsService>>,
    Path(load_id): Path<String>,
) -> impl IntoResponse {
    match service.trace_shipment(&load_id).await {
        Ok(Some(trace)) => Json(trace).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Shipment not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/graph/location/{zip5}/reachable
pub async fn get_reachable_destinations(
    State(service): State<Arc<AnalyticsService>>,
    Path(zip5): Path<String>,
    Query(params): Query<ReachableQuery>,
) -> impl IntoResponse {
    let min_volume = params.min_volume.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);

    match service.get_reachable_destinations(&zip5, min_volume, limit).await {
        Ok(reachable) => Json(reachable).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
