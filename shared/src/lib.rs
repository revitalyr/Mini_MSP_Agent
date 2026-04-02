//! # Mini MSP Shared Library
//! 
//! This library contains shared data structures and types used by both
//! the Mini MSP Agent and Server components. It provides a common
//! interface for communication and data exchange.
//! 
//! ## Data Structures
//! 
//! - **Heartbeat**: Agent status and metrics reporting
//! - **Metrics**: System performance metrics (CPU, RAM, Disk)
//! - **Command**: Command enumeration for agent operations
//! - **AgentConfig**: Agent configuration structure
//! 
//! ## Serialization
//! 
//! All structures support JSON serialization/deserialization using serde,
//! enabling seamless communication over HTTP and WebSocket protocols.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Agent heartbeat message containing status and metrics
/// 
/// Sent periodically by agents to report their current state
/// and system performance metrics.
/// 
/// # Fields
/// 
/// * `agent_id` - Unique identifier for the agent
/// * `timestamp` - Unix timestamp of the heartbeat
/// * `metrics` - Current system performance metrics
/// * `hostname` - System hostname
/// * `uptime` - System uptime in seconds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Heartbeat {
    pub agent_id: String,
    pub timestamp: i64,
    pub metrics: Metrics,
    pub hostname: String,
    pub uptime: u64,
}

/// System performance metrics
/// 
/// Contains current resource usage percentages for monitoring
/// and alerting purposes.
/// 
/// # Fields
/// 
/// * `cpu` - CPU usage percentage (0.0 - 100.0)
/// * `ram` - RAM usage percentage (0.0 - 100.0)
/// * `disk` - Disk usage percentage (0.0 - 100.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub cpu: f32,
    pub ram: f32,
    pub disk: f32,
}

/// Command enumeration for agent operations
/// 
/// Represents different types of commands that can be
/// sent to agents for execution.
/// 
/// # Variants
/// 
/// * `GetProcesses` - Retrieve running processes list
/// * `Exec` - Execute shell command with specified string
/// * `GetFile` - Read file contents at specified path
/// * `GetSystemInfo` - Get comprehensive system information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Command {
    GetProcesses,
    Exec { cmd: String },
    GetFile { path: String },
    GetSystemInfo,
    GetDirectoryInfoData { 
        path: String, 
        include_subdirs: bool, 
        show_hidden: bool, 
        max_depth: u32 
    },
    GetPluginRegistry,
    GetEventData { path: String },
    GetWatchersData,
    GetFileReaderData { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse {
    pub command_id: Option<String>,
    pub r#type: String,
    pub status: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub server_url: String,
    pub ws_url: String,
    pub interval: u64,
    pub agent_id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            ws_url: "ws://localhost:8080/ws".to_string(),
            interval: 30,
            agent_id: Uuid::new_v4().to_string(),
        }
    }
}
