//! Lane Clustering Analytics
//! Clusters lanes into behavioral families for standardized playbooks
//!
//! Run: ./target/release/analytics_clustering [section]
//! Sections: all, clusters, lanes, playbooks, similar

use anyhow::Result;
use nyc_last_mile::{db, carrier_names::get_carrier_name, location_names::format_lane_short};
use serde::{Deserialize, Serialize};
use std::env;

/// Lane metrics for clustering (raw counts from DB)
#[derive(Debug, Clone, Deserialize)]
struct LaneMetricsRaw {
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    avg_delay: f64,        // actual - goal (negative = early)
    transit_variance: f64,
    early_count: i64,
    ontime_count: i64,
    late_count: i64,
    avg_transit: f64,
}

/// Lane metrics with computed rates
#[derive(Debug, Clone)]
struct LaneMetrics {
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    avg_delay: f64,
    transit_variance: f64,
    early_rate: f64,
    on_time_rate: f64,
    late_rate: f64,
}

/// Lane with cluster assignment
#[derive(Debug, Clone, Serialize)]
struct ClusteredLane {
    origin_zip: String,
    dest_zip: String,
    volume: i64,
    avg_delay: f64,
    variance: f64,
    early_rate: f64,
    on_time_rate: f64,
    late_rate: f64,
    cluster_id: u8,
    cluster_name: String,
}

/// Cluster definition
#[derive(Debug, Clone)]
struct ClusterDef {
    id: u8,
    name: &'static str,
    description: &'static str,
    playbook: Vec<&'static str>,
}

/// Day-of-week pattern for a lane
#[derive(Debug, Clone, Deserialize)]
struct LaneDowPattern {
    origin_zip: String,
    dest_zip: String,
    dow: i32,
    volume: i64,
    late_rate: f64,
}

/// Carrier mix for a lane
#[derive(Debug, Clone, Deserialize)]
struct LaneCarrierMix {
    origin_zip: String,
    dest_zip: String,
    carrier_ref: String,
    carrier_mode: String,
    volume: i64,
    late_rate: f64,
}

fn get_cluster_definitions() -> Vec<ClusterDef> {
    vec![
        ClusterDef {
            id: 1,
            name: "Early & Stable",
            description: "Consistently arrive 0.5-2 days early with low variance",
            playbook: vec![
                "Implement hold-until policies at local depot",
                "Offer tight customer delivery windows",
                "Consider tightening SLA promises (reduce buffer)",
                "Use for premium time-slot offerings",
            ],
        },
        ClusterDef {
            id: 2,
            name: "On-Time & Reliable",
            description: "High on-time rate with predictable transit",
            playbook: vec![
                "Maintain current operations - these are your best lanes",
                "Use as benchmark for other lanes",
                "Suitable for guaranteed delivery promises",
                "Monitor for degradation, protect capacity",
            ],
        },
        ClusterDef {
            id: 3,
            name: "High-Jitter",
            description: "Average is OK but high variance - unpredictable",
            playbook: vec![
                "Add buffer days to customer promises",
                "Avoid 'guaranteed by noon' commitments",
                "Route to lockers/pickup points to handle timing uncertainty",
                "Investigate root cause: carrier issues? weather corridors?",
            ],
        },
        ClusterDef {
            id: 4,
            name: "Systematically Late",
            description: "Consistently miss SLA - structural problem",
            playbook: vec![
                "Downgrade promise (next-day → 2-day) for these lanes",
                "Negotiate with carriers or switch providers",
                "Consider pre-positioning inventory closer to destination",
                "Flag for carrier performance review",
            ],
        },
        ClusterDef {
            id: 5,
            name: "Low Volume / Mixed",
            description: "Insufficient data or mixed patterns",
            playbook: vec![
                "Apply conservative SLA buffers",
                "Monitor as volume grows",
                "Consider consolidating with similar lanes",
                "Default to standard operating procedures",
            ],
        },
    ]
}

/// Assign a lane to a cluster based on its metrics
fn assign_cluster(lane: &LaneMetrics) -> (u8, &'static str) {
    // Minimum volume threshold for confident clustering
    if lane.volume < 20 {
        return (5, "Low Volume / Mixed");
    }

    // Cluster 1: Early & Stable
    // - Average arrival is early (negative delay)
    // - Low variance (consistent)
    if lane.avg_delay < -0.3 && lane.transit_variance < 2.0 && lane.early_rate > 0.3 {
        return (1, "Early & Stable");
    }

    // Cluster 4: Systematically Late (check before high-jitter)
    // - High late rate regardless of variance
    if lane.late_rate > 0.45 {
        return (4, "Systematically Late");
    }

    // Cluster 3: High-Jitter
    // - High variance regardless of mean
    if lane.transit_variance > 3.5 {
        return (3, "High-Jitter");
    }

    // Cluster 2: On-Time & Reliable
    // - Good on-time rate with reasonable variance
    if lane.on_time_rate > 0.55 && lane.transit_variance < 2.5 {
        return (2, "On-Time & Reliable");
    }

    // Default: Mixed patterns
    (5, "Low Volume / Mixed")
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
    println!("{}  LANE CLUSTERING ANALYTICS  {}", "█".repeat(29), "█".repeat(30));
    println!("{}  Behavioral Families for Last-Mile Playbooks  {}", "█".repeat(20), "█".repeat(21));
    println!("{}\n", "█".repeat(90));

    // Fetch all lane metrics (raw counts - avoid SurrealDB integer division bug)
    let lanes_raw: Vec<LaneMetricsRaw> = db
        .query(r#"
            SELECT
                origin_zip,
                dest_zip,
                count() as volume,
                math::mean(actual_transit_days - goal_transit_days) as avg_delay,
                math::variance(actual_transit_days) as transit_variance,
                count(IF otd = "Early" THEN 1 END) as early_count,
                count(IF otd = "OnTime" THEN 1 END) as ontime_count,
                count(IF otd = "Late" THEN 1 END) as late_count,
                math::mean(actual_transit_days) as avg_transit
            FROM shipment
            GROUP BY origin_zip, dest_zip
        "#)
        .await?
        .take(0)?;

    // Compute rates in Rust to avoid SurrealDB integer division
    let lanes: Vec<LaneMetrics> = lanes_raw
        .into_iter()
        .map(|raw| {
            let vol = raw.volume as f64;
            LaneMetrics {
                origin_zip: raw.origin_zip,
                dest_zip: raw.dest_zip,
                volume: raw.volume,
                avg_delay: raw.avg_delay,
                transit_variance: raw.transit_variance,
                early_rate: raw.early_count as f64 / vol,
                on_time_rate: raw.ontime_count as f64 / vol,
                late_rate: raw.late_count as f64 / vol,
            }
        })
        .collect();

    // Cluster all lanes
    let clustered_lanes: Vec<ClusteredLane> = lanes
        .iter()
        .map(|lane| {
            let (cluster_id, cluster_name) = assign_cluster(lane);
            ClusteredLane {
                origin_zip: lane.origin_zip.clone(),
                dest_zip: lane.dest_zip.clone(),
                volume: lane.volume,
                avg_delay: lane.avg_delay,
                variance: lane.transit_variance,
                early_rate: lane.early_rate,
                on_time_rate: lane.on_time_rate,
                late_rate: lane.late_rate,
                cluster_id,
                cluster_name: cluster_name.to_string(),
            }
        })
        .collect();

    match section {
        "all" => {
            run_cluster_summary(&clustered_lanes).await?;
            run_cluster_details(&clustered_lanes).await?;
            run_playbooks().await?;
        }
        "clusters" => run_cluster_summary(&clustered_lanes).await?,
        "lanes" => run_cluster_details(&clustered_lanes).await?,
        "playbooks" => run_playbooks().await?,
        "similar" => {
            let target = args.get(2).cloned().unwrap_or_default();
            run_similar_lanes(&clustered_lanes, &target).await?;
        }
        _ => {
            println!("Unknown section: {}", section);
            println!("Available: all, clusters, lanes, playbooks, similar <lane>");
        }
    }

    println!("\n{}", "█".repeat(90));
    Ok(())
}

async fn run_cluster_summary(lanes: &[ClusteredLane]) -> Result<()> {
    print_section_header("CLUSTER SUMMARY");

    let clusters = get_cluster_definitions();

    // Calculate stats per cluster
    for cluster in &clusters {
        let cluster_lanes: Vec<&ClusteredLane> = lanes
            .iter()
            .filter(|l| l.cluster_id == cluster.id)
            .collect();

        let total_lanes = cluster_lanes.len();
        let total_volume: i64 = cluster_lanes.iter().map(|l| l.volume).sum();
        let avg_delay: f64 = if total_lanes > 0 {
            cluster_lanes.iter().map(|l| l.avg_delay).sum::<f64>() / total_lanes as f64
        } else {
            0.0
        };
        let avg_variance: f64 = if total_lanes > 0 {
            cluster_lanes.iter().map(|l| l.variance).sum::<f64>() / total_lanes as f64
        } else {
            0.0
        };
        let avg_late_rate: f64 = if total_lanes > 0 {
            cluster_lanes.iter().map(|l| l.late_rate).sum::<f64>() / total_lanes as f64
        } else {
            0.0
        };

        let indicator = match cluster.id {
            1 => "🟢",
            2 => "🟢",
            3 => "🟡",
            4 => "🔴",
            _ => "⚪",
        };

        println!("{} Cluster {}: {} ({} lanes, {} shipments)",
                 indicator, cluster.id, cluster.name, total_lanes, total_volume);
        println!("   {}", cluster.description);
        println!("   Avg Delay: {:+.2} days | Variance: {:.2} | Late Rate: {:.1}%",
                 avg_delay, avg_variance, avg_late_rate * 100.0);

        // Show top 5 lanes in this cluster
        let mut sorted_lanes = cluster_lanes.clone();
        sorted_lanes.sort_by(|a, b| b.volume.cmp(&a.volume));

        if !sorted_lanes.is_empty() {
            print!("   Top lanes: ");
            for (i, lane) in sorted_lanes.iter().take(5).enumerate() {
                if i > 0 { print!(", "); }
                print!("{}", format_lane_short(&lane.origin_zip, &lane.dest_zip));
            }
            println!();
        }
        println!();
    }

    // Overall distribution
    print_subsection("Cluster Distribution");
    println!("  {:25} {:>10} {:>12} {:>12}",
             "Cluster", "Lanes", "Shipments", "% Volume");
    println!("  {}", "─".repeat(61));

    let total_volume: i64 = lanes.iter().map(|l| l.volume).sum();

    for cluster in &clusters {
        let cluster_lanes: Vec<&ClusteredLane> = lanes
            .iter()
            .filter(|l| l.cluster_id == cluster.id)
            .collect();

        let lane_count = cluster_lanes.len();
        let volume: i64 = cluster_lanes.iter().map(|l| l.volume).sum();
        let pct = (volume as f64 / total_volume as f64) * 100.0;

        println!("  {:25} {:>10} {:>12} {:>11.1}%",
                 cluster.name, lane_count, volume, pct);
    }

    Ok(())
}

async fn run_cluster_details(lanes: &[ClusteredLane]) -> Result<()> {
    print_section_header("LANES BY CLUSTER");

    let clusters = get_cluster_definitions();

    for cluster in &clusters {
        let mut cluster_lanes: Vec<&ClusteredLane> = lanes
            .iter()
            .filter(|l| l.cluster_id == cluster.id)
            .collect();

        if cluster_lanes.is_empty() {
            continue;
        }

        // Sort by volume descending
        cluster_lanes.sort_by(|a, b| b.volume.cmp(&a.volume));

        print_subsection(&format!("Cluster {}: {} ({} lanes)",
                                  cluster.id, cluster.name, cluster_lanes.len()));

        println!("  {:20} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10}",
                 "Lane", "Volume", "Avg Delay", "Variance", "Early%", "OnTime%", "Late%");
        println!("  {}", "─".repeat(78));

        for lane in cluster_lanes.iter().take(15) {
            let route = format_lane_short(&lane.origin_zip, &lane.dest_zip);
            println!("  {:20} {:>8} {:>+9.2}d {:>10.2} {:>9.1}% {:>9.1}% {:>9.1}%",
                     route, lane.volume, lane.avg_delay, lane.variance,
                     lane.early_rate * 100.0, lane.on_time_rate * 100.0, lane.late_rate * 100.0);
        }

        if cluster_lanes.len() > 15 {
            println!("  ... and {} more lanes", cluster_lanes.len() - 15);
        }
    }

    Ok(())
}

async fn run_playbooks() -> Result<()> {
    print_section_header("CLUSTER PLAYBOOKS");

    let clusters = get_cluster_definitions();

    for cluster in &clusters {
        let indicator = match cluster.id {
            1 => "🟢",
            2 => "🟢",
            3 => "🟡",
            4 => "🔴",
            _ => "⚪",
        };

        println!("{} CLUSTER {}: {}", indicator, cluster.id, cluster.name.to_uppercase());
        println!("   {}", cluster.description);
        println!();
        println!("   Recommended Actions:");
        for (i, action) in cluster.playbook.iter().enumerate() {
            println!("   {}. {}", i + 1, action);
        }
        println!();
    }

    Ok(())
}

async fn run_similar_lanes(lanes: &[ClusteredLane], target: &str) -> Result<()> {
    print_section_header(&format!("LANES SIMILAR TO: {}", target));

    // Find the target lane
    let target_lane = lanes.iter().find(|l| {
        let route = format_lane_short(&l.origin_zip, &l.dest_zip);
        route.to_lowercase().contains(&target.to_lowercase()) ||
        l.origin_zip.contains(target) ||
        l.dest_zip.contains(target)
    });

    match target_lane {
        Some(lane) => {
            let route = format_lane_short(&lane.origin_zip, &lane.dest_zip);
            println!("Target Lane: {}", route);
            println!("  Cluster: {} - {}", lane.cluster_id, lane.cluster_name);
            println!("  Volume: {} | Avg Delay: {:+.2}d | Variance: {:.2}",
                     lane.volume, lane.avg_delay, lane.variance);
            println!("  Early: {:.1}% | On-Time: {:.1}% | Late: {:.1}%",
                     lane.early_rate * 100.0, lane.on_time_rate * 100.0, lane.late_rate * 100.0);

            // Find similar lanes (same cluster)
            print_subsection("Similar Lanes (Same Cluster)");

            let similar: Vec<&ClusteredLane> = lanes
                .iter()
                .filter(|l| l.cluster_id == lane.cluster_id &&
                           !(l.origin_zip == lane.origin_zip && l.dest_zip == lane.dest_zip))
                .take(20)
                .collect();

            println!("  {:20} {:>8} {:>10} {:>10} {:>10}",
                     "Lane", "Volume", "Avg Delay", "Variance", "Late%");
            println!("  {}", "─".repeat(60));

            for sim in &similar {
                let sim_route = format_lane_short(&sim.origin_zip, &sim.dest_zip);
                println!("  {:20} {:>8} {:>+9.2}d {:>10.2} {:>9.1}%",
                         sim_route, sim.volume, sim.avg_delay, sim.variance, sim.late_rate * 100.0);
            }

            println!("\n  Total {} lanes share the '{}' playbook",
                     similar.len() + 1, lane.cluster_name);
        }
        None => {
            println!("Lane not found matching: {}", target);
            println!("Try: ./target/release/analytics_clustering similar DFW");
            println!("  or: ./target/release/analytics_clustering similar 750");
        }
    }

    Ok(())
}
