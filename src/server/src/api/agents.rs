//! Agent management endpoints
//! 
//! Управление подключенными агентами

use axum::{extract::{State, Path}, response::Json, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

/// Timeout in seconds to consider agent offline (2 minutes)
const AGENT_TIMEOUT_SECS: i64 = 120;

/// Calculate agent status based on last_seen timestamp
fn calculate_status(last_seen: u64) -> &'static str {
    let now = chrono::Utc::now().timestamp() as u64;
    if (now - last_seen) < AGENT_TIMEOUT_SECS as u64 {
        "online"
    } else {
        "offline"
    }
}

/// List all connected agents with online/offline status
pub async fn list_agents(State(app_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = app_state.agents.lock().unwrap();
    let now = chrono::Utc::now().timestamp() as u64;
    
    let agents_list: Vec<_> = agents.iter()
        .map(|(id, info)| {
            let status = calculate_status(info.last_seen);
            let seconds_ago = now - info.last_seen;
            json!({
                "id": id,
                "hostname": info.hostname,
                "platform": info.platform,
                "version": info.version,
                "status": status,
                "last_seen": info.last_seen,
                "seconds_ago": seconds_ago
            })
        })
        .collect();
    
    Json(json!({
        "agents": agents_list,
        "count": agents_list.len(),
        "online_count": agents_list.iter().filter(|a| a["status"] == "online").count(),
        "offline_count": agents_list.iter().filter(|a| a["status"] == "offline").count()
    }))
}

/// Send command to specific agent
pub async fn send_command(
    Path(agent_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<Value>
) -> Result<Json<Value>, StatusCode> {
    let _command = payload.get("command")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    // Check if agent exists
    let agents = app_state.agents.lock().unwrap();
    if !agents.contains_key(&agent_id) {
        return Ok(Json(json!({
            "success": false,
            "error": "Agent not found"
        })));
    }
    
    // Forward command to agent via broker or WebSocket
    // For now, return error indicating command should be sent via WebSocket
    Ok(Json(json!({
        "success": false,
        "error": "Commands should be sent via WebSocket connection",
        "websocket_endpoint": "/ws"
    })))
}
