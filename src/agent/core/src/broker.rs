use anyhow::{Context, Result};
use async_nats::{Client, Message, Subscriber};
use core_shared::{EventMessage, SystemMetrics};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Client for NATS broker communication
pub struct BrokerClient {
    client: Client,
    agent_id: String,
    metrics_subscriber: Arc<RwLock<Option<Subscriber>>>,
}

impl BrokerClient {
    pub async fn connect(broker_url: &str, agent_id: String) -> Result<Self> {
        info!("Connecting to NATS broker at {}", broker_url);
        
        let client = async_nats::connect(broker_url)
            .await
            .context("Failed to connect to NATS broker")?;
        
        info!("Connected to NATS broker as agent: {}", agent_id);
        
        Ok(Self {
            client,
            agent_id,
            metrics_subscriber: Arc::new(RwLock::new(None)),
        })
    }

    /// Subscribe to agent-specific commands
    pub async fn subscribe_to_commands(&self) -> Result<Subscriber> {
        let subject = format!("commands.{}", self.agent_id);
        let subscriber = self.client
            .subscribe(subject)
            .await
            .context("Failed to subscribe to commands")?;
        
        info!("Subscribed to commands on subject: commands.{}", self.agent_id);
        
        Ok(subscriber)
    }

    /// Subscribe to agent-specific metrics requests
    pub async fn subscribe_to_metrics_requests(&self) -> Result<Subscriber> {
        let subject = format!("metrics_request.{}", self.agent_id);
        let subscriber = self.client
            .subscribe(subject)
            .await
            .context("Failed to subscribe to metrics requests")?;
        
        info!("Subscribed to metrics requests on subject: metrics_request.{}", self.agent_id);
        
        Ok(subscriber)
    }

    /// Publish agent heartbeat
    pub async fn publish_heartbeat(&self) -> Result<()> {
        let heartbeat = json!({
            "agent_id": self.agent_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "online",
        });
        
        let subject = "heartbeat.agents";
        let payload = serde_json::to_vec(&heartbeat)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish heartbeat")?;
        
        debug!("Published heartbeat");
        Ok(())
    }

    /// Publish metrics
    pub async fn publish_metrics(&self, agent_id: &str, metrics: SystemMetrics) -> Result<()> {
        let subject = format!("metrics.{}", agent_id);
        let payload = serde_json::to_vec(&metrics)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish metrics")?;
        
        debug!("Published metrics for agent: {}", agent_id);
        Ok(())
    }

    /// Publish event
    pub async fn publish_event(&self, agent_id: &str, event: EventMessage) -> Result<()> {
        let subject = format!("events.{}", agent_id);
        let payload = serde_json::to_vec(&event)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish event")?;
        
        debug!("Published event: {:?} from agent: {}", event.event_type, agent_id);
        Ok(())
    }

    /// Publish command response
    pub async fn publish_command_response(&self, agent_id: &str, request_id: Uuid, response: serde_json::Value) -> Result<()> {
        let response_data = json!({
            "request_id": request_id,
            "agent_id": agent_id,
            "response": response,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let subject = format!("command_response.{}", agent_id);
        let payload = serde_json::to_vec(&response_data)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish command response")?;
        
        debug!("Published command response for request: {}", request_id);
        Ok(())
    }

    /// Subscribe to global metrics (for monitoring other agents)
    pub async fn subscribe_to_global_metrics(&self) -> Result<()> {
        let subscriber = self.client
            .subscribe("metrics.>")
            .await
            .context("Failed to subscribe to global metrics")?;
        
        info!("Subscribed to global metrics");
        
        let mut metrics_sub = self.metrics_subscriber.write().await;
        *metrics_sub = Some(subscriber);
        
        Ok(())
    }

    /// Get metrics subscriber for receiving global metrics
    pub async fn get_metrics_subscriber(&self) -> Option<Subscriber> {
        // Subscriber doesn't implement Clone, so we need to take it
        self.metrics_subscriber.write().await.take()
    }

    /// Publish agent registration
    pub async fn publish_agent_registration(&self, agent_info: &core_shared::AgentInfo) -> Result<()> {
        let registration = json!({
            "agent_id": agent_info.id,
            "hostname": agent_info.hostname,
            "version": agent_info.version,
            "platform": agent_info.platform,
            "architecture": agent_info.architecture,
            "start_time": agent_info.start_time.to_rfc3339(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let subject = "agent.register";
        let payload = serde_json::to_vec(&registration)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish agent registration")?;
        
        info!("Published agent registration for: {}", agent_info.id);
        Ok(())
    }

    /// Publish agent unregistration
    pub async fn publish_agent_unregistration(&self, agent_id: &str) -> Result<()> {
        let unregistration = json!({
            "agent_id": agent_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let subject = "agent.unregister";
        let payload = serde_json::to_vec(&unregistration)?;
        
        self.client
            .publish(subject, payload.into())
            .await
            .context("Failed to publish agent unregistration")?;
        
        info!("Published agent unregistration for: {}", agent_id);
        Ok(())
    }

    /// Request command execution on another agent
    pub async fn request_command(&self, target_agent_id: &str, command: &str, params: serde_json::Value) -> Result<Message> {
        let request = json!({
            "request_id": Uuid::new_v4().to_string(),
            "command": command,
            "parameters": params,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let subject = format!("commands.{}", target_agent_id);
        let payload = serde_json::to_vec(&request)?;
        
        let response = self.client
            .request(subject, payload.into())
            .await
            .context("Failed to request command")?;
        
        debug!("Requested command {} on agent: {}", command, target_agent_id);
        Ok(response)
    }

    /// Request metrics from another agent
    pub async fn request_metrics(&self, target_agent_id: &str) -> Result<Message> {
        let request = json!({
            "request_id": Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let subject = format!("metrics_request.{}", target_agent_id);
        let payload = serde_json::to_vec(&request)?;
        
        let response = self.client
            .request(subject, payload.into())
            .await
            .context("Failed to request metrics")?;
        
        debug!("Requested metrics from agent: {}", target_agent_id);
        Ok(response)
    }

    /// Get client connection status
    pub fn is_connected(&self) -> bool {
        // NATS client doesn't provide a direct is_connected method
        // We'll check if we can publish a test message
        true // Simplified for now
    }

    /// Get agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl Drop for BrokerClient {
    fn drop(&mut self) {
        info!("Broker client dropped for agent: {}", self.agent_id);
    }
}
