# Mini MSP Agent - C++ Plugins

Cross-platform C++23 plugins for system monitoring and forensic artifact collection.

## Overview

This directory contains the C++ plugin system that provides:
- **SystemPluginV3** - Base system metrics (CPU, memory, processes)
- **ForensicPlugin** - Platform-specific forensic artifact collection

## Architecture

```
src/plugins/
├── include/              # Plugin interface headers
│   ├── plugin_interface.h      # Core FFI interface
│   ├── semantic_types.h        # Type definitions
│   └── ...
├── src/
│   ├── system_plugin_v3.cpp    # Base system plugin (all platforms)
│   ├── windows/
│   │   └── forensic_plugin.cpp # Windows forensics (registry, event logs)
│   ├── linux/
│   │   └── forensic_plugin.cpp # Linux forensics (/proc, systemd)
│   └── macos/
│       └── forensic_plugin.cpp # macOS forensics (LaunchAgents, kexts)
├── CMakeLists.txt        # Multi-platform build configuration
└── CMakePresets.json     # Platform presets (Windows/Linux/macOS)
```

## Build

### Requirements
- CMake 3.20+
- C++23 compatible compiler:
  - Windows: MSVC 2022+
  - Linux: GCC 13+ or Clang 17+
  - macOS: Xcode 15+ (Clang)

### Build Commands

```bash
# Windows
cmake --preset windows-release
cmake --build --preset windows-release

# Linux
cmake --preset linux-release
cmake --build --preset linux-release

# macOS (Intel)
cmake --preset macos-intel-release
cmake --build --preset macos-intel-release

# macOS (ARM64)
cmake --preset macos-arm-release
cmake --build --preset macos-arm-release
```

### Output
- `SystemPluginV3.dll/.so/.dylib` - Base plugin
- `ForensicPlugin.dll/.so/.dylib` - Forensic plugin (per platform)

## Plugin Interface

### Exported Functions

```cpp
extern "C" {
    // Plugin metadata
    const char* get_plugin_info();           // Returns "name:version:description"
    PluginInterface* get_plugin_interface(); // Returns function table
    
    // Lifecycle
    bool plugin_initialize();
    void plugin_cleanup();
}
```

### Function Table (PluginInterface)

| Function | Purpose |
|----------|---------|
| `get_plugin_info` | Plugin metadata |
| `init` | Initialize plugin |
| `cleanup` | Cleanup resources |
| `get_system_metrics` | CPU, RAM, disk usage |
| `get_processes` | Process enumeration |
| `get_system_info` | OS version, hostname |
| `execute_command` | Run shell commands |
| `read_file` | Read file contents |
| `free_memory` | Free plugin-allocated memory |

## Platform Forensics

### Windows (ForensicPlugin.dll)
- Registry autorun keys (HKLM/HKCU)
- Running processes with handles
- AmCache.hve parsing
- Windows Event Logs

### Linux (ForensicPlugin.so)
- `/proc` filesystem parsing
- Kernel modules (`lsmod`)
- Systemd units
- Crontab entries

### macOS (ForensicPlugin.dylib)
- LaunchAgents/Daemons
- Kernel extensions
- Code signature checks
- SIP status

## FFI Integration

Rust server loads plugins via `libloading`:

```rust
// src/server/src/plugin_loader.rs
let plugin = PluginLoader::load()?;  // Auto-detects platform
let info = plugin.interface().get_plugin_info()?;
```

## License

MIT - See project root LICENSE
