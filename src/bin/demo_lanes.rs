//! Lane Analysis Demo
//! Run: ./target/release/demo_lanes

use anyhow::Result;
use nyc_last_mile::{db, location_names::{format_lane_short, get_location_short, get_location_long}};
use serde::Deserialize;

/// Parse a lane_ref like "750xx->786xx" and format it nicely
fn format_lane_ref(lane_ref: &str) -> String {
    if let Some((origin, dest)) = lane_ref.split_once("->") {
        format_lane_short(origin, dest)
    } else {
        lane_ref.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct LaneVolume {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    shipments: i64,
    avg_transit: f64,
    on_time_rate: f64,
}

#[derive(Debug, Deserialize)]
struct OriginDest {
    location: String,
    shipments: i64,
}

#[derive(Debug, Deserialize)]
struct LanePerformance {
    lane_ref: String,
    shipments: i64,
    late_rate: f64,
    avg_delay: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "=".repeat(85));
    println!("                         LANE ANALYSIS DASHBOARD");
    println!("{}\n", "=".repeat(85));

    // Top lanes by volume
    let top_lanes: Vec<LaneVolume> = db
        .query(r#"
            SELECT
                lane_ref,
                origin_zip,
                dest_zip,
                count() as shipments,
                math::mean(actual_transit_days) as avg_transit,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate
            FROM shipment
            GROUP BY lane_ref, origin_zip, dest_zip
            ORDER BY shipments DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;

    println!("TOP 20 LANES BY VOLUME");
    println!("{}", "-".repeat(85));
    println!("  {:25} {:>10} {:>10} {:>15} {:>12}",
             "Lane (Origin -> Dest)", "Shipments", "Avg Days", "On-Time Rate", "Performance");
    println!("  {}", "-".repeat(79));

    for lane in &top_lanes {
        let route = format_lane_short(&lane.origin_zip, &lane.dest_zip);
        let on_time_pct = lane.on_time_rate * 100.0;
        let perf = if on_time_pct >= 70.0 { "Good" }
                   else if on_time_pct >= 50.0 { "Fair" }
                   else { "Poor" };

        println!("  {:25} {:>10} {:>10.1} {:>14.1}% {:>12}",
                 route, lane.shipments, lane.avg_transit, on_time_pct, perf);
    }

    // Top Origin DCs
    println!("\n\nTOP 10 ORIGIN DCs (Distribution Centers)");
    println!("{}", "-".repeat(50));

    let top_origins: Vec<OriginDest> = db
        .query(r#"
            SELECT
                origin_zip as location,
                count() as shipments
            FROM shipment
            GROUP BY origin_zip
            ORDER BY shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:6} {:22} {:>12}", "DC", "Location", "Shipments");
    println!("  {}", "-".repeat(42));
    for o in &top_origins {
        println!("  {:6} {:22} {:>12}",
                 get_location_short(&o.location),
                 get_location_long(&o.location),
                 o.shipments);
    }

    // Top Delivery Regions
    println!("\n\nTOP 10 DELIVERY REGIONS");
    println!("{}", "-".repeat(50));

    let top_dests: Vec<OriginDest> = db
        .query(r#"
            SELECT
                dest_zip as location,
                count() as shipments
            FROM shipment
            GROUP BY dest_zip
            ORDER BY shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:6} {:22} {:>12}", "Region", "Location", "Shipments");
    println!("  {}", "-".repeat(42));
    for d in &top_dests {
        println!("  {:6} {:22} {:>12}",
                 get_location_short(&d.location),
                 get_location_long(&d.location),
                 d.shipments);
    }

    // Worst performing lanes (high volume)
    println!("\n\nWORST PERFORMING LANES (min 50 shipments)");
    println!("{}", "-".repeat(70));

    let worst_lanes: Vec<LanePerformance> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    count() as shipments,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                GROUP BY lane_ref
            ) WHERE shipments >= 50
            ORDER BY late_rate DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>12} {:>12}",
             "Lane", "Shipments", "Late Rate", "Avg Delay");
    println!("  {}", "-".repeat(51));

    for lane in &worst_lanes {
        println!("  {:15} {:>10} {:>11.1}% {:>11.1}d",
                 format_lane_ref(&lane.lane_ref), lane.shipments, lane.late_rate * 100.0, lane.avg_delay);
    }

    // Best performing lanes (high volume)
    println!("\n\nBEST PERFORMING LANES (min 50 shipments)");
    println!("{}", "-".repeat(70));

    let best_lanes: Vec<LanePerformance> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    count() as shipments,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                GROUP BY lane_ref
            ) WHERE shipments >= 50
            ORDER BY late_rate ASC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>12} {:>12}",
             "Lane", "Shipments", "Late Rate", "Avg Delay");
    println!("  {}", "-".repeat(51));

    for lane in &best_lanes {
        let delay_str = if lane.avg_delay < 0.0 {
            format!("{:.1}d early", -lane.avg_delay)
        } else {
            format!("{:.1}d", lane.avg_delay)
        };
        println!("  {:15} {:>10} {:>11.1}% {:>12}",
                 format_lane_ref(&lane.lane_ref), lane.shipments, lane.late_rate * 100.0, delay_str);
    }

    println!("\n{}", "=".repeat(85));
    println!();

    Ok(())
}
