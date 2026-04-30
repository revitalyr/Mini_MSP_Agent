#!/bin/bash

# Build script for C++ plugins

set -e

PLUGIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PLUGIN_DIR/build"
AGENT_PLUGIN_DIR="$PLUGIN_DIR/../agent/plugins"

echo "🔨 Building Mini MSP Agent C++ plugins..."

# Create build directory
mkdir -p "$BUILD_DIR"
mkdir -p "$AGENT_PLUGIN_DIR"

# Configure and build
cd "$BUILD_DIR"

echo "📦 Configuring CMake..."
cmake .. -DCMAKE_BUILD_TYPE=Release

echo "🏗️  Building plugins..."
cmake --build . --config Release

# Copy plugins to agent directory
echo "📋 Copying plugins to agent directory..."

if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    # Windows
    if [ -f "Release/system_plugin.dll" ]; then
        cp "Release/system_plugin.dll" "$AGENT_PLUGIN_DIR/"
        echo "✅ Copied system_plugin.dll"
    fi
else
    # Linux/macOS
    if [ -f "libsystem_plugin.so" ]; then
        cp "libsystem_plugin.so" "$AGENT_PLUGIN_DIR/"
        echo "✅ Copied libsystem_plugin.so"
    elif [ -f "libsystem_plugin.dylib" ]; then
        cp "libsystem_plugin.dylib" "$AGENT_PLUGIN_DIR/"
        echo "✅ Copied libsystem_plugin.dylib"
    fi
fi

echo "🎉 Plugin build completed!"
echo "📁 Plugin location: $AGENT_PLUGIN_DIR"
