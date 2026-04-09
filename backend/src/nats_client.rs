use async_nats::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

pub struct NatsClient {
    client: Client,
    agents: Arc<RwLock<Vec<String>>>,
}

impl NatsClient {
    pub async fn connect(url: &str) -> Result<Self, async_nats::Error> {
        let client = async_nats::connect(url).await?;
        
        info!("Connected to NATS broker");
        
        Ok(Self {
            client,
            agents: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn send_command(&self, agent_id: &str, command: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let command_id = uuid::Uuid::new_v4().to_string();
        let request = serde_json::json!({
            "command_id": command_id,
            "command": command
        });

        let subject = format!("commands.{}", agent_id);
        let payload = serde_json::to_vec(&request)?;

        // Send command
        let msg = self.client
            .request(&subject, payload.into())
            .await?;

        // Parse response
        let response: Value = serde_json::from_slice(&msg.payload)?;
        
        debug!("Command {} response: {:?}", command_id, response);
        
        Ok(response)
    }

    pub async fn subscribe_agent_events(&self, agent_id: &str) -> Result<async_nats::Subscriber, Box<dyn std::error::Error>> {
        let subject = format!("events.{}", agent_id);
        let subscriber = self.client.subscribe(subject).await?;
        
        Ok(subscriber)
    }

    pub async fn subscribe_heartbeats(&self) -> Result<async_nats::Subscriber, Box<dyn std::error::Error>> {
        let subscriber = self.client.subscribe("heartbeat.>").await?;
        Ok(subscriber)
    }

    pub async fn get_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.clone()
    }

    pub async fn register_agent(&self, agent_id: &str) {
        let mut agents = self.agents.write().await;
        if !agents.contains(&agent_id.to_string()) {
            agents.push(agent_id.to_string());
            info!("Agent registered: {}", agent_id);
        }
    }
}
