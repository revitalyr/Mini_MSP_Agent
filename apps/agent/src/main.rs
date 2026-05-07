use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, error, debug, warn, Level};
use uuid::Uuid;
use futures_util::stream::StreamExt;
use std::sync::{Arc, Mutex}; // Keep for plugin_manager
use std::collections::HashMap; // Keep for plugin_manager

#[derive(Debug, serde::Deserialize)]
struct Config {
    #[allow(dead_code)]
    broker_url: Option<String>,
    #[allow(dead_code)]
    interval: Option<u64>,
    #[allow(dead_code)]
    agent_id: Option<String>,
    #[allow(dead_code)]
    log_level: Option<String>, // Keep for logging setup
    #[allow(dead_code)]
    log_dir: Option<String>, // Keep for logging setup
}

impl Default for Config {
    fn default() -> Self {
        Config {
            broker_url: Some("nats://localhost:4222".to_string()),
            interval: Some(30),
            agent_id: Some("unix-agent-001".to_string()),
            log_level: Some("info".to_string()),
            log_dir: Some("logs".to_string()),
        }
    }
}

fn load_config(path: &str) -> Config {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    // Fill in defaults for missing values
                    let default = Config::default();
                    config.broker_url.get_or_insert_with(|| default.broker_url.unwrap());
                    config.interval.get_or_insert_with(|| default.interval.unwrap());
                    config.agent_id.get_or_insert_with(|| default.agent_id.unwrap());
                    config.log_level.get_or_insert_with(|| default.log_level.unwrap());
                    config.log_dir.get_or_insert_with(|| default.log_dir.unwrap());
                    config
                }
                Err(e) => {
                    warn!("Failed to parse config file: {}, using defaults", e);
                    Config::default()
                }
            }
        }
        Err(_) => {
            warn!("Config file not found: {}, using defaults", path);
            Config::default()
        }
    }
}

use mini_msp_shared::{AgentInfo, CommandResponse, Heartbeat, CommandRequest};

mod plugin_loader;

/// Get current platform name
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// Register agent with API server
async fn register_with_api(agent_info: &AgentInfo) -> Result<()> {
    let client = reqwest::Client::new();
    let registration_data = json!({
        "id": agent_info.id,
        "hostname": agent_info.hostname,
        "version": agent_info.version,
        "plugins": vec![
            "system_monitor",
            "network_scanner", 
            "process_manager",
            "file_watcher"
        ]
    });
    
    match client.post("http://localhost:5000/api/register")
        .header("Content-Type", "application/json")
        .json(&registration_data)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                info!("API registration successful");
                Ok(())
            } else {
                warn!("API registration failed: {}", response.status());
                Err(anyhow::anyhow!("API registration failed"))
            }
        }
        Err(e) => {
            warn!("API registration error: {}", e);
            Err(anyhow::anyhow!("API registration error: {}", e))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = "configs/agent.toml".to_string(); // Renamed config file for clarity
    let mut plugin_dir_arg: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--plugin-dir" => {
                if i + 1 < args.len() {
                    plugin_dir_arg = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    // Load config
    let mut config = load_config(&config_path);
    info!("Loaded config from: {}", config_path);

    // Initialize tracing with stdout logging (file logging disabled due to compatibility issues)
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stdout)
        .init();

    info!("Starting Simple Mini MSP Agent with trait-based connections");

    // Create agent info
    let agent_id = config.agent_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let agent_info = AgentInfo {
        id: agent_id,
        hostname: gethostname::gethostname().to_string_lossy().to_string(),
        platform: get_platform(),
        version: "0.1.0".to_string(), // Agent version
        last_seen: chrono::Utc::now().timestamp() as u64, // Use last_seen from shared AgentInfo
    };

    info!("Agent ID: {}", agent_info.id);
    info!("Hostname: {}", agent_info.hostname);

    // Load plugins - agent is pure orchestrator, all data comes from plugins
    let mut plugin_manager = plugin_loader::PluginManager::new();
    let plugin_dir = plugin_dir_arg
        .or_else(|| std::env::var("PLUGIN_DIR").ok())
        .unwrap_or_else(|| "./plugins".to_string());
    info!("Loading plugins from directory: {}", plugin_dir);
    if let Err(e) = plugin_manager.load_from_directory(&plugin_dir) {
        warn!("Failed to load plugins from {}: {}", plugin_dir, e);
    } else {
        info!("Loaded plugins: {:?}", plugin_manager.plugin_names());
    }
    
    // Log if no plugins available (agent is pure orchestrator)
    if !plugin_manager.has_plugins() {
        warn!("No plugins loaded - agent will have limited functionality");
    }
    
    let plugin_manager = std::sync::Arc::new(std::sync::Mutex::new(plugin_manager));

    // Connect to NATS (required)
    let broker_url = config.broker_url.unwrap_or_else(|| "nats://localhost:4222".to_string());
    info!("Connecting to NATS at {}...", broker_url);
    let nats_client: async_nats::Client = async_nats::connect(&broker_url).await?;
    info!("Connected to NATS successfully");

    // Send agent registration to API
    // The server will now discover agents via heartbeats, but API registration can still be useful for initial setup.
    // Also register with API server for real data
    if let Err(e) = register_with_api(&agent_info).await {
        error!("Failed to register with API: {}", e);
    } else {
        info!("Successfully registered with API server");
    }

    // Start heartbeat task - minimal, no hardcoded metrics
    let nats_heartbeat = nats_client.clone();
    let agent_id = agent_info.id.clone();
    let agent_hostname = agent_info.hostname.clone();
    let agent_platform = agent_info.platform.clone();
    let heartbeat_plugin_manager = plugin_manager.clone();
    let _heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Get metrics from plugins if available
            let metrics = {
                let pm = heartbeat_plugin_manager.lock().unwrap();
                match pm.route_command("get_metrics", None) {
                    Ok(data) => data,
                    Err(_) => json!({"source": "agent", "status": "no_plugins"})
                }
            };

            // Get plugin count
            let plugin_count = {
                let pm = heartbeat_plugin_manager.lock().unwrap();
                pm.plugin_names().len()
            };

            // Send NATS heartbeat - all data from plugins
            let heartbeat = json!({
                "agent_id": agent_id,
                "hostname": agent_hostname,
                "platform": agent_platform,
                "timestamp": chrono::Utc::now().timestamp(),
                "metrics": metrics,
                "plugin_count": plugin_count
            });

            if let Err(e) = nats_heartbeat.publish("agent.heartbeat", heartbeat.to_string().into()).await {
                error!("Failed to send heartbeat: {}", e);
            } else {
                debug!("NATS heartbeat sent");
            }
        }
    });

    // Subscribe to commands via NATS
    let nats_commands = nats_client.clone();
    let agent_id_for_commands = agent_info.id.clone();
    let nats_plugin_manager = plugin_manager.clone();
    let _command_task = tokio::spawn(async move {
        let topic = format!("agent.{}.commands", agent_id_for_commands);
        info!("Subscribing to NATS command topic: {}", topic);
        
        let mut subscriber: async_nats::Subscriber = match nats_commands.subscribe(topic.clone()).await {
            Ok(sub) => sub,
            Err(e) => {
                error!("Failed to subscribe to commands topic: {}", e);
                return;
            }
        };
        
        info!("Successfully subscribed to command topic: {}", topic);
        
        while let Some(msg) = subscriber.next().await {
            if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                info!("Received command via NATS: {}", payload);
                
                if let Ok(cmd_value) = serde_json::from_str::<Value>(payload) {
                    if let Some(command) = cmd_value.get("command").and_then(|v| v.as_str()) {
                        // Route command to plugins - agent is pure orchestrator
                        let params = cmd_value.get("params");
                        let response = {
                            let pm = nats_plugin_manager.lock().unwrap();
                            match pm.route_command(command, params) {
                                Ok(plugin_response) => {
                                    // Wrap plugin response
                                    json!({
                                        "type": "command_response",
                                        "command": command,
                                        "success": true,
                                        "data": plugin_response,
                                        "timestamp": chrono::Utc::now()
                                    })
                                }
                                Err(e) => {
                                    // No plugin available or plugin error
                                    json!({
                                        "type": "command_response",
                                        "command": command,
                                        "success": false,
                                        "error": format!("Plugin error: {}", e),
                                        "timestamp": chrono::Utc::now()
                                    })
                                }
                            }
                        };

                        // Publish response back via NATS
                        let response_topic = format!("agent.{}.responses", agent_id_for_commands);
                        let payload_bytes = serde_json::to_vec(&response).unwrap_or_default();
                        
                        // Compression logic: use Zstd for payloads > 1024 bytes
                        if payload_bytes.len() > 1024 {
                            if let Ok(compressed) = zstd::encode_all(&payload_bytes[..], 3) {
                                let compressed_len = compressed.len();
                                let mut headers = async_nats::HeaderMap::new();
                                headers.insert("Content-Encoding", "zstd");
                                
                                if let Err(e) = nats_commands.publish_with_headers(
                                    response_topic.clone(), 
                                    headers, 
                                    compressed.into()
                                ).await {
                                    error!("Failed to publish compressed response: {}", e);
                                } else {
                                    info!("Compressed command response published ({} -> {} bytes)", 
                                          payload_bytes.len(), compressed_len);
                                }
                                continue; // Skip regular publish
                            }
                        }

                        if let Err(e) = nats_commands.publish(response_topic.clone(), payload_bytes.into()).await {
                            error!("Failed to publish response: {}", e);
                        } else {
                            info!("Command response published to {}", response_topic);
                        }
                    }
                }
            }
        }
    });

    // Keep agent running and wait for Ctrl+C
    info!("Agent is running (NATS-only mode). Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, agent shutting down...");
    
    Ok(())
}
