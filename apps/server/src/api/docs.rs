//! OpenAPI documentation and schema definitions
//! 
//! This module provides OpenAPI/Swagger documentation for the API,
//! ensuring consistency between server and frontend.

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::custom_plugin::{CustomCommandRequest, CustomCommandResponse, CustomMetrics, PluginInfo};
use crate::api::plugins::{LoadPluginRequest, LoadPluginResponse, UnloadPluginResponse, PluginHealth};

/// Main OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Mini MSP Server API",
        version = "0.1.0",
        description = "API for managing agents, plugins, and system commands"
    ),
    paths(
        super::agents::list_agents,
        super::agents::send_command,
        super::plugins::list_plugins,
        super::plugins::load_plugin,
        super::plugins::unload_plugin,
        super::plugins::get_plugin_metrics,
        super::plugins::plugin_health,
        super::plugins::execute_command,
    ),
    components(
        schemas(
            Agent,
            AgentList,
            AgentStatus,
            CommandRequest,
            CommandResponse,
            PluginInfo,
            CustomCommandRequest,
            CustomCommandResponse,
            CustomMetrics,
            LoadPluginRequest,
            LoadPluginResponse,
            UnloadPluginResponse,
            PluginHealth,
            SystemInfo,
            ErrorResponse,
        )
    ),
    tags(
        (name = "agents", description = "Agent management endpoints"),
        (name = "plugins", description = "Plugin management endpoints"),
        (name = "commands", description = "Command execution endpoints"),
    )
)]
pub struct ApiDoc;

/// Serve Swagger UI for API documentation
pub fn swagger_routes() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}

/// Agent information
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct Agent {
    /// Unique agent identifier (UUID)
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    
    /// Agent hostname
    #[schema(example = "server-01")]
    pub hostname: String,
    
    /// Operating system platform
    #[schema(example = "linux")]
    pub platform: String,
    
    /// Agent version
    #[schema(example = "0.1.0")]
    pub version: String,
    
    /// Current agent status (online/offline)
    pub status: AgentStatus,
    
    /// Unix timestamp of last heartbeat
    pub last_seen: i64,
    
    /// Seconds since last heartbeat
    pub seconds_ago: i64,
}

/// Agent status enumeration
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
#[schema(example = "online")]
pub enum AgentStatus {
    Online,
    Offline,
}

/// List of agents response
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct AgentList {
    /// List of agents
    pub agents: Vec<Agent>,
    /// Total agent count
    pub count: usize,
    /// Number of online agents
    pub online_count: usize,
    /// Number of offline agents
    pub offline_count: usize,
}

/// Command request payload
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct CommandRequest {
    /// Command type to execute
    #[schema(example = "GetSystemInfo")]
    pub command: String,
    
    /// Optional command parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    
    /// Command payload (alternative format)
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    
    /// Command type discriminator
    #[serde(rename = "type")]
    #[schema(example = "GetSystemInfo")]
    pub command_type: String,
}

/// Command response
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct CommandResponse {
    /// Whether the command was successful
    pub success: bool,
    
    /// Response data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Agent not found")]
    pub error: Option<String>,
    
    /// Command name
    pub command: String,
    
    /// Timestamp of response
    pub timestamp: String,
    
    /// Response type
    #[serde(rename = "type")]
    pub response_type: String,
}


/// System information data
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct SystemInfo {
    /// Operating system platform
    pub platform: String,
    
    /// System hostname
    pub hostname: String,
    
    /// CPU architecture
    pub architecture: String,
    
    /// OS version
    pub version: String,
    
    /// CPU usage percentage
    pub cpu_usage: f32,
    
    /// Memory usage percentage
    pub memory_usage: f32,
    
    /// Total memory in bytes
    pub total_memory: u64,
    
    /// Available memory in bytes
    pub available_memory: u64,
    
    /// Disk usage percentage
    pub disk_usage: f32,
    
    /// System uptime in seconds
    pub uptime: u64,
}

/// Error response
#[derive(utoipa::ToSchema, serde::Serialize, serde::Deserialize, Debug)]
pub struct ErrorResponse {
    /// Error status
    pub status: String,
    
    /// Error message
    pub error: String,
    
    /// Optional error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

