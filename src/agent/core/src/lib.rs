//! Mini MSP Agent Core Library
//! 
//! This is the core library that contains the orchestrator, broker client,
//! configuration manager, and built-in plugins. It provides a modular
//! architecture for system monitoring and management.

pub mod orchestrator;
pub mod broker;
pub mod config;

// Re-export key types
pub use orchestrator::Orchestrator;
pub use broker::BrokerClient;
pub use config::ConfigManager;
