//! Performance test for enhanced SurrealDB schema
//! Tests record links, graph traversal, and vector search patterns

use anyhow::Result;
use nyc_last_mile::db_enhanced;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    name: String,
    description: String,
    duration_ms: f64,
    rows_returned: usize,
}

// Use serde_json::Value for all queries to handle record links

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db_path = "data/lastmile_enhanced.db";
    info!("Connecting to SurrealDB (enhanced) at {}", db_path);
    let db = db_enhanced::connect(db_path).await?;

    let mut results: Vec<BenchmarkResult> = Vec::new();

    info!("\n========================================");
    info!("  SurrealDB Performance Benchmark");
    info!("  ENHANCED SCHEMA");
    info!("========================================\n");

    // ============================================================
    // TEST 1: Simple aggregation (baseline comparison)
    // ============================================================
    info!("TEST 1: Simple count aggregation");
    let start = Instant::now();
    let count: Option<i64> = db
        .query("SELECT count() FROM shipment GROUP ALL")
        .await?
        .take("count")?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "simple_count".to_string(),
        description: "Count all shipments".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: 1,
    });
    info!("  Count: {:?}, Time: {:.2}ms", count, duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 2: Carrier OTD rates using record link
    // Uses direct record link instead of string comparison
    // ============================================================
    info!("\nTEST 2: Carrier OTD rates via record link");
    let start = Instant::now();
    let carrier_otd: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>carrier as carrier_id, total, on_time, otd_rate FROM (
                SELECT
                    carrier,
                    count() as total,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) as on_time,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate
                FROM shipment
                GROUP BY carrier
                ORDER BY total DESC
                LIMIT 20
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_otd_record_link".to_string(),
        description: "Top 20 carriers by OTD using record links".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_otd.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_otd.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 3: Lane-carrier combinations using record links
    // ============================================================
    info!("\nTEST 3: Lane-carrier performance matrix (record links)");
    let start = Instant::now();
    let lane_carriers: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>lane as lane_id, <string>carrier as carrier_id, shipments FROM (
                SELECT
                    lane,
                    carrier,
                    count() as shipments
                FROM shipment
                GROUP BY lane, carrier
                ORDER BY shipments DESC
                LIMIT 50
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "lane_carrier_matrix_record".to_string(),
        description: "Top 50 lane-carrier combos via record links".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: lane_carriers.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", lane_carriers.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 4: Carrier shipment totals using record link
    // ============================================================
    info!("\nTEST 4: Carrier shipment totals (record link)");
    let start = Instant::now();
    let carrier_stats: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>carrier as carrier_id, total_shipments FROM (
                SELECT
                    carrier,
                    count() as total_shipments
                FROM shipment
                GROUP BY carrier
                ORDER BY total_shipments DESC
                LIMIT 20
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_shipment_totals_record".to_string(),
        description: "Shipment count per carrier (record link)".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_stats.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_stats.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 5: Origin ZIP statistics using record link
    // ============================================================
    info!("\nTEST 5: Shipments by origin (record link)");
    let start = Instant::now();
    let zip_stats: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>origin as origin_id, shipments FROM (
                SELECT
                    origin,
                    count() as shipments
                FROM shipment
                GROUP BY origin
                ORDER BY shipments DESC
                LIMIT 20
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "shipments_by_origin_record".to_string(),
        description: "Shipments per origin (record link)".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: zip_stats.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", zip_stats.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 6: Specific carrier's lanes using record link
    // Uses direct record reference instead of string filter
    // ============================================================
    info!("\nTEST 6: Specific carrier's lane performance (record link)");
    let start = Instant::now();
    let carrier_lanes: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>lane as lane_id, <string>carrier as carrier_id, shipments FROM (
                SELECT
                    lane,
                    carrier,
                    count() as shipments
                FROM shipment
                WHERE carrier = carrier:0e32a59c0c8e
                GROUP BY lane, carrier
                ORDER BY shipments DESC
                LIMIT 20
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_lanes_record_link".to_string(),
        description: "Lanes for specific carrier (record link)".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_lanes.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_lanes.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 7: Lane stats for vector similarity
    // ============================================================
    info!("\nTEST 7: Compute lane stats (for vector search)");
    let start = Instant::now();
    let lane_stats: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>lane as lane_id, otd_rate, avg_transit, volume FROM (
                SELECT
                    lane,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate,
                    math::mean(actual_transit_days) as avg_transit,
                    count() as volume
                FROM shipment
                GROUP BY lane
            ) WHERE volume > 50
            ORDER BY otd_rate DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "lane_stats_for_similarity".to_string(),
        description: "Compute lane stats (for vector search)".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: lane_stats.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", lane_stats.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 8: Complex aggregation with filtering
    // ============================================================
    info!("\nTEST 8: Complex aggregation - late shipments by carrier and DOW");
    let start = Instant::now();
    let complex: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>carrier as carrier_id, ship_dow, total, late_count, avg_delay FROM (
                SELECT
                    carrier,
                    ship_dow,
                    count() as total,
                    count() as late_count,
                    math::mean(actual_transit_days - goal_transit_days) as avg_delay
                FROM shipment
                WHERE otd = "Late"
                GROUP BY carrier, ship_dow
                ORDER BY late_count DESC
                LIMIT 50
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "complex_late_analysis".to_string(),
        description: "Late shipments by carrier and DOW".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: complex.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", complex.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 9: Best carrier per lane (using record links)
    // ============================================================
    info!("\nTEST 9: Best carrier per lane (record links)");
    let start = Instant::now();
    let best_carriers: Vec<serde_json::Value> = db
        .query(r#"
            SELECT <string>lane as lane_id, <string>carrier as carrier_id, shipments, otd_rate FROM (
                SELECT
                    lane,
                    carrier,
                    count() as shipments,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate
                FROM shipment
                GROUP BY lane, carrier
            ) WHERE shipments > 10
            ORDER BY lane_id, otd_rate DESC
            LIMIT 100
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "best_carrier_per_lane".to_string(),
        description: "Best carrier per lane (record links)".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: best_carriers.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", best_carriers.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 10: Filtered scan
    // ============================================================
    info!("\nTEST 10: Filtered scan - specific criteria");
    let start = Instant::now();
    let filtered: Vec<serde_json::Value> = db
        .query(r#"
            SELECT load_id, <string>carrier as carrier_id, <string>lane as lane_id, actual_transit_days, goal_transit_days FROM (
                SELECT
                    load_id,
                    carrier,
                    lane,
                    actual_transit_days,
                    goal_transit_days
                FROM shipment
                WHERE otd = "Late"
                  AND actual_transit_days > goal_transit_days + 2
                  AND carrier_mode = "LTL"
                ORDER BY actual_transit_days DESC
                LIMIT 100
            )
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "filtered_scan".to_string(),
        description: "Significantly late LTL shipments".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: filtered.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", filtered.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // Summary
    // ============================================================
    info!("\n========================================");
    info!("  BENCHMARK SUMMARY (ENHANCED)");
    info!("========================================");

    let total_time: f64 = results.iter().map(|r| r.duration_ms).sum();

    println!("\n| Test | Description | Time (ms) | Rows |");
    println!("|------|-------------|-----------|------|");
    for r in &results {
        println!("| {} | {} | {:.2} | {} |",
            r.name,
            r.description,
            r.duration_ms,
            r.rows_returned
        );
    }
    println!("|------|-------------|-----------|------|");
    println!("| **TOTAL** | | **{:.2}** | |", total_time);

    // Save results to file
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let results_file = format!("results/perf_enhanced_{}.md", timestamp);

    std::fs::create_dir_all("results")?;

    let mut output = String::new();
    output.push_str("# SurrealDB Performance Benchmark - Enhanced Schema\n\n");
    output.push_str(&format!("**Date:** {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    output.push_str(&format!("**Database:** {}\n", db_path));
    output.push_str("**Schema:** Record links + Graph edges (RELATE)\n\n");

    output.push_str("## Results\n\n");
    output.push_str("| Test | Description | Time (ms) | Rows |\n");
    output.push_str("|------|-------------|-----------|------|\n");
    for r in &results {
        output.push_str(&format!("| {} | {} | {:.2} | {} |\n",
            r.name,
            r.description,
            r.duration_ms,
            r.rows_returned
        ));
    }
    output.push_str(&format!("|------|-------------|-----------|------|\n"));
    output.push_str(&format!("| **TOTAL** | | **{:.2}** | |\n\n", total_time));

    output.push_str("## Schema Enhancements\n\n");
    output.push_str("1. **Record Links**: Shipments use `carrier: carrier:abc123` instead of `carrier_ref: \"abc123\"`\n");
    output.push_str("2. **Graph Edges**: `shipped_by`, `from_origin`, `to_destination`, `on_lane` relations\n");
    output.push_str("3. **Deterministic IDs**: Entities have predictable IDs for direct lookup\n");
    output.push_str("4. **Indexed Record Links**: carrier and lane fields are indexed\n");

    std::fs::write(&results_file, &output)?;
    info!("\nResults saved to: {}", results_file);

    Ok(())
}
