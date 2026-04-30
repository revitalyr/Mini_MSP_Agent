#!/bin/bash
set -e

echo "Building Mini MSP Agent Core..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in the agent core directory"
    exit 1
fi

# Clean previous build
echo "Cleaning previous build..."
cargo clean

# Build in release mode with optimizations
echo "Building core library..."
cargo build --release

# Get build information
echo "Build information:"
echo "  Target: $(rustc -vV | grep 'host:' | cut -d' ' -f2)"
echo "  Profile: release"
echo "  Optimizations: size-optimized"

# Check binary size
if [ -f "target/release/deps/libmini_msp_core.rlib" ]; then
    SIZE=$(du -h target/release/deps/libmini_msp_core.rlib | cut -f1)
    echo "  Core library size: $SIZE"
fi

if [ -f "../../target/release/agent" ]; then
    SIZE=$(du -h ../../target/release/agent | cut -f1)
    echo "  Agent binary size: $SIZE"
fi

echo "Build completed successfully!"
