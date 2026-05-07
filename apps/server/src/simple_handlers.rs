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
use crate::semantic_types::{Timestamp, format_timestamp};

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
