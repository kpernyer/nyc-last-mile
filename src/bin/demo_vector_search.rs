//! Vector Similarity Search Demo
//!
//! Use case: Find carriers/lanes with similar performance characteristics
//!
//! Performance vectors encode:
//! - [otd_rate, avg_transit, volume_normalized, variance]
//!
//! This enables queries like:
//! - "Find carriers similar to my best performer"
//! - "Find lanes with similar characteristics"

use anyhow::Result;
use nyc_last_mile::db_enhanced;
use serde_json::Value;
use tracing::info;

#[derive(Debug)]
struct CarrierPerformance {
    carrier_id: String,
    otd_rate: f64,
    avg_transit: f64,
    volume: i64,
    variance: f64,
}

impl CarrierPerformance {
    /// Compute normalized vector: [otd/100, transit/10, log(vol)/5, var/5]
    fn to_vector(&self) -> [f64; 4] {
        [
            self.otd_rate / 100.0,
            self.avg_transit / 10.0,
            (self.volume as f64 + 1.0).log10() / 5.0,
            self.variance / 5.0,
        ]
    }

    /// Euclidean distance to another carrier
    fn distance_to(&self, other: &CarrierPerformance) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();
        let mut sum = 0.0;
        for i in 0..4 {
            sum += (v1[i] - v2[i]).powi(2);
        }
        sum.sqrt()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db_path = "data/lastmile_enhanced.db";
    info!("Connecting to SurrealDB (enhanced) at {}", db_path);
    let db = db_enhanced::connect(db_path).await?;

    info!("\n========================================");
    info!("  Vector Similarity Search Demo");
    info!("========================================\n");

    // =========================================================================
    // Step 1: Compute carrier performance metrics
    // =========================================================================
    info!("Step 1: Computing carrier performance metrics...\n");

    let carriers_data: Vec<Value> = db
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
                ORDER BY total_shipments DESC
            ) WHERE total_shipments > 50
        "#)
        .await?
        .take(0)?;

    // Convert to structs
    let carriers: Vec<CarrierPerformance> = carriers_data
        .iter()
        .map(|c| CarrierPerformance {
            carrier_id: c["carrier_id"].as_str().unwrap_or("?").to_string(),
            otd_rate: c["otd_rate"].as_f64().unwrap_or(0.0),
            avg_transit: c["avg_transit"].as_f64().unwrap_or(0.0),
            volume: c["total_shipments"].as_i64().unwrap_or(0),
            variance: c["transit_variance"].as_f64().unwrap_or(0.0),
        })
        .collect();

    println!("Found {} carriers with >50 shipments\n", carriers.len());
    println!("| Carrier | OTD Rate | Avg Transit | Volume | Variance | Vector |");
    println!("|---------|----------|-------------|--------|----------|--------|");
    for c in carriers.iter().take(10) {
        let v = c.to_vector();
        println!("| {} | {:.1}% | {:.2} days | {} | {:.2} | [{:.2},{:.2},{:.2},{:.2}] |",
            &c.carrier_id[8..], // Just show last part of ID
            c.otd_rate,
            c.avg_transit,
            c.volume,
            c.variance,
            v[0], v[1], v[2], v[3]
        );
    }

    // =========================================================================
    // Step 2: Find reference carrier (best OTD with high volume)
    // =========================================================================
    info!("\nStep 2: Finding best carrier for reference...\n");

    // Find best carrier: highest OTD rate among high-volume carriers
    let reference = carriers
        .iter()
        .filter(|c| c.volume > 100)
        .max_by(|a, b| a.otd_rate.partial_cmp(&b.otd_rate).unwrap())
        .expect("No reference carrier found");

    let ref_vec = reference.to_vector();
    println!("Reference carrier: {}", reference.carrier_id);
    println!("  OTD Rate: {:.1}%", reference.otd_rate);
    println!("  Avg Transit: {:.2} days", reference.avg_transit);
    println!("  Volume: {} shipments", reference.volume);
    println!("  Vector: [{:.3}, {:.3}, {:.3}, {:.3}]", ref_vec[0], ref_vec[1], ref_vec[2], ref_vec[3]);

    // =========================================================================
    // Step 3: Find most similar carriers by vector distance
    // =========================================================================
    info!("\nStep 3: Finding similar carriers using vector distance...\n");

    let mut distances: Vec<(&CarrierPerformance, f64)> = carriers
        .iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.distance_to(c)))
        .collect();

    distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("🎯 Top 5 carriers most similar to {}:", reference.carrier_id);
    println!("\n| Rank | Carrier | Distance | OTD Rate | Avg Transit | Volume |");
    println!("|------|---------|----------|----------|-------------|--------|");
    for (i, (c, dist)) in distances.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.1}% | {:.2} days | {} |",
            i + 1,
            &c.carrier_id[8..],
            dist,
            c.otd_rate,
            c.avg_transit,
            c.volume
        );
    }

    // =========================================================================
    // Step 4: Show how this would work with SurrealDB MTREE index
    // =========================================================================
    info!("\n\n========================================");
    info!("  Production Implementation");
    info!("========================================\n");

    println!("In production with SurrealDB's native vector support:\n");
    println!("1. Store vectors in carrier records:");
    println!("   UPDATE carrier:abc123 SET perf_vector = [0.95, 0.30, 0.96, 0.35];");
    println!();
    println!("2. Create MTREE index for fast similarity search:");
    println!("   DEFINE INDEX idx_carrier_perf ON carrier FIELDS perf_vector MTREE DIMENSION 4;");
    println!();
    println!("3. Query similar carriers using K-nearest neighbors:");
    println!("   SELECT * FROM carrier");
    println!("   WHERE perf_vector <|4|> [0.95, 0.30, 0.96, 0.35]");
    println!("   LIMIT 5;");
    println!();
    println!("The <|K|> operator finds the K nearest vectors using the index.\n");

    // =========================================================================
    // Step 5: Business Applications
    // =========================================================================
    println!("📊 Business Use Cases for Vector Similarity:\n");
    println!("1. CARRIER DIVERSIFICATION");
    println!("   → Given your best carrier, find backup carriers with similar performance");
    println!("   → Reduce risk by identifying alternatives before problems occur\n");

    println!("2. LANE EXPANSION");
    println!("   → Find lanes with similar characteristics to successful ones");
    println!("   → Target expansion efforts on high-probability success lanes\n");

    println!("3. ANOMALY DETECTION");
    println!("   → Carriers whose vectors drift from their historical norm");
    println!("   → Early warning for performance degradation\n");

    println!("4. CARRIER RECOMMENDATION");
    println!("   → For a new lane, find carriers that perform well on similar lanes");
    println!("   → Vector similarity between lane characteristics and carrier strengths\n");

    Ok(())
}
