# SurrealDB Vector Search Benchmark: Native HNSW vs Application-Side

**Date:** 2024-12-04

## Executive Summary

This benchmark compares three approaches for finding similar carriers using vector similarity:

1. **Application-Side (Rust)**: Load vectors into memory, compute distances in Rust
2. **Native HNSW**: Use SurrealDB's built-in HNSW index with KNN queries
3. **SQL Brute Force**: Compute distances using SurrealQL math functions

## Results

| Metric | Method | Time (ms) | Relative Speed |
|--------|--------|-----------|----------------|
| Euclidean | Application | 0.0005 | **3,168x faster** |
| Euclidean | Native HNSW | 1.5840 | 1.0x |
| Euclidean | SQL Brute | 1.6597 | 0.95x |
| Cosine | Application | 0.0004 | **3,938x faster** |
| Cosine | Native HNSW | 1.5753 | 1.0x |
| Manhattan | Application | 0.0003 | **5,670x faster** |
| Manhattan | Native HNSW | 1.7009 | 1.0x |

## Key Findings

### 1. Application-side is ~3,000-5,000x faster for small datasets

For 33 carriers, Rust native calculations massively outperform database queries:
- Zero network/IPC overhead
- No query parsing or serialization
- Data already in memory

### 2. Native HNSW vs SQL Brute Force are comparable at this scale

With only 33 vectors, the HNSW index provides no advantage:
- HNSW: ~1.6ms average
- SQL Brute: ~1.7ms average
- Both are dominated by query overhead, not computation

### 3. SurrealDB Rust SDK Quirk: `SELECT *` fails silently

When using the Rust SDK with record links:
```rust
// This returns empty results due to 'id' (Thing type) serialization
let results: Vec<Value> = db.query("SELECT * FROM carrier").await?.take(0)?;

// This works - explicitly cast id to string
let results: Vec<Value> = db.query("SELECT <string>id as carrier_id, ... FROM carrier").await?.take(0)?;
```

## When to Use Each Approach

### Application-Side (Rust)
**Best for:**
- Small to medium datasets (< 10,000 vectors)
- Batch processing
- When data is already loaded
- Maximum performance required

### Native HNSW
**Best for:**
- Large datasets (100,000+ vectors)
- Real-time queries without data pre-loading
- Distributed queries
- When O(log n) scaling matters

### SQL Brute Force
**Best for:**
- Development/debugging
- Infrequent queries on small data
- When index maintenance overhead isn't justified

## SurrealDB Vector Query Syntax

### HNSW Index Definition
```sql
DEFINE INDEX idx_carrier_vec ON carrier
  FIELDS perf_vector
  HNSW DIMENSION 4
  DIST EUCLIDEAN
  TYPE F64;
```

### KNN Query
```sql
-- Find 5 nearest neighbors using EUCLIDEAN distance
SELECT <string>id as carrier_id, perf_vector
FROM carrier
WHERE perf_vector <|5,EUCLIDEAN|> [0.95, 0.30, 0.35, 0.20];
```

### Distance Functions (for brute force)
```sql
SELECT
  <string>id as carrier_id,
  vector::distance::euclidean(perf_vector, [0.95, 0.30, 0.35, 0.20]) as dist
FROM carrier
WHERE perf_vector != NONE
ORDER BY dist
LIMIT 5;
```

## Recommendations

1. **For this logistics use case**: Use application-side calculations
   - 33 carriers is trivial to hold in memory
   - ~1000x faster query response
   - Simpler code, no index maintenance

2. **Consider native HNSW when**:
   - Scaling to 10,000+ entities
   - Building an API where data isn't pre-loaded
   - Need to query from multiple clients

3. **Vector normalization matters**:
   - Our vectors: `[otd/100, transit/10, log(vol)/5, variance/5]`
   - Keeps dimensions comparable
   - Prevents high-magnitude features from dominating

## Test Configuration

- **Dataset**: 33 carriers with >10 shipments
- **Vector dimensions**: 4 (OTD rate, transit time, volume, variance)
- **Iterations**: 100 per benchmark
- **Database**: SurrealDB v2 with RocksDB backend
