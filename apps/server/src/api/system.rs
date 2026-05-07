//! System Information Endpoints
//! 
//! Provides system information and forensic metrics from plugins.

use axum::{response::Json, http::StatusCode, extract::{State, Path}};
use serde_json::{json, Value};
use std::process::Command;
use std::sync::Arc;
use crate::AppState;
use tracing::{info, warn, instrument};
use crate::api::agents::{send_agent_command_nats, calculate_status}; // Import calculate_status
use crate::semantic_types::{Timestamp, format_timestamp};

/// Get forensic metrics from C++ plugin
pub async fn get_forensic_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let agent_id = {
        let agents = state.agents.lock().unwrap();
        agents.iter()
            .filter(|(_, info)| calculate_status(info.last_seen) == "online")
            .map(|(id, _)| id.clone())
            .next()
            .unwrap_or_else(|| "5824f0ee-56d1-40ab-9c89-70b112b57a01".to_string()) // Fallback to hardcoded
    };
    let command_payload = json!({"command": "get_metrics", "params": {}});
    
    match send_agent_command_nats(Path(agent_id), State(app_state), Json(command_payload)).await {
        Ok(Json(response)) => {
            if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                Ok(Json(response.get("data").cloned().unwrap_or(json!({}))))
            } else {
                Ok(Json(json!({
                    "status": "error",
                    "error": response.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error from agent"),
                    "supported_commands": response.get("supported_commands").cloned().unwrap_or(json!([])),
                })))
            }
        }
        Err(e) => {
            let error_message = match e {
                StatusCode::NOT_FOUND => "Agent not found or offline".to_string(),
                StatusCode::BAD_REQUEST => "Invalid command request".to_string(),
                _ => format!("Server error: {:?}", e),
            };
            Ok(Json(json!({
                "status": "error",
                "error": error_message,
                "supported_commands": ["get_status", "get_metrics"] // Fallback for UI
            })))
        }
    }
}

/// Get system information
pub async fn get_system_info() -> Result<Json<Value>, StatusCode> {
    let os_info = get_os_info();
    let hostname = get_hostname();
    
    Ok(Json(json!({
        "platform": os_info.platform,
        "platform_name": os_info.name,
        "icon": os_info.icon,
        "hostname": hostname,
        "architecture": os_info.architecture,
        "version": os_info.version
    })))
}

/// Get forensic data from C++ plugin
pub async fn get_forensic_data(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let agent_id = {
        let agents = state.agents.lock().unwrap();
        agents.iter()
            .filter(|(_, info)| calculate_status(info.last_seen) == "online")
            .map(|(id, _)| id.clone())
            .next()
            .unwrap_or_else(|| "5824f0ee-56d1-40ab-9c89-70b112b57a01".to_string()) // Fallback to hardcoded
    };
    let command_payload = json!({"command": "get_forensic", "params": {}});
    
    match send_agent_command_nats(Path(agent_id), State(app_state), Json(command_payload)).await {
        Ok(Json(response)) => {
            if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                Ok(Json(response.get("data").cloned().unwrap_or(json!({}))))
            } else {
                Ok(Json(json!({
                    "status": "error",
                    "error": response.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error from agent"),
                    "supported_commands": response.get("supported_commands").cloned().unwrap_or(json!([])),
                })))
            }
        }
        Err(e) => {
            let error_message = match e {
                StatusCode::NOT_FOUND => "Agent not found or offline".to_string(),
                StatusCode::BAD_REQUEST => "Invalid command request".to_string(),
                _ => format!("Server error: {:?}", e),
            };
            Ok(Json(json!({
                "status": "error",
                "error": error_message,
                "supported_commands": ["get_forensic"] // Fallback for UI
            })))
        }
    }
}

/// Execute JSON command on plugin and forward response directly to web
/// No server-side processing - plugin controls response format
pub async fn execute_plugin_json(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let agent_id = {
        let agents = state.agents.lock().unwrap();
        agents.iter()
            .filter(|(_, info)| calculate_status(info.last_seen) == "online")
            .map(|(id, _)| id.clone())
            .next()
            .unwrap_or_else(|| "5824f0ee-56d1-40ab-9c89-70b112b57a01".to_string()) // Fallback to hardcoded
    };

    let command_payload = json!({
        "command": request.get("cmd").cloned().unwrap_or(json!("unknown_command")),
        "params": request.get("params").cloned().unwrap_or(json!({})),
    });

    match send_agent_command_nats(Path(agent_id), State(state), Json(command_payload)).await {
        Ok(Json(response)) => {
            if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                Ok(Json(response.get("data").cloned().unwrap_or(json!({}))))
            } else {
                Ok(Json(json!({
                    "status": "error",
                    "error": response.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error from agent"),
                    "supported_commands": response.get("supported_commands").cloned().unwrap_or(json!([])),
                })))
            }
        }
        Err(e) => {
            let error_message = match e {
                StatusCode::NOT_FOUND => "Agent not found or offline".to_string(),
                StatusCode::BAD_REQUEST => "Invalid command request".to_string(),
                _ => format!("Server error: {:?}", e),
            };
            Ok(Json(json!({
                "status": "error",
                "error": error_message,
                "supported_commands": ["execute_json"] // Fallback for UI
            })))
        }
    }
}

struct OSInfo {
    platform: String,
    name: String,
    icon: String,
    architecture: String,
    version: String,
}

fn get_os_info() -> OSInfo {
    #[cfg(target_os = "windows")]
    {
        OSInfo {
            platform: "windows".to_string(),
            name: "Windows".to_string(),
            icon: "🪟".to_string(),
            architecture: "x64".to_string(),
            version: get_windows_version(),
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        OSInfo {
            platform: "linux".to_string(),
            name: "Linux".to_string(),
            icon: "🐧".to_string(),
            architecture: "x64".to_string(),
            version: get_linux_version(),
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        OSInfo {
            platform: "macos".to_string(),
            name: "macOS".to_string(),
            icon: "🍎".to_string(),
            architecture: "x64".to_string(),
            version: get_macos_version(),
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        OSInfo {
            platform: "unknown".to_string(),
            name: "Unknown".to_string(),
            icon: "❓".to_string(),
            architecture: "unknown".to_string(),
            version: "unknown".to_string(),
        }
    }
}

#[cfg(target_os = "windows")]
fn get_windows_version() -> String {
    match Command::new("cmd").args(&["/C", "ver"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown Windows".to_string(),
    }
}

#[cfg(target_os = "linux")]
fn get_linux_version() -> String {
    match Command::new("uname").args(&["-r"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown Linux".to_string(),
    }
}

#[cfg(target_os = "macos")]
fn get_macos_version() -> String {
    match Command::new("sw_vers").args(&["-productVersion"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown macOS".to_string(),
    }
}

fn get_hostname() -> String {
    match Command::new("hostname").output() {
        Ok(output) => {
            let hostname_str = String::from_utf8_lossy(&output.stdout);
            hostname_str.trim().to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}
