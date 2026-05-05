//! Simple Handlers for Mini MSP Server
//! 
//! Optimized for fast compilation with semantic type safety.

use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde_json::json;
use std::sync::Arc;

use crate::AppState;
use mini_msp_shared::AgentInfo;
use crate::semantic_types::{Timestamp, Duration, format_timestamp};

/// Simple health check with semantic timestamp
pub async fn health_check() -> Json<serde_json::Value> {
    let timestamp: Timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as Timestamp;
    
    Json(json!({
        "status": "ok",
        "timestamp": timestamp,
        "timestamp_formatted": format_timestamp(timestamp)
    }))
}

// Simple agents list
pub async fn list_agents(State(app_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = app_state.agents.lock().unwrap();
    
    let agents_list: Vec<_> = agents.iter()
        .map(|(id, info)| {
            json!({
                "id": id,
                "hostname": info.hostname,
                "version": info.version,
                "platform": info.platform,
            })
        })
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
    
    // Extract agent info from payload with fallbacks
    let hostname = payload.get("hostname")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    let platform = payload.get("platform")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    let version = payload.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");
    
    // Update agent
    {
        let mut agents = app_state.agents.lock().unwrap();
        let agent = AgentInfo {
            id: agent_id.to_string(),
            hostname: hostname.to_string(),
            version: version.to_string(),
            platform: platform.to_string(),
            last_seen: chrono::Utc::now().timestamp() as Timestamp,
        };
        agents.insert(agent_id.to_string(), agent);
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

// Browse directory handler
pub async fn browse_directory(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = body.get("path").and_then(|p| p.as_str()).unwrap_or("/home");
    
    // List directory contents
    let mut entries = vec![];
    if let Ok(dir) = std::fs::read_dir(path) {
        for entry in dir.flatten() {
            if let Ok(metadata) = entry.metadata() {
                let file_type = if metadata.is_dir() {
                    "directory"
                } else if metadata.is_file() {
                    "file"
                } else {
                    "other"
                };
                
                entries.push(json!({
                    "name": entry.file_name().to_string_lossy().to_string(),
                    "path": entry.path().to_string_lossy().to_string(),
                    "type": file_type,
                    "size": metadata.len(),
                }));
            }
        }
    }
    
    Ok(Json(json!({
        "path": path,
        "entries": entries,
        "count": entries.len(),
    })))
}
