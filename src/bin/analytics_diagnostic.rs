//! Diagnostic Analytics - Why is it happening?
//! Root cause analysis, carrier benchmarking, lane diagnostics
//!
//! Run: ./target/release/analytics_diagnostic [section]
//! Sections: all, carriers, lanes, hotspots, modes

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name, location_names::{format_lane_short, get_location_short, get_location_long}};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct CarrierBenchmark {
    carrier_ref: String,
    total: i64,
    otd_rate: f64,
    late_rate: f64,
    early_rate: f64,
    avg_transit: f64,
    avg_delay: f64,
    variance: f64,
}

#[derive(Debug, Deserialize)]
struct LaneDiagnostic {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    total: i64,
    otd_rate: f64,
    late_rate: f64,
    avg_delay: f64,
    network_delta: f64,
}

#[derive(Debug, Deserialize)]
struct ZipHotspot {
    zip: String,
    total: i64,
    late_rate: f64,
    avg_delay: f64,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct ModeComparison {
    carrier_mode: String,
    distance_bucket: String,
    total: i64,
    otd_rate: f64,
    avg_transit: f64,
    variance: f64,
}

#[derive(Debug, Deserialize)]
struct CarrierLanePerf {
    carrier_ref: String,
    distance_bucket: String,
    total: i64,
    otd_rate: f64,
    late_rate: f64,
}

#[derive(Debug, Deserialize)]
struct FailurePattern {
    pattern: String,
    count: i64,
    late_rate: f64,
}

fn print_section_header(title: &str) {
    println!("\n{}", "═".repeat(85));
    println!("  {}", title);
    println!("{}\n", "═".repeat(85));
}

fn print_subsection(title: &str) {
    println!("\n{}", title);
    println!("{}", "─".repeat(75));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "█".repeat(85));
    println!("{}  DIAGNOSTIC ANALYTICS - Why is it Happening?  {}", "█".repeat(17), "█".repeat(18));
    println!("{}\n", "█".repeat(85));

    // Calculate network average for benchmarking
    #[derive(Debug, Deserialize)]
    struct NetworkAvg { avg_late: f64, avg_delay: f64 }
    let network: Option<NetworkAvg> = db
        .query(r#"
            SELECT
                (count(IF otd = "Late" THEN 1 END) / count()) as avg_late,
                math::mean(actual_transit_days - goal_transit_days) as avg_delay
            FROM shipment GROUP ALL
        "#)
        .await?
        .take(0)?;

    let network_late = network.as_ref().map(|n| n.avg_late).unwrap_or(0.0);
    let network_delay = network.as_ref().map(|n| n.avg_delay).unwrap_or(0.0);

    println!("  Network Baseline: Late Rate = {:.1}%, Avg Delay = {:.2} days\n",
             network_late * 100.0, network_delay);

    match section {
        "all" => {
            run_carrier_section(&db, network_late).await?;
            run_lane_section(&db, network_late, network_delay).await?;
            run_hotspot_section(&db).await?;
            run_mode_section(&db).await?;
        }
        "carriers" => run_carrier_section(&db, network_late).await?,
        "lanes" => run_lane_section(&db, network_late, network_delay).await?,
        "hotspots" => run_hotspot_section(&db).await?,
        "modes" => run_mode_section(&db).await?,
        _ => {
            println!("Unknown section: {}", section);
            println!("Available: all, carriers, lanes, hotspots, modes");
        }
    }

    println!("\n{}", "█".repeat(85));
    Ok(())
}

async fn run_carrier_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>, network_late: f64) -> Result<()> {
    print_section_header("1. CARRIER PERFORMANCE BENCHMARKING");

    // Full carrier benchmark
    print_subsection("Carrier Performance Matrix (min 50 shipments)");

    let carriers: Vec<CarrierBenchmark> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    (count(IF otd = "Early" THEN 1 END) / count()) as early_rate,
                    math::mean(actual_transit_days) as avg_transit,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE total >= 50
            ORDER BY late_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>7} {:>8} {:>8} {:>8} {:>9} {:>8} {:>10}",
             "Carrier", "Volume", "OTD%", "Late%", "Early%", "Avg Delay", "Var", "Rating");
    println!("  {}", "─".repeat(81));

    for row in &carriers {
        let delta_vs_network = row.late_rate - network_late;
        let rating = if row.late_rate < 0.10 { "★★★ Best" }
                    else if row.late_rate < 0.20 { "★★ Good" }
                    else if row.late_rate < 0.30 { "★ Fair" }
                    else { "⚠ Poor" };

        let delta_str = if delta_vs_network > 0.0 {
            format!("+{:.1}%", delta_vs_network * 100.0)
        } else {
            format!("{:.1}%", delta_vs_network * 100.0)
        };

        println!("  {:20} {:>7} {:>7.1}% {:>7.1}% {:>7.1}% {:>8.2}d {:>7.1} {:>10}",
                 get_carrier_name(&row.carrier_ref), row.total, row.otd_rate * 100.0, row.late_rate * 100.0,
                 row.early_rate * 100.0, row.avg_delay, row.variance, rating);
    }

    // Carrier performance by distance
    print_subsection("Carrier Performance: Long-Haul vs Short-Haul");

    let carrier_by_dist: Vec<CarrierLanePerf> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    IF distance_bucket IN ["0-100", "100-250"] THEN "Short (<250mi)"
                    ELSE IF distance_bucket IN ["250-500", "500-1k"] THEN "Medium (250-1k)"
                    ELSE "Long (>1k mi)"
                    END as distance_bucket,
                    count() as total,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate
                FROM shipment
                GROUP BY carrier_ref, distance_bucket
            ) WHERE total >= 20
            ORDER BY carrier_ref, distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>18} {:>8} {:>10} {:>10}",
             "Carrier", "Distance", "Volume", "OTD%", "Late%");
    println!("  {}", "─".repeat(68));

    let mut current_carrier = String::new();
    let mut current_carrier_name = String::new();
    for row in &carrier_by_dist {
        let carrier_display = if row.carrier_ref != current_carrier {
            current_carrier = row.carrier_ref.clone();
            current_carrier_name = get_carrier_name(&row.carrier_ref);
            current_carrier_name.as_str()
        } else {
            ""
        };
        println!("  {:20} {:>18} {:>8} {:>9.1}% {:>9.1}%",
                 carrier_display, row.distance_bucket, row.total,
                 row.otd_rate * 100.0, row.late_rate * 100.0);
    }

    // Early/Late distribution shape
    print_subsection("Delivery Timing Profile by Carrier (Top 5 by Volume)");

    #[derive(Debug, Deserialize)]
    struct TimingProfile {
        carrier_ref: String,
        very_early: i64,
        early: i64,
        on_time: i64,
        late: i64,
        very_late: i64,
        total: i64,
    }

    let profiles: Vec<TimingProfile> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count(IF (actual_transit_days - goal_transit_days) < (0 - 2) THEN 1 END) as very_early,
                    count(IF (actual_transit_days - goal_transit_days) >= (0 - 2) AND (actual_transit_days - goal_transit_days) < 0 THEN 1 END) as early,
                    count(IF (actual_transit_days - goal_transit_days) = 0 THEN 1 END) as on_time,
                    count(IF (actual_transit_days - goal_transit_days) > 0 AND (actual_transit_days - goal_transit_days) <= 2 THEN 1 END) as late,
                    count(IF (actual_transit_days - goal_transit_days) > 2 THEN 1 END) as very_late,
                    count() as total
                FROM shipment
                GROUP BY carrier_ref
            )
            ORDER BY total DESC
            LIMIT 5
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>10} {:>10} {:>10} {:>10} {:>10}",
             "Carrier", "V.Early", "Early", "On-Time", "Late", "V.Late");
    println!("  {}", "─".repeat(70));

    for p in &profiles {
        let ve_pct = (p.very_early as f64 / p.total as f64) * 100.0;
        let e_pct = (p.early as f64 / p.total as f64) * 100.0;
        let ot_pct = (p.on_time as f64 / p.total as f64) * 100.0;
        let l_pct = (p.late as f64 / p.total as f64) * 100.0;
        let vl_pct = (p.very_late as f64 / p.total as f64) * 100.0;

        println!("  {:20} {:>9.1}% {:>9.1}% {:>9.1}% {:>9.1}% {:>9.1}%",
                 get_carrier_name(&p.carrier_ref), ve_pct, e_pct, ot_pct, l_pct, vl_pct);
    }

    Ok(())
}

async fn run_lane_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>, network_late: f64, network_delay: f64) -> Result<()> {
    print_section_header("2. LANE DIAGNOSTICS");

    // Worst performing lanes
    print_subsection("Worst Performing Lanes (vs Network Average)");

    let worst_lanes: Vec<LaneDiagnostic> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    count() as total,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    0.0 as network_delta
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip
            ) WHERE total >= 30
            ORDER BY late_rate DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>7} {:>8} {:>10} {:>12} {:>12}",
             "Lane (Origin->Dest)", "Volume", "Late%", "Avg Delay", "vs Network", "Action");
    println!("  {}", "─".repeat(78));

    for row in &worst_lanes {
        let route = format_lane_short(&row.origin_zip, &row.dest_zip);
        let delta = row.late_rate - network_late;
        let delta_str = format!("{:+.1}%", delta * 100.0);
        let action = if delta > 0.15 { "⚠ REVIEW" }
                    else if delta > 0.05 { "MONITOR" }
                    else { "OK" };

        println!("  {:25} {:>7} {:>7.1}% {:>9.2}d {:>12} {:>12}",
                 route, row.total, row.late_rate * 100.0, row.avg_delay, delta_str, action);
    }

    // Best performing lanes
    print_subsection("Best Performing Lanes (Learn from Success)");

    let best_lanes: Vec<LaneDiagnostic> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    count() as total,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    0.0 as network_delta
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip
            ) WHERE total >= 30
            ORDER BY late_rate ASC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>7} {:>8} {:>10} {:>15}",
             "Lane (Origin->Dest)", "Volume", "Late%", "Avg Delay", "Performance");
    println!("  {}", "─".repeat(67));

    for row in &best_lanes {
        let route = format_lane_short(&row.origin_zip, &row.dest_zip);
        let perf = if row.late_rate < 0.05 { "★★★ Excellent" }
                  else if row.late_rate < 0.10 { "★★ Very Good" }
                  else { "★ Good" };

        println!("  {:25} {:>7} {:>7.1}% {:>9.2}d {:>15}",
                 route, row.total, row.late_rate * 100.0, row.avg_delay, perf);
    }

    // Lane clustering by delay pattern
    print_subsection("Lane Delay Clustering");

    #[derive(Debug, Deserialize)]
    struct DelayCluster {
        cluster: String,
        lane_count: i64,
        total_shipments: i64,
        avg_late_rate: f64,
    }

    let clusters: Vec<DelayCluster> = db
        .query(r#"
            SELECT
                IF avg_delay < (0 - 1) THEN "Consistently Early"
                ELSE IF avg_delay < 0 THEN "Slightly Early"
                ELSE IF avg_delay < 1 THEN "On Target"
                ELSE IF avg_delay < 3 THEN "Moderately Delayed"
                ELSE "Severely Delayed"
                END as cluster,
                count() as lane_count,
                math::sum(total) as total_shipments,
                math::mean(late_rate) as avg_late_rate
            FROM (
                SELECT
                    lane_ref,
                    count() as total,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                GROUP BY lane_ref
            )
            GROUP BY cluster
            ORDER BY avg_late_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>12} {:>15} {:>15}",
             "Delay Cluster", "# Lanes", "Total Ships", "Avg Late Rate");
    println!("  {}", "─".repeat(69));

    for c in &clusters {
        println!("  {:25} {:>12} {:>15} {:>14.1}%",
                 c.cluster, c.lane_count, c.total_shipments, c.avg_late_rate * 100.0);
    }

    Ok(())
}

async fn run_hotspot_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("3. PROBLEM HOTSPOTS");

    // Origin DC hotspots
    print_subsection("Problem Origin DCs (High Late Rate)");

    let origin_hotspots: Vec<ZipHotspot> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    origin_zip as zip,
                    count() as total,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    "" as severity
                FROM shipment
                GROUP BY origin_zip
            ) WHERE total >= 50 AND late_rate > 0.20
            ORDER BY late_rate DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:6} {:22} {:>10} {:>10} {:>10} {:>12}",
             "DC", "Location", "Shipments", "Late%", "Delay", "Severity");
    println!("  {}", "─".repeat(74));

    for h in &origin_hotspots {
        let severity = if h.late_rate > 0.40 { "🔴 CRITICAL" }
                      else if h.late_rate > 0.30 { "🟠 HIGH" }
                      else { "🟡 MODERATE" };
        println!("  {:6} {:22} {:>10} {:>9.1}% {:>9.2}d {:>12}",
                 get_location_short(&h.zip), get_location_long(&h.zip),
                 h.total, h.late_rate * 100.0, h.avg_delay, severity);
    }

    // Delivery Region hotspots
    print_subsection("Problem Delivery Regions (High Late Rate)");

    let dest_hotspots: Vec<ZipHotspot> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    dest_zip as zip,
                    count() as total,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    "" as severity
                FROM shipment
                GROUP BY dest_zip
            ) WHERE total >= 50 AND late_rate > 0.20
            ORDER BY late_rate DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:6} {:22} {:>10} {:>10} {:>10} {:>12}",
             "Region", "Location", "Shipments", "Late%", "Delay", "Severity");
    println!("  {}", "─".repeat(74));

    for h in &dest_hotspots {
        let severity = if h.late_rate > 0.40 { "🔴 CRITICAL" }
                      else if h.late_rate > 0.30 { "🟠 HIGH" }
                      else { "🟡 MODERATE" };
        println!("  {:6} {:22} {:>10} {:>9.1}% {:>9.2}d {:>12}",
                 get_location_short(&h.zip), get_location_long(&h.zip),
                 h.total, h.late_rate * 100.0, h.avg_delay, severity);
    }

    // Metro vs Rural analysis
    print_subsection("Delivery Pattern: High vs Low Volume Destinations");

    #[derive(Debug, Deserialize)]
    struct VolumePattern {
        category: String,
        dest_count: i64,
        shipments: i64,
        early_rate: f64,
        late_rate: f64,
    }

    let patterns: Vec<VolumePattern> = db
        .query(r#"
            SELECT
                IF total >= 500 THEN "High Volume (≥500)"
                ELSE IF total >= 100 THEN "Medium Volume (100-499)"
                ELSE "Low Volume (<100)"
                END as category,
                count() as dest_count,
                math::sum(total) as shipments,
                math::mean(early_rate) as early_rate,
                math::mean(late_rate) as late_rate
            FROM (
                SELECT
                    dest_zip,
                    count() as total,
                    (count(IF otd = "Early" THEN 1 END) / count()) as early_rate,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate
                FROM shipment
                GROUP BY dest_zip
            )
            GROUP BY category
            ORDER BY shipments DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>12} {:>12} {:>12} {:>12}",
             "Volume Category", "# Locations", "Shipments", "Early%", "Late%");
    println!("  {}", "─".repeat(75));

    for p in &patterns {
        println!("  {:25} {:>12} {:>12} {:>11.1}% {:>11.1}%",
                 p.category, p.dest_count, p.shipments, p.early_rate * 100.0, p.late_rate * 100.0);
    }

    Ok(())
}

async fn run_mode_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("4. MODE EFFICIENCY ANALYSIS");

    // Mode comparison by distance
    print_subsection("LTL vs Truckload Performance by Distance");

    let mode_by_distance: Vec<ModeComparison> = db
        .query(r#"
            SELECT
                carrier_mode,
                distance_bucket,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                math::mean(actual_transit_days) as avg_transit,
                math::variance(actual_transit_days) as variance
            FROM shipment
            GROUP BY carrier_mode, distance_bucket
            ORDER BY carrier_mode, distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>12} {:>8} {:>10} {:>10} {:>12}",
             "Mode", "Distance", "Volume", "OTD%", "Avg Days", "Variance");
    println!("  {}", "─".repeat(66));

    let mut current_mode = String::new();
    for row in &mode_by_distance {
        let mode_display = if row.carrier_mode != current_mode {
            current_mode = row.carrier_mode.clone();
            println!("  {}", "-".repeat(66));
            row.carrier_mode.as_str()
        } else {
            ""
        };
        println!("  {:12} {:>12} {:>8} {:>9.1}% {:>9.1}d {:>12.2}",
                 mode_display, row.distance_bucket, row.total,
                 row.otd_rate * 100.0, row.avg_transit, row.variance);
    }

    // Mode variability comparison
    print_subsection("Mode Reliability Comparison");

    #[derive(Debug, Deserialize)]
    struct ModeReliability {
        carrier_mode: String,
        total: i64,
        otd_rate: f64,
        variance: f64,
        max_delay: i64,
    }

    let reliability: Vec<ModeReliability> = db
        .query(r#"
            SELECT
                carrier_mode,
                count() as total,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                math::variance(actual_transit_days) as variance,
                math::max(actual_transit_days - goal_transit_days) as max_delay
            FROM shipment
            GROUP BY carrier_mode
            ORDER BY otd_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:15} {:>10} {:>10} {:>12} {:>12} {:>15}",
             "Mode", "Volume", "OTD%", "Variance", "Max Delay", "Reliability");
    println!("  {}", "─".repeat(76));

    for r in &reliability {
        let reliability_rating = if r.variance < 2.0 && r.otd_rate > 0.7 { "★★★ High" }
                                else if r.variance < 5.0 && r.otd_rate > 0.5 { "★★ Medium" }
                                else { "★ Low" };
        println!("  {:15} {:>10} {:>9.1}% {:>12.2} {:>11}d {:>15}",
                 r.carrier_mode, r.total, r.otd_rate * 100.0, r.variance, r.max_delay, reliability_rating);
    }

    // Recommendation
    print_subsection("Mode Selection Insights");
    println!("
  Key Findings:
  • LTL dominates volume but check variance on long-haul routes
  • Truckload may reduce variability for high-value/time-critical lanes
  • Consider mode conversion for lanes with high LTL variance
  • Evaluate cost-vs-reliability tradeoff per lane
");

    Ok(())
}
