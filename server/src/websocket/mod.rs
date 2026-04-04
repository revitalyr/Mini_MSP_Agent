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
use axum::response::IntoResponse;

use crate::AppState;

/// WebSocket connection manager
pub struct WebSocketManager {
    connections: Arc<Mutex<HashMap<String, tokio_tungstenite::tungstenite::WebSocket<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_connection(
        &self,
        agent_id: String,
        ws: tokio_tungstenite::tungstenite::WebSocket<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) {
        let mut connections = self.connections.lock().await;
        connections.insert(agent_id, ws);
    }

    pub async fn remove_connection(&self, agent_id: &str) {
        let mut connections = self.connections.lock().await;
        connections.remove(agent_id);
    }

    pub async fn broadcast(&self, message: &Value) {
        let mut connections = self.connections.lock().await;
        for (_id, ws) in connections.iter_mut() {
            let _ = ws.send(Message::Text(message.to_string())).await;
        }
    }

    pub async fn get_connected_agents(&self) -> Vec<String> {
        let connections = self.connections.lock().await;
        connections.keys().cloned().collect()
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle WebSocket upgrade
pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state))
}

/// Handle individual WebSocket connection
async fn handle_socket(
    socket: tokio_tungstenite::tungstenite::WebSocket<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    app_state: Arc<AppState>,
) {
    let agent_id = Uuid::new_v4().to_string();
    
    // Add connection to manager
    app_state.ws_manager.add_connection(agent_id.clone(), socket).await;
    
    // Send welcome message
    let welcome_msg = json!({
        "type": "welcome",
        "agent_id": agent_id,
        "timestamp": chrono::Utc::now()
    });
    
    let mut connections = app_state.ws_manager.connections.lock().await;
    if let Some(ws) = connections.get_mut(&agent_id) {
        let _ = ws.send(Message::Text(welcome_msg.to_string())).await;
    }
    
    // Here you would handle the actual WebSocket communication
    // For now, we'll just keep the connection alive
}
