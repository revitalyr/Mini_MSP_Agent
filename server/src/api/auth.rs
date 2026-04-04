//! Authentication endpoints
//! 
//! JWT аутентификация и авторизация

use axum::{extract::State, response::Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Duration, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: i64,
}

/// Login endpoint
pub async fn login(
    Json(payload): Json<LoginRequest>
) -> Result<Json<LoginResponse>, StatusCode> {
    // Простая проверка (в реальном приложении здесь будет БД)
    if payload.username == "admin" && payload.password == "password" {
        let claims = json!({
            "sub": payload.username,
            "exp": Utc::now()
                .checked_add_signed(Duration::hours(24))
                .unwrap()
                .timestamp()
        });

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("your-secret-key"),
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(LoginResponse {
            token,
            expires_in: 24 * 60 * 60, // 24 часа
        }))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Refresh token endpoint
pub async fn refresh_token() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Token refreshed",
        "timestamp": Utc::now()
    }))
}
