# Mini MSP Agent - Project Structure

## Summary

Cross-platform monitoring agent with C++ plugins and Rust core.

## Folder Structure

```
Mini_MSP_Agent/
├── crates/                    # All Rust crates (flatter organization)
│   ├── shared/               # Common types and utilities
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs        # Core types: AgentConfig, EventMessage, etc.
│   │   │   ├── os/           # OS-specific modules
│   │   │   │   ├── windows.rs
│   │   │   │   ├── linux.rs
│   │   │   │   └── macos.rs
│   │   │   └── common.rs     # Cross-platform utilities
│   ├── agent/                # Agent application
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs       # Entry point
│   │   │   ├── lib.rs        # Agent logic
│   │   │   └── os/           # OS-specific agent code
│   ├── server/               # Server application
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── api/          # API modules
│   │   │   ├── plugin_loader.rs
│   │   │   └── os/           # OS-specific server code
│   └── qt_client/            # Qt client (if keeping C++)
│       ├── Cargo.toml
│       ├── src/
│       └── CMakeLists.txt
├── plugins/                  # C++ plugins (organized by OS)
│   ├── cpp/
│   │   ├── CMakeLists.txt
│   │   ├── CMakePresets.json
│   │   ├── common/           # Shared C++ headers/utilities
│   │   │   ├── plugin_interface.h
│   │   │   └── semantic_types.h
│   │   ├── windows/          # Windows-specific plugins
│   │   │   ├── system_plugin.cpp
│   │   │   ├── forensic_plugin.cpp
│   │   │   └── CMakeLists.txt
│   │   ├── linux/            # Linux-specific plugins
│   │   │   ├── system_plugin.cpp
│   │   │   └── CMakeLists.txt
│   │   └── macos/            # macOS-specific plugins
│   │       ├── system_plugin.cpp
│   │       └── CMakeLists.txt
├── docs/                     # Documentation
├── scripts/                  # Build and utility scripts
├── configs/                  # Configuration files
├── Cargo.toml                # Workspace manifest (members: "crates/*")
└── README.md
```

## Build Status

| Component | Status | Notes |
|-----------|--------|-------|
| `simple_agent` | ✅ Ready | WebSocket-based, builds successfully |
| `server` | ✅ Ready | Full-featured, builds successfully |
| `shared` | ✅ Ready | Core types defined |
| `agent-standalone` | ⚠️ Excluded | Struct mismatches with `shared` - needs sync |
| `plugins` (C++) | ✅ Ready | Cross-platform via CMakePresets.json |

## Naming Conventions

### C++ (C++23)
- Types: `PascalCase`
- Functions: `camelCase`
- Data members: `m_snake_case`
- Static members: `s_snake_case`
- Global variables: `g_snake_case`
- Constants: `kPascalCase`
- Macros: `SCREAMING_SNAKE_CASE`

### Rust
- Follows standard Rust conventions
- Constants: `SCREAMING_SNAKE_CASE`
- Types: `PascalCase`
- Functions/variables: `snake_case`

## Next Steps for Agent-Standalone

The `src/agent/standalone` crate expects these types from `shared`:
- `AgentConfig` with nested: `agent`, `broker`, `logging`, `plugins`
- `EventMessage` with fields: `id`, `source`, `event_type`, `data`, `timestamp`
- `PluginRegistry` as HashMap wrapper
- `PluginInfo` with: `name`, `version`, `description`, `author`, `status`, `loaded_at`, `last_error`
- `AgentInfo` with: `id`, `hostname`, `version`, `platform`, `architecture`, `start_time`
- `CommandRequest` struct

## Build Commands

```bash
# Rust workspace
cargo build --release

# C++ plugins (macOS example)
cmake --preset macos-debug
 cmake --build --preset macos-debug
```
