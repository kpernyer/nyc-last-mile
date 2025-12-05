# SurrealDB Performance Comparison: Baseline vs Enhanced Schema

**Date:** 2025-12-04

## Summary

| Schema | Total Time |
|--------|------------|
| Baseline (String refs) | 8760.84ms |
| Enhanced (Record links) | 9211.63ms |
| **Difference** | **+450.79ms (+5.1%)** |

## Detailed Comparison

| Test | Baseline (ms) | Enhanced (ms) | Diff (ms) | Change |
|------|---------------|---------------|-----------|--------|
| simple_count | 26.28 | 29.13 | +2.85 | +10.8% |
| carrier_otd | 1814.63 | 1771.69 | -42.94 | **-2.4%** |
| lane_carrier_matrix | 815.16 | 798.63 | -16.53 | **-2.0%** |
| carrier_shipment_totals | 591.31 | 583.92 | -7.39 | **-1.3%** |
| shipments_by_origin | 612.94 | 595.24 | -17.70 | **-2.9%** |
| carrier_lanes_lookup | 848.51 | 1030.30 | +181.79 | +21.4% |
| lane_stats | 1441.77 | 1383.07 | -58.70 | **-4.1%** |
| complex_late_analysis | 311.43 | 268.62 | -42.81 | **-13.7%** |
| best_carrier_per_lane | 1447.48 | 1909.84 | +462.36 | +31.9% |
| filtered_scan | 851.32 | 841.18 | -10.14 | **-1.2%** |

## Analysis

### Improvements (Enhanced is faster)
- **complex_late_analysis**: -13.7% - Best improvement, filtering on indexed record links
- **lane_stats**: -4.1% - Lane-level aggregation benefits from record links
- **shipments_by_origin**: -2.9% - Origin location aggregation
- **carrier_otd**: -2.4% - Carrier OTD aggregation
- **lane_carrier_matrix**: -2.0% - Multi-field grouping

### Regressions (Enhanced is slower)
- **best_carrier_per_lane**: +31.9% - Subquery with record link ORDER BY
- **carrier_lanes_lookup**: +21.4% - Specific carrier filter with record link

### Key Observations

1. **Aggregation queries generally improve** with record links (-2% to -14%)
2. **Subquery-wrapped queries show overhead** due to the need to cast record links to strings for serialization
3. **Direct record comparisons** should be faster, but the subquery pattern adds overhead

### Why Some Tests Are Slower

The enhanced schema tests use a subquery pattern to handle SurrealDB's serialization requirements:

```sql
-- Record link requires subquery wrapper for serialization
SELECT <string>carrier as carrier_id, shipments FROM (
    SELECT carrier, count() as shipments
    FROM shipment
    GROUP BY carrier
)
```

This extra layer adds overhead compared to the baseline's simpler:
```sql
SELECT carrier_ref, count() as shipments
FROM shipment
GROUP BY carrier_ref
```

### Recommendations

1. **Record links provide the most benefit** for queries that:
   - Use indexed lookups on the record link field
   - Perform complex aggregations
   - Filter on specific entities

2. **For maximum performance** with record links:
   - Use native SurrealDB tools/SDKs that handle Thing types directly
   - Avoid casting to string when possible
   - Consider using FETCH instead of JOIN patterns

3. **Graph traversal queries** (not fully tested here) should show significant benefits:
   - `SELECT ->shipped_by->carrier FROM shipment`
   - `SELECT <-shipped_by<-shipment FROM carrier`

## Schema Benefits Beyond Raw Performance

The enhanced schema provides:

1. **Data Integrity**: Record links enforce referential integrity
2. **Cleaner Queries**: Direct entity references instead of string comparisons
3. **Graph Traversal**: Enables native graph query patterns
4. **Vector Search Ready**: Performance vectors on carriers/lanes for similarity queries
5. **Future Scaling**: Better support for complex relationship queries
