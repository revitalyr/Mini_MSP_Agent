//! Plugin Management API
//!
//! Provides HTTP endpoints for managing and interacting with custom plugins:
//! - List loaded plugins
//! - Load/unload plugins
//! - Execute custom commands
//! - Get plugin metrics

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::AppState;
use crate::custom_plugin::{CustomCommandRequest, CustomCommandResponse, CustomMetrics, PluginInfo};
use crate::api::docs::ErrorResponse;

/// List all loaded custom plugins
#[utoipa::path(
    get,
    path = "/plugins",
    tag = "plugins",
    responses(
        (status = 200, description = "List of plugins", body = [PluginInfo]),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PluginInfo>>, StatusCode> {
    let registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let plugins = registry.list_plugins();
    Ok(Json(plugins))
}

/// Load a plugin from file path
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoadPluginRequest {
    pub path: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoadPluginResponse {
    pub success: bool,
    pub plugin: Option<PluginInfo>,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/plugins/load",
    tag = "plugins",
    request_body = LoadPluginRequest,
    responses(
        (status = 200, description = "Plugin loaded", body = LoadPluginResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn load_plugin(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoadPluginRequest>,
) -> Result<Json<LoadPluginResponse>, StatusCode> {
    let mut registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match registry.load_plugin(&request.path) {
        Ok(info) => {
            info!("Loaded plugin via API: {} v{}", info.name, info.version);
            Ok(Json(LoadPluginResponse {
                success: true,
                plugin: Some(info),
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to load plugin from '{}': {}", request.path, e);
            Ok(Json(LoadPluginResponse {
                success: false,
                plugin: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Unload a plugin by name
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnloadPluginResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/plugins/{name}/unload",
    tag = "plugins",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    responses(
        (status = 200, description = "Plugin unloaded", body = UnloadPluginResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn unload_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<UnloadPluginResponse>, StatusCode> {
    let mut registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match registry.unload_plugin(&name) {
        Ok(()) => {
            info!("Unloaded plugin via API: {}", name);
            Ok(Json(UnloadPluginResponse {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to unload plugin '{}': {}", name, e);
            Ok(Json(UnloadPluginResponse {
                success: false,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Execute command on a plugin
#[utoipa::path(
    post,
    path = "/plugins/execute",
    tag = "commands",
    request_body = CustomCommandRequest,
    responses(
        (status = 200, description = "Command executed", body = CustomCommandResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn execute_command(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CustomCommandRequest>,
) -> Result<Json<CustomCommandResponse>, StatusCode> {
    let registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Execute command with parameters if provided
    let result = if let Some(ref params) = request.parameters {
        registry.execute_command_with_params(&request.plugin_name, &request.command, params)
    } else {
        registry.execute_command(&request.plugin_name, &request.command)
    };
    
    match result {
        Ok(response) => {
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Command execution failed on plugin '{}': {}", request.plugin_name, e);
            Ok(Json(CustomCommandResponse {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                execution_time_ms: 0,
            }))
        }
    }
}

/// Get metrics from a plugin
#[utoipa::path(
    get,
    path = "/plugins/{name}/metrics",
    tag = "plugins",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    responses(
        (status = 200, description = "Plugin metrics", body = CustomMetrics),
        (status = 404, description = "Plugin not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_plugin_metrics(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<CustomMetrics>, StatusCode> {
    let registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match registry.get_metrics(&name) {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => {
            error!("Failed to get metrics from plugin '{}': {}", name, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Plugin health check
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PluginHealth {
    pub name: String,
    pub loaded: bool,
    pub status: String,
}

/// Check plugin health
#[utoipa::path(
    get,
    path = "/plugins/{name}/health",
    tag = "plugins",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    responses(
        (status = 200, description = "Plugin health status", body = PluginHealth),
        (status = 404, description = "Plugin not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn plugin_health(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<PluginHealth>, StatusCode> {
    let registry = state.plugin_registry.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let plugins = registry.list_plugins();
    let plugin = plugins.iter().find(|p| p.name == name);
    
    match plugin {
        Some(p) => Ok(Json(PluginHealth {
            name: p.name.clone(),
            loaded: p.loaded,
            status: "healthy".to_string(),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}
