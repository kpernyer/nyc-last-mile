# Last-Mile Analytics Demo Script

A guided demo showcasing the MCP integration with Claude Desktop. Each section builds on the previous, telling the story of analyzing and optimizing a delivery network.

---

## Prerequisites

Make sure the API server is running:
```bash
./target/release/api_server --rest-only
```

---

## Act 1: The Big Picture

### Scene 1: Network Overview
**Ask Claude:**
> "Give me an executive summary of our delivery network. How many shipments, lanes, and what's our overall on-time performance?"

*This uses `get_lane_clusters` to show the 5 behavioral clusters and overall metrics.*

### Scene 2: Understanding the Clusters
**Ask Claude:**
> "Explain the 5 lane clusters. Which ones should I be most concerned about?"

*Shows the clustering intelligence - Early & Stable, On-Time & Reliable, High-Jitter, Systematically Late, Low Volume.*

---

## Act 2: Finding Problems

### Scene 3: The Trouble Spots
**Ask Claude:**
> "Where are our biggest friction zones? Show me the destinations causing the most delivery problems."

*Uses `get_friction_zones` to identify problem destinations.*

### Scene 4: Systematically Late Lanes
**Ask Claude:**
> "Show me the lanes that are systematically late. Which routes consistently miss their SLA?"

*Uses `get_lanes_in_cluster` with cluster 4 (Systematically Late).*

### Scene 5: Regional Deep Dive
**Ask Claude:**
> "How is the Denver region performing? Break down the issues there."

*Uses `get_regional_performance` with "DEN" or "804".*

---

## Act 3: Hidden Opportunities

### Scene 6: Early Deliveries
**Ask Claude:**
> "Are there lanes where we're arriving too early? This might indicate over-provisioned transit times we could optimize."

*Uses `get_early_delivery_analysis` to find optimization opportunities.*

### Scene 7: Terminal Performance
**Ask Claude:**
> "Rank our distribution centers by outbound delivery performance. Which terminals are our best and worst performers?"

*Uses `get_terminal_performance` to score origin DCs.*

---

## Act 4: Actionable Recommendations

### Scene 8: Playbook for Problem Lanes
**Ask Claude:**
> "What's the recommended playbook for handling our systematically late lanes?"

*Uses `get_cluster_playbook` for cluster 4.*

### Scene 9: High-Jitter Strategy
**Ask Claude:**
> "We have some lanes with unpredictable delivery times. What strategies should we use for these high-jitter routes?"

*Uses `get_cluster_playbook` for cluster 3 (High-Jitter).*

### Scene 10: Finding Similar Patterns
**Ask Claude:**
> "I'm having issues with the DFW to Denver lane. Are there other lanes with similar behavior that I should apply the same strategy to?"

*Uses `find_similar_lanes` to group lanes by behavior.*

---

## Act 5: Strategic Questions

### Scene 11: Carrier Negotiation
**Ask Claude:**
> "Based on our lane performance data, which routes should I prioritize when renegotiating carrier contracts?"

*Combines friction zones, late lanes, and terminal data for a strategic view.*

### Scene 12: SLA Optimization
**Ask Claude:**
> "Where should we tighten our SLA promises because we're over-delivering, and where should we add buffer days?"

*Combines early analysis with jitter analysis for SLA recommendations.*

### Scene 13: Capacity Planning
**Ask Claude:**
> "If I want to improve overall on-time delivery from 64% to 75%, which lanes should I focus on first for the biggest impact?"

*Strategic analysis combining volume, late rates, and cluster assignments.*

---

## Bonus: Conversational Follow-ups

The real power is in follow-up questions. Try these after any query:

- "Drill down on the worst one"
- "What's causing this?"
- "Compare that to our best performing lane"
- "What would you recommend?"
- "Show me the data behind that"

---

## Quick Reference: All Tools

| Tool | Best For |
|------|----------|
| `get_lane_clusters` | Executive overview, cluster summary |
| `get_lanes_in_cluster` | Finding lanes in a specific behavior group |
| `get_lane_profile` | Deep dive on a specific route |
| `get_cluster_playbook` | Actionable recommendations |
| `find_similar_lanes` | Pattern matching, applying strategies |
| `get_early_delivery_analysis` | Finding optimization opportunities |
| `get_regional_performance` | Geographic analysis |
| `get_friction_zones` | Problem identification |
| `get_terminal_performance` | DC/warehouse benchmarking |

---

## Sample Conversation Flow

Here's a natural 5-minute demo conversation:

1. **"What's the state of our delivery network?"**
   - Gets overview of 73K shipments, 64% on-time

2. **"That's not great. Where are the problems?"**
   - Shows friction zones and late lanes

3. **"Tell me more about the Denver situation"**
   - Regional deep dive

4. **"What should we do about it?"**
   - Gets playbook recommendations

5. **"Are there other lanes with the same issue?"**
   - Finds similar patterns to apply fixes at scale

---

## Tips for a Great Demo

1. **Start broad, then drill down** - Executive summary → Problem areas → Specific lanes
2. **Ask "why" questions** - Claude will use multiple tools to explain
3. **Request recommendations** - Shows the prescriptive analytics
4. **Use natural language** - No need for technical terms, Claude understands "late", "problems", "best", "worst"
5. **Follow up conversationally** - "Tell me more", "Why?", "What else?"

---

*Demo script for NYC Last-Mile Analytics MCP Integration*
