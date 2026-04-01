use axum::extract::ws::{WebSocket, Message};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::Command;
use serde_json;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct WebSocketManager {
    agents: HashMap<String, AgentConnection>,
}

#[derive(Debug)]
struct AgentConnection {
    sender: futures_util::stream::SplitSink<WebSocket, Message>,
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
        sender: futures_util::stream::SplitSink<WebSocket, Message>,
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
        println!("WS: Attempting to send to agent: {}", agent_id);
        if let Some(connection) = self.agents.get_mut(agent_id) {
            println!("WS: Found agent connection");
            let command_json = serde_json::to_string(command)
                .map_err(|e| format!("Failed to serialize command: {}", e))?;
            
            println!("WS: Sending JSON: {}", command_json);
            connection
                .sender
                .send(Message::Text(command_json))
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;
            
            connection.last_activity = std::time::Instant::now();
            info!("Command sent to agent {}: {:?}", agent_id, command);
            println!("WS: Command sent successfully");
            
            Ok(())
        } else {
            println!("WS: Agent {} not found in connections", agent_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_manager_new() {
        let manager = WebSocketManager::new();
        assert_eq!(manager.get_connected_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_websocket_manager_cleanup_inactive() {
        let mut manager = WebSocketManager::new();
        
        // Just test that cleanup doesn't panic
        manager.cleanup_inactive(std::time::Duration::from_nanos(1));
        
        assert_eq!(manager.get_connected_agents().len(), 0);
    }
}
