# MCP Server Graph-Oriented Tools Proposal

This document proposes new MCP tools that leverage the graph relationships in the synthetic database.

## Current Tools (Analytics-Focused)

| Tool | Description |
|------|-------------|
| `get_lane_clusters` | Behavioral cluster summary |
| `get_lanes_in_cluster` | Lanes in a cluster |
| `get_lane_profile` | Metrics for a specific lane |
| `get_cluster_playbook` | Recommended actions |
| `find_similar_lanes` | Similarity search |
| `get_early_delivery_analysis` | Early delivery patterns |
| `get_regional_performance` | Region metrics |
| `get_friction_zones` | Problem destinations |
| `get_terminal_performance` | DC/terminal scores |

## Proposed Graph-Oriented Tools

### 1. `get_carrier_network`

**Purpose**: Show a carrier's operational network - what lanes they serve and performance on each.

**Input**:
```json
{
  "carrier_id": "19936bf01cc6",
  "limit": 20
}
```

**Output**:
```json
{
  "carrier_id": "19936bf01cc6",
  "display_name": "XPO Logistics",
  "network": {
    "total_lanes": 45,
    "total_shipments": 12500,
    "origins": ["75024", "44110", "17227"],
    "destinations": ["17227", "85701", "33101"],
    "lanes": [
      {
        "lane": "75024→17227",
        "volume": 850,
        "otd_rate": 0.92,
        "avg_transit": 2.1
      }
    ]
  }
}
```

**Graph Query**:
```sql
SELECT
  carrier_id,
  count(<-shipped_by<-shipment) as shipments,
  array::distinct(<-shipped_by<-shipment.lane_zip5_pair) as lanes
FROM carrier WHERE carrier_id = $id;
```

---

### 2. `get_location_connections`

**Purpose**: Show what locations are directly connected to a ZIP5 (inbound and outbound).

**Input**:
```json
{
  "zip5": "75024",
  "direction": "both"  // "inbound", "outbound", or "both"
}
```

**Output**:
```json
{
  "zip5": "75024",
  "location": "Dallas, TX",
  "connections": {
    "outbound": {
      "count": 156,
      "top_destinations": ["17227", "33101", "85701"],
      "total_volume": 8500
    },
    "inbound": {
      "count": 23,
      "top_origins": ["44110", "60601", "90210"],
      "total_volume": 1200
    }
  }
}
```

**Graph Query**:
```sql
-- Outbound connections
SELECT dest_zip5, count() as volume
FROM shipment WHERE origin_zip5 = $zip5
GROUP BY dest_zip5 ORDER BY volume DESC;

-- Inbound connections
SELECT origin_zip5, count() as volume
FROM shipment WHERE dest_zip5 = $zip5
GROUP BY origin_zip5 ORDER BY volume DESC;
```

---

### 3. `trace_shipment`

**Purpose**: Follow all graph edges from a shipment to show complete context.

**Input**:
```json
{
  "load_id": "786caec8eb1b"
}
```

**Output**:
```json
{
  "shipment": {
    "load_id": "786caec8eb1b",
    "otd": "OnTime",
    "transit_days": 2
  },
  "carrier": {
    "carrier_id": "19936bf01cc6",
    "display_name": "XPO Logistics"
  },
  "origin": {
    "zip5": "75024",
    "zip3": "750xx",
    "location": "Dallas, TX"
  },
  "destination": {
    "zip5": "17227",
    "zip3": "172xx",
    "location": "Harrisburg, PA"
  },
  "lane": {
    "zip5_pair": "75024→17227",
    "zip3_pair": "750xx→172xx"
  }
}
```

**Graph Query**:
```sql
SELECT
  *,
  ->shipped_by->carrier as carrier,
  ->origin5_at->location5 as origin,
  ->dest5_at->location5 as destination,
  ->on_lane5->lane5 as lane
FROM shipment WHERE load_id = $id;
```

---

### 4. `get_reachable_destinations`

**Purpose**: From a ZIP5, what destinations can be reached and via which carriers.

**Input**:
```json
{
  "origin_zip5": "75024",
  "min_volume": 10,
  "limit": 50
}
```

**Output**:
```json
{
  "origin": "75024",
  "reachable_destinations": [
    {
      "dest_zip5": "17227",
      "location": "Harrisburg, PA",
      "volume": 850,
      "carriers": ["XPO Logistics", "FedEx Freight"],
      "avg_transit": 2.3,
      "otd_rate": 0.89
    }
  ],
  "summary": {
    "total_destinations": 156,
    "total_carriers": 8
  }
}
```

---

### 5. `get_carrier_comparison`

**Purpose**: Compare carriers on the same lane.

**Input**:
```json
{
  "origin": "75024",
  "dest": "17227"
}
```

**Output**:
```json
{
  "lane": "75024→17227",
  "carriers": [
    {
      "carrier": "XPO Logistics",
      "volume": 450,
      "otd_rate": 0.94,
      "avg_transit": 2.1,
      "late_rate": 0.04
    },
    {
      "carrier": "FedEx Freight",
      "volume": 280,
      "otd_rate": 0.88,
      "avg_transit": 2.4,
      "late_rate": 0.09
    }
  ],
  "recommendation": "XPO Logistics has better OTD (94% vs 88%) on this lane"
}
```

---

### 6. `get_network_topology`

**Purpose**: Overview of the graph structure.

**Input**: None

**Output**:
```json
{
  "nodes": {
    "shipments": 145931,
    "carriers": 117,
    "locations_zip3": 806,
    "locations_zip5": 31395,
    "lanes_zip3": 970,
    "lanes_zip5": 140144
  },
  "edges": {
    "shipped_by": 145931,
    "origin5_at": 145931,
    "dest5_at": 145931,
    "on_lane5": 145931,
    "connects5": 280288
  },
  "density": {
    "avg_shipments_per_carrier": 1247,
    "avg_shipments_per_lane": 1.04,
    "avg_destinations_per_origin": 18.3
  }
}
```

---

### 7. `find_path`

**Purpose**: Find connection path between two locations.

**Input**:
```json
{
  "from_zip5": "75024",
  "to_zip5": "17227"
}
```

**Output**:
```json
{
  "direct_connection": true,
  "paths": [
    {
      "type": "direct",
      "lane": "75024→17227",
      "volume": 850,
      "carriers": ["XPO", "FedEx"]
    }
  ],
  "alternatives": [
    {
      "type": "via_hub",
      "path": "75024→44110→17227",
      "total_volume": 120
    }
  ]
}
```

---

## Implementation Plan

### Phase 1: Basic Graph Queries (No new edges needed)

These use existing field-based queries:
1. `get_carrier_network` - Query by carrier_ref field
2. `get_location_connections` - Query by origin/dest_zip5 fields
3. `get_reachable_destinations` - Aggregate query

### Phase 2: Edge-Based Traversals (Requires graph edges)

These require `add_graph_edges` to be run first:
4. `trace_shipment` - Uses ->shipped_by, ->origin5_at, etc.
5. `get_network_topology` - Counts edges

### Phase 3: Advanced Network Analysis

6. `get_carrier_comparison` - Multi-carrier lane analysis
7. `find_path` - Path finding between locations

---

## API Endpoints to Add

```
GET /api/v1/graph/carrier/{carrier_id}/network
GET /api/v1/graph/location/{zip5}/connections
GET /api/v1/graph/shipment/{load_id}/trace
GET /api/v1/graph/location/{zip5}/reachable
GET /api/v1/graph/lane/{origin}/{dest}/carriers
GET /api/v1/graph/topology
GET /api/v1/graph/path/{from}/{to}
```

---

## Example Prompts for Claude

With these tools, users can ask:

- "What's XPO's network look like? Which lanes do they serve?"
- "Where can I ship to from Dallas (75024)?"
- "Show me the details of shipment 786caec8eb1b - carrier, origin, destination"
- "Compare carriers on the Dallas to Harrisburg lane"
- "What's the overall structure of the shipping network?"
- "Is there a direct connection from 75024 to 17227?"
