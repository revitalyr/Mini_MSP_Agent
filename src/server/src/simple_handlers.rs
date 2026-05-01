//! Simple handlers for Mini MSP Server
//! 
//! Optimized for fast compilation

use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

// Simple health check
pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

// Simple agents list
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

// Simple heartbeat handler
pub async fn handle_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agent_id = payload.get("agent_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Update agent
    {
        let mut agents = app_state.agents.lock().unwrap();
        agents.insert(agent_id.to_string(), "online".to_string());
    }
    
    Ok(Json(json!({
        "status": "ack",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    })))
}

// Simple command handler
pub async fn send_command(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
    Json(command): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agents = app_state.agents.lock().unwrap();
    
    // Check if agent exists and is connected
    if !agents.contains_key(&agent_id) {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // For now, just return a response indicating the command was received
    // In a real implementation, you'd forward this to the agent via WebSocket
    Ok(Json(json!({
        "status": "command_sent",
        "agent_id": agent_id,
        "command": command,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    })))
}
