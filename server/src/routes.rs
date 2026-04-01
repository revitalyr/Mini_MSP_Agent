use axum::{
    extract::{ws::WebSocket, ws::{Message, Sender}, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
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

    let mut agents = state.agents.lock().unwrap();
    
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
    let (sender, receiver) = socket.split();
    let mut sender = sender; // Make sender mutable for cloning
    let agent_id = Arc::new(std::sync::Mutex::new(None::<String>));
    
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
                                                let mut id_guard = agent_id.lock().unwrap();
                                                *id_guard = Some(agent_id_str.to_string());
                                                
                                                // Register agent in WebSocket manager
                                                let mut ws_manager = state.ws_manager.lock().unwrap();
                                                ws_manager.register_agent(agent_id_str.to_string(), sender.clone()).await;
                                                
                                                info!("Agent {} registered via WebSocket", agent_id_str);
                                                
                                                // Send acknowledgment
                                                let response = serde_json::json!({
                                                    "type": "registered",
                                                    "status": "ok"
                                                });
                                                
                                                if let Err(e) = sender.send(Message::Text(response.to_string())).await {
                                                    error!("Failed to send registration response: {}", e);
                                                    break;
                                                }
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
                    Ok(Message::Ping(payload)) => {
                        debug!("Received ping, sending pong");
                        if let Err(e) = sender.send(Message::Pong(payload)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
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

    // Clean up agent from WebSocket manager
    if let Some(id_guard) = agent_id.lock().unwrap().as_ref() {
        let mut ws_manager = state.ws_manager.lock().unwrap();
        ws_manager.remove_agent(id_guard).await;
        info!("Agent {} disconnected", id_guard);
    }
}
