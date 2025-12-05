//! Benchmark: Native HNSW vs Application-Side Vector Search
//!
//! Compares performance of:
//! 1. SurrealDB native HNSW index queries
//! 2. Application-side distance calculations
//!
//! Tests multiple distance metrics: EUCLIDEAN, COSINE, MANHATTAN

use anyhow::Result;
use nyc_last_mile::db_enhanced;
use serde_json::Value;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone)]
struct CarrierVector {
    carrier_id: String,
    vector: [f64; 4],
    otd_rate: f64,
    volume: i64,
}

impl CarrierVector {
    fn euclidean_distance(&self, other: &[f64; 4]) -> f64 {
        self.vector.iter().zip(other.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    fn cosine_similarity(&self, other: &[f64; 4]) -> f64 {
        let dot: f64 = self.vector.iter().zip(other.iter()).map(|(a, b)| a * b).sum();
        let mag1: f64 = self.vector.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let mag2: f64 = other.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if mag1 == 0.0 || mag2 == 0.0 { 0.0 } else { dot / (mag1 * mag2) }
    }

    fn manhattan_distance(&self, other: &[f64; 4]) -> f64 {
        self.vector.iter().zip(other.iter())
            .map(|(a, b)| (a - b).abs())
            .sum()
    }
}

#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    method: String,
    duration_ms: f64,
    results_count: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db_path = "data/lastmile_enhanced.db";
    info!("Connecting to SurrealDB at {}", db_path);
    let db = db_enhanced::connect(db_path).await?;

    println!("\n══════════════════════════════════════════════════════════════");
    println!("  Native HNSW vs Application-Side Vector Search Benchmark");
    println!("══════════════════════════════════════════════════════════════\n");

    let mut benchmarks: Vec<BenchmarkResult> = Vec::new();

    // =========================================================================
    // Step 1: Compute and store carrier vectors in database
    // =========================================================================
    info!("Step 1: Computing carrier performance vectors...\n");

    let start = Instant::now();

    // First, get carrier stats
    let carrier_stats: Vec<Value> = db
        .query(r#"
            SELECT <string>carrier as carrier_id, otd_rate, avg_transit, total_shipments, transit_variance FROM (
                SELECT
                    carrier,
                    count() as total_shipments,
                    count(IF otd = "OnTime" THEN true ELSE NONE END) * 100.0 / count() as otd_rate,
                    math::mean(actual_transit_days) as avg_transit,
                    math::stddev(actual_transit_days) as transit_variance
                FROM shipment
                GROUP BY carrier
            ) WHERE total_shipments > 10
        "#)
        .await?
        .take(0)?;

    println!("Found {} carriers with >10 shipments", carrier_stats.len());

    // Store vectors in carrier records (using UPSERT to create if not exists)
    let mut stored_count = 0;
    for c in &carrier_stats {
        let carrier_id = c["carrier_id"].as_str().unwrap_or("").to_string();
        let otd = c["otd_rate"].as_f64().unwrap_or(0.0);
        let transit = c["avg_transit"].as_f64().unwrap_or(0.0);
        let volume = c["total_shipments"].as_i64().unwrap_or(0);
        let variance = c["transit_variance"].as_f64().unwrap_or(0.0);

        // Normalized vector: [otd/100, transit/10, log(vol)/5, var/5]
        let vec = [
            otd / 100.0,
            transit / 10.0,
            ((volume as f64) + 1.0).log10() / 5.0,
            variance / 5.0,
        ];

        // Upsert carrier record with vector
        // carrier_id is already "carrier:abc123" format
        let update_query = format!(
            "UPSERT {} SET perf_vector = [{}, {}, {}, {}], otd_rate = {}, volume = {}",
            carrier_id, vec[0], vec[1], vec[2], vec[3], otd, volume
        );
        let result = db.query(&update_query).await;

        match &result {
            Ok(_) => stored_count += 1,
            Err(e) => {
                if stored_count == 0 {
                    println!("  Update error: {}", e.to_string().lines().next().unwrap_or("error"));
                }
            }
        }
    }

    let compute_time = start.elapsed();
    println!("Stored {} carrier vectors in {:.2}ms\n", stored_count, compute_time.as_secs_f64() * 1000.0);

    // =========================================================================
    // Step 2: Create HNSW indexes for different distance metrics
    // =========================================================================
    info!("Step 2: Creating HNSW indexes...\n");

    // Try to create HNSW indexes (may fail if already exists or not supported)
    let index_results = vec![
        ("EUCLIDEAN", db.query(
            "DEFINE INDEX idx_carrier_vec_euclidean ON carrier FIELDS perf_vector HNSW DIMENSION 4 DIST EUCLIDEAN TYPE F64"
        ).await),
        ("COSINE", db.query(
            "DEFINE INDEX idx_carrier_vec_cosine ON carrier FIELDS perf_vector HNSW DIMENSION 4 DIST COSINE TYPE F64"
        ).await),
        ("MANHATTAN", db.query(
            "DEFINE INDEX idx_carrier_vec_manhattan ON carrier FIELDS perf_vector HNSW DIMENSION 4 DIST MANHATTAN TYPE F64"
        ).await),
    ];

    for (name, result) in index_results {
        match result {
            Ok(_) => println!("  ✓ Created HNSW index with {} distance", name),
            Err(e) => println!("  ✗ {} index: {}", name, e.to_string().lines().next().unwrap_or("error")),
        }
    }
    println!();

    // =========================================================================
    // Step 3: Load all carriers for application-side comparison
    // =========================================================================
    info!("Step 3: Loading carrier vectors for comparison...\n");

    let carriers_with_vectors: Vec<Value> = db
        .query(r#"
            SELECT <string>id as carrier_id, perf_vector, otd_rate, volume
            FROM carrier
            WHERE perf_vector != NONE
        "#)
        .await?
        .take(0)?;

    let carriers: Vec<CarrierVector> = carriers_with_vectors
        .iter()
        .filter_map(|c| {
            let vec_arr = c["perf_vector"].as_array()?;
            if vec_arr.len() != 4 { return None; }
            Some(CarrierVector {
                carrier_id: c["carrier_id"].as_str()?.to_string(),
                vector: [
                    vec_arr[0].as_f64()?,
                    vec_arr[1].as_f64()?,
                    vec_arr[2].as_f64()?,
                    vec_arr[3].as_f64()?,
                ],
                otd_rate: c["otd_rate"].as_f64().unwrap_or(0.0),
                volume: c["volume"].as_i64().unwrap_or(0),
            })
        })
        .collect();

    println!("Loaded {} carriers with vectors\n", carriers.len());

    // Pick a reference vector (best performing carrier by OTD rate)
    let reference = carriers.iter()
        .max_by(|a, b| a.otd_rate.partial_cmp(&b.otd_rate).unwrap())
        .expect("No reference carrier");

    let ref_vec = reference.vector;
    println!("Reference carrier: {}", reference.carrier_id);
    println!("  Vector: [{:.4}, {:.4}, {:.4}, {:.4}]", ref_vec[0], ref_vec[1], ref_vec[2], ref_vec[3]);
    println!("  OTD: {:.1}%, Volume: {}\n", reference.otd_rate, reference.volume);

    // =========================================================================
    // Step 4: Benchmark - Application-Side Calculations
    // =========================================================================
    println!("══════════════════════════════════════════════════════════════");
    println!("  BENCHMARK: Application-Side Distance Calculations");
    println!("══════════════════════════════════════════════════════════════\n");

    let iterations = 100;

    // Euclidean - Application Side
    let start = Instant::now();
    for _ in 0..iterations {
        let mut results: Vec<_> = carriers.iter()
            .filter(|c| c.carrier_id != reference.carrier_id)
            .map(|c| (c, c.euclidean_distance(&ref_vec)))
            .collect();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let _top5: Vec<_> = results.into_iter().take(5).collect();
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    println!("  EUCLIDEAN (app-side):  {:.4} ms/query", duration);
    benchmarks.push(BenchmarkResult {
        name: "Euclidean".to_string(),
        method: "Application".to_string(),
        duration_ms: duration,
        results_count: 5,
    });

    // Cosine - Application Side
    let start = Instant::now();
    for _ in 0..iterations {
        let mut results: Vec<_> = carriers.iter()
            .filter(|c| c.carrier_id != reference.carrier_id)
            .map(|c| (c, c.cosine_similarity(&ref_vec)))
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let _top5: Vec<_> = results.into_iter().take(5).collect();
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    println!("  COSINE (app-side):     {:.4} ms/query", duration);
    benchmarks.push(BenchmarkResult {
        name: "Cosine".to_string(),
        method: "Application".to_string(),
        duration_ms: duration,
        results_count: 5,
    });

    // Manhattan - Application Side
    let start = Instant::now();
    for _ in 0..iterations {
        let mut results: Vec<_> = carriers.iter()
            .filter(|c| c.carrier_id != reference.carrier_id)
            .map(|c| (c, c.manhattan_distance(&ref_vec)))
            .collect();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let _top5: Vec<_> = results.into_iter().take(5).collect();
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    println!("  MANHATTAN (app-side):  {:.4} ms/query", duration);
    benchmarks.push(BenchmarkResult {
        name: "Manhattan".to_string(),
        method: "Application".to_string(),
        duration_ms: duration,
        results_count: 5,
    });

    // =========================================================================
    // Step 5: Benchmark - Native SurrealDB HNSW Queries
    // =========================================================================
    println!("\n══════════════════════════════════════════════════════════════");
    println!("  BENCHMARK: Native SurrealDB HNSW Queries");
    println!("══════════════════════════════════════════════════════════════\n");

    let ref_vec_surreal = format!("[{}, {}, {}, {}]", ref_vec[0], ref_vec[1], ref_vec[2], ref_vec[3]);

    // Euclidean - Native HNSW
    // SurrealDB KNN syntax: <|K,DIST|> for K nearest neighbors
    // NOTE: Must use explicit field selection, SELECT * fails due to id serialization
    let query = format!(
        "SELECT <string>id as carrier_id, perf_vector FROM carrier WHERE perf_vector <|5,EUCLIDEAN|> {}",
        ref_vec_surreal
    );
    let start = Instant::now();
    let mut native_euclidean_works = false;
    for i in 0..iterations {
        match db.query(&query).await {
            Ok(mut response) => {
                let results: Vec<Value> = response.take(0).unwrap_or_default();
                if i == 0 && !results.is_empty() {
                    native_euclidean_works = true;
                }
            }
            Err(_) => break,
        }
    }
    if native_euclidean_works {
        let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
        println!("  EUCLIDEAN (native):    {:.4} ms/query", duration);
        benchmarks.push(BenchmarkResult {
            name: "Euclidean".to_string(),
            method: "Native HNSW".to_string(),
            duration_ms: duration,
            results_count: 5,
        });
    }

    // Cosine - Native HNSW
    let query = format!(
        "SELECT <string>id as carrier_id, perf_vector FROM carrier WHERE perf_vector <|5,COSINE|> {}",
        ref_vec_surreal
    );
    let start = Instant::now();
    let mut native_cosine_works = false;
    for i in 0..iterations {
        match db.query(&query).await {
            Ok(mut response) => {
                let results: Vec<Value> = response.take(0).unwrap_or_default();
                if i == 0 && !results.is_empty() {
                    native_cosine_works = true;
                }
            }
            Err(e) => {
                if i == 0 {
                    println!("  COSINE (native):       FAILED - {}", e.to_string().lines().next().unwrap_or("error"));
                }
                break;
            }
        }
    }
    if native_cosine_works {
        let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
        println!("  COSINE (native):       {:.4} ms/query", duration);
        benchmarks.push(BenchmarkResult {
            name: "Cosine".to_string(),
            method: "Native HNSW".to_string(),
            duration_ms: duration,
            results_count: 5,
        });
    }

    // Manhattan - Native HNSW
    let query = format!(
        "SELECT <string>id as carrier_id, perf_vector FROM carrier WHERE perf_vector <|5,MANHATTAN|> {}",
        ref_vec_surreal
    );
    let start = Instant::now();
    let mut native_manhattan_works = false;
    for i in 0..iterations {
        match db.query(&query).await {
            Ok(mut response) => {
                let results: Vec<Value> = response.take(0).unwrap_or_default();
                if i == 0 && !results.is_empty() {
                    native_manhattan_works = true;
                }
            }
            Err(e) => {
                if i == 0 {
                    println!("  MANHATTAN (native):    FAILED - {}", e.to_string().lines().next().unwrap_or("error"));
                }
                break;
            }
        }
    }
    if native_manhattan_works {
        let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
        println!("  MANHATTAN (native):    {:.4} ms/query", duration);
        benchmarks.push(BenchmarkResult {
            name: "Manhattan".to_string(),
            method: "Native HNSW".to_string(),
            duration_ms: duration,
            results_count: 5,
        });
    }

    // =========================================================================
    // Step 6: Benchmark - Brute Force SQL (no index)
    // =========================================================================
    println!("\n══════════════════════════════════════════════════════════════");
    println!("  BENCHMARK: SQL-Based Distance (No Index)");
    println!("══════════════════════════════════════════════════════════════\n");

    // Try brute-force SQL query (avoid SELECT * due to id serialization issues)
    let query = format!(r#"
        SELECT
            <string>id as carrier_id,
            perf_vector,
            math::sqrt(
                math::pow(perf_vector[0] - {}, 2) +
                math::pow(perf_vector[1] - {}, 2) +
                math::pow(perf_vector[2] - {}, 2) +
                math::pow(perf_vector[3] - {}, 2)
            ) as distance
        FROM carrier
        WHERE perf_vector != NONE
        ORDER BY distance ASC
        LIMIT 5
    "#, ref_vec[0], ref_vec[1], ref_vec[2], ref_vec[3]);

    let start = Instant::now();
    let mut sql_works = false;
    for i in 0..iterations {
        match db.query(&query).await {
            Ok(mut response) => {
                let results: Vec<Value> = response.take(0).unwrap_or_default();
                if i == 0 && !results.is_empty() {
                    sql_works = true;
                }
            }
            Err(e) => {
                if i == 0 {
                    println!("  EUCLIDEAN (SQL):       FAILED - {}", e.to_string().lines().next().unwrap_or("error"));
                }
                break;
            }
        }
    }
    if sql_works {
        let duration = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
        println!("  EUCLIDEAN (SQL):       {:.4} ms/query", duration);
        benchmarks.push(BenchmarkResult {
            name: "Euclidean".to_string(),
            method: "SQL Brute Force".to_string(),
            duration_ms: duration,
            results_count: 5,
        });
    }

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n══════════════════════════════════════════════════════════════");
    println!("  SUMMARY");
    println!("══════════════════════════════════════════════════════════════\n");

    println!("| Metric | Method | Time (ms) | Speedup |");
    println!("|--------|--------|-----------|---------|");

    // Group by metric and compare
    for metric in &["Euclidean", "Cosine", "Manhattan"] {
        let app_time = benchmarks.iter()
            .find(|b| b.name == *metric && b.method == "Application")
            .map(|b| b.duration_ms);

        let native_time = benchmarks.iter()
            .find(|b| b.name == *metric && b.method == "Native HNSW")
            .map(|b| b.duration_ms);

        let sql_time = benchmarks.iter()
            .find(|b| b.name == *metric && b.method == "SQL Brute Force")
            .map(|b| b.duration_ms);

        if let Some(app) = app_time {
            println!("| {} | Application | {:.4} | baseline |", metric, app);
        }
        if let Some(native) = native_time {
            let speedup = app_time.map(|a| a / native).unwrap_or(0.0);
            println!("| {} | Native HNSW | {:.4} | {:.2}x |", metric, native, speedup);
        }
        if let Some(sql) = sql_time {
            let speedup = app_time.map(|a| a / sql).unwrap_or(0.0);
            println!("| {} | SQL Brute | {:.4} | {:.2}x |", metric, sql, speedup);
        }
    }

    println!("\n📊 Notes:");
    println!("   • Application-side: All data loaded into memory, computed in Rust");
    println!("   • Native HNSW: Uses SurrealDB's built-in HNSW index (approximate)");
    println!("   • SQL Brute Force: Computes distance in SurrealQL (no index)");
    println!("   • {} carriers with vectors, {} iterations per benchmark", carriers.len(), iterations);

    Ok(())
}
