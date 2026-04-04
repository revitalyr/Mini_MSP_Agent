//! API модуль - обработка HTTP запросов
//! 
//! Содержит все HTTP endpoints и обработчики запросов:
//! - Health checks
//! - Authentication endpoints
//! - Agent management
//! - WebSocket connections

pub mod health;
pub mod agents;
pub mod auth;

// Re-export commonly used types
pub use health::*;
pub use agents::*;
pub use auth::*;
