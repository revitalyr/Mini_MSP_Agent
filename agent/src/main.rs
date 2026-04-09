//! Enhanced Mini MSP Agent with modern plugin architecture
//! 
//! Features:
//! - Platform-specific plugin loading
//! - Hot-reload support
//! - Enhanced security with sandboxing
//! - Optimized binary size
//! - Improved error handling

use anyhow::Result;
use clap::{Arg, Command};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// Import modules
mod broker;
mod commands;
mod network;
mod telemetry;
pub mod security;
mod plugin_loader;

// Import types
use crate::network::{HttpClient, WebSocketClient};
use crate::security::SecurityPolicy;
use crate::plugin_loader::EnhancedPluginManager;
use plugins::PluginEventType;
use broker::{BrokerClient, BrokerLoop, BrokerDeps};
use config::Config;

/// Get platform-specific plugin directory
fn get_platform_plugin_dir(base_dir: &str) -> String {
    let base_path = Path::new(base_dir);
    
    #[cfg(target_os = "windows")]
    {
        base_path.join("windows").to_string_lossy().to_string()
    }
    
    #[cfg(target_os = "linux")]
    {
        base_path.join("linux").to_string_lossy().to_string()
    }
    
    #[cfg(target_os = "macos")]
    {
        base_path.join("macos").to_string_lossy().to_string()
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // Fallback to base directory for unsupported platforms
        base_dir.to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("mini-msp-agent")
        .version("0.1.0")
        .about("Enhanced Mini MSP Agent with modern plugin architecture")
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
                .default_value("plugins"),
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
                .help("Run as daemon")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Config::load(config_path)?;

    // Initialize logging with level from config and file logging
    let log_level = config.log_level.as_str(); // "debug" | "info" | "warn" | "error"
    
    // File logging with daily rotation in configurable directory
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, "agent.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()           // RUST_LOG has priority
                .unwrap_or_else(|_| EnvFilter::new(log_level))
        )
        .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking))  // file
        .with(tracing_subscriber::fmt::layer().json())                            // stdout
        .init();

    info!("Starting Enhanced Mini MSP Agent v{}", env!("CARGO_PKG_VERSION"));
    info!("Loaded configuration for agent: {}", config.agent_id);

    // Initialize enhanced plugin manager
    let plugin_dir = matches.get_one::<String>("plugin-dir").unwrap();
    let platform_plugin_dir = get_platform_plugin_dir(&plugin_dir);
    let hot_reload_enabled = matches.get_flag("hot-reload");
    
    info!("Loading plugins from platform-specific directory: {}", platform_plugin_dir);
    info!("Hot-reload enabled: {}", hot_reload_enabled);
    
    let mut plugin_manager = EnhancedPluginManager::new(
        PathBuf::from(&platform_plugin_dir),
        hot_reload_enabled,
        !config.disable_signature_check,
    );
    
    // Load all plugins
    let loaded_count = plugin_manager.load_all_plugins().await?;
    info!("Successfully loaded {} plugins", loaded_count);
    
    // Check if any plugins are loaded
    if plugin_manager.plugin_count().await == 0 {
        warn!("No plugins loaded! Agent will run with limited functionality.");
    } else {
        info!("Agent running with {} loaded plugins", plugin_manager.plugin_count().await);
    }
    
    // List loaded plugins
    let loaded_plugins = plugin_manager.list_plugins().await;
    info!("Loaded {} plugins:", loaded_plugins.len());
    for plugin_info in loaded_plugins {
        info!("  - {} v{} (loaded at {:?})", 
              plugin_info.name, plugin_info.version, plugin_info.loaded_at);
    }
    
    // Initialize broker client
    let broker_client = BrokerClient::connect(&config.broker_url).await?;
    
    // Enable hot-reload if requested
    if hot_reload_enabled {
        info!("Plugin hot-reload enabled");
        // TODO: Start hot-reload monitoring task
    }
    
    // Create broker dependencies
    let http_client = HttpClient::new(config.clone());
    let policy = SecurityPolicy::new(config.allowed_commands.clone(), config.max_file_size);
    let ws_client = WebSocketClient::new(config.clone(), Arc::new(plugin_manager.clone()), policy.clone());
    
    let broker_deps = BrokerDeps {
        telemetry: Arc::new(telemetry::TelemetryCollector::new()),
        command_timeout_secs: 30,
    };
    
    // Start broker loop
    if let Some(broker) = broker_client {
        info!("Starting broker-based communication");
        let broker_loop = BrokerLoop::new(Some(broker), config.agent_id.clone(), broker_deps);
        broker_loop.run().await?;
    } else {
        warn!("Broker not available, running in standalone mode");
        
        // Standalone mode - periodic metrics collection
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(config.interval));
        
        loop {
            interval.tick().await;
            
            // Collect metrics from all plugins
            match plugin_manager.collect_all_metrics().await {
                Ok(metrics) => {
                    info!("Collected metrics from {} plugins", metrics.len());
                    for (plugin_name, plugin_metrics) in metrics {
                        info!("  {}: {}", plugin_name, plugin_metrics);
                    }
                }
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                }
            }
        }
    }

    Ok(())
}
