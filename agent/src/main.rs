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
use tracing::{error, info, warn};
use tracing_subscriber::{self, EnvFilter, prelude::*};

mod config;
mod telemetry;
mod network;
mod plugins;
mod broker;
mod commands;
pub mod security;

use crate::network::{HttpClient, WebSocketClient};
use crate::security::SecurityPolicy;
use config::Config;
use telemetry::TelemetryCollector;
use std::sync::Arc;
use plugins::{PluginManager, PluginEventType};
use broker::{BrokerClient, BrokerLoop, PluginEventPublisher, BrokerDeps};

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
                .default_value("agent/plugins"),
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
    let mut plugin_manager = PluginManager::new()
        .with_signature_check(config.disable_signature_check);
    let plugin_dir = matches.get_one::<String>("plugin-dir").unwrap();
    let hot_reload_enabled = matches.get_flag("hot-reload");
    
    // Initialize broker client with encapsulated retry mechanism
    let broker_client = BrokerClient::connect(&config.broker_url).await?;

    // Set up combined event callback for local logging and NATS publishing
    let event_publisher = Arc::new(PluginEventPublisher::new(broker_client.clone(), config.agent_id.clone()));
    plugin_manager.set_event_callback(move |event_type, name, msg| {
        // 1. Local logging
        match event_type {
            PluginEventType::Loaded => info!("Plugin loaded: {} - {}", name, msg),
            PluginEventType::Unloaded => info!("Plugin unloaded: {} - {}", name, msg),
            PluginEventType::Error => error!("Plugin error: {} - {}", name, msg),
            PluginEventType::StatusChanged => info!("Plugin status changed: {} - {}", name, msg),
        }

        // 2. NATS publishing (async)
        let pub_inner = event_publisher.clone();
        let name_inner = name.to_string();
        let msg_inner = msg.to_string();
        let et_inner = event_type.clone();
        tokio::spawn(async move {
            let data = serde_json::json!({ 
                "event": format!("{:?}", et_inner), 
                "message": msg_inner 
            });
            let _ = pub_inner.publish_event(&name_inner, data).await;
        });
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
        warn!("No system plugin loaded! Agent will run with limited functionality.");
        // Don't return error, continue with limited functionality
    }
    
    // List loaded plugins
    let loaded_plugins = plugin_manager.get_loaded_plugins();
    info!("Loaded {} plugins:", loaded_plugins.len());
    for plugin_name in loaded_plugins {
        let status = plugin_manager.get_plugin_status(&plugin_name);
        info!("  - {} ({:?})", plugin_name, status);
    }
    
    // Load plugins and display status
    let registry = plugin_manager.list_plugins();
    info!("Loaded {} plugins:", registry.len());
    for entry in registry {
        info!("  {} v{} - {}", entry.name, entry.version, entry.description);
        
        // Get detailed registry entry to access library_path
        if let Ok(registry_entry) = plugin_manager.get_registry_entry(&entry.name) {
            info!("    📂 Library: {}", registry_entry.library_path);
            info!("    🕒 Last loaded: {:?}", registry_entry.last_loaded);
            info!("    🕒 Last unloaded: {:?}", registry_entry.last_unloaded);
        }
    }

    // Test plugin functionality
    if let Ok(system_info) = plugin_manager.get_system_info() {
        info!("✓ System info plugin available");
        // Convert SystemInfoData to SystemInfo to use get_system_summary
        let telemetry_system_info = telemetry::SystemInfo {
            os_type: system_info.os_type.clone(),
            os_version: system_info.os_version.clone(),
            hostname: system_info.hostname.clone(),
            uptime: system_info.uptime,
            cpu_cores: system_info.cpu_cores,
            total_memory: system_info.total_memory,
            available_memory: system_info.available_memory,
        };
        info!("  {}", telemetry_system_info.get_system_summary());
        info!("  Memory: {:.1}/{:.1} GB ({:.1}%)", 
               system_info.available_memory as f64 / 1024.0 / 1024.0 / 1024.0,
               system_info.total_memory as f64 / 1024.0 / 1024.0 / 1024.0,
               ((system_info.total_memory - system_info.available_memory) as f64 / system_info.total_memory as f64) * 100.0);
    } else {
        warn!("✗ System info plugin not available");
    }

    if let Ok(dir_info) = plugin_manager.get_directory_info_data(".", false, false, 10) {
        info!("✓ Directory info plugin available");
        info!("  {}", dir_info.get_summary());
        info!("  {}", dir_info.get_scan_details());
    } else {
        warn!("✗ Directory info plugin not available");
    }

    // Test additional plugin functionality
    if plugin_manager.is_plugin_loaded("modern_system_plugin") {
        info!("✓ System plugin is loaded");
        
        // Test individual plugin loader status
        if let Ok(plugin) = plugin_manager.get_plugin("modern_system_plugin") {
            info!("✓ Plugin loader status: loaded={}", plugin.is_loaded());
        }
        
        // Test sensor data
        let sensors = plugin_manager.get_sensor_history();
        info!("✓ Sensor data available: {} readings", sensors.len());
        
        // Use sensor data fields
        for sensor in sensors.iter().take(3) {
            info!("  {}", sensor.get_formatted());
        }
        
        // Test camera data
        if let Ok(camera_data) = plugin_manager.get_camera_data() {
            info!("✓ Camera data available");
            info!("  {}", camera_data.get_camera_info());
        } else {
            info!("✗ Camera data not available");
        }
        
        // Test processing results
        if let Ok(processing_results) = plugin_manager.get_processing_results() {
            info!("✓ Processing results available");
            info!("  {}", processing_results.get_processing_summary());
        } else {
            info!("✗ Processing results not available");
        }
        
        // Test command execution
        if let Ok(cmd_result) = plugin_manager.execute_command("echo test") {
            info!("Command execution available: {}", cmd_result.get_summary());
        }
        
        // Test async plugin loading
        info!("Testing async plugin loading...");
        let test_plugin_path = if cfg!(target_os = "windows") {
            "plugins/modern_system_plugin.dll"
        } else if cfg!(target_os = "macos") {
            "plugins/modern_system_plugin.dylib"
        } else {
            "plugins/modern_system_plugin.so"
        };
        if let Ok(_) = plugin_manager.load_plugin_async("test_async_plugin", test_plugin_path).await {
            info!("✓ Async plugin loading successful");
        }
        
        // Test graceful plugin unload
        if let Ok(_) = plugin_manager.unload_plugin_graceful("test_async_plugin") {
            info!("✓ Graceful plugin unload successful");
        }
        
        // Test plugin status management
        info!("Plugin status management:");
        for plugin_name in &["modern_system_plugin", "modern_directory_info_plugin"] {
            let status = plugin_manager.get_plugin_status(plugin_name);
            match status {
                plugins::manager::PluginStatus::Loading => {
                    info!("  🔄 {} is currently loading...", plugin_name);
                }
                plugins::manager::PluginStatus::Loaded => {
                    info!("  ✅ {} is loaded and ready", plugin_name);
                }
                plugins::manager::PluginStatus::Unloading => {
                    info!("  ⏹️ {} is unloading...", plugin_name);
                }
                plugins::manager::PluginStatus::Active => {
                    info!("  🚀 {} is active and processing", plugin_name);
                }
                plugins::manager::PluginStatus::Error => {
                    info!("  ❌ {} has errors", plugin_name);
                }
                plugins::manager::PluginStatus::Unloaded => {
                    info!("  ⭕ {} is unloaded", plugin_name);
                }
            }
        }
        
        // Test additional plugin methods
        if let Ok(event_data) = plugin_manager.get_event_data("/tmp") {
            info!("✓ Event data available");
            info!("  {}", event_data.get_event_summary());
            
            // Real-time event monitoring simulation
            if event_data.events_count > 0 {
                info!("  📁 Active file monitoring detected");
            }
        } else {
            info!("✗ Event data not available");
        }
        
        if let Ok(watchers_data) = plugin_manager.get_watchers_data() {
            info!("✓ Watchers data available");
            info!("  {}", watchers_data.get_watchers_summary());
            
            // File system watcher management
            if watchers_data.active_watchers > 0 {
                info!("  👁️  {} active directory watchers", watchers_data.active_watchers);
            }
        } else {
            info!("✗ Watchers data not available");
        }
        
        if let Ok(file_reader_data) = plugin_manager.get_file_reader_data("/tmp/test.txt") {
            info!("✓ File reader data available");
            info!("  {}", file_reader_data.get_file_info());
            info!("  Preview: {}", file_reader_data.get_content_preview());
            
            // Advanced file reading with encoding detection
            if file_reader_data.size > 0 {
                info!("  📄 File content successfully read with {} encoding", file_reader_data.encoding);
            }
        } else {
            info!("✗ File reader data not available");
        }
        
        if let Ok(video_frame) = plugin_manager.get_video_frame() {
            info!("✓ Video frame available");
            info!("  {}", video_frame.get_frame_info());
            
            // Video capture and processing
            if video_frame.data.len() > 0 {
                info!("  🎥 Video frame captured successfully");
                info!("  📐 Resolution: {}x{}", video_frame.width, video_frame.height);
            }
        } else {
            info!("✗ Video frame not available");
        }
    }

    // Start plugin lifecycle monitoring
    let plugin_manager_clone = plugin_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            
            // Monitor plugin statuses and health
            let registry = plugin_manager_clone.get_plugin_registry();
            for entry in registry {
                match entry.status {
                    plugins::manager::PluginStatus::Error => {
                        warn!("⚠️ Plugin '{}' in error state: {}", entry.name, entry.status_message);
                    }
                    plugins::manager::PluginStatus::Loading => {
                        info!("⏳ Plugin '{}' still loading...", entry.name);
                    }
                    plugins::manager::PluginStatus::Unloading => {
                        info!("⏹️ Plugin '{}' unloading...", entry.name);
                    }
                    _ => {}
                }
            }
        }
    });

    // Start sensor polling to fill the queue
    plugin_manager.start_sensor_polling(1000);

    // Initialize shared dependencies
    let telemetry = TelemetryCollector::new(plugin_manager.clone());
    let http_client = HttpClient::new(config.clone());
    let policy = SecurityPolicy::new(config.allowed_commands.clone(), config.max_file_size);

    let ws_client = WebSocketClient::new(config.clone(), plugin_manager.clone(), policy.clone()); // policy.clone() is fine here

    // Start WebSocket client in background
    let ws_client_clone = ws_client.clone();
    tokio::spawn(async move {
        ws_client_clone.run().await;
    });

    info!("Starting agent with broker-based communication");

    // Wrap broker loop in a retry mechanism to handle manual reconnects on terminal failure
    loop {
        let loop_broker_client = BrokerClient::connect(&config.broker_url).await?;
        let loop_deps = BrokerDeps {
            plugin_manager: plugin_manager.clone(),
            telemetry: telemetry.clone(),
            http_client: http_client.clone(),
            policy: policy.clone(),
        config: config.clone(),
            command_timeout_secs: config.command_timeout_secs,
        };
        let loop_instance = BrokerLoop::new(loop_broker_client, config.agent_id.clone(), loop_deps);
        
        if let Err(e) = loop_instance.run().await {
            error!("Broker loop encountered a terminal error: {}. Attempting full client restart...", e);
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        } else {
            break; // Clean shutdown
        }
    }

    Ok(())
}
