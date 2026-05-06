//! WebSocket Module - WebSocket Connection Management
//! 
//! Handles WebSocket connections from agents with semantic type safety.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
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
use tracing::{info, error, debug, warn};

use crate::AppState;
use mini_msp_shared::AgentInfo;

/// Broadcast channel capacity for agent responses
const BROADCAST_CHANNEL_CAPACITY: usize = 100;

/// Broadcast channel for agent responses
pub type ResponseBroadcast = broadcast::Sender<String>;

/// Simple WebSocket manager for agent connections
pub struct WebSocketManager {
    /// Agent ID to status mapping
    connections: Arc<Mutex<HashMap<String, String>>>,
    /// Response broadcast transmitter
    response_tx: ResponseBroadcast,
}

impl WebSocketManager {
    /// Create a new WebSocket manager with configured broadcast capacity
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            response_tx: tx,
        }
    }
    
    pub fn get_response_channel(&self) -> ResponseBroadcast {
        self.response_tx.clone()
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
    
    pub fn subscribe_responses(&self) -> broadcast::Receiver<String> {
        self.response_tx.subscribe()
    }
    
    pub async fn broadcast_response(&self, response: String) {
        let _ = self.response_tx.send(response);
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
    
    // Use shared WebSocket manager from app_state
    let ws_manager = app_state.ws_manager.clone();
    
    // Note: Agent will be added to connections when it sends agent_register message
    // For now, just log the connection
    info!("WebSocket connection established, waiting for agent registration...");
    
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

    // Create channel for forwarding agent responses to this WebSocket client
    let (forward_tx, mut forward_rx) = tokio::sync::mpsc::channel::<String>(100);
    
    // Subscribe to agent response broadcasts and forward via channel
    let mut response_rx = ws_manager.subscribe_responses();
    tokio::spawn(async move {
        while let Ok(response) = response_rx.recv().await {
            if forward_tx.send(response).await.is_err() {
                break; // Channel closed
            }
        }
    });

    // Handle messages and forwarded responses concurrently
    info!("Starting WebSocket message loop for agent: {}", agent_id);
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg = receiver.next() => {
                info!("WebSocket receiver got message for {}: {:?}", agent_id, msg);
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("RAW WebSocket TEXT message from connection {}: {}", agent_id, text);
                        
                        let parse_result = serde_json::from_str::<Value>(&text);
                        match parse_result {
                            Ok(value) => {
                                info!("Parsed JSON from {}: {:?}", agent_id, value);
                                
                                let msg_type = value.get("type").and_then(|v| v.as_str());
                                info!("Message type: {:?}", msg_type);
                                
                                if let Some(_command_type) = msg_type {
                                    info!("Processing command type: {}", _command_type);
                                    if _command_type == "register" || _command_type == "agent_register" {
                                        info!("Agent registration message: {}", value);
                                        let actual_agent_id = value.get("agent")
                                            .and_then(|a| a.get("id"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or(&agent_id);
                                        
                                        let hostname = value.get("agent")
                                            .and_then(|a| a.get("hostname"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        
                                        let platform = value.get("agent")
                                            .and_then(|a| a.get("platform"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        
                                        let version = value.get("agent")
                                            .and_then(|a| a.get("version"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("1.0.0");
                                        
                                        info!("Registering agent {} ({}:{}) to WebSocket connections", actual_agent_id, hostname, platform);
                                        ws_manager.add_connection(actual_agent_id.to_string()).await;
                                        
                                        // Also add to AppState.agents for list_agents endpoint
                                        {
                                            let mut agents = app_state.agents.lock().unwrap();
                                            let agent_info = AgentInfo {
                                                id: actual_agent_id.to_string(),
                                                hostname: hostname.to_string(),
                                                platform: platform.to_string(),
                                                version: version.to_string(),
                                                last_seen: chrono::Utc::now().timestamp() as u64,
                                            };
                                            agents.insert(actual_agent_id.to_string(), agent_info);
                                            info!("Agent {} added to AppState.agents, total agents: {}", actual_agent_id, agents.len());
                                        }
                                        
                                        info!("Agent {} successfully registered via WebSocket", actual_agent_id);
                                    } else if let Some(target_agent_id) = value.get("agent_id").and_then(|v| v.as_str()) {
                                        if let Some(command) = value.get("command").and_then(|v| v.as_str()) {
                                            let connected_agents = ws_manager.get_connected_agents().await;
                                            if connected_agents.contains(&target_agent_id.to_string()) {
                                                info!("Forwarding command {} to agent {}", command, target_agent_id);
                                                let command_msg = json!({
                                                    "command": command,
                                                    "command_id": value.get("command_id").and_then(|v| v.as_str()).unwrap_or(&uuid::Uuid::new_v4().to_string()),
                                                    "timestamp": Utc::now().timestamp()
                                                });
                                                if let Err(e) = sender.send(Message::Text(command_msg.to_string())).await {
                                                    error!("Failed to forward command to agent {}: {}", target_agent_id, e);
                                                } else {
                                                    info!("Command {} forwarded to agent {}", command, target_agent_id);
                                                }
                                            } else {
                                                // Try NATS fallback
                                                if let Some(ref broker) = app_state.broker_client {
                                                    info!("Agent {} not connected via WebSocket, trying NATS...", target_agent_id);
                                                    let command_msg = json!({
                                                        "command": command,
                                                        "command_id": value.get("command_id").and_then(|v| v.as_str()).unwrap_or(&uuid::Uuid::new_v4().to_string()),
                                                        "timestamp": Utc::now().timestamp()
                                                    });
                                                    let topic = format!("agent.{}.commands", target_agent_id);
                                                    match broker.client().publish(topic.clone(), command_msg.to_string().into()).await {
                                                        Ok(_) => {
                                                            info!("Command {} sent to agent {} via NATS topic {}", command, target_agent_id, topic);
                                                            let ok_response = json!({
                                                                "type": "command_sent",
                                                                "message": format!("Command sent to agent {} via NATS", target_agent_id),
                                                                "timestamp": Utc::now().timestamp()
                                                            });
                                                            if let Err(e) = sender.send(Message::Text(ok_response.to_string())).await {
                                                                error!("Failed to send OK response: {}", e);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error!("Failed to send command via NATS to {}: {}", target_agent_id, e);
                                                            let error_response = json!({
                                                                "type": "error",
                                                                "message": format!("Agent {} not connected via WebSocket or NATS", target_agent_id),
                                                                "timestamp": Utc::now().timestamp()
                                                            });
                                                            if let Err(e) = sender.send(Message::Text(error_response.to_string())).await {
                                                                error!("Failed to send error response: {}", e);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    error!("Agent {} not connected and NATS broker unavailable", target_agent_id);
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
                                    }
                                } else {
                                    debug!("Echoing message from agent: {}", agent_id);
                                    let response = json!({
                                        "type": "echo",
                                        "original": value,
                                        "timestamp": Utc::now().timestamp()
                                    });
                                    if let Err(e) = sender.send(Message::Text(response.to_string())).await {
                                        error!("Failed to send echo: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse JSON from {}: {}. Raw text: {}", agent_id, e, text);
                            }
                        }
                    }
                    Some(Ok(Message::Close(close))) => {
                        info!("WebSocket closed for agent {}: {:?}", agent_id, close);
                        break;
                    }
                    Some(Ok(Message::Ping(ping))) => {
                        debug!("WebSocket ping from agent {}: {:?}", agent_id, ping);
                    }
                    Some(Ok(Message::Pong(pong))) => {
                        debug!("WebSocket pong from agent {}: {:?}", agent_id, pong);
                    }
                    Some(Ok(Message::Binary(bin))) => {
                        warn!("WebSocket binary message from agent {}: {} bytes", agent_id, bin.len());
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for agent {}: {}", agent_id, e);
                        break;
                    }
                    None => {
                        info!("WebSocket receiver ended for agent {}", agent_id);
                        break;
                    }
                }
            }
            // Forward agent responses to this client
            response = forward_rx.recv() => {
                match response {
                    Some(resp) => {
                        if let Err(e) = sender.send(Message::Text(resp)).await {
                            error!("Failed to send agent response to client: {}", e);
                            break;
                        }
                    }
                    None => break, // Channel closed
                }
            }
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

/// Send command to agent and wait for response
/// Uses NATS broker if available, otherwise returns None
pub async fn send_command_to_agent(
    agent_id: &str,
    app_state: &crate::AppState,
    command: Value,
) -> Option<Value> {
    use tokio::time::{timeout, Duration};
    
    // Get broker client
    let broker = match app_state.broker_client {
        Some(ref b) => b,
        None => {
            error!("No NATS broker available to send command to agent {}", agent_id);
            return None;
        }
    };
    
    // Subscribe to agent responses BEFORE sending command
    let response_topic = format!("agent.{}.responses", agent_id);
    let mut response_sub = match broker.client().subscribe(response_topic.clone()).await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to responses for agent {}: {}", agent_id, e);
            return None;
        }
    };
    
    // Send command via NATS
    let command_topic = format!("agent.{}.commands", agent_id);
    let command_str = command.to_string();
    
    if let Err(e) = broker.client().publish(command_topic.clone(), command_str.into()).await {
        error!("Failed to send command to agent {}: {}", agent_id, e);
        return None;
    }
    
    info!("Command sent to agent {} via NATS topic {}", agent_id, command_topic);
    
    // Wait for response (with timeout)
    let wait_result = timeout(Duration::from_secs(10), async {
        while let Some(msg) = response_sub.next().await {
            if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                if let Ok(response) = serde_json::from_str::<Value>(payload) {
                    return Some(response);
                }
            }
        }
        None
    }).await;
    
    match wait_result {
        Ok(response) => response,
        Err(_) => {
            warn!("Timeout waiting for response from agent {}", agent_id);
            None
        }
    }
}
