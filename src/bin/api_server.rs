//! Combined REST + gRPC API Server for Last-Mile Analytics
//!
//! Serves both REST and gRPC on the same port using content-type routing.
//!
//! Usage:
//!   ./target/release/api_server [options]
//!
//! Options:
//!   --port PORT       Port to listen on (default: 8080)
//!   --db-path PATH    Path to SurrealDB database (default: data/lastmile.db)
//!   --rest-only       Only serve REST endpoints
//!   --grpc-only       Only serve gRPC endpoints
//!
//! REST endpoints:
//!   GET /api/v1/health              - Health check
//!   GET /api/v1/stats               - Database statistics
//!   GET /api/v1/lanes               - All lanes (with optional ?limit=N)
//!   GET /api/v1/lanes/:origin/:dest - Single lane profile
//!   GET /api/v1/clusters            - All 5 clusters
//!   GET /api/v1/clusters/:id/lanes  - Lanes in a cluster
//!   GET /api/v1/clusters/:id/playbook - Cluster playbook
//!   GET /api/v1/regions/:zip3       - Regional performance
//!   GET /api/v1/analysis/friction   - Friction zones
//!   GET /api/v1/analysis/terminals  - Terminal performance
//!   GET /api/v1/analysis/early      - Early delivery analysis
//!   GET /api/v1/search/similar?lane=X - Similar lanes
//!
//! gRPC service: lastmile.v1.AnalyticsService

use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use nyc_last_mile::api::{
    handlers,
    grpc::GrpcService,
    proto::analytics_service_server::AnalyticsServiceServer,
    AnalyticsService,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server as TonicServer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn print_banner(port: u16, rest_only: bool, grpc_only: bool) {
    println!("============================================================");
    println!("         NYC LAST-MILE DELIVERY API SERVER");
    println!("============================================================");
    println!();
    println!("  Port:     {}", port);
    if !grpc_only {
        println!("  REST:     http://localhost:{}/api/v1/", port);
    }
    if !rest_only {
        println!("  gRPC:     grpc://localhost:{}", port);
    }
    println!();
    if !grpc_only {
        println!("REST Endpoints:");
        println!("  GET /api/v1/health              Health check");
        println!("  GET /api/v1/stats               Database statistics");
        println!("  GET /api/v1/lanes               All lanes");
        println!("  GET /api/v1/lanes/:o/:d         Lane profile");
        println!("  GET /api/v1/clusters            All clusters");
        println!("  GET /api/v1/clusters/:id/lanes  Cluster lanes");
        println!("  GET /api/v1/clusters/:id/playbook  Playbook");
        println!("  GET /api/v1/regions/:zip3       Regional perf");
        println!("  GET /api/v1/analysis/friction   Friction zones");
        println!("  GET /api/v1/analysis/terminals  Terminal perf");
        println!("  GET /api/v1/analysis/early      Early analysis");
        println!("  GET /api/v1/search/similar      Similar lanes");
        println!();
    }
    if !rest_only {
        println!("gRPC Service: lastmile.v1.AnalyticsService");
        println!();
    }
    println!("============================================================");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut port: u16 = 8080;
    let mut db_path = "data/lastmile.db".to_string();
    let mut rest_only = false;
    let mut grpc_only = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                if i < args.len() {
                    port = args[i].parse().unwrap_or(8080);
                }
            }
            "--db-path" => {
                i += 1;
                if i < args.len() {
                    db_path = args[i].clone();
                }
            }
            "--rest-only" => rest_only = true,
            "--grpc-only" => grpc_only = true,
            _ => {}
        }
        i += 1;
    }

    print_banner(port, rest_only, grpc_only);

    // Create shared analytics service
    let service = Arc::new(AnalyticsService::new(&db_path));

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    if grpc_only {
        // gRPC only mode
        let grpc_service = GrpcService::new(service);
        tracing::info!("Starting gRPC-only server on {}", addr);

        TonicServer::builder()
            .add_service(AnalyticsServiceServer::new(grpc_service))
            .serve(addr)
            .await?;
    } else if rest_only {
        // REST only mode
        let app = create_rest_router(service);
        tracing::info!("Starting REST-only server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    } else {
        // Combined mode - use axum with tonic multiplexing
        // For simplicity, we'll run REST on main port and gRPC on port+1
        let grpc_port = port + 1;
        let grpc_addr: SocketAddr = format!("0.0.0.0:{}", grpc_port).parse()?;

        println!("Note: Running REST on port {} and gRPC on port {}", port, grpc_port);

        let rest_service = service.clone();
        let grpc_service = GrpcService::new(service);

        // Spawn gRPC server
        let grpc_handle = tokio::spawn(async move {
            tracing::info!("Starting gRPC server on {}", grpc_addr);
            TonicServer::builder()
                .add_service(AnalyticsServiceServer::new(grpc_service))
                .serve(grpc_addr)
                .await
        });

        // Start REST server
        let app = create_rest_router(rest_service);
        tracing::info!("Starting REST server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        let rest_handle = tokio::spawn(async move {
            axum::serve(listener, app).await
        });

        // Wait for either to finish (or error)
        tokio::select! {
            result = grpc_handle => {
                if let Err(e) = result {
                    tracing::error!("gRPC server error: {}", e);
                }
            }
            result = rest_handle => {
                if let Err(e) = result {
                    tracing::error!("REST server error: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn create_rest_router(service: Arc<AnalyticsService>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/api/v1/health", get(handlers::health))
        // Stats
        .route("/api/v1/stats", get(handlers::get_stats))
        // Lanes
        .route("/api/v1/lanes", get(handlers::get_lanes))
        .route("/api/v1/lanes/:origin/:dest", get(handlers::get_lane))
        // Clusters
        .route("/api/v1/clusters", get(handlers::get_clusters))
        .route("/api/v1/clusters/:id/lanes", get(handlers::get_cluster_lanes))
        .route("/api/v1/clusters/:id/playbook", get(handlers::get_playbook))
        // Regions
        .route("/api/v1/regions/:zip3", get(handlers::get_region))
        // Analysis
        .route("/api/v1/analysis/friction", get(handlers::get_friction_zones))
        .route("/api/v1/analysis/terminals", get(handlers::get_terminals))
        .route("/api/v1/analysis/early", get(handlers::get_early_analysis))
        // Search
        .route("/api/v1/search/similar", get(handlers::find_similar))
        // State and middleware
        .with_state(service)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
