//! Add graph edges to an existing synthetic database
//!
//! Usage:
//!   cargo run --release --bin add_graph_edges -- --db data/synthetic.db

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "add_graph_edges")]
#[command(about = "Add graph edges (RELATE statements) to existing database")]
struct Args {
    /// Database path
    #[arg(long, default_value = "data/synthetic.db")]
    db: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let args = Args::parse();

    println!("🔗 Add Graph Edges");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Database: {}", args.db.display());
    println!();

    info!("Connecting to SurrealDB at {:?}", args.db);
    let db = Surreal::new::<RocksDb>(args.db.to_str().unwrap()).await?;
    db.use_ns("lastmile").use_db("shipping").await?;

    // Define edge tables if not exist
    info!("Ensuring edge tables exist...");
    db.query(r#"
        DEFINE TABLE IF NOT EXISTS shipped_by SCHEMALESS;
        DEFINE TABLE IF NOT EXISTS origin5_at SCHEMALESS;
        DEFINE TABLE IF NOT EXISTS dest5_at SCHEMALESS;
        DEFINE TABLE IF NOT EXISTS on_lane5 SCHEMALESS;
        DEFINE TABLE IF NOT EXISTS connects5 SCHEMALESS;
    "#).await?;

    // Get counts first
    let shipment_count: Option<i64> = db
        .query("SELECT count() FROM shipment GROUP ALL")
        .await?
        .take("count")?;
    println!("Found {} shipments to process", shipment_count.unwrap_or(0));
    println!();

    // Create edges using subqueries that return record IDs directly
    info!("Creating shipped_by edges (shipment → carrier)...");
    db.query(r#"
        FOR $s IN (SELECT *, meta::id(id) as sid FROM shipment) {
            LET $carrier = (SELECT * FROM carrier WHERE carrier_id = $s.carrier_ref);
            IF array::len($carrier) > 0 {
                RELATE (type::thing('shipment', $s.sid))->shipped_by->($carrier[0].id);
            };
        };
    "#).await?;

    info!("Creating origin5_at edges (shipment → origin location5)...");
    db.query(r#"
        FOR $s IN (SELECT *, meta::id(id) as sid FROM shipment) {
            LET $loc = (SELECT * FROM location5 WHERE zip5 = $s.origin_zip5);
            IF array::len($loc) > 0 {
                RELATE (type::thing('shipment', $s.sid))->origin5_at->($loc[0].id);
            };
        };
    "#).await?;

    info!("Creating dest5_at edges (shipment → dest location5)...");
    db.query(r#"
        FOR $s IN (SELECT *, meta::id(id) as sid FROM shipment) {
            LET $loc = (SELECT * FROM location5 WHERE zip5 = $s.dest_zip5);
            IF array::len($loc) > 0 {
                RELATE (type::thing('shipment', $s.sid))->dest5_at->($loc[0].id);
            };
        };
    "#).await?;

    info!("Creating on_lane5 edges (shipment → lane5)...");
    db.query(r#"
        FOR $s IN (SELECT *, meta::id(id) as sid FROM shipment) {
            LET $lane = (SELECT * FROM lane5 WHERE zip5_pair = $s.lane_zip5_pair);
            IF array::len($lane) > 0 {
                RELATE (type::thing('shipment', $s.sid))->on_lane5->($lane[0].id);
            };
        };
    "#).await?;

    info!("Creating connects5 edges (lane5 → location5)...");
    db.query(r#"
        FOR $lane IN (SELECT *, meta::id(id) as lid FROM lane5) {
            LET $origin = (SELECT * FROM location5 WHERE zip5 = $lane.origin_zip5);
            LET $dest = (SELECT * FROM location5 WHERE zip5 = $lane.dest_zip5);
            IF array::len($origin) > 0 AND array::len($dest) > 0 {
                RELATE (type::thing('lane5', $lane.lid))->connects5->($origin[0].id) SET direction = 'origin';
                RELATE (type::thing('lane5', $lane.lid))->connects5->($dest[0].id) SET direction = 'dest';
            };
        };
    "#).await?;

    // Count edges
    println!();
    println!("✅ Graph edges created!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let shipped_by: Option<i64> = db.query("SELECT count() FROM shipped_by GROUP ALL").await?.take("count")?;
    let origin5_at: Option<i64> = db.query("SELECT count() FROM origin5_at GROUP ALL").await?.take("count")?;
    let dest5_at: Option<i64> = db.query("SELECT count() FROM dest5_at GROUP ALL").await?.take("count")?;
    let on_lane5: Option<i64> = db.query("SELECT count() FROM on_lane5 GROUP ALL").await?.take("count")?;
    let connects5: Option<i64> = db.query("SELECT count() FROM connects5 GROUP ALL").await?.take("count")?;

    println!("🔗 Edge counts:");
    println!("  shipped_by:  {:>8?}", shipped_by);
    println!("  origin5_at:  {:>8?}", origin5_at);
    println!("  dest5_at:    {:>8?}", dest5_at);
    println!("  on_lane5:    {:>8?}", on_lane5);
    println!("  connects5:   {:>8?}", connects5);

    Ok(())
}
