# Mini MSP Agent CLI Usage Guide

## Overview

The `mini-msp-agent` CLI tool provides comprehensive management capabilities for Mini MSP Agent instances, including agent lifecycle management, monitoring, reporting, and plugin management.

## Installation

```bash
# Build from source
cargo build --release --bin mini-msp-agent

# The binary will be available at:
# target/release/mini-msp-agent (Linux/macOS)
# target/release/mini-msp-agent.exe (Windows)
```

## Quick Start

```bash
# Start the agent
mini-msp-agent agent start --config config.toml --hot-reload

# Check agent status
mini-msp-agent agent status

# List all agents
mini-msp-agent agent list --format table

# Generate a report
mini-msp-agent report --output report.json --format json

# Show logs
mini-msp-agent logs --follow
```

## Command Structure

The CLI follows a hierarchical command structure:

```
mini-msp-agent [SUBCOMMAND] [SUB-SUBCOMMAND] [OPTIONS]
```

### Main Commands

- `agent` - Agent lifecycle and management
- `report` - Generate and manage reports
- `logs` - View and follow agent logs
- `plugin` - Plugin management
- `config` - Configuration management

## Agent Management

### Start Agent

```bash
# Basic start
mini-msp-agent agent start

# With custom configuration
mini-msp-agent agent start --config /path/to/config.toml

# Start as daemon
mini-msp-agent agent start --daemon

# With hot-reload enabled
mini-msp-agent agent start --hot-reload

# With custom plugin directory
mini-msp-agent agent start --plugin-dir /path/to/plugins

# Full command example
mini-msp-agent agent start \
  --config production.toml \
  --daemon \
  --plugin-dir /opt/mini-msp/plugins \
  --hot-reload
```

### Stop Agent

```bash
# Stop all agents
mini-msp-agent agent stop

# Stop specific agent (in future versions)
mini-msp-agent agent stop --agent-id agent-001
```

### Agent Status

```bash
# Check if agent is running
mini-msp-agent agent status

# Output example:
# Agent is running (PIDs: 12345, 12346)
```

### List Agents

```bash
# Default table format
mini-msp-agent agent list

# JSON output
mini-msp-agent agent list --format json

# CSV output
mini-msp-agent agent list --format csv

# Table output (default)
mini-msp-agent agent list --format table
```

#### Table Format Example

```
ID           Status   Hostname      Uptime    Last Heartbeat       Version Platform
--------------------------------------------------------------------------------
agent-001    active   server-01     86400     1640995200          1.0.0   linux
agent-002    active   server-02     43200     1640995200          1.0.0   linux
```

### Send Commands to Agent

```bash
# Send command to specific agent
mini-msp-agent agent send --agent-id agent-001 --command "get_processes"

# Send system info command
mini-msp-agent agent send --agent-id agent-001 --command "get_system_info"

# Send custom command
mini-msp-agent agent send --agent-id agent-001 --command "exec --cmd 'ps aux'"
```

## Reporting

### Generate Reports

```bash
# Generate JSON report (default)
mini-msp-agent report

# Custom output file
mini-msp-agent report --output /path/to/report.json

# HTML report
mini-msp-agent report --format html --output report.html

# CSV report
mini-msp-agent report --format csv --output report.csv

# Report for specific agent
mini-msp-agent report --agent-id agent-001

# Report with time range
mini-msp-agent report --time-range "2024-01-01T00:00:00Z/2024-01-02T00:00:00Z"

# Combined options
mini-msp-agent report \
  --output monthly_report.html \
  --format html \
  --agent-id agent-001 \
  --time-range "2024-01-01/2024-01-31"
```

### Report Formats

#### JSON Report Structure

```json
{
  "generated_at": 1640995200,
  "agent_id": "agent-001",
  "time_range": "2024-01-01/2024-01-02",
  "agents": [
    {
      "id": "agent-001",
      "status": "active",
      "hostname": "server-01",
      "uptime": 86400,
      "last_heartbeat": 1640995200,
      "version": "1.0.0",
      "platform": "linux",
      "plugins": [
        {
          "name": "system_plugin",
          "version": "1.0.0",
          "status": "active",
          "capabilities": ["system_metrics", "process_management"]
        }
      ]
    }
  ],
  "metrics": [
    {
      "timestamp": 1640995200,
      "cpu_usage": 45.2,
      "ram_usage": 67.8,
      "disk_usage": 23.1,
      "processes": 156,
      "network_io": {
        "bytes_sent": 1024000,
        "bytes_received": 2048000,
        "packets_sent": 1500,
        "packets_received": 2000
      }
    }
  ],
  "summary": {
    "total_agents": 1,
    "active_agents": 1,
    "avg_cpu_usage": 45.2,
    "avg_ram_usage": 67.8,
    "avg_disk_usage": 23.1,
    "total_processes": 156,
    "alerts": ["High memory usage detected"]
  }
}
```

#### HTML Report Features

- **Interactive Dashboard**: Visual metrics display
- **Responsive Design**: Works on desktop and mobile
- **Charts and Graphs**: Visual representation of data
- **Export Options**: Print and save functionality
- **Real-time Updates**: Auto-refresh capability

#### CSV Report Structure

```csv
generated_at,agent_id,time_range,total_agents,active_agents,avg_cpu_usage,avg_ram_usage,avg_disk_usage,total_processes,alerts
1640995200,agent-001,2024-01-01/2024-01-02,1,1,45.2,67.8,23.1,156,High memory usage detected
```

## Log Management

### View Logs

```bash
# View all logs
mini-msp-agent logs

# View logs for specific agent
mini-msp-agent logs --agent-id agent-001

# View last 100 lines
mini-msp-agent logs --lines 100

# Follow logs (tail -f style)
mini-msp-agent logs --follow

# Combined options
mini-msp-agent logs --agent-id agent-001 --lines 50 --follow
```

### Log Format

```
2024-01-01 12:00:00 INFO Starting Mini MSP Agent
2024-01-01 12:00:01 INFO Loaded plugin: system_plugin
2024-01-01 12:00:02 INFO Plugin hot-reload enabled
2024-01-01 12:00:03 INFO Agent initialized successfully
2024-01-01 12:00:04 INFO Starting telemetry collection
2024-01-01 12:00:05 ERROR Failed to connect to server: Connection refused
2024-01-01 12:00:06 WARN Retrying connection in 5 seconds
```

## Plugin Management

### List Plugins

```bash
# List all loaded plugins
mini-msp-agent plugin list
```

#### Plugin List Output

```
Name                 Version   Status   Capabilities
----------------------------------------------------------------------
system_plugin        1.0.0     active   system_metrics, process_management
network_plugin       1.2.0     active   network_monitoring, bandwidth_tracking
security_plugin      0.9.0     inactive firewall_management, intrusion_detection
```

### Load Plugin

```bash
# Load plugin from file
mini-msp-agent plugin load --path /path/to/plugin.so

# Load plugin with absolute path
mini-msp-agent plugin load --path /opt/mini-msp/plugins/custom_plugin.dll
```

### Unload Plugin

```bash
# Unload plugin by name
mini-msp-agent plugin unload --name system_plugin

# Unload multiple plugins
mini-msp-agent plugin unload --name network_plugin
mini-msp-agent plugin unload --name security_plugin
```

### Reload Plugin

```bash
# Reload plugin (useful for hot-reload)
mini-msp-agent plugin reload --name system_plugin

# Reload after updating plugin file
mini-msp-agent plugin reload --name custom_plugin
```

### Plugin Status

```bash
# Get detailed status for specific plugin
mini-msp-agent plugin status --name system_plugin
```

#### Plugin Status Output

```
Plugin: system_plugin
Status: active
Version: 1.0.0
Last loaded: 2024-01-01 12:00:00
Capabilities: system_metrics, process_management
Memory usage: 15.2 MB
Uptime: 2h 30m 15s
Errors: 0
```

## Configuration Management

### Show Configuration

```bash
# Display current configuration
mini-msp-agent config show
```

#### Configuration Output

```
server_url = "http://localhost:8080"
ws_url = "ws://localhost:8080/ws"
interval = 30
agent_id = "auto-generated"
log_level = "info"
hot_reload = false
```

### Get Configuration Value

```bash
# Get specific configuration value
mini-msp-agent config get --key server_url
mini-msp-agent config get --key interval
mini-msp-agent config get --key log_level
```

#### Get Output

```
server_url = "http://localhost:8080"
```

### Set Configuration Value

```bash
# Set configuration values
mini-msp-agent config set --key server_url --value "http://prod-server:8080"
mini-msp-agent config set --key interval --value "60"
mini-msp-agent config set --key log_level --value "debug"
```

### Validate Configuration

```bash
# Validate current configuration
mini-msp-agent config validate
```

#### Validation Output

```
Validating configuration...
✓ server_url: Valid URL
✓ ws_url: Valid WebSocket URL
✓ interval: Valid number (>= 1)
✓ agent_id: Valid format
✓ log_level: Valid level
Configuration is valid
```

### Reset Configuration

```bash
# Reset to default values
mini-msp-agent config reset
```

## Advanced Usage

### Batch Operations

```bash
# Start multiple agents with different configurations
for config in config1.toml config2.toml config3.toml; do
  mini-msp-agent agent start --config $config --daemon
done

# Generate reports for all agents
mini-msp-agent report --agent-id all --format json --output all_agents_report.json

# Reload all plugins
for plugin in system_plugin network_plugin security_plugin; do
  mini-msp-agent plugin reload --name $plugin
done
```

### Monitoring Scripts

```bash
#!/bin/bash
# monitor_agents.sh

while true; do
  echo "=== $(date) ==="
  mini-msp-agent agent status
  mini-msp-agent agent list --format table | head -10
  echo ""
  sleep 60
done
```

### Report Automation

```bash
#!/bin/bash
# generate_reports.sh

DATE=$(date +%Y-%m-%d)
REPORT_DIR="/reports/mini-msp"

# Create daily reports
mini-msp-agent report \
  --time-range "$DATE" \
  --format html \
  --output "$REPORT_DIR/daily_$DATE.html"

# Create weekly summary (Sundays)
if [ $(date +%u) -eq 7 ]; then
  mini-msp-agent report \
    --time-range "$(date -d '7 days ago' +%Y-%m-%d)/$DATE" \
    --format json \
    --output "$REPORT_DIR/weekly_$(date +%Y-%U).json"
fi
```

### Integration with Monitoring Tools

```bash
# Prometheus metrics export
mini-msp-agent report --format json | jq '.summary' | \
  curl -X POST http://prometheus-pushgateway/api/v1/metrics/job/mini-msp \
  --data-binary @-

# Grafana dashboard data
mini-msp-agent report --agent-id all --format json | \
  jq '.agents[] | {id: .id, cpu: .metrics[-1].cpu_usage, ram: .metrics[-1].ram_usage}' > \
  /var/lib/grafana/mini-msp-data.json
```

## Troubleshooting

### Common Issues

#### Agent Won't Start

```bash
# Check configuration
mini-msp-agent config validate

# Check plugin directory
ls -la /path/to/plugins/

# Check dependencies
ldd /path/to/agent/binary

# View detailed logs
mini-msp-agent logs --lines 100
```

#### Plugin Loading Issues

```bash
# Check plugin status
mini-msp-agent plugin list

# Try reloading plugin
mini-msp-agent plugin reload --name problematic_plugin

# Check plugin file permissions
ls -la /path/to/plugins/problematic_plugin.so
```

#### Report Generation Issues

```bash
# Check agent connectivity
mini-msp-agent agent status

# Generate minimal report
mini-msp-agent report --agent-id agent-001 --time-range "1h"

# Check output directory permissions
ls -la /path/to/output/
```

### Debug Mode

```bash
# Enable debug logging
mini-msp-agent config set --key log_level --value debug

# Start agent with debug output
RUST_LOG=debug mini-msp-agent agent start --config debug.toml

# View debug logs
mini-msp-agent logs --follow | grep DEBUG
```

## Environment Variables

```bash
# Set default server URL
export MINI_MSP_SERVER_URL="http://prod-server:8080"

# Set default plugin directory
export MINI_MSP_PLUGIN_DIR="/opt/mini-msp/plugins"

# Enable debug mode
export MINI_MSP_DEBUG="true"

# Set log level
export MINI_MSP_LOG_LEVEL="info"

# Use environment variables
mini-msp-agent agent start
```

## Configuration File Reference

### Complete Configuration Example

```toml
[agent]
server_url = "http://localhost:8080"
ws_url = "ws://localhost:8080/ws"
interval = 30
agent_id = "auto-generated"
log_level = "info"

[plugins]
directory = "./plugins"
hot_reload = false
auto_discovery = true

[reporting]
default_format = "json"
output_directory = "./reports"
include_metrics = true
include_alerts = true

[security]
allowed_commands = ["ps", "top", "df", "free", "uptime"]
max_file_size = 100000
enable_ssl = true
```

### Configuration Validation Rules

- `server_url`: Must be valid HTTP/HTTPS URL
- `ws_url`: Must be valid WebSocket URL
- `interval`: Must be positive integer (>= 1)
- `agent_id`: Must be valid UUID or "auto-generated"
- `log_level`: Must be one of: trace, debug, info, warn, error

## Performance Tips

### CLI Performance

```bash
# Use table format for large lists (faster than JSON)
mini-msp-agent agent list --format table

# Limit log lines for better performance
mini-msp-agent logs --lines 1000

# Use CSV for large datasets (smaller file size)
mini-msp-agent report --format csv

# Disable color output in scripts
NO_COLOR=1 mini-msp-agent agent list
```

### Report Generation Optimization

```bash
# Generate reports for specific time ranges only
mini-msp-agent report --time-range "1h"  # Last hour
mini-msp-agent report --time-range "24h" # Last day

# Use agent-specific reports for large deployments
mini-msp-agent report --agent-id agent-001

# Exclude metrics for faster generation
mini-msp-agent report --no-metrics
```

## Integration Examples

### Docker Integration

```dockerfile
FROM mini-msp/cli:latest

# Use CLI in Docker scripts
COPY start-agent.sh /usr/local/bin/
COPY monitor.sh /usr/local/bin/

CMD ["/usr/local/bin/start-agent.sh"]
```

### Kubernetes Integration

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mini-msp-config
data:
  start.sh: |
    #!/bin/bash
    mini-msp-agent agent start --config /etc/mini-msp/config.toml
  monitor.sh: |
    #!/bin/bash
    mini-msp-agent agent status
    mini-msp-agent report --format json --output /tmp/report.json
```

### CI/CD Pipeline Integration

```yaml
# GitHub Actions example
- name: Start Agent
  run: |
    mini-msp-agent agent start --config ci-config.toml
    
- name: Wait for Agent
  run: |
    timeout 60 bash -c 'until mini-msp-agent agent status; do sleep 1; done'
    
- name: Generate Report
  run: |
    mini-msp-agent report --format json --output ci-report.json
    
- name: Upload Report
  uses: actions/upload-artifact@v3
  with:
    name: agent-report
    path: ci-report.json
```

## Help and Documentation

### Built-in Help

```bash
# Main help
mini-msp-agent --help

# Subcommand help
mini-msp-agent agent --help
mini-msp-agent report --help
mini-msp-agent plugin --help
mini-msp-agent config --help

# Specific command help
mini-msp-agent agent start --help
mini-msp-agent report --help
```

### Version Information

```bash
# Show version
mini-msp-agent --version

# Show build information
mini-msp-agent --version --verbose
```

This comprehensive CLI tool provides complete control over Mini MSP Agent instances with intuitive commands, flexible output formats, and powerful automation capabilities.
