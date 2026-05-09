# Windows Development Setup Guide

## Prerequisites

### 1. Visual Studio 2022

Install Visual Studio 2022 with C++ development tools:

```powershell
# Option 1: Download from Microsoft
# https://visualstudio.microsoft.com/downloads/

# Option 2: Use Visual Studio Installer
# Run as Administrator and select:
# - Desktop development with C++
# - Windows 10/11 SDK
# - CMake tools
```

### 2. CMake

Install CMake 3.20+:

```powershell
# Option 1: Download from cmake.org
# https://cmake.org/download/

# Option 2: Use Chocolatey
choco install cmake

# Option 3: Use winget
winget install Kitware.CMake
```

### 3. Git

```powershell
# Option 1: Download from git-scm.com
# https://git-scm.com/download/win

# Option 2: Use Chocolatey
choco install git

# Option 3: Use winget
winget install Git.Git
```

## Environment Setup

### Developer Command Prompt

Always run build commands from **Developer Command Prompt**:

```cmd
# Start Developer Command Prompt for VS 2022
"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe" /useenv

# Or use the start menu shortcut:
# "Developer Command Prompt for VS 2022"
```

### Environment Variables

Set these permanently or in your shell:

```cmd
# Visual Studio C++ compiler
set VCToolsInstallDir=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.38.33130
set PATH=%VCToolsInstallDir%\bin\Hostx64\x64;%PATH%

# CMake
set PATH=%PATH%;C:\Program Files\CMake\bin

# Git
set PATH=%PATH%;C:\Program Files\Git\cmd
```

## Build Commands

### Full Build

```powershell
# From Developer Command Prompt
.\scripts\build-all.ps1

# Build specific components
.\scripts\build-all.ps1 -Components shared,agent,server
```

### Manual Build

```powershell
# Shared libraries
cargo build --release --manifest-path shared/Cargo.toml

# Agent
cargo build --release --manifest-path apps/agent/Cargo.toml

# Server
cargo build --release --manifest-path apps/server/Cargo.toml

# C++ Plugins (from Developer Command Prompt)
cd plugins
mkdir build
cd build
cmake .. -G "Visual Studio 17 2022" -A x64
cmake --build . --config Release

# Qt Client (requires additional setup)
cd apps/qt_client
mkdir build
cd build
cmake .. -G "Visual Studio 17 2022" -A x64
cmake --build . --config Release
```

## Troubleshooting

### Common Issues

1. **CMAKE_CXX_COMPILER not set**
   ```cmd
   # Solution: Use Developer Command Prompt
   # Or set manually:
   set CMAKE_CXX_COMPILER=cl.exe
   ```

2. **Ninja generator error**
   ```cmd
   # Solution: Use Visual Studio generator
   cmake .. -G "Visual Studio 17 2022" -A x64
   ```

3. **cl.exe not found**
   ```cmd
   # Solution: Run from Developer Command Prompt
   # Or add to PATH:
   set PATH=%PATH%;C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.38.33130\bin\Hostx64\x64
   ```

4. **Build fails with MSB8022**
   ```cmd
   # Solution: Install Windows SDK
   # Run Visual Studio Installer and modify:
   # - Windows 10/11 SDK (latest)
   # - C++ tools (latest)
   ```

## Alternative Setup

### Using vcpkg

```cmd
# Install vcpkg
git clone https://github.com/Microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat

# Install dependencies
.\vcpkg install cmake openssl qt6-base qt6-widgets qt6-gui

# Use vcpkg toolchain
cmake .. -DCMAKE_TOOLCHAIN_FILE=C:\vcpkg\scripts\buildsystems\vcpkg.cmake
```

### Using Chocolatey

```powershell
# Install all dependencies
choco install visualstudio2022community cmake git openssl

# Refresh environment
refreshenv
```

## Verification

Test your setup:

```cmd
# Test compiler
cl.exe

# Test CMake
cmake --version

# Test Git
git --version

# Test build
.\scripts\build-all.ps1 -Components shared
```

## Project Structure

```
Mini_MSP_Agent/
├── apps/
│   ├── agent/          # Rust agent
│   ├── server/         # Rust server
│   └── qt_client/       # Qt GUI client
├── plugins/            # C++ plugins
├── shared/             # Rust shared libraries
├── scripts/            # Build and start scripts
└── docs/              # Documentation
```

## Next Steps

After setup is complete:

1. Build all components: `.\scripts\build-all.ps1`
2. Start the system: `.\scripts\start-all.ps1`
3. Check logs in `logs/` directory
4. Access web interface at `http://localhost:8081`

## Support

- **Visual Studio**: https://docs.microsoft.com/en-us/visualstudio/
- **CMake**: https://cmake.org/documentation/
- **Qt**: https://doc.qt.io/qt-6/
- **Project Issues**: Check GitHub Issues
