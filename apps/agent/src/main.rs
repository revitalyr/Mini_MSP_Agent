use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, error, debug, warn};
use uuid::Uuid;
use futures_util::stream::StreamExt;

mod websocket;
mod plugin_loader;

#[derive(Debug, serde::Deserialize)]
struct Config {
    ws_url: Option<String>,
    broker_url: Option<String>,
    #[allow(dead_code)]
    server_url: Option<String>,
    #[allow(dead_code)]
    interval: Option<u64>,
    #[allow(dead_code)]
    agent_id: Option<String>,
    #[allow(dead_code)]
    log_level: Option<String>,
    #[allow(dead_code)]
    log_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ws_url: Some("ws://localhost:8080/ws".to_string()),
            broker_url: Some("nats://localhost:4222".to_string()),
            server_url: Some("http://localhost:8080".to_string()),
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
                    if config.ws_url.is_none() {
                        config.ws_url = default.ws_url;
                    }
                    if config.broker_url.is_none() {
                        config.broker_url = default.broker_url;
                    }
                    config
                }
                Err(e) => {
                    warn!("Failed to parse config: {}, using defaults", e);
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AgentInfo {
    id: String,
    hostname: String,
    platform: String,
    version: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

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
    let mut config_path = "configs/config.toml".to_string();
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
    let config = load_config(&config_path);
    info!("Loaded config from: {}", config_path);

    // Initialize tracing with stdout logging (file logging disabled due to compatibility issues)
    tracing_subscriber::fmt()
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
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now(),
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
    let broker_url = config.broker_url.as_ref().map(|s| s.as_str()).unwrap_or("nats://localhost:4222");
    info!("Connecting to NATS at {}...", broker_url);
    let nats_client = async_nats::connect(broker_url).await?;
    info!("Connected to NATS successfully");

    // Connect to WebSocket using trait-based approach
    let ws_url = config.ws_url.as_ref().map(|s| s.as_str()).unwrap_or("ws://localhost:8080/ws");
    info!("Connecting to WebSocket at {}...", ws_url);
    let ws_result = websocket::WebSocketClient::connect(ws_url).await?;
    let mut ws_client = ws_result.client;
    
    info!("Connected via {} successfully", ws_client.connection_type());

    // Send agent registration to WebSocket and API
    let registration = json!({
        "type": "agent_register",
        "agent": agent_info
    });
    
    info!("Sending agent registration...");
    ws_client.send_json(&registration).await?;
    
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
        
        let mut subscriber = match nats_commands.subscribe(topic.clone()).await {
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
                        if let Err(e) = nats_commands.publish(response_topic.clone(), serde_json::to_vec(&response).unwrap_or_default().into()).await {
                            error!("Failed to publish response: {}", e);
                        } else {
                            info!("Command response published to {}", response_topic);
                        }
                    }
                }
            }
        }
    });

    // Handle WebSocket messages using trait-based connection
    info!("Starting message handling loop with trait-based connection...");

    // Run message handling in background
    let ws_client_ref = std::sync::Arc::new(tokio::sync::Mutex::new(ws_client));
    let ws_client_clone = ws_client_ref.clone();
    let ws_plugin_manager = plugin_manager.clone();

    let _message_task = tokio::spawn(async move {
        let mut client = ws_client_clone.lock().await;
        while let Some(parsed) = client.receive_json().await.unwrap_or(None) {
            debug!("Received message via {}: {:?}",
                   client.connection_type(), parsed);

            if let Some(msg_type) = parsed.get("type").and_then(|v: &serde_json::Value| v.as_str()) {
                match msg_type {
                    "command" => {
                        info!("Received command: {:?}", parsed);
                        if let Some(cmd) = parsed.get("command").and_then(|v: &serde_json::Value| v.as_str()) {
                            // Route command to plugins - agent is pure orchestrator
                            let params = parsed.get("params");
                            let response = {
                                let pm = ws_plugin_manager.lock().unwrap();
                                match pm.route_command(cmd, params) {
                                    Ok(plugin_response) => {
                                        // Wrap plugin response
                                        json!({
                                            "type": "command_response",
                                            "command": cmd,
                                            "success": true,
                                            "data": plugin_response,
                                            "timestamp": chrono::Utc::now()
                                        })
                                    }
                                    Err(e) => {
                                        // No plugin available or plugin error
                                        json!({
                                            "type": "command_response",
                                            "command": cmd,
                                            "success": false,
                                            "error": format!("Plugin error: {}", e),
                                            "timestamp": chrono::Utc::now()
                                        })
                                    }
                                }
                            };

                            if let Err(e) = client.send_json(&response).await {
                                error!("Failed to send command response: {}", e);
                            } else {
                                info!("Command response sent via {}",
                                      client.connection_type());
                            }
                        }
                    }
                    "ping" => {
                        let pong = json!({
                            "type": "pong",
                            "timestamp": chrono::Utc::now()
                        });
                        if let Err(e) = client.send_json(&pong).await {
                            error!("Failed to send pong: {}", e);
                        }
                    }
                    _ => {
                        debug!("Unknown message type: {}", msg_type);
                    }
                }
            }
        }
    });
    
    // Keep agent running indefinitely with graceful shutdown support
    info!("Agent is running with trait-based connection. Press Ctrl+C to stop.");
    
    // Create shutdown signal handler
    let mut shutdown_signal = std::pin::pin!(tokio::signal::ctrl_c());
    
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                let is_connected = ws_client_ref.lock().await.is_connected().await;
                if is_connected {
                    info!("Agent heartbeat - still running via {}", 
                          ws_client_ref.lock().await.connection_type());
                } else {
                    warn!("Agent disconnected, attempting to reconnect...");
                }
            }
            _ = shutdown_signal.as_mut() => {
                info!("Shutdown signal received, closing connection...");
                ws_client_ref.lock().await.close().await.ok();
                info!("Agent shutdown complete");
                break;
            }
        }
    }
    
    Ok(())
}
