# Synthetic Data Generation & Ingestion Tools

This document describes the tools for generating synthetic shipping data and ingesting it into SurrealDB with optional graph relationships.

## Overview

The synthetic data pipeline consists of three tools:

| Tool | Purpose | Output |
|------|---------|--------|
| `generate_synthetic` | Create synthetic CSV data from original dataset | CSV file with ZIP5 codes |
| `ingest_synthetic` | Load synthetic CSV into SurrealDB | Database with entities |
| `add_graph_edges` | Add graph relationships to existing database | RELATE edges |

## Tool 1: generate_synthetic

Generates synthetic shipment records by cloning and perturbing existing data with controlled random variation. Adds population-weighted ZIP5 codes.

### Usage

```bash
./target/release/generate_synthetic [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--multiplier <N>` | 1.0 | Synthetic records per original (1.0 = double dataset) |
| `--date-jitter <N>` | 14 | Maximum days to shift ship/delivery dates |
| `--transit-jitter <N>` | 1 | Maximum transit day variation (±N days) |
| `--otd-flip-rate <F>` | 0.15 | Probability of flipping OTD status (0.0-1.0) |
| `--distance-jitter <F>` | 0.10 | Distance variation percentage (±10%) |
| `--seed <N>` | random | Random seed for reproducibility |
| `--input <PATH>` | raw-data/last-mile-data.csv | Input CSV path |
| `--output <PATH>` | data/synthetic_data.csv | Output CSV path |
| `--include-original` | false | Include original records in output |
| `--generate-zip5` | false | Generate ZIP5 codes from ZIP3 |

### Examples

```bash
# Double the dataset with ZIP5 codes (reproducible)
./target/release/generate_synthetic \
  --include-original \
  --generate-zip5 \
  --seed 42

# Triple the dataset with more variation
./target/release/generate_synthetic \
  --multiplier 2.0 \
  --date-jitter 30 \
  --transit-jitter 2 \
  --otd-flip-rate 0.25 \
  --include-original \
  --generate-zip5

# Generate only synthetic records (no originals)
./target/release/generate_synthetic \
  --multiplier 1.0 \
  --generate-zip5 \
  --output data/synthetic_only.csv
```

### Output Format

The output CSV includes all original columns plus:

| New Column | Description | Example |
|------------|-------------|---------|
| `origin_zip5` | 5-digit origin ZIP code | `75024` |
| `dest_zip5` | 5-digit destination ZIP code | `17227` |
| `lane_zip5_pair` | Lane at ZIP5 granularity | `75024→17227` |
| `is_synthetic` | Record origin flag | `true` / `false` |

### ZIP5 Population Weighting

ZIP5 codes are generated using Census-based population weights. Urban cores (e.g., `7502x` in Dallas) are weighted higher than suburban/rural areas (e.g., `7509x`).

Major metros with population data:
- Dallas (750xx), Houston (770xx), Austin (786xx)
- Cleveland (441xx), Harrisburg (172xx)
- Los Angeles (900xx), Chicago (606xx), NYC (100xx)
- Phoenix (850xx), Seattle (980xx), Denver (800xx)
- And 10+ more regions

---

## Tool 2: ingest_synthetic

Loads synthetic CSV data into SurrealDB with extended schema supporting ZIP5 locations and lanes.

### Usage

```bash
./target/release/ingest_synthetic [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input <PATH>` | data/synthetic_data.csv | Input CSV path |
| `--db <PATH>` | data/synthetic.db | Database path |
| `--clear` | false | Clear existing database before ingesting |
| `--batch-size <N>` | 1000 | Batch size for inserts |
| `--graph` | false | Create graph edges (slower) |

### Examples

```bash
# Basic ingestion (fast, no graph edges)
./target/release/ingest_synthetic \
  --input data/synthetic_data.csv \
  --db data/synthetic.db \
  --clear

# Full ingestion with graph edges
./target/release/ingest_synthetic \
  --input data/synthetic_data.csv \
  --db data/synthetic.db \
  --clear \
  --graph
```

### Database Schema

#### Entity Tables

| Table | Type | Key Field | Description |
|-------|------|-----------|-------------|
| `shipment` | Schemaless | `load_id` | Shipment records |
| `carrier` | Schemafull | `carrier_id` | Carrier entities |
| `location` | Schemafull | `zip3` | ZIP3 regions |
| `location5` | Schemafull | `zip5` | ZIP5 locations |
| `lane` | Schemafull | `lane_id` | ZIP3-level lanes |
| `lane5` | Schemafull | `zip5_pair` | ZIP5-level lanes |

#### Graph Edge Tables (when `--graph` enabled)

| Table | Relationship | Description |
|-------|--------------|-------------|
| `shipped_by` | shipment → carrier | Carrier that handled shipment |
| `origin5_at` | shipment → location5 | Origin ZIP5 location |
| `dest5_at` | shipment → location5 | Destination ZIP5 location |
| `on_lane5` | shipment → lane5 | Lane used for shipment |
| `connects5` | lane5 → location5 | Lane endpoints |

### Indexes

```sql
-- Shipment indexes
DEFINE INDEX load_id_idx ON shipment FIELDS load_id UNIQUE;
DEFINE INDEX actual_ship_idx ON shipment FIELDS actual_ship;
DEFINE INDEX otd_idx ON shipment FIELDS otd;
DEFINE INDEX carrier_mode_idx ON shipment FIELDS carrier_mode;
DEFINE INDEX origin_zip5_idx ON shipment FIELDS origin_zip5;
DEFINE INDEX dest_zip5_idx ON shipment FIELDS dest_zip5;
DEFINE INDEX is_synthetic_idx ON shipment FIELDS is_synthetic;
```

---

## Tool 3: add_graph_edges

Adds graph relationships (RELATE edges) to an existing database that was ingested without the `--graph` flag.

### Usage

```bash
./target/release/add_graph_edges --db <PATH>
```

### Example

```bash
# Add edges to existing database
./target/release/add_graph_edges --db data/synthetic.db
```

### Performance

For ~146K shipments:
- Time: ~90 seconds
- Edges created: ~864K total
  - `shipped_by`: 146K
  - `origin5_at`: 146K
  - `dest5_at`: 146K
  - `on_lane5`: 146K
  - `connects5`: ~280K (2 per lane)

---

## Complete Workflow

### Step 1: Generate Synthetic Data

```bash
./target/release/generate_synthetic \
  --include-original \
  --generate-zip5 \
  --seed 42 \
  --output data/synthetic_data.csv
```

**Output**: `data/synthetic_data.csv` with ~146K records (73K original + 73K synthetic)

### Step 2: Ingest into Database

```bash
./target/release/ingest_synthetic \
  --input data/synthetic_data.csv \
  --db data/synthetic.db \
  --clear
```

**Output**: SurrealDB database with entities and indexes

### Step 3: Add Graph Edges (Optional)

```bash
./target/release/add_graph_edges --db data/synthetic.db
```

**Output**: Graph relationships enabling traversal queries

---

## Verifying the Data

### Check Record Counts

```sql
-- Entity counts
SELECT count() FROM shipment GROUP ALL;
SELECT count() FROM carrier GROUP ALL;
SELECT count() FROM location5 GROUP ALL;
SELECT count() FROM lane5 GROUP ALL;

-- Edge counts (if graph enabled)
SELECT count() FROM shipped_by GROUP ALL;
SELECT count() FROM origin5_at GROUP ALL;
SELECT count() FROM on_lane5 GROUP ALL;
```

### Sample Queries

```sql
-- Original vs synthetic records
SELECT is_synthetic, count() as cnt
FROM shipment
GROUP BY is_synthetic;

-- Top origin ZIP5 codes
SELECT origin_zip5, count() as cnt
FROM shipment
GROUP BY origin_zip5
ORDER BY cnt DESC
LIMIT 10;

-- OTD distribution
SELECT otd, count() as cnt
FROM shipment
GROUP BY otd;
```

---

## Troubleshooting

### "Database index already contains" error
Duplicate `load_id` - the random ID generator had a collision. This is rare (<0.01%) and can be ignored.

### Slow ingestion
- Use `--clear` to start fresh (avoids index conflicts)
- Don't use `--graph` during initial ingest; add edges separately with `add_graph_edges`

### Memory issues with large datasets
- Increase system memory or use smaller multipliers
- Process in batches using multiple CSV files
