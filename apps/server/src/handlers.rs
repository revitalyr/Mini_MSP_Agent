//! Simple modular server for Mini MSP Agent
//! 
//! Optimized modular structure with fast compilation

use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
    http::StatusCode,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

use crate::config::Config;
use crate::broker::BrokerClient;

// Simple AppState for fast compilation
#[derive(Clone)]
pub struct AppState {
    pub agents: Mutex<std::collections::HashMap<String, String>>,
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

/// Handle plugin events
pub async fn handle_plugin_event(
    State(_app_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Received plugin event: {:?}", payload);
    Ok(Json(json!({
        "status": "processed",
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    })))
}
