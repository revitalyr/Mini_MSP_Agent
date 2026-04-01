#!/bin/bash

# Systemd service installation script for Mini MSP Agent

set -e

SERVICE_NAME="mini-msp-agent"
INSTALL_DIR="/opt/mini-msp-agent"
CONFIG_DIR="/etc/mini-msp-agent"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
USER="mini-msp-agent"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   print_error "This script must be run as root"
   exit 1
fi

print_status "Installing Mini MSP Agent as systemd service..."

# Create user
if ! id "$USER" &>/dev/null; then
    print_status "Creating user: $USER"
    useradd -r -s /bin/false -d $INSTALL_DIR $USER
fi

# Create directories
print_status "Creating directories..."
mkdir -p $INSTALL_DIR
mkdir -p $CONFIG_DIR
mkdir -p /var/log/mini-msp-agent

# Build and install binary
print_status "Building agent binary..."
cargo build --release --bin mini_msp_agent

print_status "Installing binary..."
cp target/release/mini_msp_agent $INSTALL_DIR/
chmod +x $INSTALL_DIR/mini_msp_agent

# Install configuration
print_status "Installing configuration..."
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cp configs/config.toml $CONFIG_DIR/config.toml
    print_warning "Please edit $CONFIG_DIR/config.toml to match your environment"
fi

# Create systemd service
print_status "Creating systemd service..."
cat > $SERVICE_FILE << EOF
[Unit]
Description=Mini MSP Agent
After=network.target

[Service]
Type=simple
User=$USER
Group=$USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/mini_msp_agent --config $CONFIG_DIR/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=mini-msp-agent

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$CONFIG_DIR /var/log/mini-msp-agent

[Install]
WantedBy=multi-user.target
EOF

# Set permissions
print_status "Setting permissions..."
chown -R $USER:$USER $INSTALL_DIR
chown -R $USER:$USER $CONFIG_DIR
chown -R $USER:$USER /var/log/mini-msp-agent
chmod 755 $INSTALL_DIR
chmod 644 $CONFIG_DIR/config.toml

# Reload systemd and enable service
print_status "Reloading systemd..."
systemctl daemon-reload

print_status "Enabling service..."
systemctl enable $SERVICE_NAME

print_status "Starting service..."
systemctl start $SERVICE_NAME

# Check status
sleep 2
if systemctl is-active --quiet $SERVICE_NAME; then
    print_status "✅ Mini MSP Agent service is running!"
    print_status "Status: systemctl status $SERVICE_NAME"
    print_status "Logs: journalctl -u $SERVICE_NAME -f"
    print_status "Configuration: $CONFIG_DIR/config.toml"
else
    print_error "❌ Failed to start service"
    print_error "Check logs: journalctl -u $SERVICE_NAME"
    exit 1
fi

print_status "Installation completed successfully!"
