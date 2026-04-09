use axum::{
    extract::{Path, State, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::nats_client::NatsClient;

pub struct AppState {
    pub nats: NatsClient,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub agent_id: String,
    pub command: String,
    pub params: Option<serde_json::Value>,
}

pub async fn handle_command(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CommandRequest>,
) -> Json<serde_json::Value> {
    let command = if let Some(params) = request.params {
        json!({
            "type": request.command,
            "params": params
        })
    } else {
        json!({ "type": request.command })
    };

    match state.nats.send_command(&request.agent_id, command).await {
        Ok(response) => Json(response),
        Err(e) => Json(json!({
            "error": e.to_string(),
            "status": "failed"
        })),
    }
}

pub async fn list_agents(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = state.nats.get_agents().await;
    Json(json!({ "agents": agents }))
}

pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.nats.send_command(&agent_id, json!({ "type": "get_metrics" })).await {
        Ok(response) => Json(response),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.nats.send_command(&agent_id, json!({ "type": "get_plugin_registry" })).await {
        Ok(response) => Json(response),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    Query(params): Query<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    
    match state.nats.send_command(&agent_id, json!({
        "type": "get_directory_info",
        "path": path
    })).await {
        Ok(response) => Json(response),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    Json(params): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    match state.nats.send_command(&agent_id, json!({
        "type": "upload_file",
        "params": params
    })).await {
        Ok(response) => Json(response),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
