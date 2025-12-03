# MCP Integration Plan: Last-Mile Analytics

## Goal
Make analytics available via MCP for LLM integration, enabling conversational queries like:
- "Which inbound lanes feed my last-mile in similar ways?"
- "Where are we arriving too early?"
- "Which terminals struggle during peak?"

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  User/LLM       │────▶│  MCP Server     │────▶│  SurrealDB      │
│  Natural        │     │  (Rust)         │     │  Analytics      │
│  Language Query │◀────│  Tools          │◀────│                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

---

## Phase 1: Lane Clustering Analytics
**Status: COMPLETE**

Build clustering to identify lane "behavioral families":
1. Early & Stable lanes (43 lanes, 6.8% volume)
2. On-Time & Reliable lanes (333 lanes, 43.2% volume)
3. High-Jitter lanes (36 lanes, 7.7% volume)
4. Systematically Late lanes (112 lanes, 14.3% volume)
5. Low Volume / Mixed (3908 lanes, 28.0% volume)

### Tasks
- [x] Calculate per-lane metrics (avg delay, variance, early/on-time/late rates)
- [x] Implement rule-based clustering algorithm
- [x] Label clusters with meaningful names
- [x] Generate playbook recommendations per cluster
- [x] Create `analytics_clustering.rs` binary
- [x] Add "similar lanes" query feature

### Deliverables
- `src/bin/analytics_clustering.rs` - DONE
- CLI commands: `clusters`, `lanes`, `playbooks`, `similar <lane>`
- Actionable playbooks per cluster

---

## Phase 2: MCP Server Foundation
**Status: COMPLETE**

Create MCP server exposing analytics as tools.

### Tasks
- [x] Set up MCP server structure in Rust (simple JSON-RPC over stdio)
- [x] Implement basic tool protocol
- [x] Add tool: `get_lane_clusters()`
- [x] Add tool: `get_lanes_in_cluster(cluster_id)`
- [x] Add tool: `get_lane_profile(origin, dest)`
- [x] Add tool: `get_cluster_playbook(cluster_id)`
- [x] Add tool: `find_similar_lanes(pattern)`

### Deliverables
- `src/bin/mcp_server.rs` - DONE
- MCP configuration for Claude Desktop - DONE (see `docs/claude-desktop-config.json`)

---

## Phase 3: Early Delivery Analysis
**Status: NOT STARTED**

Quantify "too early" patterns and their last-mile impact.

### Tasks
- [ ] Define "too early" threshold (>1 day before promised)
- [ ] Analyze early patterns by ZIP3, carrier, DOW
- [ ] Quantify hidden costs (storage, failed first attempts)
- [ ] Generate hold-until recommendations
- [ ] Add MCP tool: `get_early_delivery_analysis()`

### Deliverables
- Early delivery analytics in clustering binary
- MCP tool for early analysis queries

---

## Phase 4: Geo-Spatial & Regional Analysis
**Status: NOT STARTED**

ZIP3-level friction mapping and regional performance.

### Tasks
- [ ] Map OTD performance by ZIP3 region
- [ ] Identify friction zones (high late rate, high variance)
- [ ] Seasonal comparison (normal vs peak)
- [ ] Add MCP tool: `get_regional_performance(zip3)`
- [ ] Add MCP tool: `get_friction_zones()`

### Deliverables
- Regional analytics
- MCP tools for geo queries

---

## Phase 5: Terminal Performance Index
**Status: NOT STARTED**

Score terminals/DCs on their hand-off performance.

### Tasks
- [ ] Build terminal performance metrics
- [ ] Create performance index (0-100 score)
- [ ] Decompose delays (linehaul vs terminal vs last-mile)
- [ ] Trigger recommendations for gig/overflow capacity
- [ ] Add MCP tool: `get_terminal_performance()`

### Deliverables
- Terminal performance index
- Capacity trigger recommendations

---

## MCP Tools Summary (Target State)

| Tool | Phase | Description |
|------|-------|-------------|
| `get_lane_clusters()` | 1 | Returns all clusters with characteristics |
| `get_lanes_in_cluster(id)` | 1 | List lanes in a cluster |
| `get_lane_profile(origin, dest)` | 1 | Lane metrics + cluster assignment |
| `get_cluster_playbook(id)` | 1 | Recommended last-mile strategy |
| `find_similar_lanes(origin, dest)` | 1 | Lanes behaving similarly |
| `get_early_delivery_analysis()` | 3 | Early delivery patterns |
| `get_regional_performance(zip3)` | 4 | ZIP3-level metrics |
| `get_friction_zones()` | 4 | Problem regions |
| `get_terminal_performance()` | 5 | DC/terminal scores |

---

## Progress Log

| Date | Phase | Update |
|------|-------|--------|
| 2024-12-03 | - | Plan created |
| 2024-12-03 | 1 | Lane clustering complete - 5 clusters with playbooks |
| 2024-12-03 | 1 | Fixed SurrealDB integer division bug - rates now computed in Rust |
| 2024-12-03 | 2 | MCP Server complete - 5 tools exposed via JSON-RPC over stdio |

