#!/bin/bash
set -e

echo "🚀 Starting Mini MSP Agent (All Components)..."

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to start component
start_component() {
    local component=$1
    local cmd=$2
    local name=$3
    
    echo "🔄 Starting $name..."
    $cmd &
    local pid=$!
    echo "✅ $name started (PID: $pid)"
    echo $pid >> "/tmp/mini_msp_pids.txt"
    return $pid
}

# Check prerequisites
if ! command_exists cargo; then
    echo "❌ Rust/Cargo not found. Please install Rust first."
    exit 1
fi

if ! command_exists nats-server; then
    echo "❌ NATS server not found. Please install NATS server."
    exit 1
fi

# Create logs directory
mkdir -p logs

# Clean up old PID file
rm -f /tmp/mini_msp_pids.txt

# Start NATS server
echo "📡 Starting NATS server..."
nats-server -m 8222 -p 4222 &
NATS_PID=$!
echo "✅ NATS server started (PID: $NATS_PID)"
echo $NATS_PID > "/tmp/nats.pid"

# Wait for NATS to start
echo "⏳ Waiting for NATS to start..."
for i in {1..30}; do
    if nc -z localhost 4222; then
        echo "✅ NATS server is ready"
        break
    fi
    sleep 1
    if [ $((i % 5)) -eq 0 ]; then
        echo "Attempt $i/30: Checking NATS on port 4222..."
    fi
done

if ! nc -z localhost 4222; then
    echo "❌ NATS server failed to start within 30 seconds"
    kill $NATS_PID 2>/dev/null || true
    exit 1
fi

# Start server
if [ -f "apps/server/target/release/server" ]; then
    start_component "server" "apps/server/target/release/server" "Server"
    SERVER_PID=$?
else
    echo "⚠️  Server binary not found, skipping..."
fi

# Start Qt client
if [ -f "apps/qt_client/build/qt_client" ]; then
    start_component "qt_client" "apps/qt_client/build/qt_client" "Qt Client"
    QT_PID=$?
else
    echo "⚠️  Qt client binary not found, skipping..."
fi

# Start agent
if [ -f "apps/agent/target/release/agent" ]; then
    start_component "agent" "apps/agent/target/release/agent" "Agent"
    AGENT_PID=$?
else
    echo "⚠️  Agent binary not found, skipping..."
fi

echo ""
echo "🎉 Mini MSP Agent started successfully!"
echo ""
echo "📊 Running components:"
echo "NATS:   localhost:4222 (monitoring: localhost:8222)"
if [ -n "$SERVER_PID" ]; then
    echo "Server:  Running (PID: $SERVER_PID)"
fi
if [ -n "$QT_PID" ]; then
    echo "Qt GUI: Running (PID: $QT_PID)"
fi
if [ -n "$AGENT_PID" ]; then
    echo "Agent:  Running (PID: $AGENT_PID)"
fi
echo ""

# Function to cleanup
cleanup() {
    echo ""
    echo "🛑 Stopping all components..."
    
    # Kill all processes from PID file
    if [ -f "/tmp/mini_msp_pids.txt" ]; then
        while read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
                echo "✅ Process $pid stopped"
            fi
        done < "/tmp/mini_msp_pids.txt"
        rm -f "/tmp/mini_msp_pids.txt"
    fi
    
    # Kill NATS server
    if [ -f "/tmp/nats.pid" ]; then
        NATS_PID=$(cat "/tmp/nats.pid")
        if kill -0 "$NATS_PID" 2>/dev/null; then
            kill "$NATS_PID" 2>/dev/null || true
            echo "✅ NATS server stopped"
        fi
        rm -f "/tmp/nats.pid"
    fi
    
    echo "👋 All components stopped"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

echo "🔧 Press Ctrl+C to stop all components"

# Wait indefinitely
while true; do
    sleep 1
    
    # Check if critical components are still running
    if ! kill -0 $NATS_PID 2>/dev/null; then
        echo "❌ NATS server died unexpectedly"
        cleanup
        exit 1
    fi
    
    if [ -n "$AGENT_PID" ] && ! kill -0 $AGENT_PID 2>/dev/null; then
        echo "⚠️  Agent died unexpectedly"
    fi
done
