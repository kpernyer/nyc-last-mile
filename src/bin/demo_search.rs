//! Interactive Search Demo
//! Run: ./target/release/demo_search <query>
//! Examples:
//!   ./target/release/demo_search carrier zhxp001
//!   ./target/release/demo_search lane 100->200
//!   ./target/release/demo_search origin 100
//!   ./target/release/demo_search late
//!   ./target/release/demo_search stats

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name, location_names::format_lane_short};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct ShipmentResult {
    load_id: String,
    carrier_mode: String,
    carrier_ref: String,
    origin_zip: String,
    dest_zip: String,
    actual_transit_days: i32,
    goal_transit_days: i32,
    otd: String,
    distance_bucket: String,
}

#[derive(Debug, Deserialize)]
struct CarrierStats {
    carrier_ref: String,
    total: i64,
    on_time: i64,
    late: i64,
    avg_transit: f64,
}

#[derive(Debug, Deserialize)]
struct LaneStats {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    total: i64,
    on_time_rate: f64,
    avg_transit: f64,
}

fn print_usage() {
    println!("\nUsage: demo_search <command> [args]\n");
    println!("Commands:");
    println!("  carrier <id>     - Search shipments by carrier (partial match)");
    println!("  lane <pattern>   - Search lanes (e.g., 'DFW' or 'DFW->AUS')");
    println!("  origin <zip3>    - Search shipments from origin DC");
    println!("  dest <zip3>      - Search shipments to delivery region");
    println!("  late             - Show recent late shipments");
    println!("  early            - Show recent early shipments");
    println!("  long             - Show longest transit times");
    println!("  stats            - Quick stats summary");
    println!("\nExamples:");
    println!("  ./target/release/demo_search carrier xpo");
    println!("  ./target/release/demo_search origin 750");
    println!("  ./target/release/demo_search late");
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let db = db::connect("data/lastmile.db").await?;
    let command = args[1].as_str();

    match command {
        "carrier" => {
            let pattern = args.get(2).cloned().unwrap_or_default();
            if pattern.is_empty() {
                println!("Please provide a carrier ID pattern");
                return Ok(());
            }

            println!("\nSearching for carrier: {}...\n", pattern);

            let stats: Vec<CarrierStats> = db
                .query(r#"
                    SELECT
                        carrier_ref,
                        count() as total,
                        count(IF otd = "OnTime" THEN 1 END) as on_time,
                        count(IF otd = "Late" THEN 1 END) as late,
                        math::mean(actual_transit_days) as avg_transit
                    FROM shipment
                    WHERE carrier_ref CONTAINS $pattern
                    GROUP BY carrier_ref
                    ORDER BY total DESC
                    LIMIT 10
                "#)
                .bind(("pattern", pattern.clone()))
                .await?
                .take(0)?;

            if stats.is_empty() {
                println!("No carriers found matching '{}'", pattern);
            } else {
                println!("{:20} {:>10} {:>10} {:>10} {:>10}",
                         "Carrier", "Shipments", "On-Time", "Late", "Avg Days");
                println!("{}", "-".repeat(62));

                for s in &stats {
                    println!("{:20} {:>10} {:>10} {:>10} {:>10.1}",
                             get_carrier_name(&s.carrier_ref), s.total, s.on_time, s.late, s.avg_transit);
                }
            }
        }

        "lane" => {
            let pattern = args.get(2).cloned().unwrap_or_default();
            if pattern.is_empty() {
                println!("Please provide a lane pattern (e.g., '100' or '100->200')");
                return Ok(());
            }

            println!("\nSearching lanes matching: {}...\n", pattern);

            let lanes: Vec<LaneStats> = db
                .query(r#"
                    SELECT
                        lane_ref,
                        origin_zip,
                        dest_zip,
                        count() as total,
                        (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate,
                        math::mean(actual_transit_days) as avg_transit
                    FROM shipment
                    WHERE lane_ref CONTAINS $pattern
                       OR origin_zip CONTAINS $pattern
                       OR dest_zip CONTAINS $pattern
                    GROUP BY lane_ref, origin_zip, dest_zip
                    ORDER BY total DESC
                    LIMIT 15
                "#)
                .bind(("pattern", pattern.clone()))
                .await?
                .take(0)?;

            if lanes.is_empty() {
                println!("No lanes found matching '{}'", pattern);
            } else {
                println!("{:25} {:>10} {:>12} {:>10}",
                         "Route", "Shipments", "On-Time%", "Avg Days");
                println!("{}", "-".repeat(59));

                for l in &lanes {
                    let route = format_lane_short(&l.origin_zip, &l.dest_zip);
                    println!("{:25} {:>10} {:>11.1}% {:>10.1}",
                             route, l.total, l.on_time_rate * 100.0, l.avg_transit);
                }
            }
        }

        "origin" => {
            let zip = args.get(2).cloned().unwrap_or_default();
            if zip.is_empty() {
                println!("Please provide an origin DC ZIP3");
                return Ok(());
            }

            println!("\nShipments from Origin DC: {}...\n", zip);

            let shipments: Vec<ShipmentResult> = db
                .query(r#"
                    SELECT load_id, carrier_mode, carrier_ref, origin_zip, dest_zip,
                           actual_transit_days, goal_transit_days, otd, distance_bucket
                    FROM shipment
                    WHERE origin_zip CONTAINS $zip
                    ORDER BY actual_ship DESC
                    LIMIT 20
                "#)
                .bind(("zip", zip))
                .await?
                .take(0)?;

            print_shipments(&shipments);
        }

        "dest" => {
            let zip = args.get(2).cloned().unwrap_or_default();
            if zip.is_empty() {
                println!("Please provide a delivery region ZIP3");
                return Ok(());
            }

            println!("\nShipments to Delivery Region: {}...\n", zip);

            let shipments: Vec<ShipmentResult> = db
                .query(r#"
                    SELECT load_id, carrier_mode, carrier_ref, origin_zip, dest_zip,
                           actual_transit_days, goal_transit_days, otd, distance_bucket
                    FROM shipment
                    WHERE dest_zip CONTAINS $zip
                    ORDER BY actual_ship DESC
                    LIMIT 20
                "#)
                .bind(("zip", zip))
                .await?
                .take(0)?;

            print_shipments(&shipments);
        }

        "late" => {
            println!("\nRecent LATE shipments...\n");

            let shipments: Vec<ShipmentResult> = db
                .query(r#"
                    SELECT load_id, carrier_mode, carrier_ref, origin_zip, dest_zip,
                           actual_transit_days, goal_transit_days, otd, distance_bucket
                    FROM shipment
                    WHERE otd = "Late"
                    ORDER BY actual_transit_days DESC
                    LIMIT 25
                "#)
                .await?
                .take(0)?;

            print_shipments(&shipments);
        }

        "early" => {
            println!("\nRecent EARLY shipments...\n");

            let shipments: Vec<ShipmentResult> = db
                .query(r#"
                    SELECT load_id, carrier_mode, carrier_ref, origin_zip, dest_zip,
                           actual_transit_days, goal_transit_days, otd, distance_bucket
                    FROM shipment
                    WHERE otd = "Early"
                    ORDER BY actual_transit_days ASC
                    LIMIT 25
                "#)
                .await?
                .take(0)?;

            print_shipments(&shipments);
        }

        "long" => {
            println!("\nLongest transit times...\n");

            let shipments: Vec<ShipmentResult> = db
                .query(r#"
                    SELECT load_id, carrier_mode, carrier_ref, origin_zip, dest_zip,
                           actual_transit_days, goal_transit_days, otd, distance_bucket
                    FROM shipment
                    ORDER BY actual_transit_days DESC
                    LIMIT 25
                "#)
                .await?
                .take(0)?;

            print_shipments(&shipments);
        }

        "stats" => {
            println!("\n=== QUICK STATS ===\n");

            #[derive(Debug, Deserialize)]
            struct QuickStats {
                total: i64,
                on_time: i64,
                late: i64,
                early: i64,
                avg_transit: f64,
            }

            let stats: Option<QuickStats> = db
                .query(r#"
                    SELECT
                        count() as total,
                        count(IF otd = "OnTime" THEN 1 END) as on_time,
                        count(IF otd = "Late" THEN 1 END) as late,
                        count(IF otd = "Early" THEN 1 END) as early,
                        math::mean(actual_transit_days) as avg_transit
                    FROM shipment
                    GROUP ALL
                "#)
                .await?
                .take(0)?;

            if let Some(s) = stats {
                println!("Total Shipments:    {:>10}", s.total);
                println!("On-Time:            {:>10} ({:.1}%)", s.on_time, (s.on_time as f64 / s.total as f64) * 100.0);
                println!("Late:               {:>10} ({:.1}%)", s.late, (s.late as f64 / s.total as f64) * 100.0);
                println!("Early:              {:>10} ({:.1}%)", s.early, (s.early as f64 / s.total as f64) * 100.0);
                println!("Avg Transit Days:   {:>10.1}", s.avg_transit);
            }
        }

        _ => {
            println!("Unknown command: {}", command);
            print_usage();
        }
    }

    println!();
    Ok(())
}

fn print_shipments(shipments: &[ShipmentResult]) {
    if shipments.is_empty() {
        println!("No shipments found.");
        return;
    }

    println!("{:15} {:12} {:20} {:12} {:>6} {:>6} {:>8}",
             "Load ID", "Mode", "Carrier", "Route", "Goal", "Actual", "Status");
    println!("{}", "-".repeat(87));

    for s in shipments {
        let route = format_lane_short(&s.origin_zip, &s.dest_zip);
        let delay = s.actual_transit_days - s.goal_transit_days;
        let status = if delay > 0 {
            format!("{} (+{})", s.otd, delay)
        } else if delay < 0 {
            format!("{} ({})", s.otd, delay)
        } else {
            s.otd.clone()
        };

        println!("{:15} {:12} {:20} {:12} {:>6} {:>6} {:>8}",
                 s.load_id, s.carrier_mode, get_carrier_name(&s.carrier_ref), route,
                 s.goal_transit_days, s.actual_transit_days, status);
    }
}
