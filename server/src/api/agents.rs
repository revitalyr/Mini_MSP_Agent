//! Agent management endpoints
//! 
//! Управление подключенными агентами

use axum::{extract::{State, Path}, response::Json, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

/// List all connected agents
pub async fn list_agents(State(app_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = app_state.agents.lock().unwrap();
    
    let agents_list: Vec<_> = agents.iter()
        .map(|(id, info)| json!({
            "id": id,
            "status": info,
            "last_seen": "now"
        }))
        .collect();
    
    Json(json!({
        "agents": agents_list,
        "count": agents_list.len()
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
