//! System Information Endpoints
//! 
//! Provides system information and forensic metrics from plugins.

use axum::{response::Json, http::StatusCode, extract::State};
use serde_json::{json, Value};
use std::process::Command;
use std::sync::Arc;
use crate::AppState;
use tracing::{info, warn};
use crate::semantic_types::{Timestamp, format_timestamp};

/// Get forensic metrics from C++ plugin
pub async fn get_forensic_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let forensic = state.forensic_plugin.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(ref loader) = *forensic {
        let interface = loader.interface();
        
        // Get system metrics from forensic plugin
        match interface.get_system_metrics() {
            Ok(metrics) => {
                info!("Retrieved forensic metrics from plugin: {} v{}", loader.name(), loader.version());
                // Convert C-string hostname to Rust String
                let hostname = unsafe {
                    std::ffi::CStr::from_ptr(metrics.hostname.as_ptr())
                        .to_string_lossy()
                        .to_string()
                };
                
                let timestamp: Timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as Timestamp;
                
                return Ok(Json(json!({
                    "source": "forensic_plugin",
                    "plugin_name": loader.name(),
                    "plugin_version": loader.version(),
                    "metrics": {
                        "hostname": hostname,
                        "ram_usage": metrics.ram_usage,
                        "cpu_usage": metrics.cpu_usage,
                        "uptime": metrics.uptime
                    },
                    "timestamp": timestamp,
                    "timestamp_formatted": format_timestamp(timestamp)
                })));
            }
            Err(e) => {
                warn!("Failed to get forensic metrics: {}", e);
                return Ok(Json(json!({
                    "source": "forensic_plugin",
                    "plugin_name": loader.name(),
                    "plugin_version": loader.version(),
                    "error": e.to_string(),
                    "status": "error"
                })));
            }
        }
    }
    
    // No forensic plugin loaded
    Ok(Json(json!({
        "source": "forensic_plugin",
        "status": "not_loaded",
        "message": "Forensic plugin is not loaded or available"
    })))
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
    let forensic = state.forensic_plugin.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(ref loader) = *forensic {
        let interface = loader.interface();
        
        match interface.get_forensic_data() {
            Ok(Some(data)) => {
                info!("Retrieved forensic data: {} findings", data.count);
                
                // Convert findings to JSON
                let findings: Vec<Value> = if data.count > 0 && !data.findings.is_null() {
                    let slice = unsafe { std::slice::from_raw_parts(data.findings, data.count) };
                    slice.iter().map(|f| {
                        let category = unsafe { std::ffi::CStr::from_ptr(f.category.as_ptr()) }.to_string_lossy();
                        let artifact_type = unsafe { std::ffi::CStr::from_ptr(f.artifact_type.as_ptr()) }.to_string_lossy();
                        let path = unsafe { std::ffi::CStr::from_ptr(f.path.as_ptr()) }.to_string_lossy();
                        let value = unsafe { std::ffi::CStr::from_ptr(f.value.as_ptr()) }.to_string_lossy();
                        let details = unsafe { std::ffi::CStr::from_ptr(f.details.as_ptr()) }.to_string_lossy();
                        
                        json!({
                            "category": category.to_string(),
                            "artifact_type": artifact_type.to_string(),
                            "path": path.to_string(),
                            "value": value.to_string(),
                            "suspicious": f.suspicious,
                            "details": details.to_string()
                        })
                    }).collect()
                } else {
                    Vec::new()
                };
                
                return Ok(Json(json!({
                    "source": "forensic_plugin",
                    "plugin_name": loader.name(),
                    "plugin_version": loader.version(),
                    "collection_time": data.collection_time,
                    "count": data.count,
                    "findings": findings,
                    "status": "success"
                })));
            }
            Ok(None) => {
                return Ok(Json(json!({
                    "source": "forensic_plugin",
                    "plugin_name": loader.name(),
                    "plugin_version": loader.version(),
                    "status": "not_implemented",
                    "message": "Forensic data collection not implemented in this plugin version"
                })));
            }
            Err(e) => {
                warn!("Failed to get forensic data: {}", e);
                return Ok(Json(json!({
                    "source": "forensic_plugin",
                    "plugin_name": loader.name(),
                    "plugin_version": loader.version(),
                    "status": "error",
                    "error": e.to_string()
                })));
            }
        }
    }
    
    Ok(Json(json!({
        "source": "forensic_plugin",
        "status": "not_loaded",
        "message": "Forensic plugin is not loaded or available"
    })))
}

/// Execute JSON command on plugin and forward response directly to web
/// No server-side processing - plugin controls response format
pub async fn execute_plugin_json(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let forensic = state.forensic_plugin.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(ref loader) = *forensic {
        let interface = loader.interface();
        
        // Serialize request to JSON string
        let json_request = match serde_json::to_string(&request) {
            Ok(s) => s,
            Err(e) => return Ok(Json(json!({
                "status": "error",
                "error": format!("Failed to serialize request: {}", e)
            }))),
        };
        
        match interface.execute_json(&json_request) {
            Ok(Some(response_json)) => {
                // Parse plugin response and return as-is (server just forwards)
                match serde_json::from_str::<Value>(&response_json) {
                    Ok(parsed) => {
                        info!("Plugin JSON response forwarded (no server processing)");
                        Ok(Json(parsed))
                    }
                    Err(_) => {
                        // If not valid JSON, wrap as string
                        Ok(Json(json!({
                            "status": "ok",
                            "raw_response": response_json
                        })))
                    }
                }
            }
            Ok(None) => {
                // Plugin doesn't implement execute_json, fall back to legacy
                Ok(Json(json!({
                    "status": "not_implemented",
                    "message": "Plugin doesn't support direct JSON exchange"
                })))
            }
            Err(e) => {
                warn!("Plugin execute_json failed: {}", e);
                Ok(Json(json!({
                    "status": "error",
                    "error": e.to_string()
                })))
            }
        }
    } else {
        Ok(Json(json!({
            "status": "not_loaded",
            "message": "Forensic plugin is not loaded"
        })))
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
