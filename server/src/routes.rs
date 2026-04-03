use axum::{
    extract::{ws::WebSocket, ws::Message, State, WebSocketUpgrade, Path, FromRequestParts},
    response::{IntoResponse, Json, Response},
    http::{StatusCode, request::Parts},
    async_trait,
};
use futures_util::{StreamExt};
use jsonwebtoken::{decode, encode, Header, DecodingKey, EncodingKey, Validation, Algorithm};
use mini_msp_shared::{Command, Heartbeat};
use serde_json::json;
use std::{sync::Arc, time::Instant, collections::HashMap};
use tracing::{debug, error, info, warn};

use crate::{AgentInfo, AppState};
use mini_msp_shared::AgentResponse;

const JWT_SECRET: &[u8] = b"your_ultra_secure_secret_change_this";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (обычно username или ID)
    pub exp: usize,  // Expiration time
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub exp: usize,
}

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn login(
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    // В реальном приложении здесь должна быть проверка хеша пароля из БД
    if payload.username == "admin" && payload.password == "password" {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_secs() as usize;

        // Access Token на 15 минут
        let access_claims = Claims { sub: payload.username.clone(), exp: now + 900 };
        // Refresh Token на 7 дней
        let refresh_claims = RefreshClaims { sub: payload.username, exp: now + 604800 };

        let token = encode(&Header::default(), &access_claims, &EncodingKey::from_secret(JWT_SECRET));
        let refresh_token = encode(&Header::default(), &refresh_claims, &EncodingKey::from_secret(JWT_SECRET));

        match (token, refresh_token) {
            (Ok(t), Ok(rt)) => (StatusCode::OK, Json(json!({ "token": t, "refreshToken": rt }))).into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate tokens").into_response(),
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid credentials" }))).into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    Json(payload): Json<RefreshRequest>,
) -> impl IntoResponse {
    let token_data = decode::<RefreshClaims>(
        &payload.refresh_token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    );

    match token_data {
        Ok(data) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_secs() as usize;

            let access_claims = Claims { sub: data.claims.sub, exp: now + 900 };
            let new_token = encode(&Header::default(), &access_claims, &EncodingKey::from_secret(JWT_SECRET));

            match new_token {
                Ok(t) => (StatusCode::OK, Json(json!({ "token": t }))).into_response(),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Token generation failed").into_response(),
            }
        }
        Err(_) => (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid refresh token" }))).into_response(),
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Извлекаем заголовок Authorization
        let auth_header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "Missing authorization header"}))))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token type"}))));
        }

        let token = &auth_header[7..];
        
        // Декодируем и валидируем токен
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::new(Algorithm::HS256),
        ).map_err(|e| {
            error!("JWT Validation error: {}", e);
            (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid or expired token"})))
        })?;

        Ok(token_data.claims)
    }
}

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
            
            Json(response).into_response()
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

pub async fn get_directory_info_data(
    State(state): State<AppState>,
    _claims: Claims,
    Path(agent_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let path = params.get("path").cloned().unwrap_or_default();
    let include_subdirs = params.get("include_subdirs").map(|v| v == "true").unwrap_or(false);
    let show_hidden = params.get("show_hidden").map(|v| v == "true").unwrap_or(false);
    let max_depth = params.get("max_depth").and_then(|v| v.parse().ok()).unwrap_or(1);

    let command = Command::GetDirectoryInfoData { 
        path, 
        include_subdirs, 
        show_hidden, 
        max_depth 
    };

    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "sent", "agent_id": agent_id}))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_event_data_endpoint(
    State(state): State<AppState>,
    _claims: Claims,
    Path(agent_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let path = params.get("path").cloned().unwrap_or_default();
    let command = Command::GetEventData { path };

    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "sent", "agent_id": agent_id}))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_watchers_data_endpoint(
    State(state): State<AppState>,
    _claims: Claims,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let command = Command::GetWatchersData;

    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "sent", "agent_id": agent_id}))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_file_reader_data_endpoint(
    State(state): State<AppState>,
    _claims: Claims,
    Path(agent_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let path = params.get("path").cloned().unwrap_or_default();
    let command = Command::GetFileReaderData { path };

    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "sent", "agent_id": agent_id}))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_plugin_registry_data(
    State(state): State<AppState>,
    _claims: Claims,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let command = Command::GetPluginRegistry;
    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "sent", "agent_id": agent_id}))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
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
                                            
                                            if let Ok(response) = serde_json::from_value::<mini_msp_shared::CommandResponse>(json_msg.clone()) {
                                                let command_id = match &response.command_id {
                                                    Some(id) => id.clone(),
                                                    None => return,
                                                };
                                                let mut ws_manager = state.ws_manager.lock().await;
                                                ws_manager.handle_response(&command_id, AgentResponse::Json(response));
                                            }

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
                    Ok(Message::Binary(data)) => {
                        // Протокол: [36 байт UUID][Данные]
                        if data.len() >= 36 {
                            if let Ok(command_id) = std::str::from_utf8(&data[0..36]) {
                                let payload = data[36..].to_vec();
                                let mut ws_manager = state.ws_manager.lock().await;
                                ws_manager.handle_response(command_id, AgentResponse::Binary { 
                                    command_id: command_id.to_string(), 
                                    data: payload 
                                });
                            }
                        }
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
    _claims: Claims,
    Path(agent_id): Path<String>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    warn!("=== HTTP COMMAND RECEIVED === agent: {}, command: {:?}", agent_id, command);
    
    // Send via WebSocket
    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_and_wait(&agent_id, command.clone()).await {
        Ok(rx) => {
            drop(ws_manager); // Освобождаем Mutex на время ожидания

            // Ждем реальный ответ от агента (с таймаутом в 10 секунд)
            let response_data = tokio::time::timeout(std::time::Duration::from_secs(10), rx).await;

            let final_data = match response_data {
                Ok(Ok(AgentResponse::Json(resp))) => serde_json::to_value(resp).unwrap_or(json!({})),
                Ok(Ok(AgentResponse::Binary { data, .. })) => {
                    // Если это бинарные данные (например, кадр), отдаем их как Raw Body
                    return (StatusCode::OK, [("Content-Type", "application/octet-stream")], data).into_response();
                },
                _ => {
                    // Если агент не ответил вовремя, возвращаем mock или ошибку
                    match &command {
                        Command::GetSystemInfo => serde_json::json!({
                    "hostname": "ASUS1",
                    "os": "Linux",
                    "cpu": "Intel Core i7",
                    "memory": "16GB",
                    "disk": "500GB SSD"
                }),
                Command::GetProcesses => serde_json::json!({"status": "error", "message": "Agent timeout: plugin data not available"}),
                        _ => serde_json::json!({"status": "timeout", "message": "Agent did not respond in time"})
                    }
                }
            };
            
            (StatusCode::OK, Json(serde_json::json!({
                "status": "success",
                "agent_id": agent_id,
                "command": command,
                "response": final_data
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
