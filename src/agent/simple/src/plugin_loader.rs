//! Plugin loader for agent - orchestrates C++ plugins
//!
//! Agent is a pure orchestrator - no direct OS interaction.
//! All system data comes from plugins via FFI.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tracing::{debug, info};

/// Plugin interface function types - wrapped in Option to allow null checks
type GetPluginInfoFn = Option<unsafe extern "C" fn() -> *const c_char>;
type InitFn = Option<unsafe extern "C" fn() -> bool>;
type CleanupFn = Option<unsafe extern "C" fn()>;
type ExecuteJsonFn = Option<unsafe extern "C" fn(*const c_char) -> *mut c_char>;
type FreeMemoryFn = Option<unsafe extern "C" fn(*mut c_void)>;

/// Plugin interface matching C++ structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginInterface {
    pub get_plugin_info: GetPluginInfoFn,
    pub init: InitFn,
    pub cleanup: CleanupFn,
    pub get_system_metrics: *const c_void,
    pub get_processes: *const c_void,
    pub execute_command: *const c_void,
    pub read_file: *const c_void,
    pub get_system_info: *const c_void,
    pub get_directory_info_data: *const c_void,
    pub get_event_data: *const c_void,
    pub get_watchers_data: *const c_void,
    pub get_file_reader_data: *const c_void,
    pub get_sensor_data: *const c_void,
    pub get_camera_data: *const c_void,
    pub get_processing_results: *const c_void,
    pub get_video_frame: *const c_void,
    pub get_forensic_data: *const c_void,
    pub free_memory: FreeMemoryFn,
    pub execute_json: ExecuteJsonFn,  // Direct JSON exchange - None if not supported
}

/// Loaded plugin instance
/// Note: `library` field is not directly accessed but must be kept alive
/// to prevent the shared library from being unloaded while in use.
pub struct LoadedPlugin {
    #[allow(dead_code)]
    library: libloading::Library,
    interface: PluginInterface,
    name: String,
}

// Safety: libloading::Library is not Send by default, but it's safe to send
// between threads as long as we don't call functions concurrently.
// We use Mutex to ensure exclusive access.
unsafe impl Send for LoadedPlugin {}
unsafe impl Sync for LoadedPlugin {}

impl LoadedPlugin {
    /// Load plugin from path
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let library = unsafe { libloading::Library::new(path.as_ref()) }
            .map_err(|e| anyhow!("Failed to load plugin library: {}", e))?;

        // Get entry point
        let get_interface: libloading::Symbol<unsafe extern "C" fn() -> *const PluginInterface> =
            unsafe { library.get(b"get_plugin_interface\0") }
                .map_err(|e| anyhow!("Failed to get plugin interface function: {}", e))?;

        let interface_ptr = unsafe { get_interface() };
        if interface_ptr.is_null() {
            return Err(anyhow!("Plugin returned null interface"));
        }

        let interface = unsafe { (*interface_ptr).clone() };

        // Verify critical function pointers are not null
        if interface.get_plugin_info.is_none() {
            return Err(anyhow!("Plugin has null get_plugin_info function"));
        }
        if interface.init.is_none() {
            return Err(anyhow!("Plugin has null init function"));
        }
        if interface.cleanup.is_none() {
            return Err(anyhow!("Plugin has null cleanup function"));
        }

        // Get plugin name
        let name = unsafe {
            let info_ptr = interface.get_plugin_info.unwrap()();
            if info_ptr.is_null() {
                "unknown".to_string()
            } else {
                let name_str = CStr::from_ptr(info_ptr).to_string_lossy().to_string();
                if name_str.is_empty() {
                    "unnamed".to_string()
                } else {
                    name_str
                }
            }
        };

        debug!("Loaded plugin '{}' from {:?}", name, path.as_ref());

        Ok(Self {
            library,
            interface,
            name,
        })
    }

    /// Initialize plugin
    pub fn init(&self) -> Result<()> {
        debug!("Initializing plugin '{}'", self.name);
        let init_fn = self.interface.init.ok_or_else(|| {
            anyhow!("Plugin '{}' has null init function", self.name)
        })?;
        let success = unsafe { init_fn() };
        if success {
            debug!("Plugin '{}' initialized successfully", self.name);
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' initialization failed", self.name))
        }
    }

    /// Cleanup plugin
    pub fn cleanup(&self) {
        if let Some(cleanup_fn) = self.interface.cleanup {
            unsafe { cleanup_fn() }
        }
    }

    /// Execute JSON command on plugin
    /// Returns: JSON response from plugin (server forwards as-is)
    pub fn execute_json(&self, request: &Value) -> Result<Value> {
        // Check if execute_json is available
        let execute_fn = self.interface.execute_json.ok_or_else(|| {
            anyhow!("Plugin '{}' does not support execute_json", self.name)
        })?;
        
        let request_str = serde_json::to_string(request)?;
        let c_request = CString::new(request_str)?;

        let response_ptr = unsafe {
            execute_fn(c_request.as_ptr())
        };

        if response_ptr.is_null() {
            return Err(anyhow!("Plugin returned null response"));
        }

        // Convert response to Rust string
        let c_str = unsafe { CStr::from_ptr(response_ptr) };
        let response_str = c_str.to_string_lossy().to_string();
        
        // Free memory using plugin's free_memory function if available
        if let Some(free_fn) = self.interface.free_memory {
            unsafe { free_fn(response_ptr as *mut c_void) };
        }

        // Parse JSON response
        let response: Value = serde_json::from_str(&response_str)
            .map_err(|e| anyhow!("Plugin returned invalid JSON: {}", e))?;

        Ok(response)
    }

    /// Check if plugin supports direct JSON exchange
    pub fn supports_json(&self) -> bool {
        self.interface.execute_json.is_some()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Plugin manager - holds loaded plugins
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Load all plugins from directory
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<()> {
        let dir = dir.as_ref();
        if !dir.exists() {
            tracing::warn!("Plugin directory not found: {}", dir.display());
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a shared library
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                if ext == "so" || ext == "dll" || ext == "dylib" {
                    match LoadedPlugin::load(&path) {
                        Ok(plugin) => {
                            tracing::debug!("Plugin '{}' loaded from {:?}, calling init...", plugin.name(), path);
                            if let Err(e) = plugin.init() {
                                tracing::warn!("Failed to initialize plugin {}: {}", path.display(), e);
                            } else {
                                tracing::info!("Loaded plugin: {} from {}", plugin.name(), path.display());
                                self.plugins.push(plugin);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load plugin {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Route command to appropriate plugin
    /// Returns: JSON response from plugin
    pub fn route_command(&self, command: &str, params: Option<&Value>) -> Result<Value> {
        // Get plugin name from params if specified
        let plugin_name = params
            .and_then(|p| p.get("plugin"))
            .and_then(|v| v.as_str());

        debug!("Routing command '{}' to plugin (requested: {:?})", command, plugin_name);
        debug!("Available plugins: {:?}", self.plugin_names());

        // Find plugin by name or use first available
        let plugin = match plugin_name {
            Some(name) => {
                debug!("Looking for plugin '{}'...", name);
                self.plugins.iter()
                    .find(|p| {
                        let matches = p.name() == name && p.supports_json();
                        debug!("  Checking '{}': name_match={}, supports_json={}", p.name(), p.name() == name, p.supports_json());
                        matches
                    })
                    .ok_or_else(|| anyhow!("Plugin '{}' not found or doesn't support JSON", name))?
            }
            None => self.plugins.iter()
                .find(|p| p.supports_json())
                .ok_or_else(|| anyhow!("No plugin available to handle command: {}", command))?
        };

        info!("Routing command '{}' to plugin '{}'", command, plugin.name());

        // Build JSON request
        let request = json!({
            "cmd": command,
            "params": params.unwrap_or(&json!({}))
        });

        // Execute on plugin
        plugin.execute_json(&request)
    }

    /// Check if any plugin is loaded
    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    /// Get list of loaded plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for plugin in &self.plugins {
            plugin.cleanup();
        }
    }
}
