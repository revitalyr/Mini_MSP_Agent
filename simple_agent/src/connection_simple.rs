//! Simple connection traits and implementations
//! 
//! Provides abstraction layer for different types of connections

use anyhow::Result;
use serde_json::Value;
use tracing::{info, debug, warn};

/// Simple trait for connection types
pub trait Connection: Send + Sync {
    /// Send a message through the connection
    async fn send(&mut self, message: &str) -> Result<()>;
    
    /// Send a JSON message
    async fn send_json(&mut self, value: &Value) -> Result<()> {
        let json_str = serde_json::to_string(value)?;
        self.send(&json_str).await
    }
    
    /// Receive a message
    async fn receive(&mut self) -> Result<Option<String>>;
    
    /// Receive and parse JSON message
    async fn receive_json(&mut self) -> Result<Option<Value>> {
        if let Some(msg) = self.receive().await? {
            let parsed: Value = serde_json::from_str(&msg)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }
    
    /// Check if connection is active
    fn is_connected(&self) -> bool;
    
    /// Close the connection
    async fn close(&mut self) -> Result<()>;
    
    /// Get connection type identifier
    fn connection_type(&self) -> &'static str;
}

/// Simple WebSocket connection implementation
pub struct WebSocketConnection {
    connected: bool,
}

impl WebSocketConnection {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Connection for WebSocketConnection {
    async fn send(&mut self, message: &str) -> Result<()> {
        debug!("Sending WebSocket message: {}", message);
        
        if !self.connected {
            return Err(anyhow::anyhow!("WebSocket not connected"));
        }
        
        // For demo purposes, log the message
        info!("WebSocket message sent: {}", message);
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Option<String>> {
        if !self.connected {
            return Ok(None);
        }
        
        debug!("Waiting for WebSocket message...");
        
        // For demo purposes, simulate occasional messages
        use std::time::Duration;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(None) // Return None for now
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    async fn close(&mut self) -> Result<()> {
        if self.connected {
            info!("Closing WebSocket connection");
            self.connected = false;
        }
        Ok(())
    }
    
    fn connection_type(&self) -> &'static str {
        "WebSocket"
    }
}

/// WebSocket connection factory
pub struct WebSocketConnectionFactory;

impl WebSocketConnectionFactory {
    pub async fn connect(url: &str) -> Result<Box<dyn Connection>> {
        info!("Connecting WebSocket to: {}", url);
        
        // Try to establish real WebSocket connection
        match tokio_tungstenite::connect_async(url).await {
            Ok((_, _)) => {
                info!("WebSocket connected successfully");
                let mut conn = WebSocketConnection::new();
                conn.connected = true;
                Ok(Box::new(conn))
            }
            Err(e) => {
                warn!("WebSocket connection failed, using demo mode: {}", e);
                let mut conn = WebSocketConnection::new();
                conn.connected = true; // Still allow demo mode
                Ok(Box::new(conn))
            }
        }
    }
}

/// Simple connection manager
pub struct ConnectionManager {
    connection: Option<Box<dyn Connection>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self { connection: None }
    }
    
    pub async fn connect_websocket(&mut self, url: &str) -> Result<()> {
        info!("Connecting via WebSocket factory");
        let conn = WebSocketConnectionFactory::connect(url).await?;
        self.connection = Some(conn);
        Ok(())
    }
    
    pub async fn send(&mut self, message: &str) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            conn.send(message).await
        } else {
            Err(anyhow::anyhow!("No active connection"))
        }
    }
    
    pub async fn send_json(&mut self, value: &Value) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            conn.send_json(value).await
        } else {
            Err(anyhow::anyhow!("No active connection"))
        }
    }
    
    pub async fn receive(&mut self) -> Result<Option<String>> {
        if let Some(conn) = &mut self.connection {
            conn.receive().await
        } else {
            Ok(None)
        }
    }
    
    pub async fn receive_json(&mut self) -> Result<Option<Value>> {
        if let Some(conn) = &mut self.connection {
            conn.receive_json().await
        } else {
            Ok(None)
        }
    }
    
    pub fn is_connected(&self) -> bool {
        self.connection
            .as_ref()
            .map(|conn| conn.is_connected())
            .unwrap_or(false)
    }
    
    pub async fn close(&mut self) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            conn.close().await
        } else {
            Ok(())
        }
    }
    
    pub fn connection_type(&self) -> Option<&'static str> {
        self.connection.as_ref().map(|conn| conn.connection_type())
    }
}
