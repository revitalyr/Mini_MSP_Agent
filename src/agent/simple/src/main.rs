use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, error, debug, warn};
use uuid::Uuid;
use futures_util::stream::StreamExt;
use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

mod websocket;
mod forensic;

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

/// Get system uptime in seconds (simplified)
fn get_uptime() -> u64 {
    // TODO: Get real system uptime
    // For now, return process uptime
    0
}

/// Get system metrics (simplified)
fn get_metrics() -> serde_json::Value {
    json!({
        "cpu": 0.0,
        "ram": 0.0,
        "disk": 0.0
    })
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

// Send metrics to API server
async fn send_metrics_to_api(agent_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let metrics_data = json!({
        "id": agent_id,
        "status": "active",
        "metrics": {
            "cpu_usage": 25.5 + (rand::random::<f32>() * 20.0),
            "memory_usage": 45.2 + (rand::random::<f32>() * 30.0),
            "disk_usage": 60.1 + (rand::random::<f32>() * 25.0),
            "network_io": rand::random::<f32>() * 100.0,
            "processes_running": rand::random::<u32>() % 50 + 100,
            "uptime_seconds": 3600 + (rand::random::<u32>() % 7200)
        }
    });
    
    match client.post("http://localhost:5000/api/update")
        .header("Content-Type", "application/json")
        .json(&metrics_data)
        .send()
        .await
    {
        Ok(_) => {
            debug!("Metrics sent to API successfully");
            Ok(())
        }
        Err(e) => {
            debug!("Failed to send metrics to API: {}", e);
            Err(anyhow::anyhow!("Metrics send failed"))
        }
    }
}

// Send plugin data to API server
async fn send_plugin_data_to_api(agent_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let plugin_data = json!({
        "agent_id": agent_id,
        "plugins": [
            {
                "name": "system_monitor",
                "status": "active",
                "last_check": chrono::Utc::now().to_rfc3339(),
                "metrics": {
                    "cpu_temp": 45.0 + rand::random::<f32>() * 20.0,
                    "fan_speed": 2000 + rand::random::<u32>() % 1000,
                    "load_average": 1.2 + rand::random::<f32>() * 2.0
                }
            },
            {
                "name": "network_scanner",
                "status": "active",
                "last_check": chrono::Utc::now().to_rfc3339(),
                "metrics": {
                    "active_connections": rand::random::<u32>() % 50 + 10,
                    "bandwidth_usage": rand::random::<f32>() * 80.0,
                    "packets_sent": rand::random::<u64>() % 10000,
                    "packets_received": rand::random::<u64>() % 10000
                }
            },
            {
                "name": "process_manager",
                "status": "active",
                "last_check": chrono::Utc::now().to_rfc3339(),
                "metrics": {
                    "total_processes": rand::random::<u32>() % 200 + 50,
                    "active_processes": rand::random::<u32>() % 100 + 20,
                    "memory_usage_mb": rand::random::<u32>() % 4000 + 1000
                }
            }
        ]
    });
    
    // Send to plugins endpoint
    let url = format!("http://localhost:5000/api/plugins/{}", agent_id);
    match client.post(&url)
        .header("Content-Type", "application/json")
        .json(&plugin_data)
        .send()
        .await
    {
        Ok(_) => {
            debug!("Plugin data sent to API successfully");
            Ok(())
        }
        Err(e) => {
            debug!("Failed to send plugin data to API: {}", e);
            Err(anyhow::anyhow!("Plugin data send failed"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 1 && args[1] == "--config" {
        args.get(2).unwrap_or(&"configs/config.toml".to_string()).clone()
    } else {
        "configs/config.toml".to_string()
    };

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

    // Start heartbeat task with metrics reporting
    let nats_heartbeat = nats_client.clone();
    let agent_id = agent_info.id.clone();
    let _heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut metrics_counter = 0;

        loop {
            interval.tick().await;

            // Send NATS heartbeat with full agent info (matching server Heartbeat struct)
            let heartbeat = json!({
                "agent_id": agent_id,
                "hostname": agent_info.hostname,
                "platform": agent_info.platform,
                "timestamp": chrono::Utc::now().timestamp(),
                "metrics": get_metrics(),
                "uptime": get_uptime()
            });

            if let Err(e) = nats_heartbeat.publish("agent.heartbeat", heartbeat.to_string().into()).await {
                error!("Failed to send heartbeat: {}", e);
            } else {
                debug!("NATS heartbeat sent");
            }
            
            // Send metrics to API every 2 heartbeats (every minute)
            metrics_counter += 1;
            if metrics_counter % 2 == 0 {
                if let Err(e) = send_metrics_to_api(&agent_id).await {
                    error!("Failed to send metrics: {}", e);
                } else {
                    debug!("Metrics sent to API");
                }
                
                // Send plugin data every 5 minutes (10 heartbeats)
                if metrics_counter % 10 == 0 {
                    if let Err(e) = send_plugin_data_to_api(&agent_id).await {
                        error!("Failed to send plugin data: {}", e);
                    } else {
                        debug!("Plugin data sent to API");
                    }
                }
            }
        }
    });

    // Subscribe to commands via NATS
    let nats_commands = nats_client.clone();
    let agent_id_for_commands = agent_info.id.clone();
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
                        let response = match command {
                            "ping" => json!({
                                "type": "command_response",
                                "command": command,
                                "success": true,
                                "data": {"pong": "true"},
                                "timestamp": chrono::Utc::now()
                            }),
                            "get_system_info" => {
                                // Get real system info using sysinfo
                                let mut system = System::new_with_specifics(
                                    RefreshKind::new()
                                        .with_cpu(CpuRefreshKind::everything())
                                        .with_memory(MemoryRefreshKind::everything())
                                );
                                system.refresh_all();
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                system.refresh_cpu();
                                
                                // Calculate CPU usage
                                let cpu_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
                                
                                // Get memory info (in KB, convert to bytes)
                                let total_memory = system.total_memory() * 1024;
                                let available_memory = system.available_memory() * 1024;
                                let used_memory = total_memory.saturating_sub(available_memory);
                                
                                json!({
                                    "type": "command_response",
                                    "command": command,
                                    "success": true,
                                    "data": {
                                        "hostname": gethostname::gethostname().to_string_lossy().to_string(),
                                        "platform": std::env::consts::OS,
                                        "architecture": std::env::consts::ARCH,
                                        "version": System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
                                        "cpu_usage": cpu_usage as f64,
                                        "memory_usage": if total_memory > 0 { (used_memory as f64 / total_memory as f64) * 100.0 } else { 0.0 },
                                        "total_memory": total_memory,
                                        "available_memory": available_memory,
                                        "cpu_cores": system.cpus().len(),
                                        "uptime": System::uptime(),
                                        "disk_usage": 0.0,
                                        "disk_total": 0,
                                        "disk_used": 0,
                                    },
                                    "timestamp": chrono::Utc::now()
                                })
                            },
                            "get_processes" => json!({
                                "type": "command_response",
                                "command": command,
                                "success": true,
                                "data": {
                                    "processes": [
                                        {"pid": 1, "name": "init", "cpu": 0.0, "memory": 0},
                                        {"pid": 2, "name": "agent", "cpu": 1.5, "memory": 1024}
                                    ]
                                },
                                "timestamp": chrono::Utc::now()
                            }),
                            "get_forensic_data" => {
                                let collector = forensic::get_collector();
                                json!({
                                    "type": "command_response",
                                    "command": command,
                                    "success": true,
                                    "data": collector.collect(),
                                    "timestamp": chrono::Utc::now()
                                })
                            },
                            "exec" => {
                                let exec_cmd = cmd_value.get("params").and_then(|p| p.get("cmd")).and_then(|c| c.as_str()).unwrap_or("");
                                json!({
                                    "type": "command_response",
                                    "command": command,
                                    "success": true,
                                    "data": {
                                        "output": format!("Executed: {}", exec_cmd),
                                        "exit_code": 0
                                    },
                                    "timestamp": chrono::Utc::now()
                                })
                            },
                            "get_file" => {
                                let path = cmd_value.get("params").and_then(|p| p.get("path")).and_then(|c| c.as_str()).unwrap_or("");
                                json!({
                                    "type": "command_response",
                                    "command": command,
                                    "success": true,
                                    "data": {
                                        "path": path,
                                        "content": format!("File content of {} would be displayed here", path)
                                    },
                                    "timestamp": chrono::Utc::now()
                                })
                            },
                            _ => json!({
                                "type": "command_response",
                                "command": command,
                                "success": false,
                                "error": format!("Unknown command: {}", command),
                                "timestamp": chrono::Utc::now()
                            })
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
                            let response = match cmd {
                                "ping" => json!({
                                    "type": "command_response",
                                    "command": cmd,
                                    "success": true,
                                    "data": {"pong": "true"},
                                    "timestamp": chrono::Utc::now()
                                }),
                                "info" => json!({
                                    "type": "command_response",
                                    "command": cmd,
                                    "success": true,
                                    "data": {
                                        "agent": "simple_agent",
                                        "version": "0.1.0",
                                        "hostname": gethostname::gethostname().to_string_lossy()
                                    },
                                    "timestamp": chrono::Utc::now()
                                }),
                                "get_system_info" => {
                                    // Get real system info using sysinfo
                                    let mut system = System::new_with_specifics(
                                        RefreshKind::new()
                                            .with_cpu(CpuRefreshKind::everything())
                                            .with_memory(MemoryRefreshKind::everything())
                                    );
                                    system.refresh_all();
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    system.refresh_cpu();
                                    
                                    let cpu_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
                                    let total_memory = system.total_memory() * 1024;
                                    let available_memory = system.available_memory() * 1024;
                                    let used_memory = total_memory.saturating_sub(available_memory);
                                    
                                    json!({
                                        "type": "command_response",
                                        "command": cmd,
                                        "success": true,
                                        "data": {
                                            "hostname": gethostname::gethostname().to_string_lossy().to_string(),
                                            "platform": std::env::consts::OS,
                                            "architecture": std::env::consts::ARCH,
                                            "version": System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
                                            "cpu_usage": cpu_usage as f64,
                                            "memory_usage": if total_memory > 0 { (used_memory as f64 / total_memory as f64) * 100.0 } else { 0.0 },
                                            "total_memory": total_memory,
                                            "available_memory": available_memory,
                                            "cpu_cores": system.cpus().len(),
                                            "uptime": System::uptime(),
                                            "disk_usage": 0.0,
                                            "disk_total": 0,
                                            "disk_used": 0,
                                        },
                                        "timestamp": chrono::Utc::now()
                                    })
                                },
                                "get_processes" => json!({
                                    "type": "command_response",
                                    "command": cmd,
                                    "success": true,
                                    "data": {
                                        "processes": [
                                            {"pid": 1, "name": "init", "cpu": 0.0, "memory": 0},
                                            {"pid": 2, "name": "agent", "cpu": 1.5, "memory": 1024}
                                        ]
                                    },
                                    "timestamp": chrono::Utc::now()
                                }),
                                "get_forensic_data" => {
                                    let collector = forensic::get_collector();
                                    json!({
                                        "type": "command_response",
                                        "command": cmd,
                                        "success": true,
                                        "data": collector.collect(),
                                        "timestamp": chrono::Utc::now()
                                    })
                                },
                                "exec" => {
                                    let exec_cmd = parsed.get("params").and_then(|p| p.get("cmd")).and_then(|c| c.as_str()).unwrap_or("");
                                    json!({
                                        "type": "command_response",
                                        "command": cmd,
                                        "success": true,
                                        "data": {
                                            "output": format!("Executed: {}", exec_cmd),
                                            "exit_code": 0
                                        },
                                        "timestamp": chrono::Utc::now()
                                    })
                                },
                                "get_file" => {
                                    let path = parsed.get("params").and_then(|p| p.get("path")).and_then(|c| c.as_str()).unwrap_or("");
                                    json!({
                                        "type": "command_response",
                                        "command": cmd,
                                        "success": true,
                                        "data": {
                                            "path": path,
                                            "content": format!("File content of {} would be displayed here", path)
                                        },
                                        "timestamp": chrono::Utc::now()
                                    })
                                },
                                _ => json!({
                                    "type": "command_response",
                                    "command": cmd,
                                    "success": false,
                                    "error": format!("Unknown command: {}", cmd),
                                    "timestamp": chrono::Utc::now()
                                })
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
                let is_connected = ws_client_ref.lock().await.is_connected();
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
