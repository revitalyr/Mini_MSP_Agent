# Mini MSP Agent Core

## Overview

The Core module is the heart of the Mini MSP Agent system. It provides a modular, orchestrator-based architecture with built-in plugins for system monitoring, file operations, and network management.

## Architecture

```
core/
src/
  main.rs           # Application entry point
  orchestrator.rs    # Core orchestrator and plugin management
  broker.rs          # NATS broker client for communication
  config.rs          # Configuration management
  lib.rs             # Public API exports

plugins/
  system_plugin/     # System metrics and information
  file_plugin/        # File system operations
  network_plugin/    # Network interface monitoring

shared/
  lib.rs             # Common types and utilities

scripts/
  build.sh           # Build script (Linux/macOS)
  deploy.sh          # Deployment script (Linux/macOS)
  build.ps1          # Build script (Windows)
  deploy.ps1         # Deployment script (Windows)
```

## Features

### Core Orchestrator
- **Plugin Management**: Dynamic loading/unloading of plugins
- **Event System**: Centralized event handling and distribution
- **Command Processing**: Command routing and execution
- **Health Monitoring**: Plugin health checks and metrics collection
- **Configuration**: Flexible configuration management

### Built-in Plugins

#### System Plugin
- CPU, memory, and disk usage monitoring
- Process listing and information
- System information (hostname, OS, kernel, etc.)
- Load average (Linux)
- Uptime tracking

#### File Plugin
- Directory listing with filtering options
- File information retrieval
- File read/write operations
- Directory creation and deletion
- File copy and move operations
- Disk usage calculation

#### Network Plugin
- Network interface monitoring
- Route table inspection
- Active connection tracking
- Ping, traceroute, and nslookup utilities
- DNS server configuration

### Broker Client
- NATS-based message brokering
- Automatic reconnection handling
- Heartbeat publishing
- Metrics distribution
- Event publishing
- Command handling

### Configuration Manager
- TOML-based configuration files
- Environment variable overrides
- Command-line argument support
- Configuration validation
- Runtime configuration updates

## Building

### Prerequisites
- Rust 1.75 or later
- Cargo (included with Rust)

### Build Commands

#### Linux/macOS
```bash
# Build core library
cd agent/core
./scripts/build.sh

# Build with debug output
./scripts/build.sh --verbose

# Clean and rebuild
./scripts/build.sh --clean
```

#### Windows
```powershell
# Build core library
cd agent/core
.\scripts\build.ps1

# Build with debug output
.\scripts\build.ps1 -Verbose

# Clean and rebuild
.\scripts\build.ps1 -Clean
```

### Cargo Commands
```bash
# Build core library
cargo build --release

# Run tests
cargo test

# Check code
cargo check
cargo clippy
```

## Deployment

### Linux/macOS
```bash
# Deploy to /opt/msp-agent
./scripts/deploy.sh

# Deploy to custom location
./scripts/deploy.sh /custom/path msp-agent msp-user
```

### Windows
```powershell
# Deploy to default location
.\scripts\deploy.ps1

# Deploy to custom location
.\scripts\deploy.ps1 -TargetDir "C:\Custom\MSP Agent" -ServiceName "CustomMSPAgent"

# Deploy with Windows service
.\scripts\deploy.ps1 -CreateService
```

## Configuration

### Configuration File (config.toml)

```toml
[agent]
id = "default-agent"
platform = "linux"
heartbeat_interval = 30
metrics_interval = 10

[broker]
url = "nats://localhost:4222"
client_id = "msp-agent"
max_reconnect_attempts = 5
reconnect_delay = 5000

[logging]
level = "info"
format = "json"
file = "/opt/msp-agent/logs/agent.log"
max_file_size = 10485760
max_files = 5

[plugins]
enabled_plugins = ["system_plugin", "file_plugin", "network_plugin"]
plugin_dirs = ["/opt/msp-agent/plugins"]
auto_reload = false
hot_reload = false

[security]
allowed_commands = [
    "get_system_info",
    "get_processes",
    "get_disk_info",
    "get_memory_info",
    "get_cpu_info",
    "get_network_info",
    "list_directory",
    "get_file_info",
    "read_file",
    "get_interfaces",
    "get_routes",
    "get_connections"
]
max_file_size = 104857600
sandbox_enabled = false
require_signature = false
```

### Environment Variables

- `MSP_AGENT_ID`: Override agent ID
- `MSP_BROKER_URL`: Override broker URL
- `MSP_LOG_LEVEL`: Override log level
- `MSP_LOG_FILE`: Override log file path

### Command Line Arguments

```bash
./agent -c config.toml -b nats://localhost:4222 -i my-agent -l info
```

## Running the Agent

### Direct Execution
```bash
./agent -c config.toml
```

### Systemd Service (Linux)
```bash
sudo systemctl start msp-agent
sudo systemctl status msp-agent
sudo journalctl -u msp-agent -f
```

### Windows Service
```powershell
Start-Service -Name MSPAgent
Get-Service -Name MSPAgent
Get-EventLog -LogName System -Source "MSPAgent" -Newest 100
```

## Plugin Development

### Plugin Interface

All plugins must implement the `Plugin` trait:

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    async fn initialize(&mut self) -> Result<(), anyhow::Error>;
    async fn shutdown(&mut self) -> Result<(), anyhow::Error>;
    
    async fn handle_command(&self, command: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, anyhow::Error>;
    async fn get_metrics(&self) -> Result<SystemMetrics, anyhow::Error>;
    
    fn health_check(&self) -> Result<(), anyhow::Error> { ... }
}
```

### Built-in Plugin Structure

Built-in plugins are compiled directly into the agent binary for maximum performance and minimal overhead.

### External Plugins

External plugins can be loaded from plugin directories specified in the configuration. They must be compiled as dynamic libraries (.so, .dll, .dylib) and export the required symbols.

## API Reference

### Commands

#### System Plugin
- `get_system_info`: Get system information
- `get_processes`: Get process list
- `get_disk_info`: Get disk information
- `get_memory_info`: Get memory information
- `get_cpu_info`: Get CPU information
- `get_network_info`: Get network information
- `get_uptime`: Get system uptime
- `get_load_average`: Get load average

#### File Plugin
- `list_directory`: List directory contents
- `get_file_info`: Get file information
- `read_file`: Read file contents
- `write_file`: Write file contents
- `create_directory`: Create directory
- `delete_file`: Delete file or directory
- `move_file`: Move file or directory
- `copy_file`: Copy file or directory
- `get_disk_usage`: Get disk usage

#### Network Plugin
- `get_interfaces`: Get network interfaces
- `get_routes`: Get routing table
- `get_connections`: Get active connections
- `ping`: Ping host
- `traceroute`: Trace route to host
- `nslookup`: DNS lookup
- `get_dns_servers`: Get DNS servers

## Monitoring and Logging

### Metrics
- System metrics are published to NATS every 10 seconds (configurable)
- Metrics include CPU, memory, disk usage, and network statistics
- Metrics are available via the broker interface

### Events
- Plugin lifecycle events (loaded, unloaded, error)
- Command execution events
- System alert events
- Network and file system events

### Logs
- Structured JSON logging with configurable levels
- Automatic log rotation with size limits
- Systemd journal integration on Linux
- Windows Event Log integration on Windows

## Security

### Command Whitelisting
Only commands listed in the `allowed_commands` configuration can be executed.

### File Access Restrictions
- Maximum file size limits enforced
- Plugin sandboxing options available
- Path traversal protection

### Network Security
- Command execution restrictions
- Network access controls
- DNS filtering options

## Troubleshooting

### Common Issues

1. **Plugin Loading Failed**
   - Check plugin dependencies
   - Verify plugin interface implementation
   - Review agent logs for specific errors

2. **Broker Connection Failed**
   - Verify NATS server is running
   - Check network connectivity
   - Review broker URL configuration

3. **Permission Denied**
   - Run with appropriate privileges
   - Check file system permissions
   - Verify service configuration

### Debug Mode
```bash
# Enable debug logging
./agent -c config.toml -l debug

# Run without daemon mode
./agent -c config.toml --no-daemon
```

### Log Locations
- **Linux**: `/opt/msp-agent/logs/agent.log` or `journalctl -u msp-agent`
- **Windows**: `C:\Program Files\MSP Agent\logs\agent.log` or Event Viewer
- **macOS**: `/opt/msp-agent/logs/agent.log` or `log show`

## Performance

### Binary Size
- Optimized for size (~8MB with all plugins)
- LTO enabled for better optimization
- Strip debug symbols in release builds

### Memory Usage
- ~50MB baseline memory usage
- Additional memory depends on plugin activity
- Efficient async I/O with Tokio

### CPU Usage
- Minimal idle CPU usage (<1%)
- Spikes during metrics collection
- Efficient plugin execution

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
