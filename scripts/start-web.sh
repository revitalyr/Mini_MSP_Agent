#!/bin/bash

echo "Starting Mini MSP Agent Web Stack..."

# Start NATS
echo "Starting NATS..."
docker-compose up -d nats

# Wait for NATS
sleep 3

# Start backend
echo "Starting web backend..."
docker-compose up -d web-backend

# Start frontend
echo "Starting web frontend..."
docker-compose up -d web-frontend

echo "Web stack started!"
echo "Web interface: http://localhost:80"
echo "API: http://localhost:3000"
echo "NATS monitoring: http://localhost:8222"
