//! Health check endpoints
//! 
//! Предоставляет endpoints для проверки состояния сервера

use axum::{extract::State, response::Json, http::StatusCode};
use serde_json::json;
use std::time::SystemTime;

use crate::AppState;

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
