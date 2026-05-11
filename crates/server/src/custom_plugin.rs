//! Custom Plugin Manager
//!
//! Provides API for loading and interacting with custom plugins
//! that implement extensible functionality (custom commands, metrics, etc.)

use std::ffi::{c_char, CStr, CString};
use std::path::Path;
use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::semantic_types::{CallCount, ErrorCount, Uptime};

/// Custom command request from client
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CustomCommandRequest {
    pub plugin_name: String,
    pub command: String,
    pub parameters: Option<serde_json::Value>,
}

/// Custom command response to client
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CustomCommandResponse {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: Uptime,
}

/// Custom metrics from plugin
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CustomMetrics {
    pub commands_executed: CallCount,
    pub errors_encountered: ErrorCount,
    pub uptime_seconds: Uptime,
    pub status: String,
}

/// Plugin information
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub loaded: bool,
}

/// Loaded custom plugin
pub struct CustomPlugin {
    library: Library,
    name: String,
    version: String,
    description: String,
}

impl CustomPlugin {
    /// Load custom plugin from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        unsafe {
            let library = Library::new(path)
                .map_err(|e| anyhow!("Failed to load custom plugin '{}': {}", path.display(), e))?;

            // Get plugin info
            type GetPluginInfoFn = unsafe extern "C" fn() -> *const c_char;
            let get_info: Symbol<GetPluginInfoFn> = library
                .get(b"get_plugin_info")
                .map_err(|e| anyhow!("Missing get_plugin_info symbol: {}", e))?;

            let info_ptr = get_info();
            if info_ptr.is_null() {
                return Err(anyhow!("Plugin returned null info"));
            }

            let info_str = CStr::from_ptr(info_ptr).to_str()
                .map_err(|e| anyhow!("Invalid plugin info string: {}", e))?;
            
            // Parse "name:version:description"
            let parts: Vec<&str> = info_str.split(':').collect();
            if parts.len() < 3 {
                return Err(anyhow!("Invalid plugin info format: {}", info_str));
            }

            let name = parts[0].to_string();
            let version = parts[1].to_string();
            let description = parts[2].to_string();

            info!("Loaded custom plugin: {} v{} - {}", name, version, description);

            // Initialize plugin
            type InitFn = unsafe extern "C" fn() -> bool;
            if let Ok(init) = library.get::<Symbol<InitFn>>(b"plugin_initialize") {
                if !init() {
                    warn!("Plugin initialization returned false");
                }
            }

            Ok(Self {
                library,
                name,
                version,
                description,
            })
        }
    }

    /// Get plugin info
    pub fn info(&self) -> PluginInfo {
        PluginInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
            loaded: true,
        }
    }

    /// Execute custom command
    pub fn execute_command(&self, command: &str) -> Result<CustomCommandResponse> {
        let start = std::time::Instant::now();
        
        unsafe {
            type ExecuteFn = unsafe extern "C" fn(*const c_char, *mut c_char, usize) -> bool;
            
            let execute: Symbol<ExecuteFn> = self.library
                .get(b"plugin_execute_command")
                .map_err(|e| anyhow!("Missing plugin_execute_command: {}", e))?;

            let cmd_cstring = CString::new(command)?;
            let mut output_buf = vec![0u8; 1024];
            
            let success = execute(
                cmd_cstring.as_ptr(),
                output_buf.as_mut_ptr() as *mut c_char,
                output_buf.len()
            );

            let output = CStr::from_ptr(output_buf.as_ptr() as *const c_char)
                .to_string_lossy()
                .to_string();

            let execution_time_ms = start.elapsed().as_millis() as u64;

            if success {
                Ok(CustomCommandResponse {
                    success: true,
                    output,
                    error: None,
                    execution_time_ms,
                })
            } else {
                Ok(CustomCommandResponse {
                    success: false,
                    output: String::new(),
                    error: Some(output),
                    execution_time_ms,
                })
            }
        }
    }

    /// Get custom metrics
    pub fn get_metrics(&self) -> Result<CustomMetrics> {
        unsafe {
            type GetMetricsFn = unsafe extern "C" fn(*mut c_char, usize) -> bool;
            
            let get_metrics: Symbol<GetMetricsFn> = self.library
                .get(b"plugin_get_metrics")
                .map_err(|e| anyhow!("Missing plugin_get_metrics: {}", e))?;

            let mut metrics_buf = vec![0u8; 512];
            
            if !get_metrics(metrics_buf.as_mut_ptr() as *mut c_char, metrics_buf.len()) {
                return Err(anyhow!("Failed to get metrics from plugin"));
            }

            let metrics_json = CStr::from_ptr(metrics_buf.as_ptr() as *const c_char)
                .to_string_lossy()
                .to_string();

            // Parse JSON metrics
            let metrics: CustomMetrics = serde_json::from_str(&metrics_json)
                .map_err(|e| anyhow!("Failed to parse metrics JSON: {}", e))?;

            Ok(metrics)
        }
    }
}

impl Drop for CustomPlugin {
    fn drop(&mut self) {
        unsafe {
            type CleanupFn = unsafe extern "C" fn();
            if let Ok(cleanup) = self.library.get::<Symbol<CleanupFn>>(b"plugin_cleanup") {
                cleanup();
            }
        }
        info!("Unloaded custom plugin: {}", self.name);
    }
}

/// Plugin registry for managing multiple custom plugins
pub struct CustomPluginRegistry {
    plugins: std::collections::HashMap<String, CustomPlugin>,
}

impl CustomPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    /// Load and register a plugin
    pub fn load_plugin<P: AsRef<Path>>(&mut self, path: P) -> Result<PluginInfo> {
        let plugin = CustomPlugin::load(path)?;
        let info = plugin.info();
        let name = info.name.clone();
        
        self.plugins.insert(name.clone(), plugin);
        info!("Registered custom plugin: {}", name);
        
        Ok(info)
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        if self.plugins.remove(name).is_some() {
            info!("Unloaded custom plugin: {}", name);
            Ok(())
        } else {
            Err(anyhow!("Plugin not found: {}", name))
        }
    }

    /// Get list of loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.values().map(|p| p.info()).collect()
    }

    /// Execute command on a specific plugin
    pub fn execute_command(&self, plugin_name: &str, command: &str) -> Result<CustomCommandResponse> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_name))?;
        
        plugin.execute_command(command)
    }

    /// Execute command with parameters on a specific plugin
    pub fn execute_command_with_params(
        &self, 
        plugin_name: &str, 
        command: &str,
        parameters: &serde_json::Value
    ) -> Result<CustomCommandResponse> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_name))?;
        
        // Serialize parameters to JSON string and append to command
        // Format: "command_name\n{param1:value1,param2:value2}"
        let params_str = serde_json::to_string(parameters)
            .unwrap_or_else(|_| "{}".to_string());
        let full_command = format!("{}\n{}", command, params_str);
        
        plugin.execute_command(&full_command)
    }

    /// Get metrics from a specific plugin
    pub fn get_metrics(&self, plugin_name: &str) -> Result<CustomMetrics> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_name))?;
        
        plugin.get_metrics()
    }

    /// Load all plugins from directory
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<Vec<PluginInfo>> {
        let dir = dir.as_ref();
        let mut loaded = Vec::new();

        if !dir.exists() {
            return Ok(loaded);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Check if it's a plugin file
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                #[cfg(target_os = "windows")]
                if ext != "dll" { continue; }
                #[cfg(target_os = "linux")]
                if ext != "so" { continue; }
                #[cfg(target_os = "macos")]
                if ext != "dylib" { continue; }

                // Try to load
                if let Ok(info) = self.load_plugin(&path) {
                    loaded.push(info);
                }
            }
        }

        Ok(loaded)
    }
}

impl Default for CustomPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
