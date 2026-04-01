use axum::{
    extract::{ws::WebSocket, ws::Message, State, WebSocketUpgrade, Path},
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::{Command, Heartbeat};
use serde_json;
use std::{sync::Arc, time::Instant};
use tracing::{debug, error, info, warn};

use crate::{AgentInfo, AppState};

pub async fn handle_heartbeat(
    State(state): State<AppState>,
    Json(heartbeat): Json<Heartbeat>,
) -> impl axum::response::IntoResponse {
    debug!("Received heartbeat from agent: {}", heartbeat.agent_id);

    let mut agents = state.agents.lock().await;
    
    let agent_info = AgentInfo {
        id: heartbeat.agent_id.clone(),
        last_heartbeat: Instant::now(),
        hostname: heartbeat.hostname.clone(),
        uptime: heartbeat.uptime,
    };
    
    agents.insert(heartbeat.agent_id.clone(), agent_info);
    
    info!("Agent {} registered/updated", heartbeat.agent_id);

    axum::Json(serde_json::json!({
        "status": "received",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket_connection(socket, state))
}

async fn handle_websocket_connection(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let agent_id = Arc::new(tokio::sync::Mutex::new(None::<String>));
    
    info!("New WebSocket connection established");

    loop {
        tokio::select! {
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(json_msg) => {
                                if let Some(msg_type) = json_msg.get("type").and_then(|v| v.as_str()) {
                                    match msg_type {
                                        "register" => {
                                            if let Some(agent_id_str) = json_msg.get("agent_id").and_then(|v| v.as_str()) {
                                                let mut id_guard = agent_id.lock().await;
                                                *id_guard = Some(agent_id_str.to_string());
                                                drop(id_guard); // Drop guard before await
                                                
                                                // Register agent in WebSocket manager
                                                let agent_id_clone = agent_id_str.to_string();
                                                let mut ws_manager = state.ws_manager.lock().await;
                                                ws_manager.register_agent(agent_id_clone, sender).await;
                                                drop(ws_manager); // Drop guard before any further awaits
                                                
                                                info!("Agent {} registered via WebSocket", agent_id_str);
                                                
                                                // Send acknowledgment
                                                let response = serde_json::json!({
                                                    "type": "registered",
                                                    "status": "ok"
                                                });
                                                
                                                // Note: We can't send after moving sender, so registration happens before move
                                                info!("Agent registration completed");
                                                break; // Exit loop after registration
                                            }
                                        }
                                        _ => {
                                            // Forward command responses to HTTP clients if needed
                                            debug!("Received message type: {}", msg_type);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse WebSocket message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Binary(_data)) => {
                        debug!("Received binary WebSocket message");
                    }
                    Ok(Message::Ping(_payload)) => {
                        debug!("Received ping, sending pong");
                        // Note: Can't send pong after moving sender
                        info!("Ping received but cannot respond (sender moved)");
                        break;
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong");
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

pub async fn send_command(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    println!("=== HTTP COMMAND RECEIVED === agent: {}, command: {:?}", agent_id, command);
    error!("=== COMMAND RECEIVED === agent: {}, command: {:?}", agent_id, command);
    info!("Sending command to agent {}: {:?}", agent_id, command);

    let mut ws_manager = state.ws_manager.lock().await;
    println!("HTTP: About to send via WebSocket manager");
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => {
            println!("HTTP: Command sent successfully");
            (StatusCode::OK, Json(serde_json::json!({
                "status": "sent",
                "agent_id": agent_id,
                "command": command
            })))
        },
        Err(e) => {
            println!("HTTP: Failed to send command: {}", e);
            error!("Failed to send command to agent {}: {}", agent_id, e);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Agent not connected: {}", e)
                })),
            )
        }
    }
}
