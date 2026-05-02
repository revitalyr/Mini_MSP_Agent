# Mini MSP Agent - Project Structure

## Summary

Cross-platform monitoring agent with C++ plugins and Rust core.

## Folder Structure

```
Mini_MSP_Agent/
├── src/
│   ├── agent/
│   │   ├── simple/          # Simple agent (WebSocket only)
│   │   └── standalone/      # Full agent (NATS broker) - TEMPORARILY EXCLUDED
│   ├── server/              # Rust server (NATS, HTTP, WebSocket)
│   ├── shared/              # Shared Rust library
│   │   └── src/
│   │       └── lib.rs       # Core types: Heartbeat, Metrics, Command, AgentConfig
│   └── plugins/             # C++23 plugins
│       ├── CMakeLists.txt
│       ├── CMakePresets.json  # MSVC, Linux GCC/Clang, macOS Intel/ARM64/Universal
│       └── src/
│           └── working_system_plugin.cpp
├── plugins/                 # C plugin headers (semantic_types.h, plugin_interface.h)
│   └── include/
├── shared/                  # C++ shared library (delegated to src/shared)
├── agent/                   # Legacy agent folder (delegated to src/agent)
├── server/                  # Legacy server folder (delegated to src/server)
├── Cargo.toml               # Workspace manifest
├── Cargo.lock               # Dependency lock
└── .gitignore
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
