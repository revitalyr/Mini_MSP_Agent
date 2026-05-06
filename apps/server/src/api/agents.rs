//! Agent Management Endpoints
//! 
//! Manages connected agents and their status.

use axum::{extract::{State, Path}, response::Json, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

use crate::{AppState, websocket::send_command_to_agent};
use crate::semantic_types::{Duration, Timestamp};
use crate::api::docs::{AgentList, CommandRequest, CommandResponse, ErrorResponse};

/// Error response helper
fn error_json(message: &str) -> Json<Value> {
    Json(json!({
        "status": "error",
        "error": message
    }))
}

/// Timeout duration to consider agent offline (2 minutes)
const AGENT_TIMEOUT_SECS: Duration = 120;

/// Calculate agent status based on last_seen timestamp
fn calculate_status(last_seen: Timestamp) -> &'static str {
    let now = chrono::Utc::now().timestamp() as Timestamp;
    if (now - last_seen) < AGENT_TIMEOUT_SECS {
        "online"
    } else {
        "offline"
    }
}

/// List all connected agents with online/offline status
#[utoipa::path(
    get,
    path = "/agents",
    tag = "agents",
    responses(
        (status = 200, description = "List of agents", body = AgentList),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_agents(State(app_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = app_state.agents.lock().unwrap();
    let now: Timestamp = chrono::Utc::now().timestamp() as Timestamp;
    
    let agents_list: Vec<_> = agents.iter()
        .map(|(id, info)| {
            let status = calculate_status(info.last_seen);
            let seconds_ago: Duration = now - info.last_seen;
            json!({
                "id": id,
                "hostname": info.hostname,
                "platform": info.platform,
                "version": info.version,
                "status": status,
                "last_seen": info.last_seen,
                "seconds_ago": seconds_ago
            })
        })
        .collect();
    
    Json(json!({
        "agents": agents_list,
        "count": agents_list.len(),
        "online_count": agents_list.iter().filter(|a| a["status"] == "online").count(),
        "offline_count": agents_list.iter().filter(|a| a["status"] == "offline").count()
    }))
}

/// Send command to specific agent
#[utoipa::path(
    post,
    path = "/agents/{agent_id}/command",
    tag = "commands",
    params(
        ("agent_id" = String, Path, description = "Agent UUID")
    ),
    request_body = CommandRequest,
    responses(
        (status = 200, description = "Command executed", body = CommandResponse),
        (status = 404, description = "Agent not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn send_command(
    Path(agent_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<Value>
) -> Result<Json<Value>, StatusCode> {
    // Check if agent exists (scope the lock)
    {
        let agents = app_state.agents.lock().unwrap();
        if !agents.contains_key(&agent_id) {
            return Ok(Json(json!({
                "success": false,
                "error": "Agent not found"
            })));
        }
    } // lock released here
    
    // Extract command type
    let command_type = payload.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Try Boost.DLL plugins first (local execution on server)
    if let Ok(guard) = app_state.boost_plugin_registry.try_lock() {
        if let Some(ref registry) = *guard {
            let boost_result = registry.execute_command_auto(command_type, Some(payload.clone()));
            if let Ok(response) = boost_result {
                // Check if plugin successfully handled the command
                if let Some(success) = response.get("success").and_then(|v| v.as_bool()) {
                    if success {
                        info!("Command '{}' executed via Boost.DLL plugin", command_type);
                        return Ok(Json(json!({
                            "success": true,
                            "agent_id": agent_id,
                            "source": "boost_plugin",
                            "status": "completed",
                            "data": response.get("data").cloned().unwrap_or(json!({})),
                            "command": command_type,
                        })));
                    } else {
                        // Plugin handled command but returned error - still valid response
                        let error_msg = response.get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Plugin returned error");
                        return Ok(Json(json!({
                            "success": false,
                            "agent_id": agent_id,
                            "source": "boost_plugin",
                            "error": error_msg,
                            "command": command_type,
                        })));
                    }
                }
            }
        }
    }
    
    // Fallback: Forward command to agent via broker or WebSocket
    let command = json!({
        "type": "command",
        "agent_id": &agent_id,
        "command": command_type,
        "params": payload.get("data").cloned().unwrap_or(json!({})),
        "payload": payload
    });
    
    match send_command_to_agent(&agent_id, &app_state, command).await {
        Some(response) => {
            if response.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(Json(json!({
                    "success": true,
                    "agent_id": agent_id,
                    "source": "agent",
                    "status": "completed",
                    "data": response.get("data").cloned().unwrap_or(json!({})),
                    "command": command_type,
                })))
            } else {
                Ok(Json(json!({
                    "success": false,
                    "agent_id": agent_id,
                    "source": "agent",
                    "error": response.get("error").and_then(|v| v.as_str()).unwrap_or("Command failed"),
                    "command": command_type,
                })))
            }
        }
        None => Ok(Json(json!({
            "success": false,
            "agent_id": agent_id,
            "error": "Agent not connected or did not respond",
            "command": command_type,
        })))
    }
}

/// Get list of available objects from plugin on agent
pub async fn get_plugin_objects(
    Path((agent_id, plugin_name)): Path<(String, String)>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // Send command to agent to get available objects from plugin
    let command = json!({
        "type": "command",
        "command": "get_available_objects",
        "params": {
            "plugin": plugin_name
        },
        "agent_id": agent_id
    });

    match send_command_to_agent(&agent_id, &app_state, command).await {
        Some(response) => {
            if response.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                let data = response.get("data").cloned().unwrap_or(json!({}));
                Ok(Json(json!({
                    "status": "ok",
                    "plugin": plugin_name,
                    "objects": data.get("objects").cloned().unwrap_or(json!([])),
                    "object_type": data.get("object_type").and_then(|v| v.as_str()).unwrap_or("item")
                })))
            } else {
                Ok(error_json(&format!(
                    "Agent returned error: {}",
                    response.get("error").and_then(|v| v.as_str()).unwrap_or("unknown")
                )))
            }
        }
        None => Ok(error_json("Agent not connected or did not respond")),
    }
}

/// Get data for specific object from plugin on agent
pub async fn get_plugin_object_data(
    Path((agent_id, plugin_name, object_id)): Path<(String, String, String)>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // Send command to agent to get object data from plugin
    let command = json!({
        "type": "command",
        "command": "get_object_data",
        "params": {
            "plugin": plugin_name,
            "object_id": object_id
        },
        "agent_id": agent_id
    });

    match send_command_to_agent(&agent_id, &app_state, command).await {
        Some(response) => {
            if response.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                let data = response.get("data").cloned().unwrap_or(json!({}));
                Ok(Json(data))
            } else {
                Ok(error_json(&format!(
                    "Agent returned error: {}",
                    response.get("error").and_then(|v| v.as_str()).unwrap_or("unknown")
                )))
            }
        }
        None => Ok(error_json("Agent not connected or did not respond")),
    }
}
