//! Simple handlers for Mini MSP Server
//! 
//! Optimized for fast compilation

use axum::{
    extract::State,
    response::Json,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::broker::BrokerClient;

// Simple AppState for fast compilation
#[derive(Clone)]
pub struct AppState {
    pub agents: Arc<Mutex<HashMap<String, String>>>,
    pub broker_client: Option<Arc<BrokerClient>>,
}

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
