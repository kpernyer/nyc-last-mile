//! Ingest synthetic data with ZIP5 fields into SurrealDB
//!
//! This binary handles the extended CSV format from generate_synthetic,
//! which includes origin_zip5, dest_zip5, lane_zip5_pair, and is_synthetic fields.
//!
//! Usage:
//!   cargo run --release --bin ingest_synthetic -- [OPTIONS]
//!
//! Options:
//!   --input <PATH>   Input CSV path (default: data/synthetic_data.csv)
//!   --db <PATH>      Database path (default: data/synthetic.db)
//!   --clear          Clear existing database before ingesting

use anyhow::Result;
use clap::Parser;
use csv::ReaderBuilder;
use nyc_last_mile::models::SyntheticCsvRecord;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;
use tracing::{info, warn};

/// Ingest synthetic shipping data into SurrealDB
#[derive(Parser, Debug)]
#[command(name = "ingest_synthetic")]
#[command(about = "Ingest synthetic shipping data with ZIP5 fields")]
struct Args {
    /// Input CSV path
    #[arg(long, default_value = "data/synthetic_data.csv")]
    input: PathBuf,

    /// Database path
    #[arg(long, default_value = "data/synthetic.db")]
    db: PathBuf,

    /// Clear existing database before ingesting
    #[arg(long)]
    clear: bool,

    /// Batch size for inserts
    #[arg(long, default_value = "1000")]
    batch_size: usize,

    /// Create graph edges (RELATE statements) - slower but enables graph queries
    #[arg(long)]
    graph: bool,
}

async fn connect(db_path: &str) -> Result<Surreal<surrealdb::engine::local::Db>> {
    let db = Surreal::new::<RocksDb>(db_path).await?;
    db.use_ns("lastmile").use_db("shipping").await?;
    Ok(db)
}

async fn init_schema(db: &Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    // Extended schema with ZIP5 support and graph relationships
    db.query(
        r#"
        -- Carrier table
        DEFINE TABLE carrier SCHEMAFULL;
        DEFINE FIELD carrier_id ON carrier TYPE string;
        DEFINE FIELD display_name ON carrier TYPE option<string>;
        DEFINE INDEX carrier_id_idx ON carrier FIELDS carrier_id UNIQUE;

        -- Location table (ZIP3 level)
        DEFINE TABLE location SCHEMAFULL;
        DEFINE FIELD zip3 ON location TYPE string;
        DEFINE FIELD state ON location TYPE option<string>;
        DEFINE INDEX zip3_idx ON location FIELDS zip3 UNIQUE;

        -- Location5 table (ZIP5 level)
        DEFINE TABLE location5 SCHEMAFULL;
        DEFINE FIELD zip5 ON location5 TYPE string;
        DEFINE FIELD zip3 ON location5 TYPE string;
        DEFINE FIELD state ON location5 TYPE option<string>;
        DEFINE INDEX zip5_idx ON location5 FIELDS zip5 UNIQUE;

        -- Lane table (ZIP3 level)
        DEFINE TABLE lane SCHEMAFULL;
        DEFINE FIELD lane_id ON lane TYPE string;
        DEFINE FIELD zip3_pair ON lane TYPE string;
        DEFINE INDEX lane_id_idx ON lane FIELDS lane_id UNIQUE;

        -- Lane5 table (ZIP5 level)
        DEFINE TABLE lane5 SCHEMAFULL;
        DEFINE FIELD zip5_pair ON lane5 TYPE string;
        DEFINE FIELD zip3_pair ON lane5 TYPE string;
        DEFINE FIELD origin_zip5 ON lane5 TYPE string;
        DEFINE FIELD dest_zip5 ON lane5 TYPE string;
        DEFINE INDEX lane5_pair_idx ON lane5 FIELDS zip5_pair UNIQUE;

        -- Shipment table (schemaless for flexibility)
        DEFINE TABLE shipment SCHEMALESS;
        DEFINE INDEX load_id_idx ON shipment FIELDS load_id UNIQUE;
        DEFINE INDEX actual_ship_idx ON shipment FIELDS actual_ship;
        DEFINE INDEX otd_idx ON shipment FIELDS otd;
        DEFINE INDEX carrier_mode_idx ON shipment FIELDS carrier_mode;
        DEFINE INDEX carrier_ref_idx ON shipment FIELDS carrier_ref;
        DEFINE INDEX origin_zip5_idx ON shipment FIELDS origin_zip5;
        DEFINE INDEX dest_zip5_idx ON shipment FIELDS dest_zip5;
        DEFINE INDEX is_synthetic_idx ON shipment FIELDS is_synthetic;

        -- Graph edge tables (relationships)
        DEFINE TABLE shipped_by SCHEMALESS;      -- shipment -> carrier
        DEFINE TABLE origin_at SCHEMALESS;       -- shipment -> location (ZIP3)
        DEFINE TABLE dest_at SCHEMALESS;         -- shipment -> location (ZIP3)
        DEFINE TABLE origin5_at SCHEMALESS;      -- shipment -> location5 (ZIP5)
        DEFINE TABLE dest5_at SCHEMALESS;        -- shipment -> location5 (ZIP5)
        DEFINE TABLE on_lane SCHEMALESS;         -- shipment -> lane (ZIP3)
        DEFINE TABLE on_lane5 SCHEMALESS;        -- shipment -> lane5 (ZIP5)
        DEFINE TABLE connects SCHEMALESS;        -- lane -> locations
        DEFINE TABLE connects5 SCHEMALESS;       -- lane5 -> location5s
        DEFINE TABLE operates_on SCHEMALESS;     -- carrier -> lane (with stats)
        "#,
    )
    .await?;

    Ok(())
}

fn extract_state(zip: &str) -> Option<String> {
    // If it's a 2-letter state code, return it
    if zip.len() == 2 && zip.chars().all(|c| c.is_ascii_uppercase()) {
        return Some(zip.to_string());
    }
    // Map ZIP3 prefix to state (simplified)
    let prefix = if zip.len() >= 3 { &zip[..3] } else { zip };
    match prefix {
        // Texas
        p if p.starts_with("75") || p.starts_with("76") || p.starts_with("77") || p.starts_with("78") || p.starts_with("79") => Some("TX".to_string()),
        // Pennsylvania
        p if p.starts_with("15") || p.starts_with("16") || p.starts_with("17") || p.starts_with("18") || p.starts_with("19") => Some("PA".to_string()),
        // Ohio
        p if p.starts_with("43") || p.starts_with("44") || p.starts_with("45") => Some("OH".to_string()),
        // California
        p if p.starts_with("90") || p.starts_with("91") || p.starts_with("92") || p.starts_with("93") || p.starts_with("94") || p.starts_with("95") || p.starts_with("96") => Some("CA".to_string()),
        // New York
        p if p.starts_with("10") || p.starts_with("11") || p.starts_with("12") || p.starts_with("13") || p.starts_with("14") => Some("NY".to_string()),
        // Florida
        p if p.starts_with("32") || p.starts_with("33") || p.starts_with("34") => Some("FL".to_string()),
        // Illinois
        p if p.starts_with("60") || p.starts_with("61") || p.starts_with("62") => Some("IL".to_string()),
        // Georgia
        p if p.starts_with("30") || p.starts_with("31") => Some("GA".to_string()),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let args = Args::parse();

    println!("📦 Synthetic Data Ingester");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Input:      {}", args.input.display());
    println!("Database:   {}", args.db.display());
    println!("Batch size: {}", args.batch_size);
    println!("Graph mode: {}", if args.graph { "enabled (creates RELATE edges)" } else { "disabled" });
    println!();

    // Clear database if requested
    if args.clear {
        info!("Clearing existing database...");
        if args.db.exists() {
            std::fs::remove_dir_all(&args.db)?;
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = args.db.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Connecting to SurrealDB at {:?}", args.db);
    let db = connect(args.db.to_str().unwrap()).await?;

    info!("Initializing schema...");
    init_schema(&db).await?;

    info!("Reading CSV from {:?}", args.input);
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&args.input)?;

    // First pass: collect all records and unique entities
    let records: Vec<SyntheticCsvRecord> = reader
        .deserialize()
        .filter_map(|r| r.ok())
        .collect();

    let total_records = records.len();
    info!("Parsed {} records from CSV", total_records);

    // Collect unique entities
    let mut carriers: HashSet<String> = HashSet::new();
    let mut locations_zip3: HashSet<String> = HashSet::new();
    let mut locations_zip5: HashSet<String> = HashSet::new();
    let mut lanes_zip3: HashMap<String, String> = HashMap::new(); // lane_id -> zip3_pair
    let mut lanes_zip5: HashSet<String> = HashSet::new(); // zip5_pair

    for record in &records {
        carriers.insert(record.carrier_pseudo.clone());
        locations_zip3.insert(record.origin_zip_3d.clone());
        locations_zip3.insert(record.dest_zip_3d.clone());
        locations_zip5.insert(record.origin_zip5.clone());
        locations_zip5.insert(record.dest_zip5.clone());
        lanes_zip3.insert(record.lane_id.clone(), record.lane_zip3_pair.clone());
        lanes_zip5.insert(record.lane_zip5_pair.clone());
    }

    info!(
        "Found {} carriers, {} ZIP3 locations, {} ZIP5 locations, {} ZIP3 lanes, {} ZIP5 lanes",
        carriers.len(),
        locations_zip3.len(),
        locations_zip5.len(),
        lanes_zip3.len(),
        lanes_zip5.len()
    );

    // Insert carriers
    info!("Inserting carriers...");
    for carrier_id in &carriers {
        db.query("CREATE carrier SET carrier_id = $carrier_id")
            .bind(("carrier_id", carrier_id.clone()))
            .await?;
    }

    // Insert ZIP3 locations
    info!("Inserting ZIP3 locations...");
    for zip3 in &locations_zip3 {
        let state = extract_state(zip3);
        db.query("CREATE location SET zip3 = $zip3, state = $state")
            .bind(("zip3", zip3.clone()))
            .bind(("state", state))
            .await?;
    }

    // Insert ZIP5 locations
    info!("Inserting ZIP5 locations...");
    for zip5 in &locations_zip5 {
        let zip3 = SyntheticCsvRecord::zip5_to_zip3(zip5);
        let state = extract_state(zip5);
        db.query("CREATE location5 SET zip5 = $zip5, zip3 = $zip3, state = $state")
            .bind(("zip5", zip5.clone()))
            .bind(("zip3", zip3))
            .bind(("state", state))
            .await?;
    }

    // Insert ZIP3 lanes
    info!("Inserting ZIP3 lanes...");
    for (lane_id, zip3_pair) in &lanes_zip3 {
        db.query("CREATE lane SET lane_id = $lane_id, zip3_pair = $zip3_pair")
            .bind(("lane_id", lane_id.clone()))
            .bind(("zip3_pair", zip3_pair.clone()))
            .await?;
    }

    // Insert ZIP5 lanes
    info!("Inserting ZIP5 lanes...");
    for zip5_pair in &lanes_zip5 {
        let parts: Vec<&str> = zip5_pair.split('→').collect();
        let (origin, dest) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            continue;
        };
        let zip3_pair = format!(
            "{}→{}",
            SyntheticCsvRecord::zip5_to_zip3(&origin),
            SyntheticCsvRecord::zip5_to_zip3(&dest)
        );
        db.query("CREATE lane5 SET zip5_pair = $zip5_pair, zip3_pair = $zip3_pair, origin_zip5 = $origin, dest_zip5 = $dest")
            .bind(("zip5_pair", zip5_pair.clone()))
            .bind(("zip3_pair", zip3_pair))
            .bind(("origin", origin))
            .bind(("dest", dest))
            .await?;
    }

    // Create graph edges for lanes if requested
    if args.graph {
        info!("Creating lane graph edges (connects5)...");
        // Connect lane5 to origin and destination location5
        db.query(r#"
            FOR $lane IN (SELECT * FROM lane5) {
                LET $origin = (SELECT * FROM location5 WHERE zip5 = $lane.origin_zip5)[0];
                LET $dest = (SELECT * FROM location5 WHERE zip5 = $lane.dest_zip5)[0];
                IF $origin != NONE AND $dest != NONE {
                    RELATE $lane.id->connects5->$origin.id SET direction = 'origin';
                    RELATE $lane.id->connects5->$dest.id SET direction = 'dest';
                };
            };
        "#).await?;
    }

    // Insert shipments
    info!("Inserting shipments...");
    let mut shipment_count = 0;
    let mut error_count = 0;
    let mut synthetic_count = 0;

    for (i, record) in records.iter().enumerate() {
        match record.to_shipment_extended() {
            Ok(shipment) => {
                let result = db
                    .query(
                        r#"
                        CREATE shipment CONTENT {
                            load_id: $load_id,
                            carrier_mode: $carrier_mode,
                            actual_ship: <datetime>$actual_ship,
                            actual_delivery: <datetime>$actual_delivery,
                            carrier_posted_service_days: $carrier_posted_service_days,
                            customer_distance: $customer_distance,
                            truckload_service_days: $truckload_service_days,
                            goal_transit_days: $goal_transit_days,
                            actual_transit_days: $actual_transit_days,
                            otd: $otd,
                            ship_dow: $ship_dow,
                            ship_week: $ship_week,
                            ship_month: $ship_month,
                            ship_year: $ship_year,
                            distance_bucket: $distance_bucket,
                            carrier_ref: $carrier_id,
                            origin_zip3: $origin_zip3,
                            dest_zip3: $dest_zip3,
                            origin_zip5: $origin_zip5,
                            dest_zip5: $dest_zip5,
                            lane_zip3_pair: $lane_zip3_pair,
                            lane_zip5_pair: $lane_zip5_pair,
                            lane_ref: $lane_id,
                            is_synthetic: $is_synthetic
                        };
                        "#,
                    )
                    .bind(("load_id", shipment.load_id.clone()))
                    .bind(("carrier_mode", format!("{:?}", shipment.carrier_mode)))
                    .bind(("actual_ship", shipment.actual_ship.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
                    .bind(("actual_delivery", shipment.actual_delivery.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
                    .bind(("carrier_posted_service_days", shipment.carrier_posted_service_days))
                    .bind(("customer_distance", shipment.customer_distance))
                    .bind(("truckload_service_days", shipment.truckload_service_days))
                    .bind(("goal_transit_days", shipment.goal_transit_days))
                    .bind(("actual_transit_days", shipment.actual_transit_days))
                    .bind(("otd", format!("{:?}", shipment.otd)))
                    .bind(("ship_dow", shipment.ship_dow))
                    .bind(("ship_week", shipment.ship_week))
                    .bind(("ship_month", shipment.ship_month))
                    .bind(("ship_year", shipment.ship_year))
                    .bind(("distance_bucket", shipment.distance_bucket.clone()))
                    .bind(("carrier_id", record.carrier_pseudo.clone()))
                    .bind(("origin_zip3", record.origin_zip_3d.clone()))
                    .bind(("dest_zip3", record.dest_zip_3d.clone()))
                    .bind(("origin_zip5", shipment.origin_zip5.clone()))
                    .bind(("dest_zip5", shipment.dest_zip5.clone()))
                    .bind(("lane_zip3_pair", record.lane_zip3_pair.clone()))
                    .bind(("lane_zip5_pair", record.lane_zip5_pair.clone()))
                    .bind(("lane_id", record.lane_id.clone()))
                    .bind(("is_synthetic", shipment.is_synthetic))
                    .await;

                match result {
                    Ok(mut response) => {
                        match response.check() {
                            Ok(_) => {
                                shipment_count += 1;
                                if shipment.is_synthetic {
                                    synthetic_count += 1;
                                }
                            }
                            Err(e) => {
                                if error_count < 5 {
                                    warn!("Query check failed for record {}: {}", i, e);
                                }
                                error_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        if error_count < 5 {
                            warn!("Query error for record {}: {}", i, e);
                        }
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                if error_count < 5 {
                    warn!("Failed to parse record {}: {}", i, e);
                }
                error_count += 1;
            }
        }

        if (i + 1) % 10000 == 0 {
            info!("Processed {}/{} records...", i + 1, total_records);
        }
    }

    // Create shipment graph edges if requested
    if args.graph {
        info!("Creating shipment graph edges (this may take a while)...");

        // shipped_by: shipment -> carrier
        info!("  Creating shipped_by edges...");
        db.query(r#"
            FOR $s IN (SELECT id, carrier_ref FROM shipment) {
                LET $carrier = (SELECT * FROM carrier WHERE carrier_id = $s.carrier_ref)[0];
                IF $carrier != NONE {
                    RELATE $s.id->shipped_by->$carrier.id;
                };
            };
        "#).await?;

        // origin5_at: shipment -> location5 (origin)
        info!("  Creating origin5_at edges...");
        db.query(r#"
            FOR $s IN (SELECT id, origin_zip5 FROM shipment) {
                LET $loc = (SELECT * FROM location5 WHERE zip5 = $s.origin_zip5)[0];
                IF $loc != NONE {
                    RELATE $s.id->origin5_at->$loc.id;
                };
            };
        "#).await?;

        // dest5_at: shipment -> location5 (destination)
        info!("  Creating dest5_at edges...");
        db.query(r#"
            FOR $s IN (SELECT id, dest_zip5 FROM shipment) {
                LET $loc = (SELECT * FROM location5 WHERE zip5 = $s.dest_zip5)[0];
                IF $loc != NONE {
                    RELATE $s.id->dest5_at->$loc.id;
                };
            };
        "#).await?;

        // on_lane5: shipment -> lane5
        info!("  Creating on_lane5 edges...");
        db.query(r#"
            FOR $s IN (SELECT id, lane_zip5_pair FROM shipment) {
                LET $lane = (SELECT * FROM lane5 WHERE zip5_pair = $s.lane_zip5_pair)[0];
                IF $lane != NONE {
                    RELATE $s.id->on_lane5->$lane.id;
                };
            };
        "#).await?;

        info!("Graph edges created!");
    }

    println!();
    println!("✅ Ingestion complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total shipments:     {:>8}", shipment_count);
    println!("  - Original:        {:>8}", shipment_count - synthetic_count);
    println!("  - Synthetic:       {:>8}", synthetic_count);
    println!("Errors:              {:>8}", error_count);
    if args.graph {
        println!("Graph edges:         created");
    }
    println!();

    // Verify counts
    let shipment_total: Option<i64> = db
        .query("SELECT count() FROM shipment GROUP ALL")
        .await?
        .take("count")?;
    let carrier_total: Option<i64> = db
        .query("SELECT count() FROM carrier GROUP ALL")
        .await?
        .take("count")?;
    let location3_total: Option<i64> = db
        .query("SELECT count() FROM location GROUP ALL")
        .await?
        .take("count")?;
    let location5_total: Option<i64> = db
        .query("SELECT count() FROM location5 GROUP ALL")
        .await?
        .take("count")?;
    let lane3_total: Option<i64> = db
        .query("SELECT count() FROM lane GROUP ALL")
        .await?
        .take("count")?;
    let lane5_total: Option<i64> = db
        .query("SELECT count() FROM lane5 GROUP ALL")
        .await?
        .take("count")?;

    println!("📊 Database totals:");
    println!("  Shipments:      {:>8?}", shipment_total);
    println!("  Carriers:       {:>8?}", carrier_total);
    println!("  ZIP3 Locations: {:>8?}", location3_total);
    println!("  ZIP5 Locations: {:>8?}", location5_total);
    println!("  ZIP3 Lanes:     {:>8?}", lane3_total);
    println!("  ZIP5 Lanes:     {:>8?}", lane5_total);

    if args.graph {
        let shipped_by: Option<i64> = db.query("SELECT count() FROM shipped_by GROUP ALL").await?.take("count")?;
        let origin5_at: Option<i64> = db.query("SELECT count() FROM origin5_at GROUP ALL").await?.take("count")?;
        let dest5_at: Option<i64> = db.query("SELECT count() FROM dest5_at GROUP ALL").await?.take("count")?;
        let on_lane5: Option<i64> = db.query("SELECT count() FROM on_lane5 GROUP ALL").await?.take("count")?;

        println!("\n🔗 Graph edges:");
        println!("  shipped_by:     {:>8?}", shipped_by);
        println!("  origin5_at:     {:>8?}", origin5_at);
        println!("  dest5_at:       {:>8?}", dest5_at);
        println!("  on_lane5:       {:>8?}", on_lane5);
    }

    Ok(())
}
