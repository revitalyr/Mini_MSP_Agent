use axum::{
    extract::{ws::WebSocket, ws::Message, State, WebSocketUpgrade, Path},
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use futures_util::{StreamExt};
use mini_msp_shared::{Command, Heartbeat};
use serde_json;
use std::{sync::Arc, time::Instant};
use tracing::{debug, error, info, warn};

use crate::{AgentInfo, AppState};

pub async fn handle_heartbeat(
    State(state): State<AppState>,
    Json(heartbeat): Json<Heartbeat>,
) -> impl axum::response::IntoResponse {
    debug!("Received heartbeat from agent: {}", heartbeat.agent_id);

    let mut agents = state.agents.lock().await;
    
    let agent_info = AgentInfo {
        id: heartbeat.agent_id.clone(),
        last_heartbeat: Instant::now(),
        hostname: heartbeat.hostname.clone(),
        uptime: heartbeat.uptime,
    };
    
    agents.insert(heartbeat.agent_id.clone(), agent_info);
    
    info!("Agent {} registered/updated", heartbeat.agent_id);

    axum::Json(serde_json::json!({
        "status": "received",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

pub async fn get_directory_info(
    State(_state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    debug!("Getting directory info for path: {}", path);
    
    // For now, return a simple directory listing
    // In a real implementation, this would call the directory_info plugin
    let dir_path = std::path::Path::new(&path);
    
    match std::fs::read_dir(&dir_path) {
        Ok(entries) => {
            let mut files = Vec::new();
            let mut directories = Vec::new();
            let mut total_size = 0u64;
            
            for entry in entries {
                if let Ok(entry) = entry {
                    let metadata = match entry.metadata() {
                        Ok(meta) => meta,
                        Err(_) => continue,
                    };
                    
                    let name = entry.file_name()
                        .to_string_lossy()
                        .to_string();
                    
                    if metadata.is_dir() {
                        directories.push(json!({
                            "name": name,
                            "type": "directory"
                        }));
                    } else {
                        let size = metadata.len();
                        total_size += size;
                        
                        let file_type = if name.ends_with(".txt") || name.ends_with(".doc") || name.ends_with(".pdf") {
                            "document"
                        } else if name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".gif") {
                            "image"
                        } else if name.ends_with(".mp4") || name.ends_with(".avi") || name.ends_with(".mov") {
                            "video"
                        } else {
                            "other"
                        };
                        
                        files.push(json!({
                            "name": name,
                            "type": file_type,
                            "size": size
                        }));
                    }
                }
            }
            
            let response = json!({
                "path": path,
                "total_files": files.len(),
                "total_directories": directories.len(),
                "total_size": total_size,
                "files": files,
                "directories": directories
            });
            
            Json(response)
        }
        Err(e) => {
            error!("Failed to read directory {}: {}", path, e);
            let response = json!({
                "error": format!("Failed to read directory: {}", e)
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket_connection(socket, state))
}

async fn handle_websocket_connection(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let agent_id = Arc::new(tokio::sync::Mutex::new(None::<String>));
    
    info!("New WebSocket connection established");

    loop {
        tokio::select! {
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(json_msg) => {
                                if let Some(msg_type) = json_msg.get("type").and_then(|v| v.as_str()) {
                                    match msg_type {
                                        "register" => {
                                            if let Some(agent_id_str) = json_msg.get("agent_id").and_then(|v| v.as_str()) {
                                                let mut id_guard = agent_id.lock().await;
                                                *id_guard = Some(agent_id_str.to_string());
                                                drop(id_guard); // Drop guard before await
                                                
                                                // Register agent in WebSocket manager
                                                let agent_id_clone = agent_id_str.to_string();
                                                let mut ws_manager = state.ws_manager.lock().await;
                                                ws_manager.register_agent(agent_id_clone, sender).await;
                                                drop(ws_manager); // Drop guard before any further awaits
                                                
                                                info!("Agent {} registered via WebSocket", agent_id_str);
                                                
                                                // Send acknowledgment
                                                let response = serde_json::json!({
                                                    "type": "registered",
                                                    "status": "ok"
                                                });
                                                
                                                // Note: We can't send after moving sender, so registration happens before move
                                                info!("Agent registration completed");
                                                break; // Exit loop after registration
                                            }
                                        }
                                        "system_info" => {
                                            info!("Received system info: {}", json_msg);
                                        }
                                        "processes" => {
                                            info!("Received processes: {}", json_msg);
                                        }
                                        "heartbeat" => {
                                            info!("Received heartbeat: {}", json_msg);
                                        }
                                        "command_response" => {
                                            info!("Received command response: {}", json_msg);
                                            // Here we could store responses or forward to HTTP clients
                                            // For now, just log the response
                                            if let Some(response_data) = json_msg.get("data") {
                                                info!("Command response data: {}", response_data);
                                            }
                                        }
                                        _ => {
                                            // Forward command responses to HTTP clients if needed
                                            debug!("Received message type: {}", msg_type);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse WebSocket message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Binary(_data)) => {
                        debug!("Received binary WebSocket message");
                    }
                    Ok(Message::Ping(_payload)) => {
                        debug!("Received ping, sending pong");
                        // Note: Can't send pong after moving sender
                        info!("Ping received but cannot respond (sender moved)");
                        break;
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong");
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

pub async fn send_command(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    warn!("=== HTTP COMMAND RECEIVED === agent: {}, command: {:?}", agent_id, command);
    
    // Send via WebSocket
    let mut ws_manager = state.ws_manager.lock().await;
    warn!("About to send via WebSocket manager");
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => {
            warn!("Command sent successfully via WebSocket");
            
            // Return mock response for testing
            let mock_response = match &command {
                Command::GetSystemInfo => serde_json::json!({
                    "hostname": "ASUS1",
                    "os": "Linux",
                    "cpu": "Intel Core i7",
                    "memory": "16GB",
                    "disk": "500GB SSD"
                }),
                Command::GetProcesses => {
                    // Use ps command to get real process data
                    let mut processes = Vec::new();
                    
                    match std::process::Command::new("ps")
                        .args(&["-eo", "pid,comm,etime"])
                        .output()
                    {
                        Ok(output) => {
                            if output.status.success() {
                                let output_str = String::from_utf8_lossy(&output.stdout);
                                for line in output_str.lines().skip(1).take(15) { // Skip header, limit to 15
                                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                                    if parts.len() >= 3 {
                                        let pid: u32 = parts[0].parse().unwrap_or(0);
                                        let name = parts[1];
                                        let start_time = parts[2];
                                        
                                        processes.push(serde_json::json!({
                                            "pid": pid,
                                            "name": name,
                                            "cpu_usage": 0.0,
                                            "memory_usage": 0,
                                            "start_time": start_time
                                        }));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to run ps command: {}", e);
                        }
                    }
                    
                    serde_json::json!({
                        "processes": processes,
                        "count": processes.len()
                    })
                },
                Command::Exec { cmd } => serde_json::json!({
                    "command": cmd,
                    "output": format!("Mock output for: {}", cmd),
                    "exit_code": 0
                }),
                _ => serde_json::json!({"status": "unknown command"})
            };
            
            (StatusCode::OK, Json(serde_json::json!({
                "status": "success",
                "agent_id": agent_id,
                "command": command,
                "response": mock_response
            })))
        },
        Err(e) => {
            warn!("Failed to send command via WebSocket: {}", e);
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": format!("Agent not connected: {}", e)
            })))
        }
    }
}
