use axum::extract::ws::{WebSocket, Message, Sender};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::Command;
use serde_json;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, error, warn};

pub struct WebSocketManager {
    agents: HashMap<String, AgentConnection>,
}

#[derive(Debug)]
struct AgentConnection {
    sender: Sender,
    last_activity: std::time::Instant,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub async fn register_agent(
        &mut self,
        agent_id: String,
        sender: Sender,
    ) {
        let connection = AgentConnection {
            sender,
            last_activity: std::time::Instant::now(),
        };
        
        self.agents.insert(agent_id.clone(), connection);
        debug!("Agent {} registered in WebSocket manager", agent_id);
    }

    pub async fn remove_agent(&mut self, agent_id: &str) {
        if self.agents.remove(agent_id).is_some() {
            debug!("Agent {} removed from WebSocket manager", agent_id);
        }
    }

    pub async fn send_to_agent(&mut self, agent_id: &str, command: &Command) -> Result<(), String> {
        if let Some(connection) = self.agents.get_mut(agent_id) {
            let command_json = serde_json::to_string(command)
                .map_err(|e| format!("Failed to serialize command: {}", e))?;
            
            connection
                .sender
                .send(Message::Text(command_json))
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;
            
            connection.last_activity = std::time::Instant::now();
            debug!("Command sent to agent {}: {:?}", agent_id, command);
            
            Ok(())
        } else {
            Err(format!("Agent {} not connected", agent_id))
        }
    }

    pub fn get_connected_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    pub fn cleanup_inactive(&mut self, timeout: std::time::Duration) {
        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();
        
        for (id, connection) in self.agents.iter() {
            if now.duration_since(connection.last_activity) > timeout {
                to_remove.push(id.clone());
            }
        }
        
        for id in to_remove {
            warn!("Removing inactive agent: {}", id);
            self.agents.remove(&id);
        }
    }
}
