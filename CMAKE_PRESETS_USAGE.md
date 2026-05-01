# CMakePresets Usage Guide

This project includes comprehensive CMakePresets.json configuration for cross-platform development supporting MSVC (Windows), GCC (Linux), and Clang (macOS/Linux).

## Available Presets

### Configure Presets
- **msvc-debug**: MSVC Debug configuration with code analysis enabled
- **msvc-release**: MSVC Release configuration optimized for production
- **gcc-debug**: GCC Debug configuration with sanitizers enabled
- **gcc-release**: GCC Release configuration optimized for production
- **clang-debug**: Clang Debug configuration for macOS with sanitizers
- **clang-release**: Clang Release configuration for macOS optimized for production
- **linux-clang-debug**: Clang Debug configuration for Linux with sanitizers
- **linux-clang-release**: Clang Release configuration for Linux optimized for production

### Build Presets
Corresponding build presets exist for each configure preset with the suffix `-build`.

### Test Presets
Test presets are available for debug configurations:
- **msvc-debug-test**
- **gcc-debug-test**
- **clang-debug-test**
- **linux-clang-debug-test**

### Package Presets
Package presets are available for release configurations:
- **msvc-release-package**
- **gcc-release-package**
- **clang-release-package**
- **linux-clang-release-package**

### Workflow Presets
CI/CD workflow presets for automated builds:
- **ci-msvc**: Complete MSVC build, test, and package workflow
- **ci-linux**: Complete Linux build (GCC + Clang), test, and package workflow
- **ci-macos**: Complete macOS build, test, and package workflow

## Usage Examples

### Configure and Build (Windows/MSVC)
```bash
# Configure debug build
cmake --preset msvc-debug

# Build debug configuration
cmake --build --preset msvc-debug-build

# Run tests
ctest --preset msvc-debug-test

# Configure and build release
cmake --preset msvc-release
cmake --build --preset msvc-release-build

# Create package
cpack --preset msvc-release-package
```

### Configure and Build (Linux/GCC)
```bash
# Configure debug build with sanitizers
cmake --preset gcc-debug

# Build debug configuration
cmake --build --preset gcc-debug-build

# Run tests
ctest --preset gcc-debug-test

# Configure and build release
cmake --preset gcc-release
cmake --build --preset gcc-release-build

# Create package
cpack --preset gcc-release-package
```

### Configure and Build (macOS/Clang)
```bash
# Configure debug build with sanitizers
cmake --preset clang-debug

# Build debug configuration
cmake --build --preset clang-debug-build

# Run tests
ctest --preset clang-debug-test

# Configure and build release
cmake --preset clang-release
cmake --build --preset clang-release-build

# Create package
cpack --preset clang-release-package
```

### CI/CD Workflows
```bash
# Run complete MSVC CI workflow
cmake --workflow ci-msvc

# Run complete Linux CI workflow
cmake --workflow ci-linux

# Run complete macOS CI workflow
cmake --workflow ci-macos
```

## Features

### Cross-Platform Support
- **Windows**: MSVC 2022 with Visual Studio generator
- **Linux**: GCC and Clang support with Ninja generator
- **macOS**: Clang support with Ninja generator

### Modern C++23 Support
- All presets configured for C++23 standard
- Compiler-specific optimizations enabled
- Modern CMake practices (3.25+)

### Development Features
- **Debug Configurations**: Include testing, sanitizers (GCC/Clang), and code analysis (MSVC)
- **Release Configurations**: Optimized for production with O3 optimization
- **Testing**: Integrated CTest support for all debug configurations
- **Packaging**: CPack support for creating distributable packages

### Build Artifacts
- Build directories: `build/{preset-name}`
- Install directories: `out/{preset-name}`
- Package formats: ZIP and TGZ

## Requirements

- **CMake**: 3.25 or higher
- **Windows**: Visual Studio 2022 with C++23 support
- **Linux**: GCC 13+ or Clang 16+ with C++23 support
- **macOS**: Xcode 15+ (Clang 16+) with C++23 support
- **Ninja**: Required for Unix-like systems (Linux/macOS)

## Customization

The toolchain files are located in `cmake/toolchains/`:
- `msvc.cmake`: MSVC-specific configuration
- `gcc.cmake`: GCC-specific configuration  
- `clang.cmake`: Clang-specific configuration

You can modify these files to adjust compiler flags, definitions, or other toolchain-specific settings.
