//! Enhanced database schema with record links, graph relationships, and vector search
//!
//! Key improvements over basic schema:
//! 1. Record links: Direct references instead of string IDs (carrier: carrier:abc123)
//! 2. Graph relationships: RELATE statements for traversable edges
//! 3. Vector search: Performance embeddings on carriers/lanes with MTREE index

use anyhow::Result;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

pub type DbConn = Surreal<Db>;

/// Initialize database connection with RocksDB backend
pub async fn connect(path: &str) -> Result<DbConn> {
    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("lastmile_enhanced").use_db("nyc").await?;
    Ok(db)
}

/// Initialize enhanced database schema with record links, graph edges, and vector indexes
pub async fn init_schema(db: &DbConn) -> Result<()> {
    db.query(
        r#"
        -- =====================================================
        -- CARRIER TABLE with performance vector
        -- =====================================================
        DEFINE TABLE carrier SCHEMAFULL;
        DEFINE FIELD carrier_id ON carrier TYPE string;
        DEFINE FIELD display_name ON carrier TYPE option<string>;
        -- Performance vector: [otd_rate, avg_transit, volume_normalized, variance]
        DEFINE FIELD perf_vector ON carrier TYPE option<array<float>>;
        DEFINE INDEX idx_carrier_id ON carrier FIELDS carrier_id UNIQUE;

        -- =====================================================
        -- LOCATION TABLE (ZIP3 regions)
        -- =====================================================
        DEFINE TABLE location SCHEMAFULL;
        DEFINE FIELD zip3 ON location TYPE string;
        DEFINE FIELD state ON location TYPE option<string>;
        DEFINE INDEX idx_zip3 ON location FIELDS zip3 UNIQUE;

        -- =====================================================
        -- LANE TABLE with performance vector
        -- =====================================================
        DEFINE TABLE lane SCHEMAFULL;
        DEFINE FIELD lane_id ON lane TYPE string;
        DEFINE FIELD zip3_pair ON lane TYPE string;
        -- Record links to origin and destination
        DEFINE FIELD origin ON lane TYPE option<record<location>>;
        DEFINE FIELD destination ON lane TYPE option<record<location>>;
        -- Performance vector: [otd_rate, avg_transit, volume_normalized, distance_normalized]
        DEFINE FIELD perf_vector ON lane TYPE option<array<float>>;
        DEFINE INDEX idx_lane_id ON lane FIELDS lane_id UNIQUE;

        -- =====================================================
        -- SHIPMENT TABLE with record links (schemaless for flexibility)
        -- =====================================================
        DEFINE TABLE shipment SCHEMALESS;

        -- Indexes
        DEFINE INDEX idx_load_id ON shipment FIELDS load_id UNIQUE;
        DEFINE INDEX idx_ship_date ON shipment FIELDS actual_ship;
        DEFINE INDEX idx_otd ON shipment FIELDS otd;
        DEFINE INDEX idx_carrier_mode ON shipment FIELDS carrier_mode;
        DEFINE INDEX idx_carrier ON shipment FIELDS carrier;
        DEFINE INDEX idx_lane ON shipment FIELDS lane;

        -- =====================================================
        -- GRAPH EDGE TABLES
        -- These enable traversal queries like:
        -- SELECT ->shipped_by->carrier FROM shipment
        -- =====================================================

        -- shipment -[shipped_by]-> carrier
        DEFINE TABLE shipped_by SCHEMAFULL TYPE RELATION FROM shipment TO carrier;

        -- shipment -[from_origin]-> location
        DEFINE TABLE from_origin SCHEMAFULL TYPE RELATION FROM shipment TO location;

        -- shipment -[to_destination]-> location
        DEFINE TABLE to_destination SCHEMAFULL TYPE RELATION FROM shipment TO location;

        -- shipment -[on_lane]-> lane
        DEFINE TABLE on_lane SCHEMAFULL TYPE RELATION FROM shipment TO lane;

        -- carrier -[operates_on]-> lane (derived relationship)
        DEFINE TABLE operates_on SCHEMAFULL TYPE RELATION FROM carrier TO lane;
        DEFINE FIELD shipment_count ON operates_on TYPE int;
        DEFINE FIELD otd_rate ON operates_on TYPE float;
        "#,
    )
    .await?;

    Ok(())
}

/// Compute and store performance vectors for carriers and lanes
/// Call this after ingestion is complete
pub async fn compute_vectors(db: &DbConn) -> Result<()> {
    // Compute carrier performance vectors
    db.query(
        r#"
        -- Update carrier performance vectors
        -- Vector: [otd_rate/100, avg_transit/10, volume_log/10, variance/10]
        UPDATE carrier SET perf_vector = (
            SELECT VALUE [
                (count(IF otd = "OnTime" THEN true ELSE NONE END) * 1.0 / count()),
                (math::mean(actual_transit_days) / 10.0),
                (math::log10(count() + 1) / 5.0),
                (math::stddev(actual_transit_days) / 10.0)
            ]
            FROM shipment
            WHERE carrier = $parent.id
            GROUP ALL
        )[0];
        "#,
    )
    .await?;

    // Compute lane performance vectors
    db.query(
        r#"
        -- Update lane performance vectors
        -- Vector: [otd_rate/100, avg_transit/10, volume_log/10, avg_distance/1000]
        UPDATE lane SET perf_vector = (
            SELECT VALUE [
                (count(IF otd = "OnTime" THEN true ELSE NONE END) * 1.0 / count()),
                (math::mean(actual_transit_days) / 10.0),
                (math::log10(count() + 1) / 5.0),
                (math::mean(customer_distance) / 1000.0)
            ]
            FROM shipment
            WHERE lane = $parent.id
            GROUP ALL
        )[0];
        "#,
    )
    .await?;

    Ok(())
}
