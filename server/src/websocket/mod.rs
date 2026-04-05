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
use tracing::{info, error};

use crate::AppState;

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
    
    // Add connection to manager
    let ws_manager = WebSocketManager::new();
    ws_manager.add_connection(agent_id.clone()).await;
    
    // Log connected agents count
    let connected_agents = ws_manager.get_connected_agents().await;
    info!("Connected agents: {:?}", connected_agents);
    
    // Add to app state agents
    {
        let mut agents = app_state.agents.lock().unwrap();
        agents.insert(agent_id.clone(), "connected".to_string());
    }

    let (mut sender, mut receiver) = socket.split();
    
    // Send welcome message
    let welcome = json!({
        "type": "welcome",
        "agent_id": agent_id,
        "timestamp": Utc::now().timestamp()
    });
    
    if let Err(e) = sender.send(Message::Text(welcome.to_string())).await {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    // Handle messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    info!("Received message from {}: {}", agent_id, value);
                    
                    // Echo back
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
