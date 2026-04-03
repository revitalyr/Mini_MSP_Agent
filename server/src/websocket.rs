use axum::extract::ws::{WebSocket, Message};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::{Command, CommandRequest, AgentResponse};
use serde_json::{self, json};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

pub struct WebSocketManager {
    agents: HashMap<String, AgentConnection>,
    pending_responses: HashMap<String, oneshot::Sender<AgentResponse>>,
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
            pending_responses: HashMap::new(),
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

    pub async fn send_and_wait(&mut self, agent_id: &str, command: Command) -> Result<oneshot::Receiver<AgentResponse>, String> {
        println!("WS: Attempting to send to agent: {}", agent_id);
        
        let command_id = uuid::Uuid::new_v4().to_string();
        let request = CommandRequest {
            command_id: command_id.clone(),
            command: command.clone(),
        };

        if let Some(connection) = self.agents.get_mut(agent_id) {
            let command_json = serde_json::to_string(&request)
                .map_err(|e| format!("Failed to serialize command: {}", e))?;
            
            // Создаем канал для ожидания ответа
            let (tx, rx) = oneshot::channel();
            self.pending_responses.insert(command_id, tx);

            connection
                .sender
                .send(Message::Text(command_json))
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;
            
            connection.last_activity = std::time::Instant::now();
            info!("Command sent to agent {}: {:?}", agent_id, command);
            
            Ok(rx)
        } else {
            Err(format!("Agent {} not connected", agent_id))
        }
    }

    pub fn handle_response(&mut self, command_id: &str, data: AgentResponse) {
        if let Some(tx) = self.pending_responses.remove(command_id) {
            let _ = tx.send(data);
            debug!("Response delivered for command ID: {}", command_id);
        } else {
            warn!("Received response for unknown command ID: {}", command_id);
        }
    }

    // Существующий метод для обратной совместимости или fire-and-forget
    pub async fn send_to_agent(&mut self, agent_id: &str, command: &Command) -> Result<(), String> {
        let _ = self.send_and_wait(agent_id, command.clone()).await?;
        Ok(())
    }

    pub fn get_connected_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    pub fn cleanup_inactive(&mut self, timeout: std::time::Duration) {
        let now = std::time::Instant::now();
        self.agents.retain(|id, connection| {
            let is_active = now.duration_since(connection.last_activity) <= timeout;
            if !is_active { warn!("Removing inactive agent: {}", id); }
            is_active
        });
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
