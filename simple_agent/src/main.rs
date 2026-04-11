use anyhow::Result;
use serde_json::json;
use std::time::Duration;
use tracing::{info, error, debug};
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,simple_agent=debug")
        .init();

    info!("Starting Simple Mini MSP Agent");

    // Generate agent ID
    let agent_id = Uuid::new_v4().to_string();
    let hostname = gethostname::gethostname().to_string_lossy().to_string();

    info!("Agent ID: {}", agent_id);
    info!("Hostname: {}", hostname);

    // Connect to NATS
    info!("Connecting to NATS at nats://localhost:4222...");
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            info!("Connected to NATS successfully");
            client
        }
        Err(e) => {
            error!("Failed to connect to NATS: {}", e);
            return Err(e.into());
        }
    };

    // Connect to WebSocket
    info!("Connecting to WebSocket at ws://localhost:8081/ws...");
    let connect_result = WebSocketClient::connect("ws://localhost:8081/ws").await?;
    let mut ws_client = connect_result.client;
    info!("Connected to WebSocket successfully");

    // Send agent registration
    let agent_info = AgentInfo {
        id: agent_id.clone(),
        hostname: hostname.clone(),
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let registration_msg = json!({
        "type": "agent_register",
        "agent": agent_info
    });

    info!("Sending agent registration...");
    if let Err(e) = ws_client.send_json(&registration_msg).await {
        error!("Failed to send registration: {}", e);
        return Err(e);
    }

    // Start heartbeat task
    let nats_heartbeat = nats_client.clone();
    let agent_id_heartbeat = agent_id.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let heartbeat = json!({
                "type": "heartbeat",
                "agent_id": agent_id_heartbeat,
                "timestamp": chrono::Utc::now(),
                "status": "alive"
            });

            if let Err(e) = nats_heartbeat.publish("agent.heartbeat", heartbeat.to_string().into()).await {
                error!("Failed to send heartbeat: {}", e);
            } else {
                debug!("Heartbeat sent");
            }
        }
    });

    // Handle WebSocket messages
    info!("Starting message handling loop...");
    while let Some(parsed) = ws_client.receive_json().await? {
        debug!("Received WebSocket message: {:?}", parsed);
        
        if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "command" => {
                    info!("Received command: {:?}", parsed);
                    if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
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

                        if let Err(e) = ws_client.send_json(&response).await {
                            error!("Failed to send command response: {}", e);
                        } else {
                            info!("Command response sent");
                        }
                    }
                }
                "ping" => {
                    let pong = json!({
                        "type": "pong",
                        "timestamp": chrono::Utc::now()
                    });
                    ws_client.send_json(&pong).await?;
                }
                _ => {
                    debug!("Unknown message type: {}", msg_type);
                }
            }
        }
    }

    // Cleanup
    heartbeat_task.abort();
    info!("Agent shutdown complete");

    Ok(())
}
