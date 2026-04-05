//! Health check endpoints
//! 
//! Предоставляет endpoints для проверки состояния сервера

use axum::response::Json;
use serde_json::json;

/// Health check endpoint
/// 
/// Возвращает JSON с статусом здоровья сервера
pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}
