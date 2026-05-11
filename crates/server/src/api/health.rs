//! Health check endpoints
//! 
//! Предоставляет endpoints для проверки состояния сервера

use axum::response::Json;
use serde_json::json;
use tracing::{info, debug};

/// Health check endpoint
/// 
/// Возвращает JSON с статусом здоровья сервера
pub async fn health_check() -> Json<serde_json::Value> {
    debug!("Health check endpoint called");
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let response = json!({
        "status": "ok",
        "timestamp": timestamp
    });
    
    debug!("Health check response: {:?}", response);
    info!("Health check completed successfully");
    
    Json(response)
}
