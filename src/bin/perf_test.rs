//! Performance test suite for SurrealDB query patterns
//! Tests baseline vs enhanced schema performance

use anyhow::Result;
use nyc_last_mile::db;
use serde::Deserialize;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    name: String,
    description: String,
    duration_ms: f64,
    rows_returned: usize,
}

#[derive(Debug, Deserialize)]
struct CarrierOtd {
    carrier_ref: String,
    total: i64,
    on_time: i64,
    otd_rate: f64,
}

#[derive(Debug, Deserialize)]
struct LaneCarrier {
    lane_ref: String,
    carrier_ref: String,
    shipments: i64,
}

#[derive(Debug, Deserialize)]
struct CarrierStats {
    carrier_ref: String,
    total_shipments: i64,
}

#[derive(Debug, Deserialize)]
struct ZipStats {
    origin_zip: String,
    shipments: i64,
}

#[derive(Debug, Deserialize)]
struct LaneStats {
    lane_ref: String,
    otd_rate: f64,
    avg_transit: f64,
    volume: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db_path = "data/lastmile.db";
    info!("Connecting to SurrealDB at {}", db_path);
    let db = db::connect(db_path).await?;

    let mut results: Vec<BenchmarkResult> = Vec::new();

    info!("\n========================================");
    info!("  SurrealDB Performance Benchmark");
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
    // TEST 2: String-based carrier lookup (current approach)
    // This simulates JOIN via string matching
    // ============================================================
    info!("\nTEST 2: Carrier OTD rates via string reference");
    let start = Instant::now();
    let carrier_otd: Vec<CarrierOtd> = db
        .query(r#"
            SELECT
                carrier_ref,
                count() as total,
                count(IF otd = "OnTime" THEN true ELSE NONE END) as on_time,
                count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate
            FROM shipment
            GROUP BY carrier_ref
            ORDER BY total DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_otd_string_ref".to_string(),
        description: "Top 20 carriers by OTD using string refs".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_otd.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_otd.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 3: Lane-carrier combinations (simulates graph query)
    // With string refs, requires GROUP BY on multiple string fields
    // ============================================================
    info!("\nTEST 3: Lane-carrier performance matrix (string refs)");
    let start = Instant::now();
    let lane_carriers: Vec<LaneCarrier> = db
        .query(r#"
            SELECT
                lane_ref,
                carrier_ref,
                count() as shipments
            FROM shipment
            GROUP BY lane_ref, carrier_ref
            ORDER BY shipments DESC
            LIMIT 50
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "lane_carrier_matrix_string".to_string(),
        description: "Top 50 lane-carrier combos via string GROUP BY".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: lane_carriers.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", lane_carriers.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 4: Carrier shipment totals (graph traversal pattern)
    // ============================================================
    info!("\nTEST 4: Carrier shipment totals (graph traversal simulation)");
    let start = Instant::now();
    let carrier_stats: Vec<CarrierStats> = db
        .query(r#"
            SELECT
                carrier_ref,
                count() as total_shipments
            FROM shipment
            GROUP BY carrier_ref
            ORDER BY total_shipments DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_shipment_totals".to_string(),
        description: "Shipment count per carrier".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_stats.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_stats.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 5: Origin ZIP statistics (graph pattern)
    // ============================================================
    info!("\nTEST 5: Shipments by origin ZIP (graph pattern)");
    let start = Instant::now();
    let zip_stats: Vec<ZipStats> = db
        .query(r#"
            SELECT
                origin_zip,
                count() as shipments
            FROM shipment
            GROUP BY origin_zip
            ORDER BY shipments DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "shipments_by_origin_zip".to_string(),
        description: "Shipments per origin ZIP".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: zip_stats.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", zip_stats.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 6: Multi-table pattern - find specific carrier's lanes
    // With string refs, requires filtering by carrier_ref
    // ============================================================
    info!("\nTEST 6: Specific carrier's lane performance");
    let start = Instant::now();
    let carrier_lanes: Vec<LaneCarrier> = db
        .query(r#"
            SELECT
                lane_ref,
                carrier_ref,
                count() as shipments
            FROM shipment
            WHERE carrier_ref = "0e32a59c0c8e"
            GROUP BY lane_ref, carrier_ref
            ORDER BY shipments DESC
            LIMIT 20
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "carrier_lanes_lookup".to_string(),
        description: "Find lanes for specific carrier".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: carrier_lanes.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", carrier_lanes.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 7: Similarity search simulation (would use vectors)
    // Find lanes with similar OTD and transit characteristics
    // This test computes lane stats which would benefit from vector indexing
    // ============================================================
    info!("\nTEST 7: Find similar lanes (vector search simulation)");
    let start = Instant::now();
    // Get all lane stats - in production this would be a vector similarity search
    let lane_stats: Vec<LaneStats> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate,
                    math::mean(actual_transit_days) as avg_transit,
                    count() as volume
                FROM shipment
                GROUP BY lane_ref
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
            SELECT
                carrier_ref,
                ship_dow,
                count() as total,
                count() as late_count,
                math::mean(actual_transit_days - goal_transit_days) as avg_delay
            FROM shipment
            WHERE otd = "Late"
            GROUP BY carrier_ref, ship_dow
            ORDER BY late_count DESC
            LIMIT 50
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
    // TEST 9: Subquery - best carrier per lane
    // ============================================================
    info!("\nTEST 9: Best carrier per lane (subquery pattern)");
    let start = Instant::now();
    let best_carriers: Vec<serde_json::Value> = db
        .query(r#"
            SELECT * FROM (
                SELECT
                    lane_ref,
                    carrier_ref,
                    count() as shipments,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate
                FROM shipment
                GROUP BY lane_ref, carrier_ref
            ) WHERE shipments > 10
            ORDER BY lane_ref, otd_rate DESC
            LIMIT 100
        "#)
        .await?
        .take(0)?;
    let duration = start.elapsed();
    results.push(BenchmarkResult {
        name: "best_carrier_per_lane".to_string(),
        description: "Best performing carrier per lane".to_string(),
        duration_ms: duration.as_secs_f64() * 1000.0,
        rows_returned: best_carriers.len(),
    });
    info!("  Rows: {}, Time: {:.2}ms", best_carriers.len(), duration.as_secs_f64() * 1000.0);

    // ============================================================
    // TEST 10: Full table scan with complex WHERE
    // ============================================================
    info!("\nTEST 10: Filtered scan - specific criteria");
    let start = Instant::now();
    let filtered: Vec<serde_json::Value> = db
        .query(r#"
            SELECT
                load_id,
                carrier_ref,
                lane_ref,
                actual_transit_days,
                goal_transit_days
            FROM shipment
            WHERE otd = "Late"
              AND actual_transit_days > goal_transit_days + 2
              AND carrier_mode = "LTL"
            ORDER BY actual_transit_days DESC
            LIMIT 100
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
    info!("  BENCHMARK SUMMARY");
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
    let results_file = format!("results/perf_baseline_{}.md", timestamp);

    std::fs::create_dir_all("results")?;

    let mut output = String::new();
    output.push_str("# SurrealDB Performance Benchmark - Baseline\n\n");
    output.push_str(&format!("**Date:** {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    output.push_str(&format!("**Database:** {}\n", db_path));
    output.push_str("**Schema:** String references (no graph, no record links)\n\n");

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

    output.push_str("## Test Descriptions\n\n");
    output.push_str("1. **simple_count**: Basic COUNT(*) aggregation\n");
    output.push_str("2. **carrier_otd_string_ref**: GROUP BY on string field with conditional aggregation\n");
    output.push_str("3. **lane_carrier_matrix_string**: Multi-field GROUP BY (would benefit from graph)\n");
    output.push_str("4. **carrier_shipment_totals**: Carrier-level aggregation (graph traversal pattern)\n");
    output.push_str("5. **shipments_by_origin_zip**: Location-based aggregation (graph pattern)\n");
    output.push_str("6. **carrier_lanes_lookup**: Find lanes for specific carrier (record link pattern)\n");
    output.push_str("7. **lane_stats_for_similarity**: Compute lane metrics (would use vector index)\n");
    output.push_str("8. **complex_late_analysis**: Multi-dimensional aggregation with filtering\n");
    output.push_str("9. **best_carrier_per_lane**: Ranked subquery pattern\n");
    output.push_str("10. **filtered_scan**: Complex WHERE clause scan\n");

    std::fs::write(&results_file, &output)?;
    info!("\nResults saved to: {}", results_file);

    Ok(())
}
