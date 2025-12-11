# Connecting to the Hosted MCP Server

Once deployed to `https://logistic.hey.sh`, MCP clients can connect using HTTP transport.

## Claude Desktop Configuration

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "lastmile": {
      "transport": {
        "type": "http",
        "url": "https://logistic.hey.sh/mcp"
      }
    }
  }
}
```

## Direct API Usage

You can also call the MCP endpoint directly:

### Initialize
```bash
curl -X POST https://logistic.hey.sh/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

### List Tools
```bash
curl -X POST https://logistic.hey.sh/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

### Call a Tool
```bash
curl -X POST https://logistic.hey.sh/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "get_lane_clusters",
      "arguments": {}
    }
  }'
```

### Get Friction Zones
```bash
curl -X POST https://logistic.hey.sh/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "get_friction_zones",
      "arguments": {"limit": 5}
    }
  }'
```

## Available Tools

| Tool | Description |
|------|-------------|
| `get_lane_clusters` | Get all 5 behavioral clusters with statistics |
| `get_lanes_in_cluster` | List lanes in a specific cluster (1-5) |
| `get_lane_profile` | Get metrics for a specific origin→dest lane |
| `get_cluster_playbook` | Get recommended actions for a cluster |
| `find_similar_lanes` | Find lanes with similar behavior patterns |
| `get_early_delivery_analysis` | Analyze early delivery patterns |
| `get_regional_performance` | Get performance for a ZIP3 region |
| `get_friction_zones` | Identify high-friction destinations |
| `get_terminal_performance` | Score origin terminals on performance |

## Health Check

```bash
curl https://logistic.hey.sh/health
```

## Server-Sent Events (SSE)

For real-time notifications (if implemented):

```bash
curl -N https://logistic.hey.sh/sse
```
