#!/bin/bash

# Development script for Mini MSP Agent

set -e

echo "🚀 Starting Mini MSP Agent Development Environment"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
if ! command_exists cargo; then
    echo "❌ Rust/Cargo not found. Please install Rust first."
    exit 1
fi

if ! command_exists docker-compose; then
    echo "❌ Docker Compose not found. Please install Docker Compose first."
    exit 1
fi

# Build the project
echo "📦 Building project..."
cargo build

# Start server in background
echo "🖥️  Starting server..."
cargo run --bin mini_msp_server -- --port 8080 &
SERVER_PID=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 3

# Check if server is running
if curl -s http://localhost:8080/health > /dev/null; then
    echo "✅ Server is running on http://localhost:8080"
else
    echo "❌ Server failed to start"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

# Start agent
echo "🤖 Starting agent..."
cargo run --bin mini_msp_agent -- --config configs/config.toml &
AGENT_PID=$!

echo "✅ Both server and agent are running!"
echo "📊 Server: http://localhost:8080"
echo "📋 Agents list: http://localhost:8080/agents"
echo "🔧 Press Ctrl+C to stop both services"

# Wait for Ctrl+C
trap 'echo "🛑 Stopping services..."; kill $SERVER_PID $AGENT_PID 2>/dev/null; exit' INT

wait
