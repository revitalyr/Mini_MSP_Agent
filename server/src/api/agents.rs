//! Agent management endpoints
//! 
//! Управление подключенными агентами

use axum::{extract::State, response::Json};
use serde_json::json;
use std::collections::HashMap;
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
