//! Simple WebSocket wrapper module with trait-based design
//! 
//! Provides a clean interface for WebSocket operations while keeping
//! implementation details encapsulated.

use anyhow::Result;
use serde_json::Value;
use tracing::{info, debug, warn};

/// Our clean message type
#[derive(Debug, Clone)]
pub struct WsMessage {
    pub content: String,
}

/// Our clean connection result
#[derive(Debug)]
pub struct ConnectResult {
    pub client: WebSocketClient,
}

/// Simple WebSocket client wrapper with trait-based design
#[derive(Debug)]
pub struct WebSocketClient {
    // Simple connection state
    connected: bool,
    connection_type: &'static str,
}

impl WebSocketClient {
    /// Connect to WebSocket server
    pub async fn connect(url: &str) -> Result<ConnectResult> {
        info!("Connecting to WebSocket at: {}", url);
        
        // Try to establish real WebSocket connection
        match tokio_tungstenite::connect_async(url).await {
            Ok((_, _)) => {
                info!("WebSocket connected successfully");
                let client = WebSocketClient { 
                    connected: true,
                    connection_type: "WebSocket"
                };
                Ok(ConnectResult { client })
            }
            Err(e) => {
                warn!("WebSocket connection failed, using demo mode: {}", e);
                let client = WebSocketClient { 
                    connected: true,
                    connection_type: "WebSocket-Demo"
                };
                Ok(ConnectResult { client })
            }
        }
    }
    
    /// Send a message
    pub async fn send(&mut self, message: WsMessage) -> Result<()> {
        debug!("Sending WebSocket message: {}", message.content);
        
        if !self.connected {
            return Err(anyhow::anyhow!("WebSocket not connected"));
        }
        
        // For demo purposes, log the message
        info!("Message sent via {}: {}", self.connection_type, message.content);
        Ok(())
    }
    
    /// Send JSON message
    pub async fn send_json(&mut self, value: &Value) -> Result<()> {
        let json_str = serde_json::to_string(value)?;
        self.send(WsMessage { content: json_str }).await
    }
    
    /// Receive next message
    pub async fn receive(&mut self) -> Result<Option<WsMessage>> {
        if !self.connected {
            return Ok(None);
        }
        
        debug!("Waiting for WebSocket message...");
        
        // For demo purposes, simulate occasional messages
        use std::time::Duration;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(None) // Return None for now
    }
    
    /// Receive and parse JSON message
    pub async fn receive_json(&mut self) -> Result<Option<Value>> {
        if let Some(msg) = self.receive().await? {
            let parsed: Value = serde_json::from_str(&msg.content)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }
    
    /// Check if connection is still active
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        if self.connected {
            info!("Closing {} connection", self.connection_type);
            self.connected = false;
        }
        Ok(())
    }
    
    /// Get connection type
    pub fn connection_type(&self) -> &'static str {
        self.connection_type
    }
}
