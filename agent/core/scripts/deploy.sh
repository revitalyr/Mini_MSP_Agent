#!/bin/bash
set -e

echo "Deploying Mini MSP Agent..."

# Configuration
TARGET_DIR=${1:-"/opt/msp-agent"}
SERVICE_NAME=${2:-"msp-agent"}
USER=${3:-"root"}

echo "Deployment configuration:"
echo "  Target directory: $TARGET_DIR"
echo "  Service name: $SERVICE_NAME"
echo "  User: $USER"

# Check if running as root
if [ "$EUID" -neq 0 ]; then
    echo "Error: This script must be run as root"
    exit 1
fi

# Create target directory
echo "Creating target directory..."
mkdir -p "$TARGET_DIR"
mkdir -p "$TARGET_DIR/logs"
mkdir -p "$TARGET_DIR/configs"
mkdir -p "$TARGET_DIR/plugins"

# Copy binary
echo "Copying agent binary..."
if [ -f "../../target/release/agent" ]; then
    cp ../../target/release/agent "$TARGET_DIR/"
    chmod +x "$TARGET_DIR/agent"
    echo "Agent binary copied to $TARGET_DIR/agent"
else
    echo "Error: Agent binary not found at ../../target/release/agent"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

# Copy default configuration
echo "Copying configuration files..."
if [ ! -f "$TARGET_DIR/configs/config.toml" ]; then
    cat > "$TARGET_DIR/configs/config.toml" << 'EOF'
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
EOF
    echo "Default configuration created at $TARGET_DIR/configs/config.toml"
fi

# Create systemd service
echo "Creating systemd service..."
cat > "/etc/systemd/system/$SERVICE_NAME.service" << EOF
[Unit]
Description=Mini MSP Agent
After=network.target
Wants=network.target

[Service]
Type=simple
User=$USER
Group=$USER
WorkingDirectory=$TARGET_DIR
ExecStart=$TARGET_DIR/agent -c $TARGET_DIR/configs/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=msp-agent

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$TARGET_DIR/logs $TARGET_DIR/plugins
ReadOnlyPaths=/proc /sys /usr

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

# Set permissions
echo "Setting permissions..."
chown -R $USER:$USER "$TARGET_DIR"
chmod +x "$TARGET_DIR/agent"

# Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload

# Enable and start service
echo "Enabling and starting service..."
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"

# Check service status
echo "Checking service status..."
systemctl status "$SERVICE_NAME" --no-pager

echo ""
echo "Deployment completed successfully!"
echo ""
echo "Service management commands:"
echo "  Start:   systemctl start $SERVICE_NAME"
echo "  Stop:    systemctl stop $SERVICE_NAME"
echo "  Restart: systemctl restart $SERVICE_NAME"
echo "  Status:  systemctl status $SERVICE_NAME"
echo "  Logs:    journalctl -u $SERVICE_NAME -f"
echo ""
echo "Configuration file: $TARGET_DIR/configs/config.toml"
echo "Log directory: $TARGET_DIR/logs"
