//! Agent management endpoints
//! 
//! Управление подключенными агентами

use axum::{extract::State, response::Json, http::StatusCode};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::AppState;

/// List all connected agents
pub async fn list_agents(State(app_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = app_state.agents.lock().await;
    
    let agents_list: Vec<_> = agents.iter()
        .map(|(id, info)| json!({
            "id": id,
            "name": info.name,
            "status": info.status,
            "last_seen": info.last_seen,
            "system_info": info.system_info
        }))
        .collect();
    
    Json(json!({
        "agents": agents_list,
        "count": agents_list.len()
    }))
}
