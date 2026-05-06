//! Plugin loader for agent - orchestrates C++ plugins
//!
//! Agent is a pure orchestrator - no direct OS interaction.
//! All system data comes from plugins via FFI.

use std::ffi::{c_char, c_float, c_int, c_void, CStr, CString};
use std::path::Path;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tracing::{debug, info};

/// Shared FFI structures matching the C++ plugin implementation
#[repr(C)]
pub struct SystemMetrics {
    pub hostname: [c_char; 256],
    pub cpu_usage: c_float,
    pub ram_usage: c_float,
    pub disk_usage: c_float,
    pub uptime: u64,
}

#[repr(C)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: [c_char; 256],
    pub start_time: u64,
}

#[repr(C)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: c_int,
    pub stdout: *mut c_char,
    pub stderr: *mut c_char,
    pub error: [c_char; 256],
}

#[repr(C)]
pub struct FileContent {
    pub success: bool,
    pub content: *mut c_char,
    pub size: usize,
    pub error: [c_char; 256],
}

#[repr(C)]
pub struct SystemInfo {
    pub os_type: [c_char; 64],
    pub os_version: [c_char; 128],
    pub hostname: [c_char; 128],
    pub uptime: u64,
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
}

/// Plugin interface function types - wrapped in Option to allow null checks
type GetPluginInfoFn = Option<unsafe extern "C" fn() -> *const c_char>;
type InitFn = Option<unsafe extern "C" fn() -> bool>;
type CleanupFn = Option<unsafe extern "C" fn()>;
type ExecuteJsonFn = Option<unsafe extern "C" fn(*const c_char) -> *mut c_char>;
type FreeMemoryFn = Option<unsafe extern "C" fn(*mut c_void)>;

type GetSystemMetricsFn = Option<unsafe extern "C" fn(*mut SystemMetrics) -> bool>;
type GetProcessesFn = Option<unsafe extern "C" fn(*mut *mut ProcessInfo, *mut usize) -> bool>;
type ExecuteCommandFn = Option<unsafe extern "C" fn(*const c_char, *mut CommandResult) -> bool>;
type ReadFileFn = Option<unsafe extern "C" fn(*const c_char, *mut FileContent) -> bool>;
type GetSystemInfoFn = Option<unsafe extern "C" fn(*mut SystemInfo) -> bool>;

/// Plugin interface matching C++ structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginInterface {
    // Core lifecycle functions
    pub get_plugin_info: GetPluginInfoFn, // Signature: () -> *const char
    pub init: InitFn,                   // Signature: () -> bool
    pub cleanup: CleanupFn,             // Signature: () -> void

    // Specialized typed functions
    pub get_system_metrics: GetSystemMetricsFn,
    pub get_processes: GetProcessesFn,
    pub execute_command: ExecuteCommandFn,
    pub read_file: ReadFileFn,
    pub get_system_info: GetSystemInfoFn,

    // Specialized data getters (Legacy/Placeholder pointers)
    pub get_directory_info_data: *const c_void,
    pub get_event_data: *const c_void,
    pub get_watchers_data: *const c_void,
    pub get_file_reader_data: *const c_void,
    pub get_sensor_data: *const c_void,
    pub get_camera_data: *const c_void,
    pub get_processing_results: *const c_void,
    pub get_video_frame: *const c_void,
    pub get_forensic_data: *const c_void,

    // Memory management
    pub free_memory: FreeMemoryFn,

    // Generic JSON interface
    pub execute_json: ExecuteJsonFn,
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

    /// Get typed system metrics from plugin
    pub fn get_system_metrics(&self) -> Result<Value> {
        let func = self.interface.get_system_metrics.ok_or_else(|| anyhow!("Not supported"))?;
        let mut metrics = std::mem::MaybeUninit::<SystemMetrics>::uninit();
        
        unsafe {
            if func(metrics.as_mut_ptr()) {
                let m = metrics.assume_init();
                Ok(json!({
                    "hostname": CStr::from_ptr(m.hostname.as_ptr()).to_string_lossy(),
                    "cpu_usage": m.cpu_usage,
                    "ram_usage": m.ram_usage,
                    "disk_usage": m.disk_usage,
                    "uptime": m.uptime
                }))
            } else {
                Err(anyhow!("Plugin failed to get metrics"))
            }
        }
    }

    /// Get typed process list from plugin
    pub fn get_processes(&self) -> Result<Value> {
        let func = self.interface.get_processes.ok_or_else(|| anyhow!("Not supported"))?;
        let mut proc_ptr: *mut ProcessInfo = std::ptr::null_mut();
        let mut count: usize = 0;

        unsafe {
            if func(&mut proc_ptr, &mut count) {
                let mut processes = Vec::with_capacity(count);
                let slice = std::slice::from_raw_parts(proc_ptr, count);
                
                for p in slice {
                    processes.push(json!({
                        "pid": p.pid,
                        "name": CStr::from_ptr(p.name.as_ptr()).to_string_lossy(),
                        "start_time": p.start_time
                    }));
                }

                // Free the array allocated by the plugin
                if let Some(free_fn) = self.interface.free_memory {
                    free_fn(proc_ptr as *mut c_void);
                }
                
                Ok(json!({ "processes": processes }))
            } else {
                Err(anyhow!("Plugin failed to get processes"))
            }
        }
    }

    /// Execute typed command on plugin
    pub fn execute_command(&self, command: &str) -> Result<Value> {
        let func = self.interface.execute_command.ok_or_else(|| anyhow!("Not supported"))?;
        let c_cmd = CString::new(command)?;
        let mut result = std::mem::MaybeUninit::<CommandResult>::uninit();

        unsafe {
            if func(c_cmd.as_ptr(), result.as_mut_ptr()) {
                let r = result.assume_init();
                let stdout = if !r.stdout.is_null() { CStr::from_ptr(r.stdout).to_string_lossy().into_owned() } else { "".into() };
                let stderr = if !r.stderr.is_null() { CStr::from_ptr(r.stderr).to_string_lossy().into_owned() } else { "".into() };
                
                // Free the strings allocated by the plugin
                if let Some(free_fn) = self.interface.free_memory {
                    if !r.stdout.is_null() { free_fn(r.stdout as *mut c_void); }
                    if !r.stderr.is_null() { free_fn(r.stderr as *mut c_void); }
                }

                Ok(json!({
                    "success": r.success,
                    "exit_code": r.exit_code,
                    "stdout": stdout,
                    "stderr": stderr
                }))
            } else {
                let r = result.assume_init();
                let err_msg = CStr::from_ptr(r.error.as_ptr()).to_string_lossy();
                Err(anyhow!("Command failed: {}", err_msg))
            }
        }
    }

    /// Read typed file from plugin
    pub fn read_file(&self, path: &str) -> Result<Value> {
        let func = self.interface.read_file.ok_or_else(|| anyhow!("Not supported"))?;
        let c_path = CString::new(path)?;
        let mut content = std::mem::MaybeUninit::<FileContent>::uninit();

        unsafe {
            if func(c_path.as_ptr(), content.as_mut_ptr()) {
                let c = content.assume_init();
                let text = if !c.content.is_null() {
                    CStr::from_ptr(c.content).to_string_lossy().into_owned()
                } else {
                    "".into()
                };

                if let Some(free_fn) = self.interface.free_memory {
                    if !c.content.is_null() { free_fn(c.content as *mut c_void); }
                }

                Ok(json!({
                    "success": c.success,
                    "content": text,
                    "size": c.size
                }))
            } else {
                let c = content.assume_init();
                let err_msg = CStr::from_ptr(c.error.as_ptr()).to_string_lossy();
                Err(anyhow!("File read failed: {}", err_msg))
            }
        }
    }

    /// Get typed system info from plugin
    pub fn get_system_info(&self) -> Result<Value> {
        let func = self.interface.get_system_info.ok_or_else(|| anyhow!("Not supported"))?;
        let mut info = std::mem::MaybeUninit::<SystemInfo>::uninit();

        unsafe {
            if func(info.as_mut_ptr()) {
                let i = info.assume_init();
                Ok(json!({
                    "os_type": CStr::from_ptr(i.os_type.as_ptr()).to_string_lossy(),
                    "os_version": CStr::from_ptr(i.os_version.as_ptr()).to_string_lossy(),
                    "hostname": CStr::from_ptr(i.hostname.as_ptr()).to_string_lossy(),
                    "uptime": i.uptime,
                    "cpu_cores": i.cpu_cores,
                    "total_memory": i.total_memory,
                    "available_memory": i.available_memory
                }))
            } else {
                Err(anyhow!("Plugin failed to get system info"))
            }
        }
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
        
        let params_val = params.cloned().unwrap_or(json!({}));

        debug!("Routing command '{}' to plugin (requested: {:?})", command, plugin_name);

        // Find plugin by name or use first available
        let plugin = match plugin_name {
            Some(name) => {
                // Try exact match first
                if let Some(plugin) = self.plugins.iter().find(|p| p.name() == name) {
                    plugin
                } else {
                    // Fallback: if requested plugin not found, try system plugin
                    // This allows modules like 'directory_info' to be handled by '_system_plugin_v3'
                    tracing::debug!("Plugin '{}' not found by exact name, trying fallback to system plugin", name);
                    self.plugins.iter()
                        .find(|p| p.name().starts_with("_system") || p.name().starts_with("System"))
                        .ok_or_else(|| anyhow!("Plugin '{}' not found", name))?
                }
            }
            None => self.plugins.iter()
                .next()
                .ok_or_else(|| anyhow!("No plugin available to handle command: {}", command))?
        };

        info!("Routing command '{}' to plugin '{}'", command, plugin.name());

        // 1. Try specialized typed FFI functions first
        let typed_result = match command {
            "GetSystemMetrics" => plugin.get_system_metrics().map(Some),
            "GetProcesses" => plugin.get_processes().map(Some),
            "GetSystemInfo" => plugin.get_system_info().map(Some),
            "Exec" => {
                let cmd_str = params_val.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
                plugin.execute_command(cmd_str).map(Some)
            },
            "GetFile" => {
                let path_str = params_val.get("path").and_then(|v| v.as_str()).unwrap_or("");
                plugin.read_file(path_str).map(Some)
            },
            _ => Ok(None),
        };

        match typed_result {
            Ok(Some(data)) => return Ok(json!({
                "status": "success",
                "data": data,
                "plugin": plugin.name()
            })),
            Err(e) if e.to_string() != "Not supported" => return Err(e),
            _ => {} // Fall through to JSON interface if typed not supported or command unknown
        }

        // 2. Fallback to execute_json
        if !plugin.supports_json() {
            return Err(anyhow!("Plugin '{}' does not support command '{}' via any interface", plugin.name(), command));
        }

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
