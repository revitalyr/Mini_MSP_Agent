//! HTTP handlers модуль
//! 
//! Общие обработчики HTTP запросов

use axum::{extract::State, response::Json, http::StatusCode};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::AppState;

/// Handle heartbeat from agents
pub async fn handle_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agent_id = payload.get("agent_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Update agent last seen
    {
        let mut agents = app_state.agents.lock().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.last_seen = chrono::Utc::now();
            agent.status = "online".to_string();
        }
    }

    Ok(Json(json!({
        "status": "ack",
        "timestamp": chrono::Utc::now().timestamp()
    })))
}

/// Handle plugin events
pub async fn handle_plugin_event(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Broadcast plugin event to all connected WebSocket clients
    let broadcast_msg = json!({
        "type": "plugin_event",
        "data": payload,
        "timestamp": chrono::Utc::now().timestamp()
    });

    let mut ws_manager = app_state.ws_manager.lock().await;
    ws_manager.broadcast(&broadcast_msg).await;

    Ok(Json(json!({
        "status": "processed",
        "timestamp": chrono::Utc::now().timestamp()
    })))
}
