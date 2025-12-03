use anyhow::Result;
use nyc_last_mile::db;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let db = db::connect("data/lastmile.db").await?;

    info!("Connected to SurrealDB");

    // Example queries
    info!("=== Database Statistics ===");

    // Count shipments by OTD status
    let otd_stats: Vec<serde_json::Value> = db
        .query("SELECT otd, count() as cnt FROM shipment GROUP BY otd")
        .await?
        .take(0)?;
    info!("OTD Distribution: {:?}", otd_stats);

    // Count by carrier mode
    let mode_stats: Vec<serde_json::Value> = db
        .query("SELECT carrier_mode, count() as cnt FROM shipment GROUP BY carrier_mode")
        .await?
        .take(0)?;
    info!("Carrier Mode Distribution: {:?}", mode_stats);

    // Top 5 busiest lanes
    let top_lanes: Vec<serde_json::Value> = db
        .query(
            r#"
            SELECT
                ->on_lane->lane.zip3_pair as lane,
                count() as shipments
            FROM shipment
            GROUP BY lane
            ORDER BY shipments DESC
            LIMIT 5
            "#,
        )
        .await?
        .take(0)?;
    info!("Top 5 Lanes: {:?}", top_lanes);

    // Average transit time by distance bucket
    let transit_by_distance: Vec<serde_json::Value> = db
        .query(
            r#"
            SELECT
                distance_bucket,
                math::mean(actual_transit_days) as avg_transit,
                math::mean(goal_transit_days) as avg_goal
            FROM shipment
            GROUP BY distance_bucket
            "#,
        )
        .await?
        .take(0)?;
    info!("Transit by Distance: {:?}", transit_by_distance);

    Ok(())
}
