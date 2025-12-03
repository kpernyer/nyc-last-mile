use anyhow::Result;
use csv::ReaderBuilder;
use nyc_last_mile::{db, models::CsvRecord};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let csv_path = PathBuf::from("raw-data/last-mile-data.csv");
    let db_path = "data/lastmile.db";

    info!("Connecting to SurrealDB at {}", db_path);
    let db = db::connect(db_path).await?;

    info!("Initializing schema...");
    db::init_schema(&db).await?;

    info!("Reading CSV from {:?}", csv_path);
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&csv_path)?;

    // Track unique entities
    let mut carriers: HashSet<String> = HashSet::new();
    let mut locations: HashSet<String> = HashSet::new();
    let mut lanes: HashSet<(String, String)> = HashSet::new();

    let mut shipment_count = 0;
    let mut error_count = 0;

    // First pass: collect unique entities and shipments
    let records: Vec<CsvRecord> = reader
        .deserialize()
        .filter_map(|r| r.ok())
        .collect();

    info!("Parsed {} records from CSV", records.len());

    // Collect unique entities
    for record in &records {
        carriers.insert(record.carrier_pseudo.clone());
        locations.insert(record.origin_zip_3d.clone());
        locations.insert(record.dest_zip_3d.clone());
        lanes.insert((record.lane_id.clone(), record.lane_zip3_pair.clone()));
    }

    info!(
        "Found {} carriers, {} locations, {} lanes",
        carriers.len(),
        locations.len(),
        lanes.len()
    );

    // Insert carriers
    info!("Inserting carriers...");
    for carrier_id in &carriers {
        db.query("CREATE carrier SET carrier_id = $carrier_id")
            .bind(("carrier_id", carrier_id.clone()))
            .await?
            .check()?;
    }

    // Insert locations
    info!("Inserting locations...");
    for zip3 in &locations {
        // Extract state if present (e.g., "PA" from "PA→TX" or just use zip)
        let state = extract_state(zip3);
        db.query("CREATE location SET zip3 = $zip3, state = $state")
            .bind(("zip3", zip3.clone()))
            .bind(("state", state))
            .await?
            .check()?;
    }

    // Insert lanes
    info!("Inserting lanes...");
    for (lane_id, zip3_pair) in &lanes {
        db.query("CREATE lane SET lane_id = $lane_id, zip3_pair = $zip3_pair")
            .bind(("lane_id", lane_id.clone()))
            .bind(("zip3_pair", zip3_pair.clone()))
            .await?
            .check()?;
    }

    // Insert shipments and relationships
    info!("Inserting shipments and relationships...");
    for (i, record) in records.iter().enumerate() {
        match record.to_shipment() {
            Ok(shipment) => {
                // Insert shipment
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
                            origin_zip: $origin_zip,
                            dest_zip: $dest_zip,
                            lane_ref: $lane_id
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
                    .bind(("origin_zip", record.origin_zip_3d.clone()))
                    .bind(("dest_zip", record.dest_zip_3d.clone()))
                    .bind(("lane_id", record.lane_id.clone()))
                    .await;

                match result {
                    Ok(mut response) => {
                        match response.check() {
                            Ok(_) => shipment_count += 1,
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

        if (i + 1) % 5000 == 0 {
            info!("Processed {}/{} records...", i + 1, records.len());
        }
    }

    info!(
        "Ingestion complete: {} shipments, {} errors",
        shipment_count, error_count
    );

    // Verify counts
    let shipment_total: Option<i64> = db
        .query("SELECT count() FROM shipment GROUP ALL")
        .await?
        .take("count")?;
    let carrier_total: Option<i64> = db
        .query("SELECT count() FROM carrier GROUP ALL")
        .await?
        .take("count")?;
    let location_total: Option<i64> = db
        .query("SELECT count() FROM location GROUP ALL")
        .await?
        .take("count")?;
    let lane_total: Option<i64> = db
        .query("SELECT count() FROM lane GROUP ALL")
        .await?
        .take("count")?;

    info!("Database totals:");
    info!("  Shipments: {:?}", shipment_total);
    info!("  Carriers: {:?}", carrier_total);
    info!("  Locations: {:?}", location_total);
    info!("  Lanes: {:?}", lane_total);

    Ok(())
}

fn extract_state(zip3: &str) -> Option<String> {
    // If it's a 2-letter state code, return it
    if zip3.len() == 2 && zip3.chars().all(|c| c.is_ascii_uppercase()) {
        return Some(zip3.to_string());
    }
    // Check if it contains state info like "PA→TX"
    if zip3.contains('→') {
        let parts: Vec<&str> = zip3.split('→').collect();
        if parts.len() == 2 && parts[0].len() == 2 {
            return Some(parts[0].to_string());
        }
    }
    None
}
