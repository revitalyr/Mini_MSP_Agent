use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::{CommandResponse, Heartbeat, CommandRequest, AgentResponse};
use reqwest::Client;
use serde_json;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::commands::handle_command;
use crate::config::Config;
use crate::plugins::PluginManager;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    config: Config,
}

impl HttpClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn send_heartbeat(&self, heartbeat: Heartbeat) -> Result<()> {
        let url = format!("{}/heartbeat", self.config.server_url);
        
        debug!("Sending heartbeat to {}", url);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Agent-ID", &heartbeat.agent_id)
            .json(&heartbeat)
            .send()
            .await
            .with_context(|| "Failed to send heartbeat request")?;

        if response.status().is_success() {
            debug!("Heartbeat sent successfully");
        } else {
            warn!("Heartbeat request failed with status: {}", response.status());
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct WebSocketClient {
    config: Config,
    plugin_manager: PluginManager,
}

impl WebSocketClient {
    pub fn new(config: Config, plugin_manager: PluginManager) -> Self {
        Self { 
            config,
            plugin_manager,
        }
    }

    pub async fn run(&self) {
        let mut reconnect_interval = Duration::from_secs(5);
        let max_reconnect_interval = Duration::from_secs(300);
        
        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("WebSocket connection closed gracefully");
                    reconnect_interval = Duration::from_secs(5); // Reset on successful connection
                }
                Err(e) => {
                    error!("WebSocket connection error: {}", e);
                }
            }

            info!("Reconnecting WebSocket in {:?}...", reconnect_interval);
            sleep(reconnect_interval).await;
            
            // Exponential backoff
            reconnect_interval = std::cmp::min(
                reconnect_interval * 2,
                max_reconnect_interval
            );
        }
    }

    async fn connect_and_run(&self) -> Result<()> {
        info!("Connecting to WebSocket: {}", self.config.ws_url);
        
        let (ws_stream, response) = connect_async(&self.config.ws_url)
            .await
            .with_context(|| "Failed to connect to WebSocket")?;

        info!("WebSocket connected, response: {:?}", response.status());

        let (mut write, mut read) = ws_stream.split();

        // Send initial agent registration
        let hostname = std::process::Command::new("hostname")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        
        let registration = serde_json::json!({
            "type": "register",
            "agent_id": self.config.agent_id,
            "hostname": hostname,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        write.send(Message::Text(registration.to_string())).await
            .with_context(|| "Failed to send registration message")?;

        // Handle incoming messages
        let mut ping_interval = interval(Duration::from_secs(30));
        
        loop {
            tokio::select! {
                Some(msg) = read.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            info!("Received WebSocket message: {}", text);
                            
                            match self.handle_message(text).await {
                                Ok(response) => {
                                    if let Some(resp_text) = response {
                                        write.send(Message::Text(resp_text.to_string())).await
                                            .with_context(|| "Failed to send response")?;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to handle message: {}", e);
                                }
                            }
                        }
                        Ok(Message::Binary(_data)) => {
                            debug!("Received binary message (not implemented)");
                        }
                        Ok(Message::Ping(payload)) => {
                            debug!("Received ping, sending pong");
                            write.send(Message::Pong(payload)).await?;
                        }
                        Ok(Message::Pong(_)) => {
                            debug!("Received pong");
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket close message received");
                            break;
                        }
                        Ok(Message::Frame(_)) => {
                            debug!("Received frame message (not implemented)");
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    debug!("Sending ping");
                    write.send(Message::Ping(vec![])).await
                        .with_context(|| "Failed to send ping")?;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&self, message: String) -> Result<Option<Message>> {
        info!("Handling WebSocket message: {}", message);
        
        // Try to parse as JSON to check message type
        let json_value: serde_json::Value = serde_json::from_str(&message)
            .with_context(|| "Failed to parse JSON message")?;
        
        // Check if this is a welcome message
        if let Some(msg_type) = json_value.get("type").and_then(|v| v.as_str()) {
            if msg_type == "welcome" {
                info!("Received welcome message, ignoring");
                return Ok(None); // Don't respond to welcome messages
            }
        }
        
        // Parse as command request
        let request: CommandRequest = serde_json::from_value(json_value)
            .with_context(|| "Failed to parse command request")?;

        info!("Parsed command: {:?}", request.command);

        match handle_command(request.command, Some(request.command_id.clone()), &self.plugin_manager, &self.config.allowed_commands, self.config.max_file_size).await {
            Ok(AgentResponse::Json(resp)) => {
                let text = serde_json::to_string(&resp)?;
                Ok(Some(Message::Text(text)))
            }
            Ok(AgentResponse::Binary { command_id, data }) => {
                // Формируем бинарный пакет: [36 байт ID][Данные]
                let mut packet = command_id.into_bytes();
                packet.extend(data);
                Ok(Some(Message::Binary(packet)))
            }
            Err(e) => {
                error!("Command execution failed: {}", e);
                
                let error_response = CommandResponse {
                    command_id: None,
                    r#type: "error".to_string(),
                    status: "error".to_string(),
                    data: serde_json::json!({
                        "error": e.to_string()
                    }),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                };
                
                let response_text = serde_json::to_string(&error_response)
                    .with_context(|| "Failed to serialize error response")?;
                Ok(Some(Message::Text(response_text)))
            }
        }
    }
}
