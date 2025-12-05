//! Similarity Metrics Comparison Demo
//!
//! Shows different distance/similarity metrics and when to use each

use anyhow::Result;
use nyc_last_mile::db_enhanced;
use serde_json::Value;
use tracing::info;

#[derive(Debug, Clone)]
struct CarrierPerformance {
    carrier_id: String,
    otd_rate: f64,
    avg_transit: f64,
    volume: i64,
    variance: f64,
}

impl CarrierPerformance {
    /// Normalized vector: [otd/100, transit/10, log(vol)/5, var/5]
    fn to_vector(&self) -> [f64; 4] {
        [
            self.otd_rate / 100.0,
            self.avg_transit / 10.0,
            (self.volume as f64 + 1.0).log10() / 5.0,
            self.variance / 5.0,
        ]
    }

    // =========================================================================
    // DISTANCE METRICS (lower = more similar)
    // =========================================================================

    /// Euclidean Distance (L2) - Standard "straight line" distance
    /// Best for: General purpose, when all dimensions are equally important
    fn euclidean_distance(&self, other: &Self) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();
        v1.iter().zip(v2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Manhattan Distance (L1) - Sum of absolute differences
    /// Best for: Robust to outliers, grid-like movement patterns
    fn manhattan_distance(&self, other: &Self) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();
        v1.iter().zip(v2.iter())
            .map(|(a, b)| (a - b).abs())
            .sum()
    }

    /// Chebyshev Distance (L∞) - Maximum difference in any dimension
    /// Best for: When worst-case difference matters most
    fn chebyshev_distance(&self, other: &Self) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();
        v1.iter().zip(v2.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, |max, x| max.max(x))
    }

    /// Weighted Euclidean - Custom weights per dimension
    /// Best for: When some metrics matter more than others
    fn weighted_euclidean(&self, other: &Self, weights: &[f64; 4]) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();
        v1.iter().zip(v2.iter()).zip(weights.iter())
            .map(|((a, b), w)| w * (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    // =========================================================================
    // SIMILARITY METRICS (higher = more similar)
    // =========================================================================

    /// Cosine Similarity - Angle between vectors (ignores magnitude)
    /// Best for: When direction/pattern matters more than absolute values
    /// Range: -1 to 1 (1 = identical direction)
    fn cosine_similarity(&self, other: &Self) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();

        let dot: f64 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let mag1: f64 = v1.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let mag2: f64 = v2.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

        if mag1 == 0.0 || mag2 == 0.0 {
            0.0
        } else {
            dot / (mag1 * mag2)
        }
    }

    /// Pearson Correlation - Linear correlation between vectors
    /// Best for: Detecting if metrics move together proportionally
    /// Range: -1 to 1 (1 = perfect positive correlation)
    fn pearson_correlation(&self, other: &Self) -> f64 {
        let v1 = self.to_vector();
        let v2 = other.to_vector();

        let n = v1.len() as f64;
        let mean1: f64 = v1.iter().sum::<f64>() / n;
        let mean2: f64 = v2.iter().sum::<f64>() / n;

        let numerator: f64 = v1.iter().zip(v2.iter())
            .map(|(a, b)| (a - mean1) * (b - mean2))
            .sum();

        let std1: f64 = v1.iter().map(|x| (x - mean1).powi(2)).sum::<f64>().sqrt();
        let std2: f64 = v2.iter().map(|x| (x - mean2).powi(2)).sum::<f64>().sqrt();

        if std1 == 0.0 || std2 == 0.0 {
            0.0
        } else {
            numerator / (std1 * std2)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db_path = "data/lastmile_enhanced.db";
    info!("Connecting to SurrealDB at {}", db_path);
    let db = db_enhanced::connect(db_path).await?;

    println!("\n========================================");
    println!("  Similarity Metrics Comparison");
    println!("========================================\n");

    // Load carrier data
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

    // Pick reference carrier
    let reference = carriers
        .iter()
        .filter(|c| c.volume > 100)
        .max_by(|a, b| a.otd_rate.partial_cmp(&b.otd_rate).unwrap())
        .expect("No reference carrier");

    let ref_vec = reference.to_vector();
    println!("📍 Reference Carrier: {}", &reference.carrier_id[8..]);
    println!("   OTD: {:.1}% | Transit: {:.2} days | Volume: {} | Var: {:.2}",
        reference.otd_rate, reference.avg_transit, reference.volume, reference.variance);
    println!("   Vector: [{:.3}, {:.3}, {:.3}, {:.3}]\n", ref_vec[0], ref_vec[1], ref_vec[2], ref_vec[3]);

    // =========================================================================
    // Compare all metrics
    // =========================================================================

    println!("═══════════════════════════════════════════════════════════════════════════");
    println!("1️⃣  EUCLIDEAN DISTANCE (L2)");
    println!("    Standard straight-line distance. Good general-purpose metric.");
    println!("    Use when: All dimensions equally important, no outliers expected.");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let mut euclidean: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.euclidean_distance(c)))
        .collect();
    euclidean.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("| Rank | Carrier | Distance | OTD | Transit | Volume |");
    println!("|------|---------|----------|-----|---------|--------|");
    for (i, (c, d)) in euclidean.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], d, c.otd_rate, c.avg_transit, c.volume);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════");
    println!("2️⃣  MANHATTAN DISTANCE (L1)");
    println!("    Sum of absolute differences. More robust to outliers.");
    println!("    Use when: Data has outliers or errors you want to minimize impact of.");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let mut manhattan: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.manhattan_distance(c)))
        .collect();
    manhattan.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("| Rank | Carrier | Distance | OTD | Transit | Volume |");
    println!("|------|---------|----------|-----|---------|--------|");
    for (i, (c, d)) in manhattan.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], d, c.otd_rate, c.avg_transit, c.volume);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════");
    println!("3️⃣  CHEBYSHEV DISTANCE (L∞)");
    println!("    Maximum difference in any single dimension.");
    println!("    Use when: Worst-case deviation matters (e.g., SLA compliance).");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let mut chebyshev: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.chebyshev_distance(c)))
        .collect();
    chebyshev.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("| Rank | Carrier | Distance | OTD | Transit | Volume |");
    println!("|------|---------|----------|-----|---------|--------|");
    for (i, (c, d)) in chebyshev.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], d, c.otd_rate, c.avg_transit, c.volume);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════");
    println!("4️⃣  WEIGHTED EUCLIDEAN (OTD-focused)");
    println!("    Custom weights: OTD=4x, Transit=2x, Volume=1x, Variance=1x");
    println!("    Use when: Some metrics are more important than others.");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let weights = [4.0, 2.0, 1.0, 1.0]; // OTD is 4x more important
    let mut weighted: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.weighted_euclidean(c, &weights)))
        .collect();
    weighted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("| Rank | Carrier | Distance | OTD | Transit | Volume |");
    println!("|------|---------|----------|-----|---------|--------|");
    for (i, (c, d)) in weighted.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], d, c.otd_rate, c.avg_transit, c.volume);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════");
    println!("5️⃣  COSINE SIMILARITY");
    println!("    Measures angle between vectors (ignores magnitude).");
    println!("    Use when: Pattern/ratio matters more than absolute values.");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let mut cosine: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.cosine_similarity(c)))
        .collect();
    cosine.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // Higher is better

    println!("| Rank | Carrier | Similarity | OTD | Transit | Volume |");
    println!("|------|---------|------------|-----|---------|--------|");
    for (i, (c, s)) in cosine.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], s, c.otd_rate, c.avg_transit, c.volume);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════");
    println!("6️⃣  PEARSON CORRELATION");
    println!("    Measures linear correlation between metric patterns.");
    println!("    Use when: Looking for carriers that 'behave similarly' over time.");
    println!("═══════════════════════════════════════════════════════════════════════════\n");

    let mut pearson: Vec<_> = carriers.iter()
        .filter(|c| c.carrier_id != reference.carrier_id)
        .map(|c| (c, reference.pearson_correlation(c)))
        .collect();
    pearson.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // Higher is better

    println!("| Rank | Carrier | Correlation | OTD | Transit | Volume |");
    println!("|------|---------|-------------|-----|---------|--------|");
    for (i, (c, r)) in pearson.iter().take(5).enumerate() {
        println!("| {} | {} | {:.4} | {:.0}% | {:.2}d | {} |",
            i+1, &c.carrier_id[8..], r, c.otd_rate, c.avg_transit, c.volume);
    }

    // =========================================================================
    // Summary & Recommendations
    // =========================================================================
    println!("\n\n========================================");
    println!("  📊 When to Use Each Metric");
    println!("========================================\n");

    println!("┌─────────────────────┬────────────────────────────────────────────────────┐");
    println!("│ METRIC              │ BEST USE CASE                                      │");
    println!("├─────────────────────┼────────────────────────────────────────────────────┤");
    println!("│ Euclidean (L2)      │ General purpose, balanced importance               │");
    println!("│ Manhattan (L1)      │ Robust to outliers, noisy data                     │");
    println!("│ Chebyshev (L∞)      │ SLA compliance, worst-case matters                 │");
    println!("│ Weighted Euclidean  │ Business priorities (e.g., OTD > Volume)           │");
    println!("│ Cosine Similarity   │ Pattern matching, scale-invariant                  │");
    println!("│ Pearson Correlation │ Behavioral similarity, trend analysis              │");
    println!("└─────────────────────┴────────────────────────────────────────────────────┘");

    println!("\n🎯 RECOMMENDATION FOR LOGISTICS:\n");
    println!("   • Finding backup carriers: WEIGHTED EUCLIDEAN (prioritize OTD)");
    println!("   • Lane similarity: COSINE (pattern matters more than scale)");
    println!("   • Anomaly detection: CHEBYSHEV (catch worst deviations)");
    println!("   • General exploration: EUCLIDEAN (balanced view)\n");

    Ok(())
}
