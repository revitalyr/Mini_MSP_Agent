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

pub mod types;
pub mod constants;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use types::*;
use constants::*;

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
    pub timestamp: Timestamp,
    pub metrics: Metrics,
    pub hostname: String,
    pub uptime: Uptime,
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

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cpu: 0.0,
            ram: 0.0,
            disk: 0.0,
        }
    }
}

/// Agent information structure
///
/// Contains basic information about an agent for registration
/// and identification purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInfo {
    pub id: String,
    pub hostname: String,
    pub version: String,
    pub platform: String,
}

/// Event message structure
///
/// Represents an event message sent between components
/// for notifications and status updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMessage {
    pub event_type: EventType,
    pub timestamp: Timestamp,
    pub payload: serde_json::Value,
}

/// Event type enumeration
///
/// Different types of events that can occur in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    PluginLoaded,
    PluginUnloaded,
    PluginError,
    CommandExecuted,
    SystemAlert,
    NetworkEvent,
    FileSystemEvent,
}

/// Plugin information structure
///
/// Contains metadata about a loaded plugin
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
}

/// Plugin status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Loaded,
    Running,
    Stopped,
    Error(String),
}

/// Plugin registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginRegistry {
    pub plugins: HashMap<String, Box<dyn Plugin>>,
}

/// Plugin trait interface
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_metrics(&self) -> Option<SystemMetrics>;
    async fn handle_command(&self, cmd: &Command) -> Result<CommandResponse, Box<dyn std::error::Error>>;
}

/// System metrics structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub disk_usage: f32,
    pub uptime: u64,
}

/// Command response structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse {
    pub command_id: Option<String>,
    pub r#type: String,
    pub status: String,
    pub data: serde_json::Value,
    pub timestamp: Timestamp,
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
        max_depth: DepthLevel 
    },
    GetPluginRegistry,
    GetEventData { path: String },
    GetWatchersData,
    GetFileReaderData { path: String },
    GetSensorData,
    GetCameraData,
    GetProcessingResults,
    GetVideoFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResponse {
    Json(CommandResponse),
    Binary { command_id: String, data: Vec<u8> },
}

/// Wrapper for sending command with unique identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandRequest {
    pub command_id: String,
    pub command: Command,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub server_url: String,
    pub ws_url: String,
    pub interval: Duration,
    pub agent_id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            ws_url: "ws://localhost:8080/ws".to_string(),
            interval: DEFAULT_HEARTBEAT_SEC,
            agent_id: Uuid::new_v4().to_string(),
        }
    }
}

// Data structures for plugin responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectoryInfoData {
    pub path: String,
    pub total_files: FileCount,
    pub total_directories: DirectoryCount,
    pub total_size_bytes: FileSize,
    pub hidden_files: FileCount,
    pub hidden_directories: FileCount,
    pub scan_timestamp: Timestamp,
    pub scan_progress: Percentage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetricsData {
    pub cpu_usage: CpuUsage,
    pub ram_usage: RamUsage,
    pub disk_usage: DiskUsage,
    pub uptime: Uptime,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventData {
    pub path: String,
    pub events_count: CallCount,
    pub buffer_usage: Percentage,
    pub last_event: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WatchersData {
    pub active_watchers: WatcherCount,
    pub total_notifications: NotificationCount,
    pub cpu_usage: CpuUsage,
    pub memory_usage_kb: MemorySize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileReaderData {
    pub path: String,
    pub content: String,
    pub size: FileSize,
    pub encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SensorData {
    pub sensor_type: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraData {
    pub camera_id: String,
    pub resolution: String,
    pub frame_rate: FrameRate,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessingResults {
    pub task_id: String,
    pub status: String,
    pub result_data: serde_json::Value,
    pub processing_time: f64,
}

/// Message broker message structure
/// 
/// Used for communication between agents and server via NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerMessage {
    pub agent_id: String,
    pub payload: BrokerPayload,
    pub timestamp: Timestamp,
}

/// Broker message payload with tagged enum for different message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BrokerPayload {
    Command(CommandRequest),
    Response(CommandResponse),
    Heartbeat(Heartbeat),
    PluginEvent { 
        plugin: String, 
        data: serde_json::Value 
    },
}
