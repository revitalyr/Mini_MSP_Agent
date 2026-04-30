use anyhow::Result;
use serde_json::json;
use std::time::Duration;
use tracing::{info, error, debug, warn};
use uuid::Uuid;

mod websocket;
use websocket::WebSocketClient;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AgentInfo {
    id: String,
    hostname: String,
    version: String,
    timestamp: chrono::DateTime<chrono::Utc>,
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
    // Initialize tracing with file logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stdout)
        .with_writer(tracing_appender::rolling::daily("logs", "agent.log"))
        .init();

    info!("Starting Simple Mini MSP Agent with trait-based connections");

    // Create agent info
    let agent_info = AgentInfo {
        id: Uuid::new_v4().to_string(),
        hostname: gethostname::gethostname().to_string_lossy().to_string(),
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now(),
    };

    info!("Agent ID: {}", agent_info.id);
    info!("Hostname: {}", agent_info.hostname);

    // Connect to NATS
    info!("Connecting to NATS at nats://localhost:4222...");
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    info!("Connected to NATS successfully");

    // Connect to WebSocket using trait-based approach
    info!("Connecting to WebSocket at ws://localhost:8081/ws...");
    let ws_result = websocket::WebSocketClient::connect("ws://localhost:8081/ws").await?;
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
            
            // Send NATS heartbeat
            let heartbeat = json!({
                "agent_id": agent_id,
                "timestamp": chrono::Utc::now(),
                "status": "active"
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
    
    // Keep agent running indefinitely
    info!("Agent is running with trait-based connection. Press Ctrl+C to stop.");
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        info!("Agent heartbeat - still running via {}", 
              ws_client_ref.lock().await.connection_type());
    }

    // Cleanup
    info!("Agent shutdown complete");
    Ok(())
}
