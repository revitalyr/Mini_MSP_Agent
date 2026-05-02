use async_nats::Client;
use mini_msp_shared::{CommandRequest, CommandResponse, Heartbeat};
use tracing::{info};
use anyhow::Result;
use std::sync::Arc;

/// NATS broker client for server
/// 
/// Handles communication with agents through message broker
#[derive(Clone)]
pub struct BrokerClient {
    nats: Client,
}

impl BrokerClient {
    /// Connect to NATS broker
    pub async fn connect(url: &str) -> Result<Self> {
        let nats = async_nats::connect(url).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to NATS: {}", e))?;
        
        info!("Connected to NATS broker at {}", url);
        Ok(Self { nats })
    }

    /// Send command to specific agent
    pub async fn send_command(&self, agent_id: &str, cmd: CommandRequest) -> Result<()> {
        let subject = format!("commands.{}", agent_id);
        let payload = serde_json::to_vec(&cmd)
            .map_err(|e| anyhow::anyhow!("Failed to serialize command: {}", e))?;
        
        self.nats.publish(subject, payload.into()).await
            .map_err(|e| anyhow::anyhow!("Failed to publish command: {}", e))?;
        
        info!("Sent command {} to agent {}", cmd.command_id, agent_id);
        Ok(())
    }

    /// Subscribe to heartbeats from all agents
    pub async fn subscribe_heartbeats(&self) -> Result<async_nats::Subscriber> {
        let subscriber = self.nats.subscribe("heartbeat.>").await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to heartbeats: {}", e))?;
        
        info!("Subscribed to heartbeat topics");
        Ok(subscriber)
    }

    /// Subscribe to responses from specific agent
    pub async fn subscribe_responses(&self, agent_id: &str) -> Result<async_nats::Subscriber> {
        let subject = format!("responses.{}.*", agent_id);
        let subscriber = self.nats.subscribe(subject).await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to responses: {}", e))?;
        
        info!("Subscribed to responses for agent {}", agent_id);
        Ok(subscriber)
    }

    /// Subscribe to all agent responses
    pub async fn subscribe_all_responses(&self) -> Result<async_nats::Subscriber> {
        let subscriber = self.nats.subscribe("responses.>").await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to all responses: {}", e))?;
        
        info!("Subscribed to all agent responses");
        Ok(subscriber)
    }

    /// Subscribe to plugin events from all agents
    pub async fn subscribe_plugin_events(&self) -> Result<async_nats::Subscriber> {
        let subscriber = self.nats.subscribe("events.*.*").await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to plugin events: {}", e))?;
        
        info!("Subscribed to plugin event topics");
        Ok(subscriber)
    }

    /// Publish heartbeat to monitoring topic (optional)
    pub async fn publish_heartbeat_ack(&self, agent_id: &str, heartbeat: &Heartbeat) -> Result<()> {
        let subject = format!("heartbeat_ack.{}", agent_id);
        let payload = serde_json::to_vec(heartbeat)
            .map_err(|e| anyhow::anyhow!("Failed to serialize heartbeat: {}", e))?;
        
        self.nats.publish(subject, payload.into()).await
            .map_err(|e| anyhow::anyhow!("Failed to publish heartbeat ack: {}", e))?;
        
        Ok(())
    }

    /// Get NATS client for advanced operations
    pub fn client(&self) -> &Client {
        &self.nats
    }
}

/// Broker message handler for processing incoming messages
pub struct BrokerMessageHandler {
    broker: Arc<BrokerClient>,
}

impl BrokerMessageHandler {
    pub fn new(broker: Arc<BrokerClient>) -> Self {
        Self { broker }
    }

    /// Get broker client reference
    pub fn broker(&self) -> &Arc<BrokerClient> {
        &self.broker
    }

    /// Process incoming heartbeat message
    pub async fn handle_heartbeat(&self, agent_id: &str, heartbeat: Heartbeat) -> Result<()> {
        info!("Received heartbeat from agent {}: CPU={}%, RAM={}%, DISK={}%", 
              agent_id, heartbeat.metrics.cpu, heartbeat.metrics.ram, heartbeat.metrics.disk);
        
        // Access broker client through public method
        let broker = self.broker();
        
        // Acknowledge heartbeat using broker client
        broker.publish_heartbeat_ack(agent_id, &heartbeat).await?;
        
        Ok(())
    }

    /// Process incoming command response
    pub async fn handle_response(&self, agent_id: &str, response: CommandResponse) -> Result<()> {
        info!("Received response from agent {}: status={}, type={}", 
              agent_id, response.status, response.r#type);
        
        // Here you would typically:
        // 1. Update command status in database/memory
        // 2. Notify waiting clients via WebSocket
        // 3. Log the response
        
        Ok(())
    }

    /// Process plugin event
    pub async fn handle_plugin_event(&self, agent_id: &str, plugin: &str, data: serde_json::Value) -> Result<()> {
        info!("Received plugin event from agent {}: plugin={}, data={:?}", 
              agent_id, plugin, data);
        
        // Here you would typically:
        // 1. Store event data
        // 2. Trigger alerts if needed
        // 3. Update plugin status
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_msp_shared::Command;

    #[tokio::test]
    async fn test_broker_connection() {
        // This test requires NATS server running
        // In real scenario, you'd use testcontainers or mock NATS
        
        let broker = BrokerClient::connect("nats://localhost:4222").await;
        assert!(broker.is_ok());
    }

    #[test]
    fn test_message_serialization() {
        let cmd = CommandRequest {
            command_id: "test-123".to_string(),
            command: Command::GetSystemInfo,
        };

        let serialized = serde_json::to_vec(&cmd);
        assert!(serialized.is_ok());

        let deserialized: CommandRequest = serde_json::from_slice(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.command_id, "test-123");
    }
}
