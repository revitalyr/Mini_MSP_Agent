//! WebSocket Client with Real Message Handling
//!
//! Provides actual WebSocket send/receive functionality

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

/// WebSocket message type
#[derive(Debug, Clone)]
pub struct WsMessage {
    pub content: String,
}

/// Connection result with sender channel
pub struct ConnectResult {
    pub client: WebSocketClient,
    pub msg_rx: mpsc::Receiver<WsMessage>,
}

/// Message receiver for WebSocket
pub struct WsReceiver {
    rx: mpsc::Receiver<WsMessage>,
}

impl WsReceiver {
    pub async fn receive(&mut self) -> Result<Option<WsMessage>> {
        Ok(self.rx.recv().await)
    }

    pub async fn receive_json(&mut self) -> Result<Option<Value>> {
        match self.rx.recv().await {
            Some(msg) => {
                let parsed: Value = serde_json::from_str(&msg.content)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
}

/// Real WebSocket client
pub struct WebSocketClient {
    sender: mpsc::Sender<String>,
    msg_rx: mpsc::Receiver<WsMessage>,
    connected: Arc<Mutex<bool>>,
}

impl WebSocketClient {
    /// Connect to WebSocket server
    pub async fn connect(url: &str) -> Result<ConnectResult> {
        info!("Connecting to WebSocket at: {}", url);

        let (ws_stream, _) = tokio_tungstenite::connect_async(url).await.map_err(|e| {
            anyhow!("WebSocket connection failed: {}", e)
        })?;

        info!("WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<String>(100);
        let (msg_tx, msg_rx) = mpsc::channel::<WsMessage>(100);
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();

        // Spawn task for sending messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await {
                    error!("WebSocket send error: {}", e);
                    break;
                }
            }
            let mut conn = connected_clone.lock().await;
            *conn = false;
        });

        // Spawn task for receiving messages
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        let _ = msg_tx.send(WsMessage { content: text }).await;
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                        info!("WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket receive error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        let client = WebSocketClient {
            sender: tx,
            msg_rx,
            connected,
        };

        Ok(ConnectResult { client, msg_rx: mpsc::channel(1).1 })  // dummy receiver, using client's instead
    }

    /// Send a message
    pub async fn send(&mut self, message: WsMessage) -> Result<()> {
        self.sender
            .send(message.content)
            .await
            .map_err(|e| anyhow!("Failed to send message: {}", e))
    }

    /// Send JSON message
    pub async fn send_json(&mut self, value: &Value) -> Result<()> {
        let json_str = serde_json::to_string(value)?;
        self.send(WsMessage { content: json_str }).await
    }

    /// Check if connection is still active
    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    /// Get connection type
    pub fn connection_type(&self) -> &'static str {
        "WebSocket"
    }

    /// Receive a message
    pub async fn receive(&mut self) -> Result<Option<WsMessage>> {
        match self.msg_rx.recv().await {
            Some(msg) => Ok(Some(msg)),
            None => Ok(None),
        }
    }

    /// Receive and parse JSON message
    pub async fn receive_json(&mut self) -> Result<Option<Value>> {
        match self.msg_rx.recv().await {
            Some(msg) => {
                let parsed: Value = serde_json::from_str(&msg.content)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        let mut conn = self.connected.lock().await;
        *conn = false;
        info!("WebSocket connection closed");
        Ok(())
    }
}
