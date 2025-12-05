# Graph Traversal Queries in SurrealDB

This document explains the graph relationships in the NYC Last-Mile database and provides examples of graph traversal queries.

## Graph Structure

### Entity Nodes

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   carrier   │     │  shipment   │     │   lane5     │
│─────────────│     │─────────────│     │─────────────│
│ carrier_id  │     │ load_id     │     │ zip5_pair   │
│ display_name│     │ otd         │     │ origin_zip5 │
└─────────────┘     │ carrier_ref │     │ dest_zip5   │
                    │ origin_zip5 │     └─────────────┘
                    │ dest_zip5   │
                    │ is_synthetic│     ┌─────────────┐
                    └─────────────┘     │  location5  │
                                        │─────────────│
                                        │ zip5        │
                                        │ zip3        │
                                        │ state       │
                                        └─────────────┘
```

### Edge Relationships

```
shipment ──shipped_by──► carrier
    │
    ├──origin5_at──► location5 (origin)
    │
    ├──dest5_at──► location5 (destination)
    │
    └──on_lane5──► lane5 ──connects5──► location5
```

| Edge | From | To | Meaning |
|------|------|-------|---------|
| `shipped_by` | shipment | carrier | "This shipment was handled by this carrier" |
| `origin5_at` | shipment | location5 | "This shipment originated from this ZIP5" |
| `dest5_at` | shipment | location5 | "This shipment was delivered to this ZIP5" |
| `on_lane5` | shipment | lane5 | "This shipment traveled on this lane" |
| `connects5` | lane5 | location5 | "This lane connects these locations" |

---

## Basic Graph Traversal Syntax

### Outbound Traversal (→)

Follow edges from a node outward:

```sql
-- From shipment, follow shipped_by edge to carrier
SELECT ->shipped_by->carrier FROM shipment:abc123;
```

### Inbound Traversal (←)

Follow edges into a node:

```sql
-- Find all shipments that point to this carrier
SELECT <-shipped_by<-shipment FROM carrier:xyz789;
```

### Chained Traversal

Combine multiple hops:

```sql
-- Shipment → Lane → Destination Location
SELECT ->on_lane5->lane5->connects5->location5 FROM shipment:abc123;
```

---

## Query Examples

### 1. Carrier Queries

#### Find all shipments by a carrier

```sql
-- Using carrier_id field
SELECT * FROM shipment WHERE carrier_ref = "19936bf01cc6" LIMIT 10;

-- Using graph traversal (requires edges)
SELECT <-shipped_by<-shipment FROM carrier WHERE carrier_id = "19936bf01cc6";
```

#### Get carriers and their shipment counts

```sql
SELECT
    carrier_id,
    count(<-shipped_by<-shipment) as shipment_count
FROM carrier
ORDER BY shipment_count DESC
LIMIT 10;
```

#### Find carriers serving a specific destination

```sql
SELECT DISTINCT ->shipped_by->carrier.carrier_id
FROM shipment
WHERE dest_zip5 = "17227";
```

### 2. Location Queries

#### All shipments from a specific ZIP5

```sql
-- Using field
SELECT * FROM shipment WHERE origin_zip5 = "75024" LIMIT 10;

-- Using graph traversal
SELECT <-origin5_at<-shipment FROM location5 WHERE zip5 = "75024";
```

#### Find all destinations reachable from an origin

```sql
SELECT DISTINCT ->on_lane5->lane5.dest_zip5
FROM shipment
WHERE origin_zip5 = "75024";
```

#### Get location with shipment volume

```sql
SELECT
    zip5,
    count(<-origin5_at<-shipment) as outbound,
    count(<-dest5_at<-shipment) as inbound
FROM location5
ORDER BY outbound DESC
LIMIT 20;
```

### 3. Lane Queries

#### Find all lanes from a ZIP5

```sql
SELECT * FROM lane5 WHERE origin_zip5 = "75024";
```

#### Get lane performance metrics

```sql
SELECT
    zip5_pair,
    count(<-on_lane5<-shipment) as volume,
    math::mean(<-on_lane5<-shipment.actual_transit_days) as avg_transit
FROM lane5
ORDER BY volume DESC
LIMIT 10;
```

#### Lanes with high late rates

```sql
SELECT
    zip5_pair,
    count(<-on_lane5<-shipment) as total,
    count(<-on_lane5<-shipment[WHERE otd = "Late"]) as late_count
FROM lane5
ORDER BY late_count DESC
LIMIT 10;
```

### 4. Multi-Hop Traversals

#### Carrier → Lanes they operate on

```sql
SELECT
    carrier_id,
    array::distinct(<-shipped_by<-shipment->on_lane5->lane5.zip5_pair) as lanes
FROM carrier
LIMIT 5;
```

#### Origin → Carriers serving that origin

```sql
SELECT
    zip5,
    array::distinct(<-origin5_at<-shipment->shipped_by->carrier.carrier_id) as carriers
FROM location5
WHERE zip5 = "75024";
```

#### Find path: Origin → Lane → Destination

```sql
SELECT
    origin_zip5,
    ->on_lane5->lane5.zip5_pair as lane,
    dest_zip5
FROM shipment
WHERE origin_zip5 = "75024"
LIMIT 10;
```

### 5. Aggregation with Graph Data

#### OTD rate by carrier

```sql
SELECT
    carrier_id,
    count(<-shipped_by<-shipment) as total,
    count(<-shipped_by<-shipment[WHERE otd = "OnTime"]) as on_time,
    count(<-shipped_by<-shipment[WHERE otd = "Late"]) as late
FROM carrier
ORDER BY total DESC
LIMIT 10;
```

#### Volume by state (using ZIP5 → ZIP3 → state)

```sql
SELECT
    state,
    count(<-origin5_at<-shipment) as shipments
FROM location5
WHERE state != NONE
GROUP BY state
ORDER BY shipments DESC;
```

---

## Advanced Patterns

### Subqueries with Graph Results

```sql
-- Find high-volume lanes and their carrier mix
SELECT
    zip5_pair,
    (SELECT DISTINCT ->shipped_by->carrier.carrier_id FROM <-on_lane5<-shipment) as carriers,
    count(<-on_lane5<-shipment) as volume
FROM lane5
ORDER BY volume DESC
LIMIT 5;
```

### Recursive-like Patterns

```sql
-- Find all locations connected to Dallas within 2 hops
SELECT DISTINCT
    ->connects5->location5.zip5 as direct,
    ->connects5->location5<-connects5<-lane5->connects5->location5.zip5 as two_hop
FROM lane5
WHERE origin_zip5 CONTAINS "750";
```

### Filtering During Traversal

```sql
-- Only follow edges to on-time shipments
SELECT
    carrier_id,
    count(<-shipped_by<-shipment[WHERE otd = "OnTime"]) as on_time_count
FROM carrier;
```

---

## Performance Considerations

### Use Indexes for Filtering

Before traversing, filter using indexed fields:

```sql
-- Good: Filter first, then traverse
SELECT ->shipped_by->carrier
FROM shipment
WHERE origin_zip5 = "75024" AND actual_ship > "2024-01-01";

-- Less efficient: Traverse everything, filter later
SELECT * FROM carrier WHERE <-shipped_by<-shipment.origin_zip5 = "75024";
```

### Limit Results

Always use LIMIT for exploratory queries:

```sql
SELECT ->on_lane5->lane5 FROM shipment LIMIT 100;
```

### Avoid Deep Traversals on Large Sets

```sql
-- Can be slow with 146K shipments
SELECT DISTINCT ->on_lane5->lane5->connects5->location5 FROM shipment;

-- Better: Start from a filtered set
SELECT DISTINCT ->on_lane5->lane5->connects5->location5
FROM shipment
WHERE carrier_ref = "19936bf01cc6";
```

---

## Common Query Patterns

### Pattern 1: "What carriers serve this route?"

```sql
SELECT DISTINCT ->shipped_by->carrier.carrier_id
FROM shipment
WHERE origin_zip5 = "75024" AND dest_zip5 = "17227";
```

### Pattern 2: "What's the OTD for this lane?"

```sql
SELECT
    count() as total,
    count(otd = "OnTime") as on_time,
    count(otd = "Late") as late
FROM shipment
WHERE lane_zip5_pair = "75024→17227"
GROUP ALL;
```

### Pattern 3: "Top destinations from an origin"

```sql
SELECT
    dest_zip5,
    count() as volume
FROM shipment
WHERE origin_zip5 = "75024"
GROUP BY dest_zip5
ORDER BY volume DESC
LIMIT 10;
```

### Pattern 4: "Carrier network analysis"

```sql
SELECT
    carrier_id,
    count(<-shipped_by<-shipment) as shipments,
    array::len(array::distinct(<-shipped_by<-shipment.origin_zip5)) as origins,
    array::len(array::distinct(<-shipped_by<-shipment.dest_zip5)) as destinations
FROM carrier
ORDER BY shipments DESC;
```

---

## Visualization Ideas

The graph structure enables network visualizations:

1. **Carrier Network Map**: Nodes = ZIP5 locations, Edges = lanes, colored by carrier
2. **Volume Heatmap**: ZIP5 regions sized by shipment volume
3. **Lane Performance**: Edges colored by OTD rate (green=good, red=poor)
4. **Carrier Comparison**: Side-by-side lane coverage for different carriers

---

## Troubleshooting

### "No edges found"

Ensure graph edges were created:
```sql
SELECT count() FROM shipped_by GROUP ALL;
```

If zero, run:
```bash
./target/release/add_graph_edges --db data/synthetic.db
```

### Slow queries

Check if indexes exist:
```sql
INFO FOR TABLE shipment;
```

### Empty results with traversal

Verify the starting record exists:
```sql
SELECT * FROM shipment WHERE load_id = "abc123";
SELECT * FROM carrier WHERE carrier_id = "xyz789";
```
