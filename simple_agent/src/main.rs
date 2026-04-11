use anyhow::Result;
use async_nats::Client;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, error, debug};
use uuid::Uuid;
use tokio_tungstenite::WebSocketStream;

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
    let (ws_stream, _) = match connect_async("ws://localhost:8081/ws").await {
        Ok(result) => {
            info!("Connected to WebSocket successfully");
            result
        }
        Err(e) => {
            error!("Failed to connect to WebSocket: {}", e);
            return Err(e.into());
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

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
    if let Err(e) = ws_sender.send(Message::Text(registration_msg.to_string())).await {
        error!("Failed to send registration: {}", e);
        return Err(e.into());
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
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message: {}", text);
                
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "command" => {
                                info!("Received command: {:?}", parsed);
                                handle_command(&parsed, &mut ws_sender).await?;
                            }
                            "ping" => {
                                let pong = json!({
                                    "type": "pong",
                                    "timestamp": chrono::Utc::now()
                                });
                                ws_sender.send(Message::Text(pong.to_string())).await?;
                            }
                            _ => {
                                debug!("Unknown message type: {}", msg_type);
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    heartbeat_task.abort();
    info!("Agent shutdown complete");

    Ok(())
}

async fn handle_command(command: &serde_json::Value, ws_sender: &mut futures_util::stream::SplitSink<WebSocketStream<tokio_tungstenite::tungstenite::stream::MaybeTlsStream<tokio::net::TcpStream>>, Message>) -> Result<()> {
    if let Some(cmd) = command.get("command").and_then(|v| v.as_str()) {
        info!("Executing command: {}", cmd);
        
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

        ws_sender.send(Message::Text(response.to_string())).await?;
        info!("Command response sent");
    }
    
    Ok(())
}
