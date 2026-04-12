//! Connection traits and implementations
//! 
//! Provides abstraction layer for different types of connections (WebSocket, HTTP, etc.)

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tracing::{info, debug, warn};

/// Trait for connection types that can send and receive messages
#[async_trait]
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

/// Trait for connection factories
#[async_trait]
pub trait ConnectionFactory: Send + Sync {
    type Connection: Connection;
    
    /// Create a new connection
    async fn connect(&self, url: &str) -> Result<Self::Connection>;
    
    /// Get factory name
    fn factory_name(&self) -> &'static str;
}

/// WebSocket connection implementation
pub struct WebSocketConnection {
    connected: bool,
    // For now, we'll use a simple implementation
    // In future, this would contain actual WebSocket stream
}

impl WebSocketConnection {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

#[async_trait]
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

#[async_trait]
impl ConnectionFactory for WebSocketConnectionFactory {
    type Connection = WebSocketConnection;
    
    async fn connect(&self, url: &str) -> Result<Self::Connection> {
        info!("Connecting WebSocket to: {}", url);
        
        // Try to establish real WebSocket connection
        match tokio_tungstenite::connect_async(url).await {
            Ok((_, _)) => {
                info!("WebSocket connected successfully");
                let mut conn = WebSocketConnection::new();
                conn.connected = true;
                Ok(conn)
            }
            Err(e) => {
                warn!("WebSocket connection failed, using demo mode: {}", e);
                let mut conn = WebSocketConnection::new();
                conn.connected = true; // Still allow demo mode
                Ok(conn)
            }
        }
    }
    
    fn factory_name(&self) -> &'static str {
        "WebSocketFactory"
    }
}

/// HTTP connection implementation (alternative to WebSocket)
pub struct HttpConnection {
    base_url: String,
    client: reqwest::Client,
}

impl HttpConnection {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Connection for HttpConnection {
    async fn send(&mut self, message: &str) -> Result<()> {
        debug!("Sending HTTP message: {}", message);
        
        // Send via HTTP POST
        let response = self.client
            .post(&format!("{}/api/message", self.base_url))
            .body(message.to_string())
            .send()
            .await?;
        
        if response.status().is_success() {
            info!("HTTP message sent successfully");
        } else {
            warn!("HTTP message failed: {}", response.status());
        }
        
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Option<String>> {
        // Poll for messages via HTTP GET
        match self.client
            .get(&format!("{}/api/messages", self.base_url))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let text = response.text().await?;
                    Ok(Some(text))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }
    
    fn is_connected(&self) -> bool {
        true // HTTP is stateless
    }
    
    async fn close(&mut self) -> Result<()> {
        info!("Closing HTTP connection");
        Ok(())
    }
    
    fn connection_type(&self) -> &'static str {
        "HTTP"
    }
}

/// HTTP connection factory
pub struct HttpConnectionFactory {
    base_url: String,
}

impl HttpConnectionFactory {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl ConnectionFactory for HttpConnectionFactory {
    type Connection = HttpConnection;
    
    async fn connect(&self, _url: &str) -> Result<Self::Connection> {
        info!("Creating HTTP connection to: {}", self.base_url);
        Ok(HttpConnection::new(self.base_url.clone()))
    }
    
    fn factory_name(&self) -> &'static str {
        "HTTPFactory"
    }
}

/// Connection manager for handling different connection types
pub struct ConnectionManager {
    connection: Option<Box<dyn Connection>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self { connection: None }
    }
    
    pub async fn connect(&mut self, factory: &dyn ConnectionFactory<Connection = Box<dyn Connection>>, url: &str) -> Result<()> {
        info!("Connecting using factory: {}", factory.factory_name());
        let conn = factory.connect(url).await?;
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
