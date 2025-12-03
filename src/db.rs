use anyhow::Result;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

pub type DbConn = Surreal<Db>;

/// Initialize database connection with RocksDB backend
pub async fn connect(path: &str) -> Result<DbConn> {
    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("lastmile").use_db("nyc").await?;
    Ok(db)
}

/// Initialize database schema
pub async fn init_schema(db: &DbConn) -> Result<()> {
    // Create tables with schema
    db.query(
        r#"
        -- Shipment table (schemaless for flexibility)
        DEFINE TABLE shipment SCHEMALESS;
        DEFINE INDEX idx_load_id ON shipment FIELDS load_id UNIQUE;
        DEFINE INDEX idx_ship_date ON shipment FIELDS actual_ship;
        DEFINE INDEX idx_otd ON shipment FIELDS otd;
        DEFINE INDEX idx_carrier_mode ON shipment FIELDS carrier_mode;

        -- Carrier table
        DEFINE TABLE carrier SCHEMAFULL;
        DEFINE FIELD carrier_id ON carrier TYPE string;
        DEFINE INDEX idx_carrier_id ON carrier FIELDS carrier_id UNIQUE;

        -- Location table (ZIP3 regions)
        DEFINE TABLE location SCHEMAFULL;
        DEFINE FIELD zip3 ON location TYPE string;
        DEFINE FIELD state ON location TYPE option<string>;
        DEFINE INDEX idx_zip3 ON location FIELDS zip3 UNIQUE;

        -- Lane table
        DEFINE TABLE lane SCHEMAFULL;
        DEFINE FIELD lane_id ON lane TYPE string;
        DEFINE FIELD zip3_pair ON lane TYPE string;
        DEFINE INDEX idx_lane_id ON lane FIELDS lane_id UNIQUE;

        -- Relationships (graph edges)
        DEFINE TABLE shipped_by SCHEMAFULL;  -- shipment -> carrier
        DEFINE TABLE origin_at SCHEMAFULL;   -- shipment -> location (origin)
        DEFINE TABLE dest_at SCHEMAFULL;     -- shipment -> location (destination)
        DEFINE TABLE on_lane SCHEMAFULL;     -- shipment -> lane
        DEFINE TABLE connects SCHEMAFULL;    -- lane -> origin location, dest location
        "#,
    )
    .await?;

    Ok(())
}
