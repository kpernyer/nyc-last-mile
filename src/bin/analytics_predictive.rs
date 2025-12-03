//! Predictive Analytics - What will happen?
//! Delay probability scoring, ETA prediction factors, volume forecasting
//!
//! Run: ./target/release/analytics_predictive [section]
//! Sections: all, delay, eta, forecast, risk

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name, location_names::format_lane_short};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct DelayProbability {
    factor: String,
    value: String,
    total: i64,
    late_probability: f64,
    avg_delay_when_late: f64,
}

#[derive(Debug, Deserialize)]
struct EtaFactor {
    carrier_ref: String,
    distance_bucket: String,
    historical_avg: f64,
    p90_transit: f64,
    reliability_score: f64,
}

#[derive(Debug, Deserialize)]
struct VolumeProjection {
    period: String,
    historical_avg: f64,
    trend: String,
}

#[derive(Debug, Deserialize)]
struct RiskScore {
    lane_ref: String,
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    delay_risk: f64,
    variability_risk: f64,
    combined_risk: f64,
}

#[derive(Debug, Deserialize)]
struct CarrierRisk {
    carrier_ref: String,
    volume: i64,
    late_prob: f64,
    variance: f64,
    risk_tier: String,
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
    println!("{}  PREDICTIVE ANALYTICS - What Will Happen?  {}", "█".repeat(18), "█".repeat(19));
    println!("{}\n", "█".repeat(85));

    match section {
        "all" => {
            run_delay_section(&db).await?;
            run_eta_section(&db).await?;
            run_forecast_section(&db).await?;
            run_risk_section(&db).await?;
        }
        "delay" => run_delay_section(&db).await?,
        "eta" => run_eta_section(&db).await?,
        "forecast" => run_forecast_section(&db).await?,
        "risk" => run_risk_section(&db).await?,
        _ => {
            println!("Unknown section: {}", section);
            println!("Available: all, delay, eta, forecast, risk");
        }
    }

    println!("\n{}", "█".repeat(85));
    Ok(())
}

async fn run_delay_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("1. DELAY LIKELIHOOD SCORING");

    // Delay probability by Day of Week
    print_subsection("Delay Probability by Day of Week");

    #[derive(Debug, Deserialize)]
    struct DowDelayProb {
        ship_dow: i32,
        total: i64,
        late_probability: f64,
        avg_delay_when_late: f64,
    }

    let by_dow: Vec<DowDelayProb> = db
        .query(r#"
            SELECT
                ship_dow,
                count() as total,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_probability,
                math::mean(IF otd = "Late" THEN actual_transit_days - goal_transit_days END) as avg_delay_when_late
            FROM shipment
            GROUP BY ship_dow
            ORDER BY late_probability DESC
        "#)
        .await?
        .take(0)?;

    let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    println!("  {:12} {:>10} {:>15} {:>18} {:>15}",
             "Ship Day", "Volume", "P(Late)", "Avg Delay if Late", "Risk Level");
    println!("  {}", "─".repeat(72));

    for row in &by_dow {
        let day = dow_names.get(row.ship_dow as usize).unwrap_or(&"???");
        let risk = if row.late_probability > 0.25 { "🔴 HIGH" }
                  else if row.late_probability > 0.18 { "🟠 MEDIUM" }
                  else { "🟢 LOW" };
        let delay = row.avg_delay_when_late;

        println!("  {:12} {:>10} {:>14.1}% {:>17.1}d {:>15}",
                 day, row.total, row.late_probability * 100.0, delay, risk);
    }

    // Delay probability by Distance
    print_subsection("Delay Probability by Distance");

    let by_distance: Vec<DelayProbability> = db
        .query(r#"
            SELECT
                "Distance" as factor,
                distance_bucket as value,
                count() as total,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_probability,
                math::mean(IF otd = "Late" THEN actual_transit_days - goal_transit_days END) as avg_delay_when_late
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY late_probability DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>10} {:>15} {:>18} {:>15}",
             "Distance", "Volume", "P(Late)", "Avg Delay if Late", "Risk Level");
    println!("  {}", "─".repeat(72));

    for row in &by_distance {
        let risk = if row.late_probability > 0.25 { "🔴 HIGH" }
                  else if row.late_probability > 0.18 { "🟠 MEDIUM" }
                  else { "🟢 LOW" };

        println!("  {:12} {:>10} {:>14.1}% {:>17.1}d {:>15}",
                 row.value, row.total, row.late_probability * 100.0,
                 row.avg_delay_when_late, risk);
    }

    // Delay probability by Carrier (Top 10)
    print_subsection("Delay Probability by Carrier (Top 10 by Volume)");

    let by_carrier: Vec<DelayProbability> = db
        .query(r#"
            SELECT
                "Carrier" as factor,
                carrier_ref as value,
                count() as total,
                (count(IF otd = "Late" THEN 1 END) / count()) as late_probability,
                math::mean(IF otd = "Late" THEN actual_transit_days - goal_transit_days END) as avg_delay_when_late
            FROM shipment
            GROUP BY carrier_ref
            ORDER BY total DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>10} {:>15} {:>18} {:>12}",
             "Carrier", "Volume", "P(Late)", "Avg Delay if Late", "Prediction");
    println!("  {}", "─".repeat(77));

    for row in &by_carrier {
        let prediction = if row.late_probability > 0.25 { "Likely Late" }
                        else if row.late_probability > 0.15 { "Possible Late" }
                        else { "Likely On-Time" };

        println!("  {:20} {:>10} {:>14.1}% {:>17.1}d {:>12}",
                 get_carrier_name(&row.value), row.total, row.late_probability * 100.0,
                 row.avg_delay_when_late, prediction);
    }

    // Combined risk matrix
    print_subsection("Combined Delay Risk Matrix (Carrier x Distance)");

    #[derive(Debug, Deserialize)]
    struct CombinedRisk {
        carrier_ref: String,
        distance_bucket: String,
        total: i64,
        late_prob: f64,
    }

    let combined: Vec<CombinedRisk> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    distance_bucket,
                    count() as total,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_prob
                FROM shipment
                GROUP BY carrier_ref, distance_bucket
            ) WHERE total >= 20
            ORDER BY late_prob DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>12} {:>10} {:>12} {:>15}",
             "Carrier", "Distance", "Volume", "P(Late)", "Risk");
    println!("  {}", "─".repeat(71));

    for row in &combined {
        let risk = if row.late_prob > 0.35 { "⚠ VERY HIGH" }
                  else if row.late_prob > 0.25 { "🔴 HIGH" }
                  else if row.late_prob > 0.15 { "🟠 MEDIUM" }
                  else { "🟢 LOW" };

        println!("  {:20} {:>12} {:>10} {:>11.1}% {:>15}",
                 get_carrier_name(&row.carrier_ref), row.distance_bucket, row.total, row.late_prob * 100.0, risk);
    }

    Ok(())
}

async fn run_eta_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("2. ETA PREDICTION FACTORS");

    // Historical transit time by carrier and distance
    print_subsection("Historical Transit Benchmarks (for ETA Calculation)");

    #[derive(Debug, Deserialize)]
    struct TransitBenchmark {
        carrier_ref: String,
        distance_bucket: String,
        total: i64,
        avg_transit: f64,
        min_transit: i64,
        max_transit: i64,
        variance: f64,
    }

    let benchmarks: Vec<TransitBenchmark> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    distance_bucket,
                    count() as total,
                    math::mean(actual_transit_days) as avg_transit,
                    math::min(actual_transit_days) as min_transit,
                    math::max(actual_transit_days) as max_transit,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY carrier_ref, distance_bucket
            ) WHERE total >= 50
            ORDER BY carrier_ref, distance_bucket
            LIMIT 30
        "#)
        .await?
        .take(0)?;

    println!("  {:18} {:>10} {:>7} {:>8} {:>6} {:>6} {:>8} {:>12}",
             "Carrier", "Distance", "Volume", "Avg", "Min", "Max", "Var", "ETA Conf.");
    println!("  {}", "─".repeat(79));

    for row in &benchmarks {
        let confidence = if row.variance < 1.5 { "High" }
                        else if row.variance < 4.0 { "Medium" }
                        else { "Low" };
        println!("  {:18} {:>10} {:>7} {:>7.1}d {:>5}d {:>5}d {:>7.1} {:>12}",
                 get_carrier_name(&row.carrier_ref), row.distance_bucket, row.total,
                 row.avg_transit, row.min_transit, row.max_transit, row.variance, confidence);
    }

    // P90 Transit Times (for conservative ETA)
    print_subsection("P90 Transit Times (Conservative ETA Buffer)");

    #[derive(Debug, Deserialize)]
    struct P90Transit {
        distance_bucket: String,
        total: i64,
        avg_transit: f64,
        goal_transit: f64,
    }

    let p90: Vec<P90Transit> = db
        .query(r#"
            SELECT
                distance_bucket,
                count() as total,
                math::mean(actual_transit_days) as avg_transit,
                math::mean(goal_transit_days) as goal_transit
            FROM shipment
            GROUP BY distance_bucket
            ORDER BY distance_bucket
        "#)
        .await?
        .take(0)?;

    println!("  {:12} {:>10} {:>12} {:>12} {:>15} {:>12}",
             "Distance", "Volume", "Avg Transit", "Goal", "Suggested ETA", "Buffer");
    println!("  {}", "─".repeat(75));

    for row in &p90 {
        let suggested_buffer = row.avg_transit * 1.5;
        let buffer = suggested_buffer - row.goal_transit;
        println!("  {:12} {:>10} {:>11.1}d {:>11.1}d {:>14.1}d {:>+11.1}d",
                 row.distance_bucket, row.total, row.avg_transit,
                 row.goal_transit, suggested_buffer, buffer);
    }

    // ETA Adjustment Recommendations
    print_subsection("ETA Adjustment Recommendations");

    println!("
  PREDICTIVE ETA MODEL FACTORS:

  Base ETA = Goal Transit Days (from SLA)

  Adjustments to apply:
  ┌─────────────────────────────────────────────────────────────────────────┐
  │ Factor              │ Condition                    │ Adjustment        │
  ├─────────────────────┼──────────────────────────────┼───────────────────┤
  │ Day of Week         │ Ship on Tuesday              │ +0.5 days         │
  │                     │ Ship on Thursday             │ +0.2 days         │
  ├─────────────────────┼──────────────────────────────┼───────────────────┤
  │ Distance            │ 100-250 miles                │ +0.3 days         │
  │                     │ 250-500 miles                │ +0.3 days         │
  │                     │ 500-1000 miles               │ +0.3 days         │
  │                     │ >2000 miles                  │ -0.2 days         │
  ├─────────────────────┼──────────────────────────────┼───────────────────┤
  │ Carrier Risk        │ High-variance carrier        │ +1.0 days         │
  │                     │ Low-variance carrier         │ -0.3 days         │
  ├─────────────────────┼──────────────────────────────┼───────────────────┤
  │ Lane History        │ Lane late rate > 30%         │ +1.5 days         │
  │                     │ Lane late rate > 20%         │ +0.5 days         │
  └─────────────────────┴──────────────────────────────┴───────────────────┘

  Predicted ETA = Base + Sum(Adjustments)
  Conservative ETA (P90) = Predicted ETA × 1.3
");

    Ok(())
}

async fn run_forecast_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("3. CAPACITY & VOLUME FORECASTING");

    // Monthly volume patterns
    print_subsection("Historical Monthly Volume Patterns");

    #[derive(Debug, Deserialize)]
    struct MonthlyVolume {
        month: i32,
        avg_shipments: f64,
        min_shipments: i64,
        max_shipments: i64,
    }

    let monthly: Vec<MonthlyVolume> = db
        .query(r#"
            SELECT
                ship_month as month,
                math::mean(cnt) as avg_shipments,
                math::min(cnt) as min_shipments,
                math::max(cnt) as max_shipments
            FROM (
                SELECT ship_month, ship_year, count() as cnt
                FROM shipment
                GROUP BY ship_year, ship_month
            )
            GROUP BY ship_month
            ORDER BY ship_month
        "#)
        .await?
        .take(0)?;

    let month_names = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    println!("  {:8} {:>12} {:>12} {:>12} {:>20}",
             "Month", "Avg Volume", "Min", "Max", "Seasonality");
    println!("  {}", "─".repeat(66));

    let overall_avg: f64 = monthly.iter().map(|m| m.avg_shipments).sum::<f64>() / monthly.len() as f64;

    for row in &monthly {
        let month_name = month_names.get(row.month as usize).unwrap_or(&"???");
        let seasonality = if row.avg_shipments > overall_avg * 1.1 { "📈 Peak" }
                         else if row.avg_shipments < overall_avg * 0.9 { "📉 Low" }
                         else { "→ Normal" };

        let bar_len = ((row.avg_shipments / 100.0).min(20.0)) as usize;
        let bar: String = "▓".repeat(bar_len);

        println!("  {:8} {:>12.0} {:>12} {:>12} {:>10} {}",
                 month_name, row.avg_shipments, row.min_shipments,
                 row.max_shipments, seasonality, bar);
    }

    // Day of week patterns
    print_subsection("Day of Week Volume Distribution");

    #[derive(Debug, Deserialize)]
    struct DowVolume {
        dow: i32,
        avg_shipments: f64,
        pct_of_week: f64,
    }

    let dow_vol: Vec<DowVolume> = db
        .query(r#"
            SELECT
                ship_dow as dow,
                math::mean(cnt) as avg_shipments,
                0.0 as pct_of_week
            FROM (
                SELECT ship_dow, ship_week, ship_year, count() as cnt
                FROM shipment
                GROUP BY ship_year, ship_week, ship_dow
            )
            GROUP BY ship_dow
            ORDER BY ship_dow
        "#)
        .await?
        .take(0)?;

    let dow_names = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let total_avg: f64 = dow_vol.iter().map(|d| d.avg_shipments).sum();

    println!("  {:12} {:>15} {:>15} {:>20}",
             "Day", "Avg/Week", "% of Week", "Capacity Need");
    println!("  {}", "─".repeat(64));

    for row in &dow_vol {
        let day = dow_names.get(row.dow as usize).unwrap_or(&"???");
        let pct = (row.avg_shipments / total_avg) * 100.0;
        let capacity = if pct > 20.0 { "🔴 HIGH" }
                      else if pct > 15.0 { "🟠 MEDIUM" }
                      else if pct > 5.0 { "🟢 NORMAL" }
                      else { "⚪ LOW" };

        println!("  {:12} {:>15.0} {:>14.1}% {:>20}",
                 day, row.avg_shipments, pct, capacity);
    }

    // Carrier capacity utilization
    print_subsection("Carrier Capacity Analysis");

    #[derive(Debug, Deserialize)]
    struct CarrierCapacity {
        carrier_ref: String,
        total_shipments: i64,
        weekly_avg: f64,
        peak_week: i64,
    }

    let carrier_cap: Vec<CarrierCapacity> = db
        .query(r#"
            SELECT
                carrier_ref,
                math::sum(cnt) as total_shipments,
                math::mean(cnt) as weekly_avg,
                math::max(cnt) as peak_week
            FROM (
                SELECT carrier_ref, ship_week, ship_year, count() as cnt
                FROM shipment
                GROUP BY carrier_ref, ship_year, ship_week
            )
            GROUP BY carrier_ref
            ORDER BY total_shipments DESC
            LIMIT 10
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>12} {:>12} {:>12} {:>15}",
             "Carrier", "Total", "Weekly Avg", "Peak Week", "Headroom");
    println!("  {}", "─".repeat(73));

    for row in &carrier_cap {
        let headroom = ((row.peak_week as f64 / row.weekly_avg) - 1.0) * 100.0;
        let headroom_str = format!("{:.0}% above avg", headroom);

        println!("  {:20} {:>12} {:>12.0} {:>12} {:>15}",
                 get_carrier_name(&row.carrier_ref), row.total_shipments, row.weekly_avg,
                 row.peak_week, headroom_str);
    }

    Ok(())
}

async fn run_risk_section(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) -> Result<()> {
    print_section_header("4. RISK SCORING & ALERTS");

    // Lane risk scores
    print_subsection("High-Risk Lanes (Composite Risk Score)");

    #[derive(Debug, Deserialize)]
    struct LaneRiskData {
        origin_zip: String,
        dest_zip: String,
        volume: i64,
        delay_risk: f64,
        variance: f64,
    }

    let lane_risks: Vec<LaneRiskData> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    origin_zip,
                    dest_zip,
                    count() as volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as delay_risk,
                    math::variance(actual_transit_days) as variance
                FROM shipment
                GROUP BY origin_zip, dest_zip
            ) WHERE volume >= 30
            ORDER BY delay_risk DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;

    println!("  {:25} {:>8} {:>12} {:>12} {:>12} {:>10}",
             "Lane", "Volume", "Delay Risk", "Var Risk", "Combined", "Alert");
    println!("  {}", "─".repeat(81));

    for row in &lane_risks {
        let route = format_lane_short(&row.origin_zip, &row.dest_zip);
        let variability_risk = row.variance / 10.0;
        let combined_risk = row.delay_risk * 0.6 + variability_risk * 0.4;
        let alert = if combined_risk > 0.35 { "🚨 CRITICAL" }
                   else if combined_risk > 0.25 { "⚠ WARNING" }
                   else if combined_risk > 0.15 { "📋 WATCH" }
                   else { "✓ OK" };

        println!("  {:25} {:>8} {:>11.1}% {:>11.2} {:>11.2} {:>10}",
                 route, row.volume, row.delay_risk * 100.0,
                 variability_risk, combined_risk, alert);
    }

    // Carrier risk tiers
    print_subsection("Carrier Risk Tiers");

    let carrier_risks: Vec<CarrierRisk> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    carrier_ref,
                    count() as volume,
                    (count(IF otd = "Late" THEN 1 END) / count()) as late_prob,
                    math::variance(actual_transit_days) as variance,
                    "" as risk_tier
                FROM shipment
                GROUP BY carrier_ref
            ) WHERE volume >= 50
            ORDER BY late_prob DESC
        "#)
        .await?
        .take(0)?;

    println!("  {:20} {:>10} {:>12} {:>10} {:>15} {:>12}",
             "Carrier", "Volume", "Late Prob", "Variance", "Risk Tier", "Action");
    println!("  {}", "─".repeat(81));

    for row in &carrier_risks {
        let (tier, action) = if row.late_prob > 0.30 && row.variance > 5.0 {
            ("🔴 CRITICAL", "Reduce load")
        } else if row.late_prob > 0.25 || row.variance > 8.0 {
            ("🟠 HIGH", "Monitor")
        } else if row.late_prob > 0.18 || row.variance > 4.0 {
            ("🟡 MEDIUM", "Track")
        } else {
            ("🟢 LOW", "Maintain")
        };

        println!("  {:20} {:>10} {:>11.1}% {:>10.1} {:>15} {:>12}",
                 get_carrier_name(&row.carrier_ref), row.volume, row.late_prob * 100.0,
                 row.variance, tier, action);
    }

    // Alert thresholds summary
    print_subsection("Alert Threshold Configuration");

    println!("
  RECOMMENDED ALERT THRESHOLDS:

  ┌──────────────────────────────────────────────────────────────────────────┐
  │ Alert Level │ Late Probability │ Variance │ Combined Score │ Action      │
  ├─────────────┼──────────────────┼──────────┼────────────────┼─────────────┤
  │ 🚨 CRITICAL │ > 35%            │ > 10     │ > 0.35         │ Immediate   │
  │ ⚠ WARNING  │ 25-35%           │ 6-10     │ 0.25-0.35      │ Same Day    │
  │ 📋 WATCH    │ 18-25%           │ 4-6      │ 0.15-0.25      │ Weekly      │
  │ ✓ OK        │ < 18%            │ < 4      │ < 0.15         │ Monthly     │
  └─────────────┴──────────────────┴──────────┴────────────────┴─────────────┘

  ESCALATION RULES:
  • CRITICAL: Auto-notify operations manager, consider volume shift
  • WARNING:  Flag for daily review, prepare backup carrier
  • WATCH:    Include in weekly performance review
  • OK:       Standard monitoring
");

    Ok(())
}
