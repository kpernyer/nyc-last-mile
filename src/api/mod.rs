//! API module for Last-Mile Analytics
//!
//! Provides both REST and gRPC interfaces to the analytics data.

pub mod proto {
    #![allow(clippy::all)]
    #![allow(warnings)]
    include!("lastmile.v1.rs");
}

pub mod service;
pub mod handlers;
pub mod grpc;

pub use service::AnalyticsService;
