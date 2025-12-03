//! Carrier Performance Analysis Demo
//! Run: ./target/release/demo_carriers

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CarrierPerformance {
    carrier_ref: String,
    total_shipments: i64,
    on_time: i64,
    late: i64,
    early: i64,
    avg_transit: f64,
}

#[derive(Debug, Deserialize)]
struct CarrierMode {
    carrier_ref: String,
    carrier_mode: String,
    cnt: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "=".repeat(80));
    println!("                    CARRIER PERFORMANCE ANALYSIS");
    println!("{}\n", "=".repeat(80));

    // Top carriers by volume
    let top_carriers: Vec<CarrierPerformance> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total_shipments,
                    count(IF otd = "OnTime" THEN 1 END) as on_time,
                    count(IF otd = "Late" THEN 1 END) as late,
                    count(IF otd = "Early" THEN 1 END) as early,
                    math::mean(actual_transit_days) as avg_transit
                FROM shipment
                GROUP BY carrier_ref
            ) ORDER BY total_shipments DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("TOP 15 CARRIERS BY VOLUME");
    println!("{}", "-".repeat(84));
    println!("  {:24} {:>10} {:>10} {:>10} {:>10} {:>10}",
             "Carrier", "Shipments", "On-Time%", "Late%", "Early%", "Avg Days");
    println!("  {}", "-".repeat(78));

    for c in &top_carriers {
        let name = get_carrier_name(&c.carrier_ref);
        let on_time_pct = (c.on_time as f64 / c.total_shipments as f64) * 100.0;
        let late_pct = (c.late as f64 / c.total_shipments as f64) * 100.0;
        let early_pct = (c.early as f64 / c.total_shipments as f64) * 100.0;

        println!("  {:24} {:>10} {:>9.1}% {:>9.1}% {:>9.1}% {:>10.2}",
                 name, c.total_shipments, on_time_pct, late_pct, early_pct, c.avg_transit);
    }

    // Best performing carriers (minimum 100 shipments)
    println!("\n\nBEST ON-TIME PERFORMANCE (min 100 shipments)");
    println!("{}", "-".repeat(80));

    let best_carriers: Vec<CarrierPerformance> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total_shipments,
                    count(IF otd = "OnTime" THEN 1 END) as on_time,
                    count(IF otd = "Late" THEN 1 END) as late,
                    count(IF otd = "Early" THEN 1 END) as early,
                    math::mean(actual_transit_days) as avg_transit
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE total_shipments >= 100
            ORDER BY on_time DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:24} {:>10} {:>10} {:>10}",
             "Carrier", "Shipments", "On-Time%", "Avg Days");
    println!("  {}", "-".repeat(58));

    for c in &best_carriers {
        let name = get_carrier_name(&c.carrier_ref);
        let on_time_pct = (c.on_time as f64 / c.total_shipments as f64) * 100.0;
        println!("  {:24} {:>10} {:>9.1}% {:>10.2}",
                 name, c.total_shipments, on_time_pct, c.avg_transit);
    }

    // Worst performing carriers
    println!("\n\nWORST ON-TIME PERFORMANCE (min 100 shipments)");
    println!("{}", "-".repeat(80));

    let worst_carriers: Vec<CarrierPerformance> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total_shipments,
                    count(IF otd = "OnTime" THEN 1 END) as on_time,
                    count(IF otd = "Late" THEN 1 END) as late,
                    count(IF otd = "Early" THEN 1 END) as early,
                    math::mean(actual_transit_days) as avg_transit
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE total_shipments >= 100
            ORDER BY late DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:24} {:>10} {:>10} {:>10}",
             "Carrier", "Shipments", "Late%", "Avg Days");
    println!("  {}", "-".repeat(58));

    for c in &worst_carriers {
        let name = get_carrier_name(&c.carrier_ref);
        let late_pct = (c.late as f64 / c.total_shipments as f64) * 100.0;
        println!("  {:24} {:>10} {:>9.1}% {:>10.2}",
                 name, c.total_shipments, late_pct, c.avg_transit);
    }

    // Carrier mode breakdown for top carrier
    if let Some(top) = top_carriers.first() {
        let top_name = get_carrier_name(&top.carrier_ref);
        println!("\n\nTOP CARRIER ({}) - MODE BREAKDOWN", top_name);
        println!("{}", "-".repeat(50));

        let modes: Vec<CarrierMode> = db
            .query(r#"
                SELECT
                    carrier_ref,
                    carrier_mode,
                    count() as cnt
                FROM shipment
                WHERE carrier_ref = $carrier
                GROUP BY carrier_ref, carrier_mode
                ORDER BY cnt DESC
            "#)
            .bind(("carrier", top.carrier_ref.clone()))
            .await?
            .take(0)?;

        for m in &modes {
            let pct = (m.cnt as f64 / top.total_shipments as f64) * 100.0;
            println!("  {:15} {:>8} ({:>5.1}%)", m.carrier_mode, m.cnt, pct);
        }
    }

    println!("\n{}", "=".repeat(80));
    println!();

    Ok(())
}
