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
pub mod system;
pub mod plugins;
pub mod docs;

// Re-export commonly used types
pub use health::*;
