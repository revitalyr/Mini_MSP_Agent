use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Heartbeat {
    pub agent_id: String,
    pub timestamp: i64,
    pub metrics: Metrics,
    pub hostname: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub cpu: f32,
    pub ram: f32,
    pub disk: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Command {
    GetProcesses,
    Exec { cmd: String },
    GetFile { path: String },
    GetSystemInfo,
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
