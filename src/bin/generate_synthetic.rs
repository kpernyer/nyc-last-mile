//! Synthetic data generator for NYC Last-Mile dataset
//!
//! Generates additional shipment records by cloning and perturbing existing data
//! with controlled random variation. Includes population-weighted ZIP5 generation.
//!
//! Usage:
//!   cargo run --release --bin generate_synthetic -- [OPTIONS]
//!
//! Options:
//!   --multiplier <N>     How many synthetic records per original (default: 1.0 = double)
//!   --date-jitter <N>    Max days to shift dates (default: 14)
//!   --transit-jitter <N> Max transit day variation (default: 1)
//!   --otd-flip-rate <F>  Probability of flipping OTD status (default: 0.15)
//!   --seed <N>           Random seed for reproducibility (optional)
//!   --output <PATH>      Output CSV path (default: data/synthetic_data.csv)

use chrono::{Duration, NaiveDateTime, Datelike, Weekday};
use csv::{ReaderBuilder, WriterBuilder};
use rand::prelude::*;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use clap::Parser;
use nyc_last_mile::zip5_population::Zip5Generator;

/// Synthetic data generator for shipping dataset
#[derive(Parser, Debug)]
#[command(name = "generate_synthetic")]
#[command(about = "Generate synthetic shipping data with controlled variation")]
struct Args {
    /// Multiplier for synthetic records (1.0 = double the dataset)
    #[arg(long, default_value = "1.0")]
    multiplier: f64,

    /// Maximum days to jitter ship/delivery dates
    #[arg(long, default_value = "14")]
    date_jitter: i64,

    /// Maximum transit day variation
    #[arg(long, default_value = "1")]
    transit_jitter: i32,

    /// Probability of flipping OTD status (0.0 - 1.0)
    #[arg(long, default_value = "0.15")]
    otd_flip_rate: f64,

    /// Distance variation percentage (0.0 - 1.0)
    #[arg(long, default_value = "0.10")]
    distance_jitter: f64,

    /// Random seed for reproducibility
    #[arg(long)]
    seed: Option<u64>,

    /// Input CSV path
    #[arg(long, default_value = "raw-data/last-mile-data.csv")]
    input: PathBuf,

    /// Output CSV path
    #[arg(long, default_value = "data/synthetic_data.csv")]
    output: PathBuf,

    /// Include original data in output
    #[arg(long, default_value = "true")]
    include_original: bool,

    /// Generate ZIP5 codes from ZIP3
    #[arg(long, default_value = "true")]
    generate_zip5: bool,
}

/// Original CSV record structure
#[derive(Debug, Clone, Deserialize)]
struct CsvRecord {
    carrier_mode: String,
    actual_ship: String,
    actual_delivery: String,
    carrier_posted_service_days: Option<f64>,
    customer_distance: Option<f64>,
    truckload_service_days: Option<f64>,
    all_modes_goal_transit_days: i32,
    actual_transit_days: i32,
    otd_designation: String,
    load_id_pseudo: String,
    carrier_pseudo: String,
    origin_zip_3d: String,
    dest_zip_3d: String,
    ship_dow: i32,
    ship_week: i32,
    ship_month: i32,
    ship_year: i32,
    lane_zip3_pair: String,
    lane_id: String,
    distance_bucket: String,
}

/// Extended output record with ZIP5 fields
#[derive(Debug, Clone, Serialize)]
struct OutputRecord {
    carrier_mode: String,
    actual_ship: String,
    actual_delivery: String,
    carrier_posted_service_days: Option<f64>,
    customer_distance: Option<f64>,
    truckload_service_days: Option<f64>,
    all_modes_goal_transit_days: i32,
    actual_transit_days: i32,
    otd_designation: String,
    load_id_pseudo: String,
    carrier_pseudo: String,
    origin_zip_3d: String,
    dest_zip_3d: String,
    origin_zip5: String,
    dest_zip5: String,
    ship_dow: i32,
    ship_week: i32,
    ship_month: i32,
    ship_year: i32,
    lane_zip3_pair: String,
    lane_zip5_pair: String,
    lane_id: String,
    distance_bucket: String,
    is_synthetic: bool,
}

/// Parse datetime from CSV format
fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
}

/// Format datetime for CSV output
fn format_datetime(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Generate a unique load ID
fn generate_load_id(rng: &mut impl Rng) -> String {
    format!("{:012x}", rng.gen::<u64>() & 0xFFFFFFFFFFFF)
}

/// Calculate day of week (0 = Monday, 6 = Sunday)
fn day_of_week(dt: &NaiveDateTime) -> i32 {
    match dt.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

/// Perturb a datetime by a random number of days
fn perturb_date(dt: NaiveDateTime, max_days: i64, rng: &mut impl Rng) -> NaiveDateTime {
    let jitter = rng.gen_range(-max_days..=max_days);
    dt + Duration::days(jitter)
}

/// Flip OTD designation with some probability
fn maybe_flip_otd(otd: &str, flip_rate: f64, rng: &mut impl Rng) -> String {
    if rng.gen::<f64>() > flip_rate {
        return otd.to_string();
    }

    // Flip to a different status
    match otd {
        "On Time" => {
            if rng.gen_bool(0.5) {
                "Delivered Early".to_string()
            } else {
                "Late".to_string()
            }
        }
        "Delivered Early" => {
            if rng.gen_bool(0.7) {
                "On Time".to_string()
            } else {
                "Late".to_string()
            }
        }
        "Late" => {
            if rng.gen_bool(0.6) {
                "On Time".to_string()
            } else {
                "Delivered Early".to_string()
            }
        }
        _ => otd.to_string(),
    }
}

/// Perturb distance with percentage variation
fn perturb_distance(distance: Option<f64>, jitter_pct: f64, rng: &mut impl Rng) -> Option<f64> {
    distance.map(|d| {
        let factor = 1.0 + rng.gen_range(-jitter_pct..=jitter_pct);
        (d * factor).max(1.0)
    })
}

/// Update distance bucket based on new distance
fn update_distance_bucket(distance: Option<f64>) -> String {
    match distance {
        None => "unknown".to_string(),
        Some(d) if d < 100.0 => "0-100".to_string(),
        Some(d) if d < 250.0 => "100-250".to_string(),
        Some(d) if d < 500.0 => "250-500".to_string(),
        Some(d) if d < 1000.0 => "500-1k".to_string(),
        Some(d) if d < 2000.0 => "1k-2k".to_string(),
        Some(_) => "2k+".to_string(),
    }
}

/// Generate a synthetic record from an original
fn generate_synthetic(
    original: &CsvRecord,
    args: &Args,
    zip5_gen: &Zip5Generator,
    rng: &mut impl Rng,
) -> Option<OutputRecord> {
    // Parse original ship date
    let ship_dt = parse_datetime(&original.actual_ship)?;
    let delivery_dt = parse_datetime(&original.actual_delivery)?;

    // Perturb ship date
    let new_ship = perturb_date(ship_dt, args.date_jitter, rng);

    // Perturb transit days
    let transit_jitter = rng.gen_range(-args.transit_jitter..=args.transit_jitter);
    let new_transit = (original.actual_transit_days + transit_jitter).max(0);

    // Calculate new delivery date based on transit time
    let original_transit = (delivery_dt - ship_dt).num_days();
    let new_delivery = new_ship + Duration::days(original_transit + transit_jitter as i64);

    // Perturb distance
    let new_distance = perturb_distance(original.customer_distance, args.distance_jitter, rng);
    let new_bucket = update_distance_bucket(new_distance);

    // Maybe flip OTD
    let new_otd = maybe_flip_otd(&original.otd_designation, args.otd_flip_rate, rng);

    // Generate ZIP5 codes
    let origin_zip5 = if args.generate_zip5 {
        zip5_gen.generate(&original.origin_zip_3d, rng)
    } else {
        original.origin_zip_3d.replace("xx", "01")
    };

    let dest_zip5 = if args.generate_zip5 {
        zip5_gen.generate(&original.dest_zip_3d, rng)
    } else {
        original.dest_zip_3d.replace("xx", "01")
    };

    let lane_zip5_pair = format!("{}→{}", origin_zip5, dest_zip5);

    Some(OutputRecord {
        carrier_mode: original.carrier_mode.clone(),
        actual_ship: format_datetime(&new_ship),
        actual_delivery: format_datetime(&new_delivery),
        carrier_posted_service_days: original.carrier_posted_service_days,
        customer_distance: new_distance,
        truckload_service_days: original.truckload_service_days,
        all_modes_goal_transit_days: original.all_modes_goal_transit_days,
        actual_transit_days: new_transit,
        otd_designation: new_otd,
        load_id_pseudo: generate_load_id(rng),
        carrier_pseudo: original.carrier_pseudo.clone(),
        origin_zip_3d: original.origin_zip_3d.clone(),
        dest_zip_3d: original.dest_zip_3d.clone(),
        origin_zip5,
        dest_zip5,
        ship_dow: day_of_week(&new_ship),
        ship_week: new_ship.iso_week().week() as i32,
        ship_month: new_ship.month() as i32,
        ship_year: new_ship.year(),
        lane_zip3_pair: original.lane_zip3_pair.clone(),
        lane_zip5_pair,
        lane_id: original.lane_id.clone(),
        distance_bucket: new_bucket,
        is_synthetic: true,
    })
}

/// Convert original record to output format
fn original_to_output(
    original: &CsvRecord,
    zip5_gen: &Zip5Generator,
    rng: &mut impl Rng,
    generate_zip5: bool,
) -> OutputRecord {
    let origin_zip5 = if generate_zip5 {
        zip5_gen.generate(&original.origin_zip_3d, rng)
    } else {
        original.origin_zip_3d.replace("xx", "01")
    };

    let dest_zip5 = if generate_zip5 {
        zip5_gen.generate(&original.dest_zip_3d, rng)
    } else {
        original.dest_zip_3d.replace("xx", "01")
    };

    let lane_zip5_pair = format!("{}→{}", origin_zip5, dest_zip5);

    OutputRecord {
        carrier_mode: original.carrier_mode.clone(),
        actual_ship: original.actual_ship.clone(),
        actual_delivery: original.actual_delivery.clone(),
        carrier_posted_service_days: original.carrier_posted_service_days,
        customer_distance: original.customer_distance,
        truckload_service_days: original.truckload_service_days,
        all_modes_goal_transit_days: original.all_modes_goal_transit_days,
        actual_transit_days: original.actual_transit_days,
        otd_designation: original.otd_designation.clone(),
        load_id_pseudo: original.load_id_pseudo.clone(),
        carrier_pseudo: original.carrier_pseudo.clone(),
        origin_zip_3d: original.origin_zip_3d.clone(),
        dest_zip_3d: original.dest_zip_3d.clone(),
        origin_zip5,
        dest_zip5,
        ship_dow: original.ship_dow,
        ship_week: original.ship_week,
        ship_month: original.ship_month,
        ship_year: original.ship_year,
        lane_zip3_pair: original.lane_zip3_pair.clone(),
        lane_zip5_pair,
        lane_id: original.lane_id.clone(),
        distance_bucket: original.distance_bucket.clone(),
        is_synthetic: false,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    println!("🔧 Synthetic Data Generator");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Input:            {}", args.input.display());
    println!("Output:           {}", args.output.display());
    println!("Multiplier:       {:.2}x ({}% synthetic)", args.multiplier, (args.multiplier * 100.0) as i32);
    println!("Date jitter:      ±{} days", args.date_jitter);
    println!("Transit jitter:   ±{} days", args.transit_jitter);
    println!("OTD flip rate:    {:.1}%", args.otd_flip_rate * 100.0);
    println!("Distance jitter:  ±{:.1}%", args.distance_jitter * 100.0);
    println!("Include original: {}", args.include_original);
    println!("Generate ZIP5:    {}", args.generate_zip5);
    if let Some(seed) = args.seed {
        println!("Random seed:      {}", seed);
    }
    println!();

    // Initialize RNG
    let mut rng: StdRng = match args.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    // Initialize ZIP5 generator
    let zip5_gen = Zip5Generator::new();

    // Ensure output directory exists
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Read input CSV
    println!("📖 Reading input data...");
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&args.input)?;

    let records: Vec<CsvRecord> = reader.deserialize().filter_map(|r| r.ok()).collect();
    let original_count = records.len();
    println!("   Found {} original records", original_count);

    // Calculate synthetic count
    let synthetic_per_record = args.multiplier;
    let expected_synthetic = (original_count as f64 * synthetic_per_record) as usize;

    println!("\n🏭 Generating synthetic data...");

    // Create output writer
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_path(&args.output)?;

    let mut total_written = 0;
    let mut synthetic_written = 0;

    // Process each record
    for (i, record) in records.iter().enumerate() {
        // Write original if requested
        if args.include_original {
            let output = original_to_output(record, &zip5_gen, &mut rng, args.generate_zip5);
            writer.serialize(&output)?;
            total_written += 1;
        }

        // Generate synthetic records
        // Use probabilistic approach for non-integer multipliers
        let base_count = synthetic_per_record.floor() as usize;
        let extra_prob = synthetic_per_record.fract();
        let extra = if rng.gen::<f64>() < extra_prob { 1 } else { 0 };
        let synthetic_count = base_count + extra;

        for _ in 0..synthetic_count {
            if let Some(synthetic) = generate_synthetic(record, &args, &zip5_gen, &mut rng) {
                writer.serialize(&synthetic)?;
                total_written += 1;
                synthetic_written += 1;
            }
        }

        // Progress indicator
        if (i + 1) % 10000 == 0 {
            println!("   Processed {}/{} records...", i + 1, original_count);
        }
    }

    writer.flush()?;

    println!("\n✅ Generation complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Original records:  {:>8}", if args.include_original { original_count } else { 0 });
    println!("Synthetic records: {:>8}", synthetic_written);
    println!("Total written:     {:>8}", total_written);
    println!("Output file:       {}", args.output.display());

    // Show sample of generated ZIP5s
    if args.generate_zip5 {
        println!("\n📍 Sample ZIP5 generation:");
        let sample_zip3s = ["750xx", "172xx", "441xx", "100xx", "900xx"];
        for zip3 in sample_zip3s {
            let samples: Vec<String> = (0..5).map(|_| zip5_gen.generate(zip3, &mut rng)).collect();
            println!("   {} → {}", zip3, samples.join(", "));
        }
    }

    Ok(())
}
