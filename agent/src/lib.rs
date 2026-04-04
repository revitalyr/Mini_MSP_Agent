//! Mini MSP Agent Library
//! 
//! Provides agent functionality for testing

pub mod broker;
pub mod commands;
pub mod config;
pub mod plugins;

// Re-export main types for testing
pub use broker::{BrokerClient, BrokerLoop};
pub use commands::handle_command;
pub use config::Config;
pub use plugins::PluginManager;
