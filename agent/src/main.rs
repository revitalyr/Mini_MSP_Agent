use anyhow::Result;
use clap::{Arg, Command};
use mini_msp_shared::{AgentConfig, Heartbeat, Metrics};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::{interval, Instant};
use tracing::{error, info, warn};
use tracing_subscriber;

mod config;
mod telemetry;
mod network;
mod commands;
mod plugins;

use config::Config;
use telemetry::TelemetryCollector;
use network::{HttpClient, WebSocketClient};
use plugins::{PluginManager, PluginEventType};

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

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .json()
        .init();

    info!("Starting Mini MSP Agent with C++ plugin architecture and hot-reload support");

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Config::load(config_path)?;
    
    info!("Loaded configuration for agent: {}", config.agent_id);

    // Initialize plugin manager and load plugins
    let mut plugin_manager = PluginManager::new();
    let plugin_dir = matches.get_one::<String>("plugin-dir").unwrap();
    let hot_reload_enabled = matches.get_flag("hot-reload");
    
    // Set up event callback for plugin events
    let plugin_manager_clone = plugin_manager.clone();
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

    // Initialize components
    let telemetry = TelemetryCollector::new(plugin_manager.clone());
    let http_client = HttpClient::new(config.clone());
    let ws_client = WebSocketClient::new(config.clone(), plugin_manager.clone());

    // Spawn telemetry and heartbeat task
    let config_clone = config.clone();
    let telemetry_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(config_clone.interval));
        
        loop {
            interval.tick().await;
            
            match telemetry.collect_metrics().await {
                Ok(metrics) => {
                    let heartbeat = Heartbeat {
                        agent_id: config_clone.agent_id.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                        metrics,
                        hostname: telemetry.get_hostname(),
                        uptime: telemetry.get_uptime(),
                    };

                    if let Err(e) = http_client.send_heartbeat(heartbeat).await {
                        error!("Failed to send heartbeat: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                }
            }
        }
    });

    // Spawn WebSocket control channel task
    let ws_task = tokio::spawn(async move {
        ws_client.run().await;
    });

    // Wait for tasks
    tokio::try_join!(telemetry_task, ws_task)?;

    Ok(())
}
