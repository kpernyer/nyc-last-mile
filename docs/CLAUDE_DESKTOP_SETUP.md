# Claude Desktop Setup Guide

This guide explains how to install Claude Desktop and configure the Last-Mile Analytics MCP server for conversational data queries.

## Step 1: Download and Install Claude Desktop

### macOS

1. Visit [claude.ai/download](https://claude.ai/download)
2. Click **Download for Mac**
3. Open the downloaded `.dmg` file
4. Drag **Claude** to your Applications folder
5. Launch Claude from Applications
6. Sign in with your Anthropic account

### Windows

1. Visit [claude.ai/download](https://claude.ai/download)
2. Click **Download for Windows**
3. Run the installer (`.exe`)
4. Follow the installation wizard
5. Launch Claude from the Start menu
6. Sign in with your Anthropic account

### Linux

Claude Desktop is not officially available for Linux. Use Claude Code CLI instead:
```bash
npm install -g @anthropic-ai/claude-code
```

## Step 2: Build the MCP Server

Before configuring Claude Desktop, build the MCP server:

```bash
cd /path/to/nyc-last-mile
cargo build --release --bin mcp_server
```

This creates the binary at `target/release/mcp_server`.

## Step 3: Copy Database to Sandbox-Friendly Location

Claude Desktop runs in a sandbox that restricts file access. Copy the database:

```bash
# Create the directory
mkdir -p ~/Library/Application\ Support/LastMileAnalytics

# Copy the database
cp -r data/lastmile.db ~/Library/Application\ Support/LastMileAnalytics/
```

## Step 4: Configure Claude Desktop

### macOS

1. Open Claude Desktop
2. Go to **Claude** menu → **Settings** (or press `Cmd + ,`)
3. Click **Developer** in the sidebar
4. Click **Edit Config** to open `claude_desktop_config.json`

Or manually edit the config file:
```bash
open ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

### Windows

The config file is located at:
```
%APPDATA%\Claude\claude_desktop_config.json
```

### Configuration Content

Add this configuration:

```json
{
  "mcpServers": {
    "last-mile-analytics": {
      "command": "/Users/YOUR_USERNAME/repo/nyc-last-mile/target/release/mcp_server",
      "args": [],
      "cwd": "/Users/YOUR_USERNAME/repo/nyc-last-mile"
    }
  }
}
```

**Important**: Replace `/Users/YOUR_USERNAME/repo/nyc-last-mile` with your actual project path.

## Step 5: Restart Claude Desktop

1. Quit Claude Desktop completely (`Cmd + Q` on Mac)
2. Relaunch Claude Desktop
3. Look for the MCP server indicator (hammer icon) in the chat interface

## Step 6: Verify the Connection

Try asking Claude:

- "What are the lane clusters?"
- "Show me the friction zones"
- "How is the Phoenix region performing?"

If Claude responds with actual data, the MCP server is working correctly.

## Troubleshooting

### "MCP server error" on startup

1. Check the server path in config is correct
2. Ensure the binary exists: `ls -la /path/to/target/release/mcp_server`
3. Ensure the binary is executable: `chmod +x /path/to/target/release/mcp_server`
4. Check Claude Desktop logs: `~/Library/Logs/Claude/`

### "Database not found" errors

1. Verify the database was copied:
   ```bash
   ls -la ~/Library/Application\ Support/LastMileAnalytics/lastmile.db
   ```
2. Re-copy if needed:
   ```bash
   cp -r data/lastmile.db ~/Library/Application\ Support/LastMileAnalytics/
   ```

### "Read-only filesystem" errors

This indicates sandbox restrictions. Make sure:
1. Database is in `~/Library/Application Support/LastMileAnalytics/`
2. The MCP server binary is built with the updated path

### Server not appearing

1. Quit and relaunch Claude Desktop
2. Check the JSON config syntax is valid
3. Verify no trailing commas in the JSON

## Available MCP Tools

Once connected, Claude can use these tools:

| Tool | Description |
|------|-------------|
| `get_lane_clusters` | Get all 5 behavioral clusters |
| `get_lanes_in_cluster` | List lanes in a cluster |
| `get_lane_profile` | Metrics for a specific lane |
| `get_cluster_playbook` | Recommended actions |
| `find_similar_lanes` | Find similar lane patterns |
| `get_early_delivery_analysis` | Early delivery patterns |
| `get_regional_performance` | Regional performance metrics |
| `get_friction_zones` | Problem destinations |
| `get_terminal_performance` | Terminal/DC scorecards |

## Example Conversations

**Strategic Analysis:**
> "Which lanes are systematically late and what should we do about them?"

**Regional Deep-Dive:**
> "How is performance in the Phoenix area? What are the main problem lanes?"

**Early Delivery Investigation:**
> "Where are we arriving too early? Are we over-promising transit times?"

**Terminal Comparison:**
> "Which terminals are performing best? Which need improvement?"

**Lane Similarity:**
> "I'm seeing issues with DFW to Tucson. What other lanes behave similarly?"
