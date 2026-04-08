use async_nats::Client;
use mini_msp_shared::{CommandRequest, CommandResponse, Heartbeat};
use tracing::{error, info, warn};
use anyhow::Result;
use futures_util::StreamExt;
use crate::plugins::PluginManager;
use crate::network::HttpClient;
use crate::telemetry::TelemetryCollector;
use crate::commands::{handle_command, ExecutionContext};
use crate::security::SecurityPolicy;

/// NATS broker client for agent
/// 
/// Handles communication with server through message broker
#[derive(Clone)]
pub struct BrokerClient {
    nats: Client,
}

impl BrokerClient {
    /// Connect to NATS broker with retry logic and automatic runtime reconnection
    pub async fn connect(url: &str) -> Result<Option<Self>> {
        if url.is_empty() {
            info!("⚠️  Broker URL is empty, skipping broker connection");
            return Ok(None);
        }

        let mut attempts = 0;
        let max_attempts = 5;
        let mut delay = std::time::Duration::from_secs(2);

        while attempts < max_attempts {
            attempts += 1;
            info!("🔍 Attempting to connect to broker (attempt {}/{}): '{}'", attempts, max_attempts, url);
            
            // Configure NATS for automatic runtime reconnection
            let options = async_nats::ConnectOptions::new()
                .reconnect_delay_callback(|attempts| {
                    std::time::Duration::from_secs(std::cmp::min(attempts as u64 * 2, 30))
                });

            match async_nats::connect_with_options(url, options).await {
                Ok(nats) => {
                    info!("✅ Successfully connected to NATS broker at {}", url);
                    return Ok(Some(Self { nats }));
                }
                Err(e) => {
                    if attempts < max_attempts {
                        error!("⚠️  Connection attempt {} failed: {}. Retrying in {:?}...", attempts, e, delay);
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                    } else {
                        error!("❌ Failed to connect to NATS broker after {} attempts. Falling back to standalone mode.", max_attempts);
                    }
                }
            }
        }

        Ok(None)
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
        
        // Use the client for direct NATS operations if needed
        let _client = self.client();
        
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
    broker: Option<BrokerClient>,
    agent_id: String,
    deps: BrokerDeps,
}

pub struct BrokerDeps {
    pub plugin_manager: PluginManager,
    pub telemetry: TelemetryCollector,
    pub http_client: HttpClient,
    pub policy: SecurityPolicy,
    pub config: crate::config::Config,
    pub command_timeout_secs: u64,
}

impl BrokerLoop {
    pub fn new(broker: Option<BrokerClient>, agent_id: String, deps: BrokerDeps) -> Self {
        Self {
            broker,
            agent_id,
            deps,
        }
    }

    /// Run the main broker loop
    pub async fn run(self) -> Result<()> {
        info!("Starting broker loop for agent {}", self.agent_id);

        match self.broker {
            Some(broker) => {
                // Subscribe to commands
                let mut command_sub = broker.subscribe_commands(&self.agent_id).await?;

                // Start heartbeat task
                let heartbeat_broker = broker.clone();
                let heartbeat_agent_id = self.agent_id.clone();
                let telemetry = self.deps.telemetry.clone();
                let http_client = self.deps.http_client.clone();
                
                // Task to monitor NATS connection health and notify UI/Log
                let pm_for_state = self.deps.plugin_manager.clone();
                let broker_for_state = broker.clone();
                let agent_id_for_state = self.agent_id.clone();
                let monitor_task = tokio::spawn(async move {
                    let mut last_state = broker_for_state.client().connection_state();
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        let current_state = broker_for_state.client().connection_state();

                        if current_state != last_state {
                            info!("NATS Connection state changed: {:?}", current_state);
                            pm_for_state.notify_event(
                                crate::plugins::PluginEventType::StatusChanged,
                                "NATS_BROKER",
                                &format!("Status: {:?}", current_state)
                            );

                            // If the connection is disconnected, break the loop to allow potential cleanup/restart
                            if matches!(current_state, async_nats::connection::State::Disconnected) {
                                warn!("NATS connection entered terminal state {:?} for agent {}. Monitor task ending.", current_state, agent_id_for_state);
                                break;
                            }
                            last_state = current_state;
                        }
                    }
                    warn!("NATS connection monitor task ended for agent {}", agent_id_for_state);
                });
                
                let heartbeat_task = tokio::spawn(async move {
                    self::heartbeat_loop(heartbeat_broker, heartbeat_agent_id, telemetry, http_client).await;
                });

                // Process commands
                while let Some(msg) = command_sub.next().await {
                    let ctx = ExecutionContext {
                        plugin_manager: &self.deps.plugin_manager,
                        policy: &self.deps.policy,
                        config: &self.deps.config,
                        command_timeout_secs: self.deps.command_timeout_secs,
                    };
                    match serde_json::from_slice::<CommandRequest>(&msg.payload) {
                Ok(cmd) => {
                    info!("Received command: {}", cmd.command_id);
                    
                    // Handle command
                    let timeout_duration = std::time::Duration::from_secs(ctx.command_timeout_secs);
                    let result = tokio::time::timeout(timeout_duration, handle_command(cmd.command.clone(), Some(cmd.command_id.clone()), ctx)).await;

                    let result = match result {
                        Ok(inner_result) => inner_result, // Command completed within timeout
                        Err(_) => { // Command timed out
                            error!("Command '{}' timed out after {} seconds", cmd.command_id, ctx.command_timeout_secs);
                            Err(anyhow::anyhow!("Command timed out after {} seconds", ctx.command_timeout_secs))
                        }
                    };

                    // Обработка результата и публикация
                    match result {
                        Ok(mini_msp_shared::AgentResponse::Json(mut response)) => {
                            // Подставляем ID команды из запроса и отправляем
                            response.command_id = Some(cmd.command_id.clone());
                            if let Err(e) = broker.publish_response(&self.agent_id, response).await {
                                error!("Failed to publish response: {}", e);
                            }
                        }
                        Ok(_) => {
                            warn!("Received non-JSON response from command handler, skipping NATS publish");
                        }
                        Err(e) => {
                            // В случае ошибки формируем корректный ответ об ошибке
                            let error_resp = CommandResponse {
                                command_id: Some(cmd.command_id.clone()),
                                r#type: "error".to_string(),
                                status: "error".to_string(),
                                data: serde_json::json!({ "error": e.to_string() }),
                                timestamp: chrono::Utc::now().timestamp(),
                            };
                            let _ = broker.publish_response(&self.agent_id, error_resp).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to deserialize command: {}", e);
                }
            }
        }

        // Clean up tasks and return error to trigger reconnection in main.rs
        heartbeat_task.abort();
        monitor_task.abort();
        error!("NATS command subscription for agent {} ended unexpectedly", self.agent_id);
        Err(anyhow::anyhow!("NATS connection lost"))
            }
            None => {
                info!("⚠️  Broker not available, running in standalone mode");
                // Run without broker - just keep the agent alive
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    info!("🔄 Agent running in standalone mode (no broker)");
                }
            }
        }
    }
}

/// Heartbeat publishing loop
async fn heartbeat_loop(broker: BrokerClient, agent_id: String, telemetry: TelemetryCollector, http_client: HttpClient) {
    use tokio::time::{interval, Duration};

    let mut interval = interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let metrics = telemetry.collect_metrics().await.ok().unwrap_or_default();
        
        // Collect additional system information
        let hostname = telemetry.get_hostname();
        let uptime = telemetry.get_uptime();
        let _processes = telemetry.get_processes().unwrap_or_default();
        let _system_info = telemetry.get_system_info().unwrap_or_else(|_| crate::telemetry::SystemInfo {
            os_type: "Unknown".to_string(),
            os_version: "Unknown".to_string(),
            hostname: "Unknown".to_string(),
            uptime: 0,
            cpu_cores: 0,
            total_memory: 0,
            available_memory: 0,
        });

        let heartbeat = Heartbeat {
            agent_id: agent_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            metrics,
            hostname,
            uptime,
        };

        if let Err(e) = broker.publish_heartbeat(&agent_id, &heartbeat).await {
            error!("Failed to publish heartbeat via NATS: {}", e);
            // Fallback to HTTP
            if let Err(http_err) = http_client.send_heartbeat(heartbeat.clone()).await {
                error!("Failed to publish heartbeat via HTTP: {}", http_err);
            } else {
                info!("Published heartbeat via HTTP fallback");
            }
        } else {
            info!("Published heartbeat via NATS");
        }
    }
}

/// Plugin event publisher
pub struct PluginEventPublisher {
    broker: Option<BrokerClient>,
    agent_id: String,
}

impl PluginEventPublisher {
    pub fn new(broker: Option<BrokerClient>, agent_id: String) -> Self {
        Self { broker, agent_id }
    }

    /// Publish plugin event
    pub async fn publish_event(&self, plugin: &str, data: serde_json::Value) -> Result<()> {
        match &self.broker {
            Some(broker) => {
                broker.publish_plugin_event(&self.agent_id, plugin, data).await
            }
            None => {
                // Broker not available, skip publishing
                tracing::debug!("Skipping event publishing - broker not available");
                Ok(())
            }
        }
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
