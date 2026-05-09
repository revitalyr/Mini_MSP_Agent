# Qt Client Setup Guide

## Windows Dependencies

### Prerequisites

1. **Visual Studio 2022** (Community/Professional/Enterprise)
   - Install with C++ development tools
   - Required for CMake compiler

2. **Qt6** (6.5+)
   ```bash
   # Option 1: Download installer from Qt website
   # https://www.qt.io/download-qt-installer
   
   # Option 2: Use vcpkg
   vcpkg install qt6-base qt6-widgets qt6-gui
   ```

3. **OpenSSL** (for NATS library)
   ```bash
   # Option 1: Install with vcpkg
   vcpkg install openssl
   
   # Option 2: Download pre-built binaries
   # https://slproweb.com/products/Win32OpenSSL.html
   
   # Option 3: Use Chocolatey
   choco install openssl
   ```

4. **CMake** (3.20+)
   ```bash
   # Option 1: Download from cmake.org
   # https://cmake.org/download/
   
   # Option 2: Use Chocolatey
   choco install cmake
   ```

## Environment Variables

Set these environment variables:

```cmd
# Qt6
set QT_DIR=C:\Qt\6.5.0\msvc2022_64\lib\cmake\Qt6

# OpenSSL (if not in PATH)
set OPENSSL_ROOT_DIR=C:\OpenSSL-Win64
set OPENSSL_ROOT=C:\OpenSSL-Win64

# CMake (if not in PATH)
set PATH=%PATH%;C:\CMake\bin
```

## Build Commands

### Full Build
```powershell
# Build all components
.\scripts\build-all.ps1

# Build specific components
.\scripts\build-all.ps1 -Components shared,agent,server
```

### Qt Client Only
```powershell
# Build only Qt client
cd apps\qt_client
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

## Troubleshooting

### Common Issues

1. **CMAKE_CXX_COMPILER not set**
   - Install Visual Studio 2022 with C++ tools
   - Run from Developer Command Prompt

2. **Could NOT find OpenSSL**
   - Install OpenSSL or set OPENSSL_ROOT_DIR
   - Use vcpkg: `vcpkg install openssl`

3. **Qt6 not found**
   - Install Qt6 or set QT_DIR
   - Use Qt Maintenance Tool

4. **Ninja generator error**
   - Use Visual Studio generator: `-G "Visual Studio 17 2022"`

### Alternative Setup with vcpkg

```bash
# Install all dependencies with vcpkg
vcpkg install qt6-base qt6-widgets qt6-gui openssl nats.c

# Configure CMake with vcpkg
cmake .. -DCMAKE_TOOLCHAIN_FILE=C:\vcpkg\scripts\buildsystems\vcpkg.cmake
```

## Linux Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install qt6-base-dev qt6-widgets-dev qt6-gui-dev cmake libssl-dev

# Fedora/RHEL
sudo dnf install qt6-qtbase-devel qt6-qtwidgets-devel qt6-qtgui-devel cmake openssl-devel

# Arch
sudo pacman -S qt6-base qt6-widgets qt6-gui cmake openssl
```

## macOS Dependencies

```bash
# Install with Homebrew
brew install qt6 cmake openssl

# Set environment
export Qt6_DIR=/usr/local/opt/qt6/lib/cmake/Qt6
export OPENSSL_ROOT_DIR=/usr/local/opt/openssl
```

## Running the System

After successful build:

```powershell
# Start all components
.\scripts\start-all.ps1

# Start individual components
.\scripts\start-all.ps1 -Components agent,server
```

## Architecture

The Qt client integrates with:
- **NATS** for messaging
- **Shared libraries** for common types
- **C++ plugins** for extensibility
- **Rust backend** for core functionality
