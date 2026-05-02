//! WebSocket модуль - управление WebSocket соединениями
//! 
//! Обработка WebSocket подключений от агентов

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error, debug};

use crate::AppState;
use mini_msp_shared::AgentInfo;

/// Simple WebSocket manager for agent connections
pub struct WebSocketManager {
    connections: Arc<Mutex<HashMap<String, String>>>, // Simplified to just track agent IDs
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, agent_id: String) {
        let mut connections = self.connections.lock().await;
        connections.insert(agent_id, "connected".to_string());
    }

    pub async fn remove_connection(&self, agent_id: &str) {
        let mut connections = self.connections.lock().await;
        connections.remove(agent_id);
    }

    pub async fn get_connected_agents(&self) -> Vec<String> {
        let connections = self.connections.lock().await;
        connections.keys().cloned().collect()
    }
}

/*
/// Get WebSocket socket for specific agent
async fn get_agent_socket(_agent_id: &str) -> Option<WebSocket> {
    // This is a simplified implementation
    // In a real scenario, you'd maintain a map of agent connections
    // For now, we'll return None to indicate the limitation
    None
}
*/

/// Handle WebSocket upgrade
pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>) {
    let agent_id = Uuid::new_v4().to_string();
    
    debug!("New WebSocket connection from agent: {}", agent_id);
    
    // Add connection to manager
    let ws_manager = WebSocketManager::new();
    ws_manager.add_connection(agent_id.clone()).await;
    
    // Log connected agents count
    let connected_agents = ws_manager.get_connected_agents().await;
    info!("Connected agents: {:?}", connected_agents);
    
    // Add to app state agents
    {
        let mut agents = app_state.agents.lock().unwrap();
        let agent = AgentInfo {
            id: agent_id.clone(),
            hostname: "unknown".to_string(),
            version: "1.0.0".to_string(),
            platform: "unknown".to_string(),
        };
        agents.insert(agent_id.clone(), agent);
        debug!("Added agent {} to app state", agent_id);
    }

    let (mut sender, mut receiver) = socket.split();
    
    // Send welcome message
    let welcome = json!({
        "type": "welcome",
        "agent_id": agent_id,
        "timestamp": Utc::now().timestamp()
    });
    
    debug!("Sending welcome message to agent: {}", agent_id);
    
    if let Err(e) = sender.send(Message::Text(welcome.to_string())).await {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    // Handle messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message from {}: {}", agent_id, text);
                
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    info!("Received message from {}: {}", agent_id, value);
                    
                    // Check if this is a command from web interface
                    if let Some(_command_type) = value.get("type").and_then(|v| v.as_str()) {
                        if _command_type == "register" {
                            // Handle agent registration
                            info!("Agent registration: {}", value);
                            
                            // Extract agent info from registration message
                            if let Some(agent_id) = value.get("agent_id").and_then(|v| v.as_str()) {
                                // Add agent to app state for API
                                {
                                    let mut agents = app_state.agents.lock().unwrap();
                                    let agent = AgentInfo {
                                        id: agent_id.to_string(),
                                        hostname: "unknown".to_string(),
                                        version: "1.0.0".to_string(),
                                        platform: "unknown".to_string(),
                                    };
                                    agents.insert(agent_id.to_string(), agent);
                                    info!("Added agent {} to app state via registration", agent_id);
                                }
                                
                                // Store mapping from WebSocket UUID to agent_id
                                // This allows us to forward commands to the correct agent
                                // For now, we'll use the agent_id from registration
                            }
                        } else if let Some(target_agent_id) = value.get("agent_id").and_then(|v| v.as_str()) {
                            if let Some(command) = value.get("command").and_then(|v| v.as_str()) {
                                // This is a command from web client - check if we have this agent connected
                                let connected_agents = ws_manager.get_connected_agents().await;
                                if connected_agents.contains(&target_agent_id.to_string()) {
                                    // Forward command to agent
                                    info!("Forwarding command {} to agent {}", command, target_agent_id);
                                    
                                    let command_msg = json!({
                                        "command": command,
                                        "command_id": value.get("command_id").and_then(|v| v.as_str()).unwrap_or(&uuid::Uuid::new_v4().to_string()),
                                        "timestamp": Utc::now().timestamp()
                                    });
                                    
                                    // Send back to the same connection (assuming it's the agent)
                                    if let Err(e) = sender.send(Message::Text(command_msg.to_string())).await {
                                        error!("Failed to forward command to agent {}: {}", target_agent_id, e);
                                    } else {
                                        info!("Command {} forwarded to agent {}", command, target_agent_id);
                                    }
                                } else {
                                    error!("Agent {} not connected", target_agent_id);
                                    let error_response = json!({
                                        "type": "error",
                                        "message": format!("Agent {} not connected", target_agent_id),
                                        "timestamp": Utc::now().timestamp()
                                    });
                                    if let Err(e) = sender.send(Message::Text(error_response.to_string())).await {
                                        error!("Failed to send error response: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        // Echo back for other messages
                        debug!("Echoing message from agent: {}", agent_id);
                        let response = json!({
                            "type": "echo",
                            "original": value,
                            "timestamp": Utc::now().timestamp()
                        });
                        
                        if let Err(e) = sender.send(Message::Text(response.to_string())).await {
                            error!("Failed to send echo: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed for agent: {}", agent_id);
                break;
            }
            Err(e) => {
                error!("WebSocket error for agent {}: {}", agent_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    ws_manager.remove_connection(&agent_id).await;
    {
        let mut agents = app_state.agents.lock().unwrap();
        agents.remove(&agent_id);
    }
    
    info!("WebSocket connection cleaned up for agent: {}", agent_id);
}
