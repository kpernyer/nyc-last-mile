//! Descriptive Analytics - What is happening?
//! Comprehensive KPIs and performance metrics
//!
//! Run: ./target/release/analytics_descriptive [section]
//! Sections: all, kpi, transit, volume, distribution

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct KpiMetric {
    total: i64,
    on_time: i64,
    late: i64,
    early: i64,
    avg_transit: f64,
    avg_goal: f64,
}

#[derive(Debug, Deserialize)]
struct GroupedOtd {
    group: String,
    total: i64,
    otd_rate: f64,
    late_rate: f64,
    early_rate: f64,
    avg_transit: f64,
}

#[derive(Debug, Deserialize)]
struct TransitDistribution {
    transit_days: i64,
    count: i64,
}

#[derive(Debug, Deserialize)]
struct VarianceMetric {
    group: String,
    total: i64,
    avg_transit: f64,
    min_transit: i64,
    max_transit: i64,
    variance: f64,
}

#[derive(Debug, Deserialize)]
struct VolumeMetric {
    group: String,
    shipments: i64,
    pct_of_total: f64,
}

#[derive(Debug, Deserialize)]
struct MonthlyTrend {
    year_month: String,
    shipments: i64,
    otd_rate: f64,
    avg_transit: f64,
}

#[derive(Debug, Deserialize)]
struct VeryLateMetric {
    category: String,
    count: i64,
    pct: f64,
}

fn print_section_header(title: &str) {
    println!("\n{}", "═".repeat(80));
    println!("  {}", title);
    println!("{}\n", "═".repeat(80));
}

fn print_subsection(title: &str) {
    println!("\n{}", title);
    println!("{}", "─".repeat(70));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "█".repeat(80));
    println!("{}  DESCRIPTIVE ANALYTICS - What is Happening?  {}", "█".repeat(15), "█".repeat(16));
    println!("{}\n", "█".repeat(80));

    match section {
        "all" => {
            run_kpi_section(&db).await?;
            run_transit_section(&db).await?;
            run_volume_section(&db).await?;
            run_distribution_section(&db).await?;
        }
        "kpi" => run_kpi_section(&db).await?,
        "transit" => run_transit_section(&db).await?,
        "volume" => run_volume_section(&db).await?,
        "distribution" => run_distribution_section(&db).await?,
        _ => {
            println!("Unknown section: {}", section);
            println!("Available: all, kpi, transit, volume, distribution");
        }
    }

    println!("\n{}", "█".repeat(80));
    Ok(())
}

async fn run_kpi_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("1. BASIC PERFORMANCE KPIs");

    // Overall KPIs
    print_subsection("Overall Network Performance");

    let overall: Option<KpiMetric> = db
        .query(r#"
            SELECT
                count() as total,
                count(IF otd = "OnTime" THEN 1 END) as on_time,
                count(IF otd = "Late" THEN 1 END) as late,
                count(IF otd = "Early" THEN 1 END) as early,
                math::mean(actual_transit_days) as avg_transit,
                math::mean(goal_transit_days) as avg_goal
            FROM shipment
            GROUP ALL
        "#)
        .await?
        .take(0)?;

    if let Some(kpi) = overall {
        let otd_rate = (kpi.on_time as f64 / kpi.total as f64) * 100.0;
        let late_rate = (kpi.late as f64 / kpi.total as f64) * 100.0;
        let early_rate = (kpi.early as f64 / kpi.total as f64) * 100.0;
        let delta = kpi.avg_transit - kpi.avg_goal;

        println!("  Total Shipments:      {:>12}", kpi.total);
        println!("  On-Time Rate:         {:>11.1}%", otd_rate);
        println!("  Late Rate:            {:>11.1}%", late_rate);
        println!("  Early Rate:           {:>11.1}%", early_rate);
        println!("  Avg Transit Days:     {:>12.2}", kpi.avg_transit);
        println!("  Avg Goal Days:        {:>12.2}", kpi.avg_goal);
        println!("  Avg Delta:            {:>+12.2} days", delta);
    }

    // OTD by Carrier (Top 15)
    print_subsection("OTD by Carrier (Top 15 by Volume)");

    #[derive(Debug, Deserialize)]
    struct CarrierOtd {
        carrier_ref: String,
        total: i64,
        on_time: i64,
        late: i64,
        early: i64,
        avg_transit: f64,
    }

    let by_carrier: Vec<CarrierOtd> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total,
                    count(IF otd = "OnTime" THEN 1 END) as on_time,
                    count(IF otd = "Late" THEN 1 END) as late,
                    count(IF otd = "Early" THEN 1 END) as early,
                    math::mean(actual_transit_days) as avg_transit
                FROM shipment
                GROUP BY carrier_ref
            )
            ORDER BY total DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:22} {:>8} {:>10} {:>10} {:>10} {:>10}",
             "Carrier", "Volume", "OTD%", "Late%", "Early%", "Avg Days");
    println!("  {}", "─".repeat(72));
    for row in &by_carrier {
        let name = get_carrier_name(&row.carrier_ref);
        let otd_rate = row.on_time as f64 / row.total as f64 * 100.0;
        let late_rate = row.late as f64 / row.total as f64 * 100.0;
        let early_rate = row.early as f64 / row.total as f64 * 100.0;
        println!("  {:22} {:>8} {:>9.1}% {:>9.1}% {:>9.1}% {:>10.1}",
                 name, row.total, otd_rate, late_rate, early_rate, row.avg_transit);
    }

    // OTD by Mode
    print_subsection("OTD by Carrier Mode");

    #[derive(Debug, Deserialize)]
    struct ModeOtd {
        carrier_mode: String,
        total: i64,
        on_time: i64,
        late: i64,
        early: i64,
        avg_transit: f64,
    }

    let by_mode: Vec<ModeOtd> = db
        .query(r#"
            SELECT
                carrier_mode,
                count() as total,
                count(IF otd = "OnTime" THEN 1 END) as on_time,
                count(IF otd = "Late" THEN 1 END) as late,
                count(IF otd = "Early" THEN 1 END) as early,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY carrier_mode
            ORDER BY total DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>10} {:>10} {:>10} {:>10}",
             "Mode", "Volume", "OTD%", "Late%", "Early%", "Avg Days");
    println!("  {}", "─".repeat(65));
    for row in &by_mode {
        let otd_rate = row.on_time as f64 / row.total as f64 * 100.0;
        let late_rate = row.late as f64 / row.total as f64 * 100.0;
        let early_rate = row.early as f64 / row.total as f64 * 100.0;
        println!("  {:15} {:>10} {:>9.1}% {:>9.1}% {:>9.1}% {:>10.1}",
                 row.carrier_mode, row.total, otd_rate, late_rate, early_rate, row.avg_transit);
    }

    // OTD by Day of Week
    print_subsection("OTD by Day of Week (Pickup Day)");

    #[derive(Debug, Deserialize)]
    struct DowOtd {
        ship_dow: i32,
        total: i64,
        on_time: i64,
        late: i64,
        early: i64,
        avg_transit: f64,
    }

    let by_dow: Vec<DowOtd> = db
        .query(r#"
            SELECT
                ship_dow,
                count() as total,
                count(IF otd = "OnTime" THEN 1 END) as on_time,
                count(IF otd = "Late" THEN 1 END) as late,
                count(IF otd = "Early" THEN 1 END) as early,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY ship_dow
            ORDER BY ship_dow
        "#)
        .await?
        .take(0)?;

    let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    println!("  {:10} {:>10} {:>10} {:>10} {:>10} {:>10}",
             "Day", "Volume", "OTD%", "Late%", "Early%", "Avg Days");
    println!("  {}", "─".repeat(60));
    for row in &by_dow {
        let day = dow_names.get(row.ship_dow as usize).unwrap_or(&"???");
        let otd_rate = row.on_time as f64 / row.total as f64 * 100.0;
        let late_rate = row.late as f64 / row.total as f64 * 100.0;
        let early_rate = row.early as f64 / row.total as f64 * 100.0;
        println!("  {:10} {:>10} {:>9.1}% {:>9.1}% {:>9.1}% {:>10.1}",
                 day, row.total, otd_rate, late_rate, early_rate, row.avg_transit);
    }

    // OTD by Distance Bucket
    print_subsection("OTD by Distance Segment");

    #[derive(Debug, Deserialize)]
    struct DistOtd {
        distance_bucket: String,
        total: i64,
        on_time: i64,
        late: i64,
        early: i64,
        avg_transit: f64,
    }

    let by_distance: Vec<DistOtd> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as total,
                count(IF otd = "OnTime" THEN 1 END) as on_time,
                count(IF otd = "Late" THEN 1 END) as late,
                count(IF otd = "Early" THEN 1 END) as early,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>10} {:>10} {:>10} {:>10} {:>10}",
             "Distance", "Volume", "OTD%", "Late%", "Early%", "Avg Days");
    println!("  {}", "─".repeat(62));
    for row in &by_distance {
        let otd_rate = row.on_time as f64 / row.total as f64 * 100.0;
        let late_rate = row.late as f64 / row.total as f64 * 100.0;
        let early_rate = row.early as f64 / row.total as f64 * 100.0;
        println!("  {:12} {:>10} {:>9.1}% {:>9.1}% {:>9.1}% {:>10.1}",
                 row.distance_bucket, row.total, otd_rate, late_rate, early_rate, row.avg_transit);
    }

    Ok(())
}

async fn run_transit_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("2. TRANSIT TIME PERFORMANCE");

    // Actual vs Goal by Distance
    print_subsection("Actual vs Goal Transit Days by Distance");

    #[derive(Debug, Deserialize)]
    struct TransitComparison {
        distance_bucket: String,
        total: i64,
        avg_actual: f64,
        avg_goal: f64,
        delta: f64,
    }

    let transit_comp: Vec<TransitComparison> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as total,
                math::mean(actual_transit_days) as avg_actual,
                math::mean(goal_transit_days) as avg_goal,
                math::mean(actual_transit_days - goal_transit_days) as delta
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>8} {:>12} {:>12} {:>12} {:>10}",
             "Distance", "Volume", "Avg Actual", "Avg Goal", "Delta", "Status");
    println!("  {}", "─".repeat(66));
    for row in &transit_comp {
        let status = if row.delta > 0.5 { "⚠ SLOW" }
                    else if row.delta < -0.5 { "✓ FAST" }
                    else { "→ OK" };
        println!("  {:12} {:>8} {:>11.2}d {:>11.2}d {:>+11.2}d {:>10}",
                 row.distance_bucket, row.total, row.avg_actual, row.avg_goal, row.delta, status);
    }

    // Transit Time Variance by Carrier
    print_subsection("Transit Time Variance by Carrier (Top 10 by Volume)");

    let variance: Vec<VarianceMetric> = db
        .query(r#"
            SELECT
                carrier_ref as group,
                count() as total,
                math::mean(actual_transit_days) as avg_transit,
                math::min(actual_transit_days) as min_transit,
                math::max(actual_transit_days) as max_transit,
                math::variance(actual_transit_days) as variance
            FROM shipment
            GROUP BY carrier_ref
            ORDER BY total DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>8} {:>8} {:>6} {:>6} {:>10} {:>10}",
             "Carrier", "Volume", "Avg", "Min", "Max", "Variance", "Reliability");
    println!("  {}", "─".repeat(70));
    for row in &variance {
        let reliability = if row.variance < 2.0 { "High" }
                         else if row.variance < 5.0 { "Medium" }
                         else { "Low" };
        println!("  {:20} {:>8} {:>7.1}d {:>5}d {:>5}d {:>10.2} {:>10}",
                 get_carrier_name(&row.group), row.total, row.avg_transit, row.min_transit,
                 row.max_transit, row.variance, reliability);
    }

    // Very Late Shipments Analysis
    print_subsection("Very Late Shipments (>2 days late)");

    let very_late: Vec<VeryLateMetric> = db
        .query(r#"
            SELECT
                "Very Late (>2d)" as category,
                count(IF (actual_transit_days - goal_transit_days) > 2 THEN 1 END) as count,
                (count(IF (actual_transit_days - goal_transit_days) > 2 THEN 1 END) / count()) as pct
            FROM shipment
            GROUP ALL
        "#)
        .await?
        .take(0)?;

    if let Some(vl) = very_late.first() {
        println!("  Very Late (>2 days): {:>8} shipments ({:.1}%)", vl.count, vl.pct * 100.0);
    }

    // Extremely late
    let extreme_late: Vec<VeryLateMetric> = db
        .query(r#"
            SELECT
                "Extreme Late (>5d)" as category,
                count(IF (actual_transit_days - goal_transit_days) > 5 THEN 1 END) as count,
                (count(IF (actual_transit_days - goal_transit_days) > 5 THEN 1 END) / count()) as pct
            FROM shipment
            GROUP ALL
        "#)
        .await?
        .take(0)?;

    if let Some(el) = extreme_late.first() {
        println!("  Extreme Late (>5 days): {:>5} shipments ({:.1}%)", el.count, el.pct * 100.0);
    }

    Ok(())
}

async fn run_volume_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("3. VOLUME ANALYTICS");

    // Get total for percentage calculation
    #[derive(Debug, Deserialize)]
    struct TotalCount { total: i64 }
    let total_result: Option<TotalCount> = db
        .query("SELECT count() as total FROM shipment GROUP ALL")
        .await?
        .take(0)?;
    let grand_total = total_result.map(|t| t.total).unwrap_or(1) as f64;

    // Shipments by Carrier
    print_subsection("Shipments by Carrier (Top 10)");

    let by_carrier: Vec<VolumeMetric> = db
        .query(r#"
            SELECT
                carrier_ref as group,
                count() as shipments,
                0.0 as pct_of_total
            FROM shipment
            GROUP BY carrier_ref
            ORDER BY shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>12} {:>12} {:>20}",
             "Carrier", "Shipments", "% of Total", "Volume Bar");
    println!("  {}", "─".repeat(66));
    for row in &by_carrier {
        let pct = (row.shipments as f64 / grand_total) * 100.0;
        let bar_len = (pct / 2.0).min(30.0) as usize;
        let bar: String = "█".repeat(bar_len);
        println!("  {:20} {:>12} {:>11.1}% {}", get_carrier_name(&row.group), row.shipments, pct, bar);
    }

    // Top Origin DCs
    print_subsection("Top 10 Origin DCs (Distribution Centers)");

    let origins: Vec<VolumeMetric> = db
        .query(r#"
            SELECT
                origin_zip as group,
                count() as shipments,
                0.0 as pct_of_total
            FROM shipment
            GROUP BY origin_zip
            ORDER BY shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>12} {:>12}", "Origin DC", "Shipments", "% of Total");
    println!("  {}", "─".repeat(41));
    for row in &origins {
        let pct = (row.shipments as f64 / grand_total) * 100.0;
        println!("  {:15} {:>12} {:>11.1}%", row.group, row.shipments, pct);
    }

    // Top Delivery Regions
    print_subsection("Top 10 Delivery Regions");

    let dests: Vec<VolumeMetric> = db
        .query(r#"
            SELECT
                dest_zip as group,
                count() as shipments,
                0.0 as pct_of_total
            FROM shipment
            GROUP BY dest_zip
            ORDER BY shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>12} {:>12}", "Region", "Shipments", "% of Total");
    println!("  {}", "─".repeat(41));
    for row in &dests {
        let pct = (row.shipments as f64 / grand_total) * 100.0;
        println!("  {:15} {:>12} {:>11.1}%", row.group, row.shipments, pct);
    }

    // Monthly Trends
    print_subsection("Monthly Volume & Performance Trends");

    #[derive(Debug, Deserialize)]
    struct MonthlyData {
        ship_year: i32,
        ship_month: i32,
        shipments: i64,
        otd_rate: f64,
        avg_transit: f64,
    }

    let monthly: Vec<MonthlyData> = db
        .query(r#"
            SELECT
                ship_year,
                ship_month,
                count() as shipments,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY ship_year, ship_month
            ORDER BY ship_year, ship_month
        "#)
        .await?
        .take(0)?;

    println!("  {:10} {:>10} {:>10} {:>10} {:>25}",
             "Month", "Shipments", "OTD%", "Avg Days", "Volume Trend");
    println!("  {}", "─".repeat(67));
    for row in &monthly {
        let year_month = format!("{}-{:02}", row.ship_year, row.ship_month);
        let bar_len = (row.shipments as f64 / 200.0).min(20.0) as usize;
        let bar: String = "▓".repeat(bar_len);
        println!("  {:10} {:>10} {:>9.1}% {:>9.1}d {}",
                 year_month, row.shipments, row.otd_rate * 100.0, row.avg_transit, bar);
    }

    Ok(())
}

async fn run_distribution_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("4. DISTRIBUTION ANALYSIS");

    // Transit Time Distribution (Histogram)
    print_subsection("Transit Time Distribution (Histogram)");

    let transit_dist: Vec<TransitDistribution> = db
        .query(r#"
            SELECT
                actual_transit_days as transit_days,
                count() as count
            FROM shipment
            GROUP BY actual_transit_days
            ORDER BY actual_transit_days
            LIMIT 20
        "#)
        .await?
        .take(0)?;

    let max_count = transit_dist.iter().map(|r| r.count).max().unwrap_or(1) as f64;

    println!("  {:>6}  {:>10}  {}", "Days", "Count", "Distribution");
    println!("  {}", "─".repeat(60));
    for row in &transit_dist {
        let bar_len = ((row.count as f64 / max_count) * 40.0) as usize;
        let bar: String = "█".repeat(bar_len);
        println!("  {:>5}d  {:>10}  {}", row.transit_days, row.count, bar);
    }

    // Delay Distribution
    print_subsection("Delay Distribution (Actual - Goal)");

    #[derive(Debug, Deserialize)]
    struct DelayDist {
        delay_bucket: String,
        count: i64,
    }

    let delay_dist: Vec<DelayDist> = db
        .query(r#"
            SELECT
                IF (actual_transit_days - goal_transit_days) < (0 - 2) THEN "Early >2d"
                ELSE IF (actual_transit_days - goal_transit_days) < 0 THEN "Early 1-2d"
                ELSE IF (actual_transit_days - goal_transit_days) = 0 THEN "On Time"
                ELSE IF (actual_transit_days - goal_transit_days) <= 2 THEN "Late 1-2d"
                ELSE "Late >2d"
                END as delay_bucket,
                count() as count
            FROM shipment
            GROUP BY delay_bucket
            ORDER BY delay_bucket
        "#)
        .await?
        .take(0)?;

    let total: i64 = delay_dist.iter().map(|r| r.count).sum();
    println!("  {:15} {:>10} {:>10} {:>30}", "Category", "Count", "Percent", "");
    println!("  {}", "─".repeat(67));
    for row in &delay_dist {
        let pct = (row.count as f64 / total as f64) * 100.0;
        let bar_len = (pct / 2.0) as usize;
        let bar: String = "█".repeat(bar_len);
        println!("  {:15} {:>10} {:>9.1}% {}", row.delay_bucket, row.count, pct, bar);
    }

    Ok(())
}
