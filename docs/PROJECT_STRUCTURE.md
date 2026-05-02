# Mini MSP Agent - Project Structure

## Summary

Cross-platform monitoring agent with C++23 plugins and Rust core.

## Directory Layout

```
Mini_MSP_Agent/
├── README.md                    # Project overview
├── Cargo.toml                   # Rust workspace manifest
├── Cargo.lock                   # Dependency lock
├── .gitignore
│
├── src/                         # Rust source code
│   ├── agent/                   # Agent implementations
│   │   ├── simple/             # WebSocket-only lightweight agent
│   │   ├── standalone/         # Full-featured agent with NATS broker
│   │   └── core/               # Shared agent core library
│   ├── server/                 # HTTP/WebSocket/NATS server
│   │   └── src/
│   │       ├── main.rs         # Server entry point
│   │       ├── ffi.rs          # FFI bindings for C++ plugins
│   │       └── ...
│   ├── shared/                 # Shared Rust types library
│   │   └── src/lib.rs          # AgentInfo, PluginRegistry, Command, etc.
│   └── plugins/                # C++23 plugins
│       ├── CMakeLists.txt
│       ├── CMakePresets.json   # Cross-platform build presets
│       ├── include/            # Plugin headers
│       └── src/
│           ├── system_plugin_v3.cpp      # Base system plugin
│           ├── windows/
│           │   └── forensic_plugin.cpp   # Windows forensics
│           ├── linux/
│           │   └── forensic_plugin.cpp   # Linux forensics
│           └── macos/
│               └── forensic_plugin.cpp   # macOS forensics
│
├── scripts/                     # Build & deploy scripts
│   ├── build/                  # Build scripts
│   │   ├── build.ps1           # Agent core build
│   │   ├── build.sh
│   │   ├── plugins.ps1         # C++ plugins build
│   │   └── build-web.sh        # Web frontend build
│   ├── deploy/                 # Deployment scripts
│   │   ├── deploy.ps1
│   │   └── deploy.sh
│   ├── install_systemd.sh      # Linux service install
│   ├── run_dev.sh              # Development runner
│   ├── start-web.sh            # Web server start
│   └── start.bat / start.ps1   # Windows starters
│
├── docs/                        # Documentation
│   ├── CLI_USAGE.md            # Command-line reference
│   ├── PROJECT_STRUCTURE.md    # This file
│   └── README-WEB.md           # Web frontend docs
│
├── web/                         # Web frontend (Vite + Tailwind)
│   ├── src/
│   ├── vite.config.ts
│   └── tailwind.config.js
│
├── tools/                       # External tools
│   └── nats-server/            # NATS messaging server
│
├── configs/                     # Configuration templates
├── tests/                       # Integration tests
└── workflows/                   # CI/CD workflows (optional)
```

## Component Overview

| Component | Language | Purpose |
|-----------|----------|---------|
| `agent-simple` | Rust | Lightweight WebSocket agent |
| `agent-standalone` | Rust | Full agent with NATS broker |
| `server` | Rust | HTTP/WebSocket/NATS hub |
| `shared` | Rust | Common types & traits |
| `SystemPluginV3` | C++23 | Base system metrics plugin |
| `ForensicPlugin` | C++23 | Platform-specific forensics |

## Build Commands

```bash
# Rust components
cargo build --release

# C++ plugins (cross-platform via presets)
cmake --preset windows-release
cmake --build --preset windows-release
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
