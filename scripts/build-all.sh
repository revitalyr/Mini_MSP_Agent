#!/bin/bash
set -e

echo "🚀 Building Mini MSP Agent for all platforms..."

# Function to build component
build_component() {
    local component=$1
    echo "📦 Building $component..."
    
    case $component in
        "agent")
            cargo build --release --manifest-path apps/agent/Cargo.toml
            ;;
        "server")
            cargo build --release --manifest-path apps/server/Cargo.toml
            ;;
        "qt_client")
            echo "🔧 Building Qt Client..."
            cd apps/qt_client
            mkdir -p build
            cd build
            cmake .. -DCMAKE_BUILD_TYPE=Release
            make -j$(nproc)
            cd ../../..
            ;;
        "shared")
            cargo build --release --manifest-path shared/Cargo.toml
            ;;
        "plugins")
            echo "🔧 Building C++ plugins..."
            mkdir -p plugins/build
            cd plugins
            cmake -S . -B build -A x64
            cmake --build build --config Release
            cd ..
            ;;
        *)
            echo "❌ Unknown component: $component"
            exit 1
            ;;
    esac
    
    if [ $? -eq 0 ]; then
        echo "✅ $component built successfully"
    else
        echo "❌ $component build failed"
        exit 1
    fi
}

# Parse arguments
if [ $# -eq 0 ]; then
    echo "Building all components..."
    build_component "shared"
    build_component "plugins"
    build_component "agent"
    build_component "server"
    build_component "qt_client"
else
    for component in "$@"; do
        build_component "$component"
    done
fi

echo "🎉 Build completed!"
echo ""
echo "📋 Available binaries:"
echo "Agent:   apps/agent/target/release/agent"
echo "Server:  apps/server/target/release/server"
echo "Qt Client: apps/qt_client/build/qt_client"
echo "Plugins:  plugins/build/Release/"
echo ""
echo "🚀 Run with: ./scripts/start.sh"
