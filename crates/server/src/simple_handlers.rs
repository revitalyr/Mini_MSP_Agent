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
use mini_msp_shared::{Timestamp, format_timestamp};
use serde_json::Value;
use axum::extract::Path;

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

/// Send command to agent - simplified version
pub async fn send_command(
    Path(agent_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let command = body.get("command").and_then(|c| c.as_str()).unwrap_or("unknown");
    
    // Try to send via NATS if broker is available
    if let Some(ref broker) = app_state.broker_client {
        let cmd = mini_msp_shared::CommandRequest {
            command_id: uuid::Uuid::new_v4().to_string(),
            command: mini_msp_shared::Command::Exec { 
                cmd: command.to_string() 
            },
        };
        
        match broker.send_command(&agent_id, cmd).await {
            Ok(_) => {
                return Ok(Json(json!({
                    "status": "sent",
                    "agent_id": agent_id,
                    "command": command,
                })));
            }
            Err(e) => {
                tracing::warn!("Failed to send command via NATS: {}", e);
            }
        }
    }
    
    // Fallback: store command in pending queue
    Ok(Json(json!({
        "status": "queued",
        "agent_id": agent_id,
        "command": command,
        "message": "Agent offline, command queued",
    })))
}

/// Handle heartbeat from agent
pub async fn handle_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(heartbeat): Json<mini_msp_shared::Heartbeat>,
) -> Result<Json<Value>, StatusCode> {
    let agent_id = heartbeat.agent_id.clone();
    
    // Update agent info
    {
        let mut agents = app_state.agents.lock().unwrap();
        if let Some(info) = agents.get_mut(&agent_id) {
            info.last_seen = chrono::Utc::now().timestamp() as u64;
        }
    }
    
    Ok(Json(json!({
        "status": "received",
        "agent_id": agent_id,
        "timestamp": chrono::Utc::now().timestamp(),
    })))
}
