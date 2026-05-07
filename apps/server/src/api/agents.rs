//! Agent Management Endpoints
//! 
//! Manages connected agents and their status, and routes commands.

use axum::{extract::{State, Path}, response::Json, http::StatusCode};
use std::sync::Arc;
use serde_json::{json, Value};

use tracing::{info, debug, instrument};

use crate::broker::BrokerClient; // Import BrokerClient
use crate::{AppState};
use crate::semantic_types::{Duration, Timestamp};
use crate::api::docs::{AgentList, CommandRequest, CommandResponse, ErrorResponse};

/// Error response helper
fn error_json(message: &str) -> Json<Value> {
    Json(json!({
        "status": "error",
        "error": message
        // No need for supported_commands here, as this is a generic error
    }))
}

/// Timeout duration to consider agent offline (2 minutes)
const AGENT_TIMEOUT_SECS: Duration = 120;

/// Calculate agent status based on last_seen timestamp
pub fn calculate_status(last_seen: Timestamp) -> &'static str {
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
    
    // Extract command type from payload
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
    
    // All agent commands are now routed via NATS Request-Reply
    #[allow(clippy::redundant_clone)] // Payload might be used elsewhere
    let nats_payload = json!({
        "command": command_type,
        "params": payload.get("data").cloned().unwrap_or(json!({})),
    });

    send_agent_command_nats(Path(agent_id), State(app_state), Json(nats_payload)).await
}

#[instrument(skip(app_state, payload))]
/// Send command to specific agent via NATS Request-Reply
/// This is the preferred method for agent communication.
#[utoipa::path(
    post,
    path = "/api/agent/{agent_id}/command", // New endpoint for NATS-based commands
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
pub async fn send_agent_command_nats(
    Path(agent_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<Value>
) -> Result<Json<Value>, StatusCode> {
    let broker_client = app_state.broker_client.clone();
    let Some(broker) = broker_client else {
        return Ok(error_json("NATS broker not connected"));
    };

    // Check if agent exists and is online
    {
        let agents = app_state.agents.lock().unwrap();
        if let Some(agent_info) = agents.get(&agent_id) {
            if calculate_status(agent_info.last_seen) != "online" {
                return Ok(error_json("Agent is offline"));
            }
        } else {
            return Ok(error_json("Agent not found"));
        }
    }

    let command_name = payload.get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_command");

    let command_request = json!({
        "command": command_name,
        "params": payload.get("params").cloned().unwrap_or(json!({})),
    });

    let subject = format!("forensic.cmd.{}", agent_id);
    info!("Sending NATS request to subject: {}", subject);

    match broker.request(subject, command_request.to_string().into_bytes()).await {
        Ok(response_msg) => { // response_msg is async_nats::Message
            let data = if let Some(encoding) = response_msg.headers.as_ref().and_then(|h| h.get("Content-Encoding")) {
                if encoding.to_string() == "zstd" {
                    zstd::decode_all(response_msg.payload.as_ref()) // Use as_ref() for payload
                        .unwrap_or_else(|_| response_msg.payload.to_vec())
                } else {
                    response_msg.payload.to_vec()
                }
            } else {
                response_msg.payload.to_vec()
            };

            let response_str = std::str::from_utf8(&data).unwrap_or("{\"status\":\"error\",\"message\":\"Invalid UTF-8 response\"}");
            debug!("Received NATS response from agent {} (size: {} bytes)", agent_id, data.len());
            
            let value: Value = serde_json::from_str(response_str)
                .unwrap_or_else(|_| json!({"status": "error", "message": "Invalid JSON response from agent"}));
            Ok(Json(value))
        }
        Err(e) => {
            Ok(error_json(&format!("NATS request failed: {}", e)))
        }
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

    let command_payload = json!({
        "command": "get_available_objects",
        "params": {
            "plugin": plugin_name
        },
    });

    match send_agent_command_nats(Path(agent_id), State(app_state), Json(command_payload)).await {
        Ok(Json(response)) => {
            if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                Ok(Json(response.get("data").cloned().unwrap_or(json!({}))))
            } else {
                Ok(error_json(&format!(
                    "Agent returned error: {}",
                    response.get("message").and_then(|v| v.as_str()).unwrap_or("unknown")
                )))
            }
        }
        Err(e) => {
            let error_message = match e {
                StatusCode::NOT_FOUND => "Agent not found or offline".to_string(),
                StatusCode::BAD_REQUEST => "Invalid command request".to_string(),
                _ => format!("Server error: {:?}", e),
            };
            Ok(error_json(&error_message))
        }
    }
}

/// Get data for specific object from plugin on agent
pub async fn get_plugin_object_data(
    Path((agent_id, plugin_name, object_id)): Path<(String, String, String)>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // Send command to agent to get object data from plugin
    let command_payload = json!({
        "command": "get_object_data", // Assuming this command exists in forensic plugin
        "params": {
            "plugin": plugin_name,
            "object_id": object_id
        },
        "agent_id": agent_id
    });

    match send_agent_command_nats(Path(agent_id), State(app_state), Json(command_payload)).await {
        Ok(Json(response)) => {
            if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                Ok(Json(response.get("data").cloned().unwrap_or(json!({}))))
            } else {
                Ok(error_json(&format!(
                    "Agent returned error: {}",
                    response.get("message").and_then(|v| v.as_str()).unwrap_or("unknown")
                )))
            }
        }
        Err(e) => {
            let error_message = match e {
                StatusCode::NOT_FOUND => "Agent not found or offline".to_string(),
                StatusCode::BAD_REQUEST => "Invalid command request".to_string(),
                _ => format!("Server error: {:?}", e),
            };
            Ok(error_json(&error_message))
        }
    }
}