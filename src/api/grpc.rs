//! gRPC service implementation for Last-Mile Analytics
//!
//! Implements the AnalyticsService gRPC service using the shared service layer.

use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::proto::analytics_service_server::AnalyticsService as GrpcAnalyticsService;
use super::proto::*;
use super::service::AnalyticsService;

pub struct GrpcService {
    service: Arc<AnalyticsService>,
}

impl GrpcService {
    pub fn new(service: Arc<AnalyticsService>) -> Self {
        Self { service }
    }
}

// Helper to convert internal LaneMetrics to proto LaneMetrics
fn to_proto_lane(l: super::service::LaneMetrics) -> LaneMetrics {
    LaneMetrics {
        origin_zip: l.origin_zip,
        dest_zip: l.dest_zip,
        route: l.route,
        volume: l.volume,
        avg_delay: (l.avg_delay * 100.0).round() / 100.0,
        transit_variance: (l.transit_variance * 100.0).round() / 100.0,
        early_rate: (l.early_rate * 1000.0).round() / 10.0,
        on_time_rate: (l.on_time_rate * 1000.0).round() / 10.0,
        late_rate: (l.late_rate * 1000.0).round() / 10.0,
        cluster_id: l.cluster_id as u32,
        cluster_name: l.cluster_name,
    }
}

#[tonic::async_trait]
impl GrpcAnalyticsService for GrpcService {
    async fn get_lanes(
        &self,
        request: Request<GetLanesRequest>,
    ) -> Result<Response<GetLanesResponse>, Status> {
        let req = request.into_inner();
        let limit = req.limit.unwrap_or(100) as usize;

        match self.service.get_lanes().await {
            Ok(lanes) => {
                let mut filtered_lanes: Vec<_> = lanes;

                // Filter by cluster if specified
                if let Some(cluster_id) = req.cluster_id {
                    filtered_lanes = filtered_lanes.into_iter()
                        .filter(|l| l.cluster_id == cluster_id as u8)
                        .collect();
                }

                filtered_lanes.truncate(limit);

                Ok(Response::new(GetLanesResponse {
                    lanes: filtered_lanes.into_iter().map(to_proto_lane).collect(),
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_lane(
        &self,
        request: Request<GetLaneRequest>,
    ) -> Result<Response<GetLaneResponse>, Status> {
        let req = request.into_inner();

        match self.service.get_lane_profile(&req.origin, &req.dest).await {
            Ok(Some(lane)) => Ok(Response::new(GetLaneResponse {
                lane: Some(to_proto_lane(lane)),
                error: String::new(),
            })),
            Ok(None) => Ok(Response::new(GetLaneResponse {
                lane: None,
                error: format!("Lane not found: {} -> {}", req.origin, req.dest),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_clusters(
        &self,
        _request: Request<GetClustersRequest>,
    ) -> Result<Response<GetClustersResponse>, Status> {
        match self.service.get_clusters().await {
            Ok(clusters) => Ok(Response::new(GetClustersResponse {
                clusters: clusters.into_iter().map(|c| Cluster {
                    id: c.id as u32,
                    name: c.name,
                    description: c.description,
                    lane_count: c.lane_count as i64,
                    total_volume: c.total_volume,
                    avg_delay: c.avg_delay,
                    avg_late_rate: c.avg_late_rate,
                }).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_cluster_lanes(
        &self,
        request: Request<GetClusterLanesRequest>,
    ) -> Result<Response<GetClusterLanesResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 20 };

        match self.service.get_lanes_in_cluster(req.cluster_id as u8, limit).await {
            Ok(lanes) => Ok(Response::new(GetClusterLanesResponse {
                lanes: lanes.into_iter().map(to_proto_lane).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_playbook(
        &self,
        request: Request<GetPlaybookRequest>,
    ) -> Result<Response<GetPlaybookResponse>, Status> {
        let req = request.into_inner();

        match self.service.get_playbook(req.cluster_id as u8) {
            Some(playbook) => Ok(Response::new(GetPlaybookResponse {
                playbook: Some(Playbook {
                    cluster_id: playbook.cluster_id as u32,
                    cluster_name: playbook.cluster_name,
                    description: playbook.description,
                    actions: playbook.actions,
                }),
            })),
            None => Err(Status::not_found(format!("Cluster {} not found", req.cluster_id))),
        }
    }

    async fn get_region(
        &self,
        request: Request<GetRegionRequest>,
    ) -> Result<Response<GetRegionResponse>, Status> {
        let req = request.into_inner();

        match self.service.get_regional_performance(&req.zip3).await {
            Ok(Some(perf)) => Ok(Response::new(GetRegionResponse {
                region: perf.region,
                summary: Some(RegionalSummary {
                    total_lanes: perf.total_lanes as i64,
                    total_volume: perf.total_volume,
                    avg_late_rate: perf.avg_late_rate,
                    avg_early_rate: perf.avg_early_rate,
                    avg_delay: perf.avg_delay,
                }),
                cluster_breakdown: perf.cluster_breakdown.into_iter().map(|c| ClusterBreakdown {
                    cluster: c.cluster,
                    lane_count: c.lane_count as i64,
                    volume: c.volume,
                }).collect(),
                highest_friction_lanes: perf.highest_friction_lanes.into_iter().map(to_proto_lane).collect(),
                error: String::new(),
            })),
            Ok(None) => Ok(Response::new(GetRegionResponse {
                region: req.zip3.clone(),
                summary: None,
                cluster_breakdown: vec![],
                highest_friction_lanes: vec![],
                error: format!("No lanes found for region '{}'", req.zip3),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_friction_zones(
        &self,
        request: Request<GetFrictionZonesRequest>,
    ) -> Result<Response<GetFrictionZonesResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 10 };

        match self.service.get_friction_zones(limit).await {
            Ok(zones) => Ok(Response::new(GetFrictionZonesResponse {
                zones: zones.into_iter().map(|z| FrictionZone {
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
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_terminals(
        &self,
        request: Request<GetTerminalsRequest>,
    ) -> Result<Response<GetTerminalsResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 5 };

        match self.service.get_terminal_performance(limit).await {
            Ok((best, worst, avg_score, total_volume, total_terminals)) => {
                let convert = |t: super::service::TerminalPerformance| TerminalPerformance {
                    origin_zip: t.origin_zip,
                    terminal: t.terminal,
                    performance_score: t.performance_score,
                    on_time_rate: t.on_time_rate,
                    late_rate: t.late_rate,
                    early_rate: t.early_rate,
                    volume: t.volume,
                    lane_count: t.lane_count,
                };

                Ok(Response::new(GetTerminalsResponse {
                    total_terminals,
                    total_volume,
                    average_score: avg_score,
                    top_performers: best.into_iter().map(convert).collect(),
                    needs_improvement: worst.into_iter().map(convert).collect(),
                    recommendations: vec![
                        "Terminals scoring below 70 may need capacity review".to_string(),
                        "Consider load balancing from low-performers to high-performers".to_string(),
                        "Review carrier mix at underperforming terminals".to_string(),
                    ],
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_early_analysis(
        &self,
        _request: Request<GetEarlyAnalysisRequest>,
    ) -> Result<Response<GetEarlyAnalysisResponse>, Status> {
        match self.service.get_early_analysis().await {
            Ok(analysis) => Ok(Response::new(GetEarlyAnalysisResponse {
                total_shipments: analysis.total_shipments,
                early_shipments: analysis.early_shipments,
                early_rate: analysis.early_rate,
                top_destinations: analysis.top_destinations.into_iter().map(|d| EarlyDestination {
                    dest_zip: d.dest_zip,
                    location: d.location,
                    early_rate: d.early_rate,
                    avg_days_early: d.avg_days_early,
                    early_shipments: d.early_shipments,
                    volume: d.volume,
                }).collect(),
                recommendations: analysis.recommendations,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn find_similar(
        &self,
        request: Request<FindSimilarRequest>,
    ) -> Result<Response<FindSimilarResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 10 };

        match self.service.find_similar_lanes(&req.pattern, limit).await {
            Ok(result) => {
                let has_target = result.target_lane.is_some();
                Ok(Response::new(FindSimilarResponse {
                    target_lane: result.target_lane.map(to_proto_lane),
                    similar_lanes: result.similar_lanes.into_iter().map(to_proto_lane).collect(),
                    shared_playbook: result.shared_playbook,
                    error: if !has_target {
                        format!("No lane found matching '{}'. Try a ZIP3 code like '750' or location name like 'DFW'.", req.pattern)
                    } else {
                        String::new()
                    },
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        match self.service.get_stats().await {
            Ok(stats) => Ok(Response::new(GetStatsResponse {
                stats: Some(Stats {
                    total_shipments: stats.total_shipments,
                    total_lanes: stats.total_lanes,
                    total_carriers: stats.total_carriers,
                    total_locations: stats.total_locations,
                    overall_on_time_rate: stats.overall_on_time_rate,
                    overall_late_rate: stats.overall_late_rate,
                    overall_early_rate: stats.overall_early_rate,
                }),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
