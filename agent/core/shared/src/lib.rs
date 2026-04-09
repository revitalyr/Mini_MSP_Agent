use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Common types shared across the agent core and plugins

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub hostname: String,
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub uptime: u64,
    pub load_average: Option<[f64; 3]>, // 1min, 5min, 15min
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub status: PluginStatus,
    pub loaded_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Loaded,
    Unloaded,
    Error,
    Loading,
    Unloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub id: Uuid,
    pub command: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub request_id: Uuid,
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub execution_time: u64, // milliseconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub permissions: String,
    pub modified: DateTime<Utc>,
    pub accessed: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub routes: Vec<RouteInfo>,
    pub connections: Vec<ConnectionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub index: u32,
    pub mtu: u32,
    pub is_up: bool,
    pub is_loopback: bool,
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub packets_received: u64,
    pub packets_sent: u64,
    pub errors_in: u64,
    pub errors_out: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub destination: String,
    pub gateway: Option<String>,
    pub interface: String,
    pub metric: Option<u32>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub protocol: String,
    pub local_address: String,
    pub remote_address: Option<String>,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    pub id: Uuid,
    pub event_type: EventType,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PluginLoaded,
    PluginUnloaded,
    PluginError,
    CommandExecuted,
    SystemAlert,
    NetworkEvent,
    FileSystemEvent,
}

/// Plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    async fn initialize(&mut self) -> Result<(), anyhow::Error>;
    async fn shutdown(&mut self) -> Result<(), anyhow::Error>;
    
    async fn handle_command(&self, command: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, anyhow::Error>;
    async fn get_metrics(&self) -> Result<SystemMetrics, anyhow::Error>;
    
    fn health_check(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

/// Plugin registry for managing loaded plugins
pub type PluginRegistry = HashMap<String, Box<dyn Plugin>>;

/// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent: AgentConfigSection,
    pub broker: BrokerConfig,
    pub logging: LoggingConfig,
    pub plugins: PluginConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigSection {
    pub id: String,
    pub hostname: Option<String>,
    pub version: String,
    pub platform: String,
    pub heartbeat_interval: u64, // seconds
    pub metrics_interval: u64,   // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerConfig {
    pub url: String,
    pub client_id: String,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay: u64, // milliseconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // json, text
    pub file: Option<String>,
    pub max_file_size: Option<u64>, // bytes
    pub max_files: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_dirs: Vec<String>,
    pub auto_reload: bool,
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub allowed_commands: Vec<String>,
    pub max_file_size: u64,
    pub sandbox_enabled: bool,
    pub require_signature: bool,
}
