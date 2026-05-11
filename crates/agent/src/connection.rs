//! Connection traits and implementations
//!
//! Provides abstraction layer for different types of connections (WebSocket, HTTP, etc.)

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::websocket::{WebSocketClient, WsMessage};

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

/// WebSocket connection implementation with real message handling
pub struct WebSocketConnection {
    client: WebSocketClient,
    msg_rx: mpsc::Receiver<WsMessage>,
}

impl WebSocketConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        info!("Connecting WebSocket to: {}", url);

        let crate::websocket::ConnectResult { client, msg_rx } =
            WebSocketClient::connect(url).await?;

        info!("WebSocket connected successfully");
        Ok(Self { client, msg_rx })
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    async fn send(&mut self, message: &str) -> Result<()> {
        self.client
            .send(WsMessage {
                content: message.to_string(),
            })
            .await
    }

    async fn receive(&mut self) -> Result<Option<String>> {
        match self.client.receive_json().await? {
            Some(value) => Ok(Some(value.to_string())),
            None => Ok(None),
        }
    }

    fn is_connected(&self) -> bool {
        // Note: This is synchronous, but WebSocketClient::is_connected is async
        // For simplicity, we assume connected until explicitly closed
        true
    }

    async fn close(&mut self) -> Result<()> {
        info!("Closing WebSocket connection");
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
        WebSocketConnection::connect(url).await
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
