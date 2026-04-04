use async_nats::Client;
use mini_msp_shared::{BrokerMessage, CommandRequest, CommandResponse, Heartbeat};
use tracing::{error, info, warn};
use anyhow::Result;
use futures_util::StreamExt;
use crate::plugins::PluginManager;
use crate::commands::handle_command;

/// NATS broker client for agent
/// 
/// Handles communication with server through message broker
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

    /// Subscribe to commands for this agent
    pub async fn subscribe_commands(&self, agent_id: &str) -> Result<async_nats::Subscriber> {
        let subject = format!("commands.{}", agent_id);
        let subscriber = self.nats.subscribe(subject).await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to commands: {}", e))?;
        
        info!("Subscribed to command topics for agent {}", agent_id);
        Ok(subscriber)
    }

    /// Publish heartbeat
    pub async fn publish_heartbeat(&self, agent_id: &str, heartbeat: &Heartbeat) -> Result<()> {
        let subject = format!("heartbeat.{}", agent_id);
        let payload = serde_json::to_vec(heartbeat)
            .map_err(|e| anyhow::anyhow!("Failed to serialize heartbeat: {}", e))?;
        
        self.nats.publish(subject, payload.into()).await
            .map_err(|e| anyhow::anyhow!("Failed to publish heartbeat: {}", e))?;
        
        Ok(())
    }

    /// Publish command response
    pub async fn publish_response(&self, agent_id: &str, response: CommandResponse) -> Result<()> {
        let subject = format!("responses.{}.{}", agent_id, 
            response.command_id.as_ref().unwrap_or(&"unknown".to_string()));
        let payload = serde_json::to_vec(&response)
            .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;
        
        self.nats.publish(subject, payload.into()).await
            .map_err(|e| anyhow::anyhow!("Failed to publish response: {}", e))?;
        
        info!("Published response for command {}", response.command_id.as_ref().unwrap_or(&"unknown".to_string()));
        Ok(())
    }

    /// Publish plugin event
    pub async fn publish_plugin_event(&self, agent_id: &str, plugin: &str, data: serde_json::Value) -> Result<()> {
        let subject = format!("events.{}.{}", agent_id, plugin);
        let payload = serde_json::to_vec(&data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize plugin event: {}", e))?;
        
        self.nats.publish(subject, payload.into()).await
            .map_err(|e| anyhow::anyhow!("Failed to publish plugin event: {}", e))?;
        
        info!("Published plugin event for plugin {}", plugin);
        Ok(())
    }

    /// Get NATS client for advanced operations
    pub fn client(&self) -> &Client {
        &self.nats
    }
}

/// Main broker loop for agent
/// 
/// Handles incoming commands and publishes responses/heartbeats
pub struct BrokerLoop {
    broker: BrokerClient,
    agent_id: String,
    plugin_manager: PluginManager,
}

impl BrokerLoop {
    pub fn new(broker: BrokerClient, agent_id: String, plugin_manager: PluginManager) -> Self {
        Self {
            broker,
            agent_id,
            plugin_manager,
        }
    }

    /// Run the main broker loop
    pub async fn run(mut self) -> Result<()> {
        info!("Starting broker loop for agent {}", self.agent_id);

        // Subscribe to commands
        let mut command_sub = self.broker.subscribe_commands(&self.agent_id).await?;

        // Start heartbeat task
        let heartbeat_broker = self.broker.clone();
        let heartbeat_agent_id = self.agent_id.clone();
        let heartbeat_task = tokio::spawn(async move {
            self::heartbeat_loop(heartbeat_broker, heartbeat_agent_id).await;
        });

        // Process commands
        while let Some(msg) = command_sub.next().await {
            match serde_json::from_slice::<CommandRequest>(&msg.payload) {
                Ok(cmd) => {
                    info!("Received command: {}", cmd.command_id);
                    
                    // Handle command
                    let result = handle_command(cmd.command.clone(), &self.plugin_manager).await;
                    
                    // Create response
                    let response = CommandResponse {
                        command_id: Some(cmd.command_id.clone()),
                        r#type: format!("{:?}", cmd.command),
                        status: if result.is_ok() { "success" } else { "error" }.to_string(),
                        data: serde_json::to_value(result).unwrap_or(serde_json::Value::Null),
                        timestamp: chrono::Utc::now().timestamp(),
                    };

                    // Publish response
                    if let Err(e) = self.broker.publish_response(&self.agent_id, response).await {
                        error!("Failed to publish response: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to deserialize command: {}", e);
                }
            }
        }

        // Clean up heartbeat task
        heartbeat_task.abort();
        Ok(())
    }
}

/// Heartbeat publishing loop
async fn heartbeat_loop(broker: BrokerClient, agent_id: String) {
    use tokio::time::{interval, Duration};
    use mini_msp_shared::Metrics;
    use sysinfo::System;

    let mut interval = interval(Duration::from_secs(30));
    let mut system = System::new_all();

    loop {
        interval.tick().await;

        // Collect system metrics
        system.refresh_all();
        
        let metrics = Metrics {
            cpu: system.global_cpu_usage(),
            ram: (system.used_memory() as f32 / system.total_memory() as f32) * 100.0,
            disk: 0.0, // TODO: Implement disk usage calculation
        };

        let heartbeat = Heartbeat {
            agent_id: agent_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            metrics,
            hostname: gethostname::gethostname().into_string().unwrap_or_else(|_| "unknown".to_string()),
            uptime: system.uptime(),
        };

        if let Err(e) = broker.publish_heartbeat(&agent_id, &heartbeat).await {
            error!("Failed to publish heartbeat: {}", e);
        } else {
            info!("Published heartbeat");
        }
    }
}

/// Plugin event publisher
pub struct PluginEventPublisher {
    broker: BrokerClient,
    agent_id: String,
}

impl PluginEventPublisher {
    pub fn new(broker: BrokerClient, agent_id: String) -> Self {
        Self { broker, agent_id }
    }

    /// Publish plugin event
    pub async fn publish_event(&self, plugin: &str, data: serde_json::Value) -> Result<()> {
        self.broker.publish_plugin_event(&self.agent_id, plugin, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_msp_shared::{Command, Metrics};

    #[tokio::test]
    async fn test_agent_broker_connection() {
        // This test requires NATS server running
        let broker = BrokerClient::connect("nats://localhost:4222").await;
        assert!(broker.is_ok());
    }

    #[test]
    fn test_heartbeat_serialization() {
        let heartbeat = Heartbeat {
            agent_id: "test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics {
                cpu: 50.0,
                ram: 60.0,
                disk: 70.0,
            },
            hostname: "test-host".to_string(),
            uptime: 3600,
        };

        let serialized = serde_json::to_vec(&heartbeat);
        assert!(serialized.is_ok());

        let deserialized: Heartbeat = serde_json::from_slice(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.agent_id, "test-agent");
        assert_eq!(deserialized.metrics.cpu, 50.0);
    }
}
