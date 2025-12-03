//! On-Time Delivery Analysis Demo
//! Run: ./target/release/demo_otd

use anyhow::Result;
use nyc_last_mile::db;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WeeklyOtd {
    ship_week: i32,
    ship_year: i32,
    total: i64,
    on_time: i64,
    late: i64,
}

#[derive(Debug, Deserialize)]
struct DowOtd {
    ship_dow: i32,
    total: i64,
    on_time_rate: f64,
    late_rate: f64,
}

#[derive(Debug, Deserialize)]
struct ModeOtd {
    carrier_mode: String,
    total: i64,
    on_time_rate: f64,
    late_rate: f64,
    avg_transit: f64,
}

#[derive(Debug, Deserialize)]
struct DistanceOtd {
    distance_bucket: String,
    total: i64,
    on_time_rate: f64,
    late_rate: f64,
    avg_delay: f64,
}

#[derive(Debug, Deserialize)]
struct MonthlyTrend {
    ship_month: i32,
    ship_year: i32,
    total: i64,
    on_time_rate: f64,
}

fn dow_name(dow: i32) -> &'static str {
    match dow {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

fn month_name(m: i32) -> &'static str {
    match m {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "???",
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "=".repeat(75));
    println!("                ON-TIME DELIVERY ANALYSIS");
    println!("{}\n", "=".repeat(75));

    // OTD by day of week
    println!("ON-TIME PERFORMANCE BY DAY OF WEEK");
    println!("{}", "-".repeat(60));

    let dow_stats: Vec<DowOtd> = db
        .query(r#"
            SELECT
                ship_dow,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_rate
            FROM shipment
            GROUP BY ship_dow
            ORDER BY ship_dow
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>10} {:>12} {:>12} {:>10}",
             "Day", "Shipments", "On-Time%", "Late%", "Visual");
    println!("  {}", "-".repeat(54));

    for stat in &dow_stats {
        let on_time_pct = stat.on_time_rate * 100.0;
        let late_pct = stat.late_rate * 100.0;
        let bar_len = (on_time_pct / 5.0) as usize;
        let bar: String = "#".repeat(bar_len);

        println!("  {:12} {:>10} {:>11.1}% {:>11.1}% {}",
                 dow_name(stat.ship_dow), stat.total, on_time_pct, late_pct, bar);
    }

    // OTD by carrier mode
    println!("\n\nON-TIME PERFORMANCE BY CARRIER MODE");
    println!("{}", "-".repeat(70));

    let mode_stats: Vec<ModeOtd> = db
        .query(r#"
            SELECT
                carrier_mode,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY carrier_mode
            ORDER BY total DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>12} {:>12} {:>12}",
             "Mode", "Shipments", "On-Time%", "Late%", "Avg Transit");
    println!("  {}", "-".repeat(63));

    for stat in &mode_stats {
        println!("  {:15} {:>10} {:>11.1}% {:>11.1}% {:>11.1}d",
                 stat.carrier_mode, stat.total,
                 stat.on_time_rate * 100.0, stat.late_rate * 100.0, stat.avg_transit);
    }

    // OTD by distance
    println!("\n\nON-TIME PERFORMANCE BY DISTANCE");
    println!("{}", "-".repeat(75));

    let distance_stats: Vec<DistanceOtd> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                math::mean(actual_transit_days - goal_transit_days) as avg_delay
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>12} {:>12} {:>12} {:>10}",
             "Distance", "Shipments", "On-Time%", "Late%", "Avg Delay", "Risk");
    println!("  {}", "-".repeat(73));

    for stat in &distance_stats {
        let late_pct = stat.late_rate * 100.0;
        let risk = if late_pct > 25.0 { "HIGH" }
                   else if late_pct > 15.0 { "MEDIUM" }
                   else { "LOW" };

        let delay_str = if stat.avg_delay >= 0.0 {
            format!("+{:.2}d", stat.avg_delay)
        } else {
            format!("{:.2}d", stat.avg_delay)
        };

        println!("  {:15} {:>10} {:>11.1}% {:>11.1}% {:>12} {:>10}",
                 stat.distance_bucket, stat.total,
                 stat.on_time_rate * 100.0, late_pct, delay_str, risk);
    }

    // Monthly trend
    println!("\n\nMONTHLY ON-TIME TREND");
    println!("{}", "-".repeat(60));

    let monthly: Vec<MonthlyTrend> = db
        .query(r#"
            SELECT
                ship_month,
                ship_year,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as on_time_rate
            FROM shipment
            GROUP BY ship_year, ship_month
            ORDER BY ship_year, ship_month
        "#)
        .await?
        .take(0)?;

    println!("  {:10} {:>10} {:>12} {:>20}",
             "Month", "Shipments", "On-Time%", "Trend");
    println!("  {}", "-".repeat(54));

    for stat in &monthly {
        let on_time_pct = stat.on_time_rate * 100.0;
        let bar_len = (on_time_pct / 5.0) as usize;
        let bar: String = "|".repeat(bar_len);

        println!("  {} {:4} {:>10} {:>11.1}% {}",
                 month_name(stat.ship_month), stat.ship_year,
                 stat.total, on_time_pct, bar);
    }

    // Summary insights
    println!("\n\nKEY INSIGHTS");
    println!("{}", "-".repeat(60));

    // Best day
    if let Some(best_day) = dow_stats.iter().max_by(|a, b|
        a.on_time_rate.partial_cmp(&b.on_time_rate).unwrap()) {
        println!("  Best shipping day:    {} ({:.1}% on-time)",
                 dow_name(best_day.ship_dow), best_day.on_time_rate * 100.0);
    }

    // Worst day
    if let Some(worst_day) = dow_stats.iter().max_by(|a, b|
        a.late_rate.partial_cmp(&b.late_rate).unwrap()) {
        println!("  Highest late rate:    {} ({:.1}% late)",
                 dow_name(worst_day.ship_dow), worst_day.late_rate * 100.0);
    }

    // Best mode
    if let Some(best_mode) = mode_stats.iter().max_by(|a, b|
        a.on_time_rate.partial_cmp(&b.on_time_rate).unwrap()) {
        println!("  Best carrier mode:    {} ({:.1}% on-time)",
                 best_mode.carrier_mode, best_mode.on_time_rate * 100.0);
    }

    println!("\n{}", "=".repeat(75));
    println!();

    Ok(())
}
