//! Database Statistics Demo
//! Run: ./target/release/demo_stats

use anyhow::Result;
use nyc_last_mile::db;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CountResult {
    count: i64,
}

#[derive(Debug, Deserialize)]
struct OtdStat {
    otd: String,
    cnt: i64,
}

#[derive(Debug, Deserialize)]
struct ModeStat {
    carrier_mode: String,
    cnt: i64,
}

#[derive(Debug, Deserialize)]
struct DistanceStat {
    distance_bucket: String,
    cnt: i64,
    avg_transit: f64,
    avg_goal: f64,
}

#[derive(Debug, Deserialize)]
struct DateRange {
    min_date: Option<String>,
    max_date: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "=".repeat(60));
    println!("         NYC LAST-MILE DELIVERY DATABASE STATISTICS");
    println!("{}\n", "=".repeat(60));

    // Entity counts
    let shipments: Option<CountResult> = db
        .query("SELECT count() as count FROM shipment GROUP ALL")
        .await?
        .take(0)?;
    let carriers: Option<CountResult> = db
        .query("SELECT count() as count FROM carrier GROUP ALL")
        .await?
        .take(0)?;
    let locations: Option<CountResult> = db
        .query("SELECT count() as count FROM location GROUP ALL")
        .await?
        .take(0)?;
    let lanes: Option<CountResult> = db
        .query("SELECT count() as count FROM lane GROUP ALL")
        .await?
        .take(0)?;

    println!("ENTITY COUNTS");
    println!("{}", "-".repeat(40));
    println!("  Shipments:  {:>10}", shipments.map(|c| c.count).unwrap_or(0));
    println!("  Carriers:   {:>10}", carriers.map(|c| c.count).unwrap_or(0));
    println!("  Locations:  {:>10}", locations.map(|c| c.count).unwrap_or(0));
    println!("  Lanes:      {:>10}", lanes.map(|c| c.count).unwrap_or(0));

    // Date range
    let date_range: Option<DateRange> = db
        .query("SELECT math::min(actual_ship) as min_date, math::max(actual_ship) as max_date FROM shipment GROUP ALL")
        .await?
        .take(0)?;

    if let Some(dr) = date_range {
        println!("\nDATE RANGE");
        println!("{}", "-".repeat(40));
        println!("  From: {}", dr.min_date.unwrap_or_default().split('T').next().unwrap_or("N/A"));
        println!("  To:   {}", dr.max_date.unwrap_or_default().split('T').next().unwrap_or("N/A"));
    }

    // OTD Distribution
    let otd_stats: Vec<OtdStat> = db
        .query("SELECT otd, count() as cnt FROM shipment GROUP BY otd ORDER BY cnt DESC")
        .await?
        .take(0)?;

    let total: i64 = otd_stats.iter().map(|s| s.cnt).sum();
    println!("\nON-TIME DELIVERY PERFORMANCE");
    println!("{}", "-".repeat(40));
    for stat in &otd_stats {
        let pct = (stat.cnt as f64 / total as f64) * 100.0;
        let bar_len = (pct / 2.0) as usize;
        let bar: String = "#".repeat(bar_len);
        println!("  {:8} {:>6} ({:>5.1}%) {}", stat.otd, stat.cnt, pct, bar);
    }

    // Carrier Mode Distribution
    let mode_stats: Vec<ModeStat> = db
        .query("SELECT carrier_mode, count() as cnt FROM shipment GROUP BY carrier_mode ORDER BY cnt DESC")
        .await?
        .take(0)?;

    println!("\nCARRIER MODE DISTRIBUTION");
    println!("{}", "-".repeat(40));
    for stat in &mode_stats {
        let pct = (stat.cnt as f64 / total as f64) * 100.0;
        println!("  {:12} {:>6} ({:>5.1}%)", stat.carrier_mode, stat.cnt, pct);
    }

    // Distance Bucket Analysis
    let distance_stats: Vec<DistanceStat> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as cnt,
                math::mean(actual_transit_days) as avg_transit,
                math::mean(goal_transit_days) as avg_goal
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("\nTRANSIT TIME BY DISTANCE");
    println!("{}", "-".repeat(60));
    println!("  {:>12}  {:>8}  {:>10}  {:>10}  {:>8}",
             "Distance", "Count", "Avg Goal", "Avg Actual", "Delta");
    println!("  {:>12}  {:>8}  {:>10}  {:>10}  {:>8}",
             "", "", "(days)", "(days)", "(days)");
    println!("  {}", "-".repeat(54));

    for stat in &distance_stats {
        let delta = stat.avg_transit - stat.avg_goal;
        let delta_str = if delta > 0.0 {
            format!("+{:.2}", delta)
        } else {
            format!("{:.2}", delta)
        };
        println!("  {:>12}  {:>8}  {:>10.2}  {:>10.2}  {:>8}",
                 stat.distance_bucket, stat.cnt, stat.avg_goal, stat.avg_transit, delta_str);
    }

    println!("\n{}", "=".repeat(60));
    println!();

    Ok(())
}
