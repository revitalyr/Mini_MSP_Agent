#!/bin/bash
set -e

echo "Building web interface..."

# Build backend
echo "Building backend..."
cd backend
cargo build --release
cd ..

# Build frontend
echo "Building frontend..."
cd web-interface
npm install
npm run build
cd ..

# Copy frontend to backend
echo "Copying frontend to backend..."
cp -r web-interface/dist backend/dist/

echo "Build complete!"
echo "Run with: docker-compose up"
