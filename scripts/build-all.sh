#!/bin/bash
set -e

echo "🚀 Building Mini MSP Agent for all platforms..."

# Function to build component
build_component() {
    local component=$1
    echo "📦 Building $component..."
    
    case $component in
        "agent")
            cargo build --release --manifest-path crates/agent/Cargo.toml
            ;;
        "server")
            cargo build --release --manifest-path crates/server/Cargo.toml
            ;;
        "qt_client")
            echo "🔧 Building Qt Client..."
            cd crates/qt_client
            mkdir -p build
            cd build
            cmake .. -DCMAKE_BUILD_TYPE=Release
            make -j$(nproc)
            cd ../../..
            ;;
        "shared")
            cargo build --release --manifest-path crates/shared/Cargo.toml
            ;;
        "plugins")
            echo "🔧 Building C++ plugins..."
            cd plugins/cpp
            mkdir -p build
            cd build
            cmake ..
            make -j$(nproc)
            cd ../..
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
echo "Agent:   target/release/agent"
echo "Server:  target/release/server"
echo "Qt Client: crates/qt_client/build/qt_client"
echo "Plugins:  plugins/cpp/build/"
echo ""
echo "🚀 Run with: ./scripts/start-all.sh"
