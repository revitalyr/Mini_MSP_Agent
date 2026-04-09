use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, debug};

use crate::nats_client::NatsClient;
use crate::api::AppState;

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Start message handler
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received WS message: {}", text);
                    
                    if let Err(e) = handle_client_message(&text, &state_clone, &tx).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    sender.send(Message::Pong(data)).await.ok();
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                _ => {}
            }
        }
    });

    // Send messages to client
    while let Some(msg) = rx.recv().await {
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

async fn handle_client_message(
    message: &str,
    state: &Arc<AppState>,
    tx: &mpsc::Sender<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg: serde_json::Value = serde_json::from_str(message)?;
    
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "subscribe_metrics" => {
            let agent_id = msg.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
            
            // Subscribe to agent metrics
            if !agent_id.is_empty() {
                let mut heartbeat_sub = state.nats.subscribe_heartbeats().await?;
                
                tokio::spawn(async move {
                    while let Some(msg) = heartbeat_sub.next().await {
                        if let Ok(msg) = msg {
                            let subject = msg.subject.as_str();
                            if subject.starts_with(&format!("heartbeat.{}", agent_id)) {
                                let heartbeat: serde_json::Value = serde_json::from_slice(&msg.payload)?;
                                tx.send(Message::Text(serde_json::to_string(&heartbeat)?)).await?;
                            }
                        }
                    }
                    Ok::<_, Box<dyn std::error::Error>>(())
                });
            }
        }

        "execute_command" => {
            let agent_id = msg.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
            let command = msg.get("command").cloned().unwrap_or(json!({}));
            
            if !agent_id.is_empty() {
                match state.nats.send_command(agent_id, command).await {
                    Ok(response) => {
                        tx.send(Message::Text(serde_json::to_string(&response)?)).await?;
                    }
                    Err(e) => {
                        tx.send(Message::Text(serde_json::json!({
                            "type": "error",
                            "error": e.to_string()
                        }).to_string())).await?;
                    }
                }
            }
        }

        "list_agents" => {
            let agents = state.nats.get_agents().await;
            tx.send(Message::Text(serde_json::json!({
                "type": "agents_list",
                "agents": agents
            }).to_string())).await?;
        }

        _ => {
            error!("Unknown message type: {}", msg_type);
        }
    }

    Ok(())
}
