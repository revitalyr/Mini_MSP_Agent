//! # Mini MSP Agent
//! 
//! A cross-platform system agent for MSP (Managed Service Provider) and fleet management.
//! This agent provides comprehensive system monitoring, plugin architecture, and real-time
//! communication with a central management server.
//! 
//! ## Features
//! 
//! - **Plugin Architecture**: Extensible C++ plugin system for custom functionality
//! - **Real-time Monitoring**: System metrics, process information, and resource usage
//! - **WebSocket Communication**: Persistent connection to management server
//! - **Hot-reload Support**: Dynamic plugin loading and unloading
//! - **Cross-platform**: Works on Linux, Windows, and macOS
//! 
//! ## Architecture
//! 
//! The agent consists of several key components:
//! 
//! - **Config Module**: Configuration management and validation
//! - **Telemetry Module**: System metrics collection and reporting
//! - **Network Module**: HTTP and WebSocket client communication
//! - **Commands Module**: Command execution and response handling
//! - **Plugins Module**: Dynamic plugin loading and management
//! 
//! ## Usage
//! 
//! ```bash
//! mini-msp-agent --config config.toml --plugin-dir ./plugins
//! ```
//! 
//! ## Configuration
//! 
//! The agent uses TOML configuration files for settings including:
//! - Server connection parameters
//! - Plugin directory paths
//! - Telemetry collection intervals
//! - Logging levels
//! 
//! ## Plugin Development
//! 
//! Plugins are developed as C++ shared libraries with a standardized interface.
//! See the `plugins` directory for examples and development guidelines.

use anyhow::Result;
use clap::{Arg, Command};
use mini_msp_shared::{AgentConfig, Heartbeat, Metrics};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::{interval, Instant};
use tracing::{error, info, warn};
use tracing_subscriber::{self, EnvFilter, prelude::*};

mod config;
mod telemetry;
mod network;
mod commands;
mod plugins;
mod broker;

use config::Config;
use telemetry::TelemetryCollector;
use network::{HttpClient, WebSocketClient};
use plugins::{PluginManager, PluginEventType};
use broker::{BrokerClient, BrokerLoop};

/// Main entry point for the Mini MSP Agent
/// 
/// This function initializes and starts the agent with the following steps:
/// 1. Parse command line arguments
/// 2. Initialize logging system
/// 3. Load configuration from file
/// 4. Initialize plugin manager
/// 5. Load plugins from specified directory
/// 6. Start telemetry collection
/// 7. Connect to management server via WebSocket
/// 8. Start main event loop with periodic heartbeats
/// 
/// # Arguments
/// 
/// * `config` - Path to configuration file (default: "config.toml")
/// * `plugin-dir` - Directory containing C++ plugins (default: "./plugins")
/// * `hot-reload` - Enable plugin hot-reload functionality
/// * `daemon` - Run as daemon/service
/// 
/// # Returns
/// 
/// Returns `Ok(())` on successful execution, `Err(e)` on any failure
/// 
/// # Example
/// 
/// ```bash
/// mini-msp-agent --config custom.toml --plugin-dir /opt/plugins --hot-reload
/// ```
#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("mini-msp-agent")
        .version("0.1.0")
        .about("Cross-platform system agent for MSP/fleet management")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("config.toml"),
        )
        .arg(
            Arg::new("plugin-dir")
                .short('p')
                .long("plugin-dir")
                .value_name("DIR")
                .help("Directory containing C++ plugins")
                .default_value("./plugins"),
        )
        .arg(
            Arg::new("hot-reload")
                .short('r')
                .long("hot-reload")
                .help("Enable plugin hot-reload")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("daemon")
                .short('d')
                .long("daemon")
                .help("Run as daemon/service")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Config::load(config_path)?;

    // Initialize logging с уровнем из конфига и записью в файл
    let log_level = config.log_level.as_str(); // "debug" | "info" | "warn" | "error"
    
    // Лог в файл с rotation по дням в настраиваемую директорию
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, "agent.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()           // RUST_LOG имеет приоритет
                .unwrap_or_else(|_| EnvFilter::new(log_level))
        )
        .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking))  // файл
        .with(tracing_subscriber::fmt::layer().json())                            // stdout
        .init();

    info!("Starting Mini MSP Agent with C++ plugin architecture and hot-reload support");
    info!("Loaded configuration for agent: {}", config.agent_id);

    // Initialize plugin manager and load plugins
    let mut plugin_manager = PluginManager::new();
    let plugin_dir = matches.get_one::<String>("plugin-dir").unwrap();
    let hot_reload_enabled = matches.get_flag("hot-reload");
    
    // Set up event callback for plugin events
    let _plugin_manager_clone = plugin_manager.clone();
    plugin_manager.set_event_callback(move |event_type, plugin_name, message| {
        match event_type {
            PluginEventType::Loaded => {
                info!("Plugin loaded: {} - {}", plugin_name, message);
            }
            PluginEventType::Unloaded => {
                info!("Plugin unloaded: {} - {}", plugin_name, message);
            }
            PluginEventType::Error => {
                error!("Plugin error: {} - {}", plugin_name, message);
            }
            PluginEventType::StatusChanged => {
                info!("Plugin status changed: {} - {}", plugin_name, message);
            }
        }
    });
    
    // Enable hot-reload if requested
    if hot_reload_enabled {
        plugin_manager.enable_hot_reload(true);
        info!("Plugin hot-reload enabled");
    }
    
    info!("Loading plugins from directory: {}", plugin_dir);
    plugin_manager.load_plugins_from_directory(plugin_dir)?;
    
    // Check if system plugin is loaded
    if !plugin_manager.is_system_plugin_loaded() {
        error!("No system plugin loaded! Agent cannot function without system plugin.");
        return Err(anyhow::anyhow!("System plugin required but not found"));
    }
    
    // List loaded plugins
    let loaded_plugins = plugin_manager.get_loaded_plugins();
    info!("Loaded {} plugins:", loaded_plugins.len());
    for plugin_name in loaded_plugins {
        let status = plugin_manager.get_plugin_status(&plugin_name);
        info!("  - {} ({:?})", plugin_name, status);
    }
    
    // Show plugin registry
    let registry = plugin_manager.get_plugin_registry();
    info!("Plugin registry:");
    for entry in registry {
        info!("  {} v{} ({}) - {:?}", entry.name, entry.version, entry.platform, entry.status);
    }

    // Initialize broker client
    let broker_client = BrokerClient::connect(&config.broker_url).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to broker: {}", e))?;

    // Initialize broker loop
    let broker_loop = BrokerLoop::new(broker_client, config.agent_id.clone(), plugin_manager.clone());

    info!("Starting agent with broker-based communication");

    // Run broker loop (this will handle commands and heartbeats)
    broker_loop.run.await?;

    Ok(())
}
