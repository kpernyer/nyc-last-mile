//! Prescriptive Analytics - What should we do?
//! Carrier optimization, mode recommendations, SLA tuning, exception management
//!
//! Run: ./target/release/analytics_prescriptive [section]
//! Sections: all, carriers, modes, sla, exceptions

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name, location_names::format_lane_short};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct CarrierLanePerformance {
    carrier_ref: String,
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    otd_rate: f64,
    late_rate: f64,
    avg_delay: f64,
}

#[derive(Debug, Deserialize)]
struct CarrierRecommendation {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    current_carrier: String,
    current_late_rate: f64,
    best_carrier: String,
    best_late_rate: f64,
    potential_improvement: f64,
}

#[derive(Debug, Deserialize)]
struct ModeConversion {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    current_mode: String,
    volume: i64,
    current_otd: f64,
    alt_mode: String,
    alt_otd: f64,
    improvement: f64,
}

#[derive(Debug, Deserialize)]
struct SlaSuggestion {
    distance_bucket: String,
    current_goal: f64,
    actual_avg: f64,
    p80_transit: f64,
    suggested_sla: f64,
    adjustment: f64,
}

#[derive(Debug, Deserialize)]
struct CarrierSlaSuggestion {
    carrier_ref: String,
    distance_bucket: String,
    volume: i64,
    avg_transit: f64,
    variance: f64,
    suggested_sla: i64,
}

#[derive(Debug, Deserialize)]
struct ExceptionAlert {
    entity_type: String,
    entity_id: String,
    metric: String,
    current_value: f64,
    baseline: f64,
    deviation: f64,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct VolumeShiftRecommendation {
    from_carrier: String,
    to_carrier: String,
    lane_ref: String,
    volume_to_shift: i64,
    expected_improvement: f64,
}

fn print_section_header(title: &str) {
    println!("\n{}", "═".repeat(90));
    println!("  {}", title);
    println!("{}\n", "═".repeat(90));
}

fn print_subsection(title: &str) {
    println!("\n{}", title);
    println!("{}", "─".repeat(80));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let db = db::connect("data/lastmile.db").await?;

    println!("\n{}", "█".repeat(90));
    println!("{}  PRESCRIPTIVE ANALYTICS - What Should We Do?  {}", "█".repeat(20), "█".repeat(21));
    println!("{}\n", "█".repeat(90));

    match section {
        "all" => {
            run_carrier_optimization(&db).await?;
            run_mode_optimization(&db).await?;
            run_sla_optimization(&db).await?;
            run_exception_management(&db).await?;
        }
        "carriers" => run_carrier_optimization(&db).await?,
        "modes" => run_mode_optimization(&db).await?,
        "sla" => run_sla_optimization(&db).await?,
        "exceptions" => run_exception_management(&db).await?,
        _ => {
            println!("Unknown section: {}", section);
            println!("Available: all, carriers, modes, sla, exceptions");
        }
    }

    println!("\n{}", "█".repeat(90));
    Ok(())
}

async fn run_carrier_optimization(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("1. CARRIER OPTIMIZATION RECOMMENDATIONS");

    // Find worst lanes and identify best carriers for them
    print_subsection("Lanes with Poor Performance - Carrier Switch Recommendations");

    // First, get the best carrier per lane
    #[derive(Debug, Clone, Deserialize)]
    struct LaneCarrierPerf {
        lane_ref: String,
        origin_zip: String,
        dest_zip: String,
        carrier_ref: String,
        volume: i64,
        late_rate: f64,
    }

    let lane_carrier_perf: Vec<LaneCarrierPerf> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    carrier_ref,
                    count() as volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip, carrier_ref
            ) WHERE volume >= 10
            ORDER BY lane_ref, late_rate ASC
        "#)
        .await?
        .take(0)?;

    // Process to find recommendations - lanes where primary carrier is underperforming
    #[derive(Debug, Deserialize)]
    struct WorstLane {
        lane_ref: String,
        origin_zip: String,
        dest_zip: String,
        total_volume: i64,
        lane_late_rate: f64,
    }

    let worst_lanes: Vec<WorstLane> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    count() as total_volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as lane_late_rate
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip
            ) WHERE total_volume >= 30 AND lane_late_rate > 0.25
            ORDER BY lane_late_rate DESC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>8} {:>10} {:>20} {:>15}",
             "Lane", "Volume", "Late%", "Primary Carrier", "Recommendation");
    println!("  {}", "─".repeat(80));

    for lane in &worst_lanes {
        // Find primary carrier for this lane
        let primary: Option<LaneCarrierPerf> = lane_carrier_perf.iter()
            .filter(|p| p.lane_ref == lane.lane_ref)
            .max_by_key(|p| p.volume)
            .cloned();

        // Find best carrier for this lane
        let best: Option<&LaneCarrierPerf> = lane_carrier_perf.iter()
            .filter(|p| p.lane_ref == lane.lane_ref && p.volume >= 5)
            .min_by(|a, b| a.late_rate.partial_cmp(&b.late_rate).unwrap());

        let route = format_lane_short(&lane.origin_zip, &lane.dest_zip);

        if let (Some(pri), Some(b)) = (&primary, best) {
            let recommendation = if pri.carrier_ref != b.carrier_ref && b.late_rate < pri.late_rate * 0.7 {
                format!("Switch to {}", get_carrier_name(&b.carrier_ref))
            } else if lane.lane_late_rate > 0.35 {
                "Review all carriers".to_string()
            } else {
                "Monitor".to_string()
            };

            println!("  {:25} {:>8} {:>9.1}% {:>20} {:>15}",
                     route, lane.total_volume, lane.lane_late_rate * 100.0,
                     get_carrier_name(&pri.carrier_ref), recommendation);
        }
    }

    // Carrier volume reallocation recommendations
    print_subsection("Volume Reallocation Opportunities");

    #[derive(Debug, Deserialize)]
    struct CarrierPerf {
        carrier_ref: String,
        total_volume: i64,
        late_rate: f64,
        avg_delay: f64,
    }

    let carrier_perf: Vec<CarrierPerf> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as total_volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE total_volume >= 50
            ORDER BY late_rate ASC
        "#)
        .await?
        .take(0)?;

    // Find best and worst performers
    let best_carriers: Vec<&CarrierPerf> = carrier_perf.iter()
        .filter(|c| c.late_rate < 0.15)
        .collect();

    let worst_carriers: Vec<&CarrierPerf> = carrier_perf.iter()
        .filter(|c| c.late_rate > 0.30)
        .collect();

    println!("\n  TOP PERFORMERS (Consider increasing volume):");
    println!("  {:20} {:>10} {:>10} {:>12} {:>20}",
             "Carrier", "Volume", "Late%", "Avg Delay", "Action");
    println!("  {}", "─".repeat(74));

    for c in best_carriers.iter().take(5) {
        println!("  {:20} {:>10} {:>9.1}% {:>11.2}d {:>20}",
                 get_carrier_name(&c.carrier_ref), c.total_volume, c.late_rate * 100.0,
                 c.avg_delay, "Increase allocation");
    }

    println!("\n  UNDERPERFORMERS (Consider reducing volume):");
    println!("  {:20} {:>10} {:>10} {:>12} {:>20}",
             "Carrier", "Volume", "Late%", "Avg Delay", "Action");
    println!("  {}", "─".repeat(74));

    for c in worst_carriers.iter().take(5) {
        let action = if c.late_rate > 0.40 { "Urgent: Reduce 50%" }
                    else if c.late_rate > 0.35 { "Reduce 30%" }
                    else { "Reduce 15%" };
        println!("  {:20} {:>10} {:>9.1}% {:>11.2}d {:>20}",
                 get_carrier_name(&c.carrier_ref), c.total_volume, c.late_rate * 100.0,
                 c.avg_delay, action);
    }

    // Optimal carrier blend recommendation
    print_subsection("Optimal Carrier Mix by Distance Segment");

    #[derive(Debug, Deserialize)]
    struct CarrierDistPerf {
        carrier_ref: String,
        distance_bucket: String,
        volume: i64,
        otd_rate: f64,
        variance: f64,
    }

    let carrier_dist: Vec<CarrierDistPerf> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    distance_bucket,
                    count() as volume,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY carrier_ref, distance_bucket
            ) WHERE volume >= 20
            ORDER BY distance_bucket, otd_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:20} {:>10} {:>10} {:>10} {:>15}",
             "Distance", "Best Carrier", "Volume", "OTD%", "Variance", "Recommendation");
    println!("  {}", "─".repeat(79));

    let distance_buckets = ["0-100", "100-250", "250-500", "500-1k", "1k-2k", "2k+"];
    for bucket in distance_buckets {
        let best_for_bucket: Option<&CarrierDistPerf> = carrier_dist.iter()
            .filter(|c| c.distance_bucket == bucket)
            .max_by(|a, b| a.otd_rate.partial_cmp(&b.otd_rate).unwrap());

        if let Some(best) = best_for_bucket {
            let rec = if best.otd_rate > 0.75 { "Primary choice" }
                     else if best.otd_rate > 0.60 { "Use with backup" }
                     else { "Seek alternatives" };

            println!("  {:12} {:20} {:>10} {:>9.1}% {:>10.1} {:>15}",
                     bucket, get_carrier_name(&best.carrier_ref), best.volume,
                     best.otd_rate * 100.0, best.variance, rec);
        }
    }

    Ok(())
}

async fn run_mode_optimization(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("2. MODE & ROUTING OPTIMIZATION");

    // Compare LTL vs TL performance on same lanes
    print_subsection("Mode Conversion Opportunities (LTL -> Truckload)");

    #[derive(Debug, Deserialize)]
    struct ModePerLane {
        lane_ref: String,
        origin_zip: String,
        dest_zip: String,
        carrier_mode: String,
        volume: i64,
        otd_rate: f64,
        variance: f64,
    }

    let mode_perf: Vec<ModePerLane> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    carrier_mode,
                    count() as volume,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip, carrier_mode
            ) WHERE volume >= 10
            ORDER BY lane_ref, carrier_mode
        "#)
        .await?
        .take(0)?;

    // Find lanes where LTL underperforms and TL is available
    println!("  {:25} {:>8} {:>10} {:>10} {:>10} {:>15}",
             "Lane", "LTL Vol", "LTL OTD%", "TL OTD%", "Diff", "Recommendation");
    println!("  {}", "─".repeat(80));

    let mut processed_lanes = std::collections::HashSet::new();
    let mut conversion_opportunities = Vec::new();

    for ltl_lane in mode_perf.iter().filter(|m| m.carrier_mode == "LTL" && m.otd_rate < 0.65) {
        if processed_lanes.contains(&ltl_lane.lane_ref) {
            continue;
        }

        // Find corresponding TL data for this lane
        let tl_data: Option<&ModePerLane> = mode_perf.iter()
            .find(|m| m.lane_ref == ltl_lane.lane_ref &&
                      (m.carrier_mode == "Truckload" || m.carrier_mode == "TL Dry" || m.carrier_mode == "TL Flatbed"));

        if let Some(tl) = tl_data {
            if tl.otd_rate > ltl_lane.otd_rate + 0.1 { // At least 10% improvement
                let route = format_lane_short(&ltl_lane.origin_zip, &ltl_lane.dest_zip);
                let diff = (tl.otd_rate - ltl_lane.otd_rate) * 100.0;
                let rec = if diff > 20.0 { "CONVERT NOW" }
                         else if diff > 10.0 { "Consider TL" }
                         else { "Monitor" };

                conversion_opportunities.push((route.clone(), ltl_lane.volume, ltl_lane.otd_rate, tl.otd_rate, diff, rec));
                processed_lanes.insert(ltl_lane.lane_ref.clone());
            }
        }
    }

    // Sort by improvement potential and show top lanes
    conversion_opportunities.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
    for (route, vol, ltl_otd, tl_otd, diff, rec) in conversion_opportunities.iter().take(15) {
        println!("  {:25} {:>8} {:>9.1}% {:>9.1}% {:>+9.1}% {:>15}",
                 route, vol, ltl_otd * 100.0, tl_otd * 100.0, diff, rec);
    }

    // Mode efficiency by distance
    print_subsection("Mode Selection Guide by Distance");

    #[derive(Debug, Deserialize)]
    struct ModeDist {
        carrier_mode: String,
        distance_bucket: String,
        volume: i64,
        otd_rate: f64,
        avg_transit: f64,
        variance: f64,
    }

    let mode_dist: Vec<ModeDist> = db
        .query(r#"
            SELECT
                carrier_mode,
                distance_bucket,
                count() as volume,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate,
                math::mean(actual_transit_days) as avg_transit,
                math::variance(actual_transit_days) as variance
            FROM shipment
            GROUP BY carrier_mode, distance_bucket
            ORDER BY distance_bucket, otd_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:12} {:>8} {:>10} {:>10} {:>10} {:>15}",
             "Distance", "Mode", "Volume", "OTD%", "Avg Days", "Variance", "Suitability");
    println!("  {}", "─".repeat(79));

    let mut current_bucket = String::new();
    for row in &mode_dist {
        if row.distance_bucket != current_bucket {
            if !current_bucket.is_empty() {
                println!("  {}", "─".repeat(79));
            }
            current_bucket = row.distance_bucket.clone();
        }

        let suitability = if row.otd_rate > 0.70 && row.variance < 3.0 { "Excellent" }
                         else if row.otd_rate > 0.55 && row.variance < 5.0 { "Good" }
                         else if row.otd_rate > 0.45 { "Fair" }
                         else { "Poor" };

        println!("  {:12} {:12} {:>8} {:>9.1}% {:>9.1}d {:>10.1} {:>15}",
                 row.distance_bucket, row.carrier_mode, row.volume,
                 row.otd_rate * 100.0, row.avg_transit, row.variance, suitability);
    }

    // Cost vs Performance tradeoff analysis
    print_subsection("Mode Selection Decision Matrix");

    println!("
  RECOMMENDED MODE BY DISTANCE & PRIORITY:

  ┌──────────────────────────────────────────────────────────────────────────────────┐
  │ Distance     │ Cost Priority        │ Speed Priority       │ Reliability Priority│
  ├──────────────┼──────────────────────┼──────────────────────┼─────────────────────┤
  │ 0-250 mi     │ LTL (cost efficient) │ TL Dry               │ TL Dry              │
  │ 250-500 mi   │ LTL                  │ TL Dry               │ TL Dry              │
  │ 500-1k mi    │ LTL with buffer      │ Truckload            │ Truckload           │
  │ 1k-2k mi     │ LTL with buffer      │ Truckload            │ Truckload           │
  │ 2k+ mi       │ Intermodal           │ Truckload expedited  │ Truckload           │
  └──────────────┴──────────────────────┴──────────────────────┴─────────────────────┘

  KEY INSIGHTS:
  • LTL cost savings diminish on lanes with >25% late rate (rework costs)
  • TL conversion typically improves OTD by 10-20% on problematic lanes
  • Consider TL for time-sensitive or high-value shipments on any lane
");

    Ok(())
}

async fn run_sla_optimization(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("3. DYNAMIC SLA OPTIMIZATION");

    // Current SLA vs actual performance by distance
    print_subsection("SLA Adjustment Recommendations by Distance");

    #[derive(Debug, Deserialize)]
    struct SlaAnalysis {
        distance_bucket: String,
        volume: i64,
        avg_goal: f64,
        avg_actual: f64,
        otd_rate: f64,
    }

    let sla_analysis: Vec<SlaAnalysis> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as volume,
                math::mean(goal_transit_days) as avg_goal,
                math::mean(actual_transit_days) as avg_actual,
                (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>8} {:>10} {:>10} {:>10} {:>12} {:>15}",
             "Distance", "Volume", "Curr SLA", "Avg Actual", "OTD%", "Suggested", "Adjustment");
    println!("  {}", "─".repeat(79));

    for row in &sla_analysis {
        // Suggest SLA based on P80 of actual performance
        let suggested = (row.avg_actual * 1.15).ceil();
        let adjustment = suggested - row.avg_goal;
        let adj_str = if adjustment > 0.0 {
            format!("+{:.0}d (relax)", adjustment)
        } else if adjustment < 0.0 {
            format!("{:.0}d (tighten)", adjustment)
        } else {
            "OK".to_string()
        };

        println!("  {:12} {:>8} {:>9.1}d {:>9.1}d {:>9.1}% {:>11.0}d {:>15}",
                 row.distance_bucket, row.volume, row.avg_goal,
                 row.avg_actual, row.otd_rate * 100.0, suggested, adj_str);
    }

    // Carrier-specific SLA recommendations
    print_subsection("Carrier-Specific SLA Recommendations (Top Carriers)");

    #[derive(Debug, Deserialize)]
    struct CarrierSla {
        carrier_ref: String,
        distance_bucket: String,
        volume: i64,
        avg_transit: f64,
        variance: f64,
    }

    let carrier_sla: Vec<CarrierSla> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    distance_bucket,
                    count() as volume,
                    math::mean(actual_transit_days) as avg_transit,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY carrier_ref, distance_bucket
            ) WHERE volume >= 30
            ORDER BY carrier_ref, distance_bucket
            LIMIT 40
        "#)
        .await?
        .take(0)?;

    println!("  {:18} {:>10} {:>8} {:>10} {:>10} {:>12} {:>10}",
             "Carrier", "Distance", "Volume", "Avg Days", "Variance", "Sugg. SLA", "Confidence");
    println!("  {}", "─".repeat(80));

    let mut current_carrier = String::new();
    let mut current_carrier_name = String::new();
    for row in &carrier_sla {
        let carrier_display = if row.carrier_ref != current_carrier {
            current_carrier = row.carrier_ref.clone();
            current_carrier_name = get_carrier_name(&row.carrier_ref);
            current_carrier_name.as_str()
        } else {
            ""
        };

        let confidence = if row.variance < 2.0 { "High" }
                        else if row.variance < 4.0 { "Medium" }
                        else { "Low" };

        let suggested_sla = (row.avg_transit * 1.2).ceil();
        println!("  {:18} {:>10} {:>8} {:>9.1}d {:>10.1} {:>11.0}d {:>10}",
                 carrier_display, row.distance_bucket, row.volume,
                 row.avg_transit, row.variance, suggested_sla, confidence);
    }

    // Lane-specific SLA tuning
    print_subsection("Lane-Specific SLA Overrides Needed");

    #[derive(Debug, Deserialize)]
    struct LaneSla {
        lane_ref: String,
        origin_zip: String,
        dest_zip: String,
        volume: i64,
        avg_goal: f64,
        avg_actual: f64,
        otd_rate: f64,
    }

    let lane_sla: Vec<LaneSla> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    count() as volume,
                    math::mean(goal_transit_days) as avg_goal,
                    math::mean(actual_transit_days) as avg_actual,
                    (count(IF otd = "OnTime" THEN 1 END) / count()) as otd_rate
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip
            ) WHERE volume >= 30 AND otd_rate < 0.60
            ORDER BY otd_rate ASC
            LIMIT 15
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>8} {:>10} {:>10} {:>10} {:>15}",
             "Lane", "Volume", "Curr SLA", "Actual", "OTD%", "Recommendation");
    println!("  {}", "─".repeat(80));

    for row in &lane_sla {
        let route = format_lane_short(&row.origin_zip, &row.dest_zip);
        let delta = row.avg_actual - row.avg_goal;
        let rec = if delta > 2.0 {
            format!("Set SLA to {:.0}d", (row.avg_actual * 1.1).ceil())
        } else if delta > 1.0 {
            format!("Add {:.0}d buffer", delta.ceil())
        } else {
            "Review carrier".to_string()
        };

        println!("  {:25} {:>8} {:>9.1}d {:>9.1}d {:>9.1}% {:>15}",
                 route, row.volume, row.avg_goal, row.avg_actual,
                 row.otd_rate * 100.0, rec);
    }

    // Customer promise date recommendations
    print_subsection("Customer Promise Date Strategy");

    println!("
  PROMISE DATE RECOMMENDATIONS:

  ┌─────────────────────────────────────────────────────────────────────────────────────┐
  │ Segment             │ Current Practice     │ Recommended                            │
  ├─────────────────────┼──────────────────────┼────────────────────────────────────────┤
  │ Standard Lanes      │ Goal Transit Days    │ Goal + 1 day buffer                    │
  │ Problem Lanes       │ Goal Transit Days    │ P80 of actual (typically Goal + 2-3d)  │
  │ Premium Customers   │ Goal Transit Days    │ Use best carrier, keep current promise │
  │ New Lanes           │ Distance-based goal  │ Add 2 day buffer until data collected  │
  └─────────────────────┴──────────────────────┴────────────────────────────────────────┘

  IMPLEMENTATION:
  • Create lane-level SLA overrides for bottom 20% performers
  • Adjust customer-facing dates weekly based on rolling 4-week performance
  • Set internal targets 1 day tighter than customer promise
");

    Ok(())
}

async fn run_exception_management(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("4. EXCEPTION MANAGEMENT & ALERTS");

    // Calculate baselines
    #[derive(Debug, Deserialize)]
    struct Baseline {
        avg_late_rate: f64,
        avg_delay: f64,
        avg_variance: f64,
    }

    let baseline: Option<Baseline> = db
        .query(r#"
            SELECT
                (count(IF otd = "Late" THEN 1 END) / count()) as avg_late_rate,
                math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                math::variance(actual_transit_days) as avg_variance
            FROM shipment
            GROUP ALL
        "#)
        .await?
        .take(0)?;

    let base = baseline.unwrap_or(Baseline { avg_late_rate: 0.2, avg_delay: 0.5, avg_variance: 4.0 });

    println!("  Network Baselines:");
    println!("  • Average Late Rate: {:.1}%", base.avg_late_rate * 100.0);
    println!("  • Average Delay: {:.2} days", base.avg_delay);
    println!("  • Average Variance: {:.2}", base.avg_variance);

    // Carrier deviations from baseline
    print_subsection("Carrier Performance Deviations (Exceeding Thresholds)");

    #[derive(Debug, Deserialize)]
    struct CarrierDeviation {
        carrier_ref: String,
        volume: i64,
        late_rate: f64,
        avg_delay: f64,
        variance: f64,
    }

    let carrier_dev: Vec<CarrierDeviation> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE volume >= 50
            ORDER BY late_rate DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>8} {:>10} {:>12} {:>12} {:>10} {:>12}",
             "Carrier", "Volume", "Late%", "vs Baseline", "Delay", "Variance", "Alert");
    println!("  {}", "─".repeat(86));

    for c in &carrier_dev {
        let late_delta = c.late_rate - base.avg_late_rate;
        let delta_str = format!("{:+.1}%", late_delta * 100.0);

        let alert = if c.late_rate > base.avg_late_rate * 1.5 && c.late_rate > 0.30 {
            "🚨 CRITICAL"
        } else if c.late_rate > base.avg_late_rate * 1.3 {
            "⚠ WARNING"
        } else if c.late_rate > base.avg_late_rate * 1.1 {
            "📋 WATCH"
        } else {
            "✓ OK"
        };

        println!("  {:20} {:>8} {:>9.1}% {:>12} {:>11.2}d {:>10.1} {:>12}",
                 get_carrier_name(&c.carrier_ref), c.volume, c.late_rate * 100.0,
                 delta_str, c.avg_delay, c.variance, alert);
    }

    // Lane deviation alerts
    print_subsection("Lane Performance Alerts (Sudden Deviations)");

    #[derive(Debug, Deserialize)]
    struct LaneDeviation {
        lane_ref: String,
        origin_zip: String,
        dest_zip: String,
        volume: i64,
        late_rate: f64,
        avg_delay: f64,
    }

    let lane_dev: Vec<LaneDeviation> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    origin_zip,
                    dest_zip,
                    count() as volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_rate,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                GROUP BY lane_ref, origin_zip, dest_zip
            ) WHERE volume >= 20 AND late_rate > 0.35
            ORDER BY late_rate DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>8} {:>10} {:>12} {:>12} {:>15}",
             "Lane", "Volume", "Late%", "Deviation", "Avg Delay", "Action");
    println!("  {}", "─".repeat(84));

    for lane in &lane_dev {
        let route = format_lane_short(&lane.origin_zip, &lane.dest_zip);
        let deviation = lane.late_rate - base.avg_late_rate;
        let dev_str = format!("{:+.1}%", deviation * 100.0);

        let action = if lane.late_rate > 0.50 {
            "Escalate now"
        } else if lane.late_rate > 0.40 {
            "Review carriers"
        } else {
            "Monitor daily"
        };

        println!("  {:25} {:>8} {:>9.1}% {:>12} {:>11.2}d {:>15}",
                 route, lane.volume, lane.late_rate * 100.0,
                 dev_str, lane.avg_delay, action);
    }

    // Escalation rules and thresholds
    print_subsection("Exception Escalation Framework");

    println!("
  TIERED ESCALATION RULES:

  ┌────────────────────────────────────────────────────────────────────────────────────────┐
  │ Severity  │ Trigger Condition                          │ Response Time │ Action       │
  ├───────────┼────────────────────────────────────────────┼───────────────┼──────────────┤
  │ CRITICAL  │ Lane late rate > 50% OR                    │ Immediate     │ Call ops mgr │
  │           │ Carrier late rate > 40% w/ vol > 100       │               │ Shift volume │
  ├───────────┼────────────────────────────────────────────┼───────────────┼──────────────┤
  │ WARNING   │ Lane late rate 35-50% OR                   │ Same day      │ Alert team   │
  │           │ Carrier +30% above baseline                │               │ Plan action  │
  ├───────────┼────────────────────────────────────────────┼───────────────┼──────────────┤
  │ WATCH     │ Lane late rate 25-35% OR                   │ 24 hours      │ Log & track  │
  │           │ New deviation from historical pattern      │               │ Root cause   │
  ├───────────┼────────────────────────────────────────────┼───────────────┼──────────────┤
  │ INFO      │ Minor variance within tolerance            │ Weekly        │ Report only  │
  └───────────┴────────────────────────────────────────────┴───────────────┴──────────────┘

  AUTOMATED ALERT CHANNELS:
  • CRITICAL: PagerDuty + SMS + Email + Slack #ops-critical
  • WARNING:  Email + Slack #ops-alerts
  • WATCH:    Daily digest email
  • INFO:     Weekly performance report
");

    // Summary action items
    print_subsection("Priority Action Items");

    println!("
  IMMEDIATE ACTIONS (This Week):
  ─────────────────────────────────────────────────────────────────────────────────");

    // Count critical carriers and lanes
    let critical_carriers = carrier_dev.iter()
        .filter(|c| c.late_rate > base.avg_late_rate * 1.5 && c.late_rate > 0.30)
        .count();

    let critical_lanes = lane_dev.iter()
        .filter(|l| l.late_rate > 0.50)
        .count();

    println!("  1. Review {} carriers with CRITICAL performance deviation", critical_carriers);
    println!("  2. Investigate {} lanes with >50% late rate", critical_lanes);
    println!("  3. Implement carrier volume reallocation on top 5 problem lanes");
    println!("  4. Adjust SLAs on lanes with consistent underperformance");
    println!("
  ONGOING MONITORING:
  ─────────────────────────────────────────────────────────────────────────────────
  • Daily: Check CRITICAL and WARNING alerts
  • Weekly: Review carrier performance trends, update risk tiers
  • Monthly: Recalibrate SLAs, evaluate carrier mix optimization
  • Quarterly: Strategic carrier partnership reviews
");

    Ok(())
}
