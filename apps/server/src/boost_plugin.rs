//! Boost.DLL Plugin Manager Integration
//!
//! Provides Rust FFI bindings to the modern C++23 Boost.DLL plugin system.
//! Replaces legacy libloading-based PluginLoader with type-safe C++ implementation.
//!
//! When C++ libraries are not available, stub implementation is used.

// Imports only needed when boost_dll feature is enabled
#[cfg(feature = "boost_dll")]
use std::ffi::{c_char, CStr, CString};

use std::path::Path;
use serde_json::Value;
use anyhow::{anyhow, Result};

// Imports only needed when boost_dll feature is enabled
#[cfg(feature = "boost_dll")]
use std::sync::Arc;
#[cfg(feature = "boost_dll")]
use parking_lot::Mutex;
#[cfg(feature = "boost_dll")]
use tracing::{info, warn, debug};

/// Opaque handle to C++ BoostPluginManager
/// 
/// Note: This type is only defined when `boost_dll` feature is enabled.
#[cfg(feature = "boost_dll")]
#[repr(C)]
pub struct BoostPluginManager {
    _private: [u8; 0],
}

// C API function types - only available when boost_dll is enabled
#[cfg(feature = "boost_dll")]
extern "C" {
    fn msp_manager_create() -> *mut BoostPluginManager;
    fn msp_manager_destroy(manager: *mut BoostPluginManager);
    
    fn msp_manager_load_plugin(
        manager: *mut BoostPluginManager,
        path: *const c_char,
        error_buffer: *mut c_char,
        error_buffer_size: usize,
    ) -> bool;
    
    fn msp_manager_unload_plugin(
        manager: *mut BoostPluginManager,
        plugin_id: *const c_char,
    ) -> bool;
    
    fn msp_manager_load_all_from_directory(
        manager: *mut BoostPluginManager,
        directory: *const c_char,
    );
    
    fn msp_manager_execute_json(
        manager: *mut BoostPluginManager,
        plugin_name: *const c_char,
        json_request: *const c_char,
    ) -> *mut c_char;
    
    fn msp_manager_execute_json_auto(
        manager: *mut BoostPluginManager,
        json_request: *const c_char,
    ) -> *mut c_char;
    
    fn msp_manager_list_plugins(manager: *mut BoostPluginManager) -> *mut c_char;
    fn msp_manager_get_plugin_count(manager: *mut BoostPluginManager) -> usize;
    
    fn msp_manager_health_check(manager: *mut BoostPluginManager) -> *mut c_char;
    fn msp_manager_get_metrics(manager: *mut BoostPluginManager) -> *mut c_char;
    
    fn msp_free_string(str: *mut c_char);
    fn msp_manager_get_last_error(buffer: *mut c_char, buffer_size: usize) -> bool;
}

/// Safe wrapper around C++ BoostPluginManager - only with boost_dll feature
#[cfg(feature = "boost_dll")]
#[derive(Debug)]
pub struct BoostPluginManagerHandle {
    ptr: *mut BoostPluginManager,
}

// Send + Sync because C++ manager uses internal locking
#[cfg(feature = "boost_dll")]
unsafe impl Send for BoostPluginManagerHandle {}
#[cfg(feature = "boost_dll")]
unsafe impl Sync for BoostPluginManagerHandle {}

#[cfg(feature = "boost_dll")]
impl BoostPluginManagerHandle {
    /// Create new plugin manager
    pub fn new() -> Result<Self> {
        let ptr = unsafe { msp_manager_create() };
        if ptr.is_null() {
            return Err(anyhow!("Failed to create BoostPluginManager"));
        }
        
        info!("Created Boost.DLL Plugin Manager");
        
        Ok(Self { ptr })
    }
    
    /// Load plugin from file path
    pub fn load_plugin<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let path_str = path.as_ref().to_string_lossy();
        let c_path = CString::new(path_str.as_bytes())?;
        
        let mut error_buffer = vec![0u8; 1024];
        
        let success = unsafe {
            msp_manager_load_plugin(
                self.ptr,
                c_path.as_ptr(),
                error_buffer.as_mut_ptr() as *mut c_char,
                error_buffer.len(),
            )
        };
        
        if success {
            let plugin_id = path.as_ref()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            info!("Loaded Boost.DLL plugin: {}", plugin_id);
            Ok(plugin_id)
        } else {
            let error_msg = CStr::from_bytes_until_nul(&error_buffer)
                .unwrap_or(CStr::from_bytes_with_nul(b"Unknown error\0").unwrap())
                .to_string_lossy()
                .to_string();
            
            Err(anyhow!("Failed to load plugin: {}", error_msg))
        }
    }
    
    /// Unload plugin by ID
    pub fn unload_plugin(&self, plugin_id: &str) -> bool {
        let Ok(c_id) = CString::new(plugin_id) else {
            return false;
        };
        
        let success = unsafe {
            msp_manager_unload_plugin(self.ptr, c_id.as_ptr())
        };
        
        if success {
            info!("Unloaded plugin: {}", plugin_id);
        } else {
            warn!("Failed to unload plugin: {}", plugin_id);
        }
        
        success
    }
    
    /// Load all plugins from directory
    pub fn load_all_from_directory<P: AsRef<Path>>(&self, directory: P) {
        let Ok(c_dir) = CString::new(directory.as_ref().to_string_lossy().as_bytes()) else {
            return;
        };
        
        unsafe {
            msp_manager_load_all_from_directory(self.ptr, c_dir.as_ptr());
        }
        
        info!("Loaded plugins from directory: {}", directory.as_ref().display());
    }
    
    /// Execute command on specific plugin
    pub fn execute_command(
        &self,
        plugin_name: Option<&str>,
        command: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        let request = serde_json::json!({
            "command": command,
            "params": params,
        });
        
        let json_request = request.to_string();
        let c_request = CString::new(json_request)?;
        
        let c_plugin = plugin_name.map(|p| CString::new(p).ok()).flatten();
        
        let response_ptr = unsafe {
            if let Some(ref name) = c_plugin {
                msp_manager_execute_json(self.ptr, name.as_ptr(), c_request.as_ptr())
            } else {
                msp_manager_execute_json_auto(self.ptr, c_request.as_ptr())
            }
        };
        
        if response_ptr.is_null() {
            return Err(anyhow!("Plugin returned null response"));
        }
        
        let response = unsafe {
            let c_str = CStr::from_ptr(response_ptr);
            let s = c_str.to_string_lossy().to_string();
            msp_free_string(response_ptr);
            s
        };
        
        debug!("Plugin response: {}", response);
        
        let value: Value = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse plugin response: {}", e))?;
        
        Ok(value)
    }
    
    /// Auto-route command to appropriate plugin
    pub fn execute_command_auto(&self, command: &str, params: Option<Value>) -> Result<Value> {
        self.execute_command(None, command, params)
    }
    
    /// List all loaded plugins
    pub fn list_plugins(&self) -> Result<Vec<Value>> {
        let json_ptr = unsafe { msp_manager_list_plugins(self.ptr) };
        
        if json_ptr.is_null() {
            return Ok(vec![]);
        }
        
        let json_str = unsafe {
            let c_str = CStr::from_ptr(json_ptr);
            let s = c_str.to_string_lossy().to_string();
            msp_free_string(json_ptr);
            s
        };
        
        let plugins: Vec<Value> = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse plugin list: {}", e))?;
        
        Ok(plugins)
    }
    
    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        unsafe { msp_manager_get_plugin_count(self.ptr) }
    }
    
    /// Health check all plugins
    pub fn health_check(&self) -> Result<Value> {
        let json_ptr = unsafe { msp_manager_health_check(self.ptr) };
        
        if json_ptr.is_null() {
            return Ok(Value::Null);
        }
        
        let json_str = unsafe {
            let c_str = CStr::from_ptr(json_ptr);
            let s = c_str.to_string_lossy().to_string();
            msp_free_string(json_ptr);
            s
        };
        
        let health: Value = serde_json::from_str(&json_str)?;
        Ok(health)
    }
    
    /// Get manager metrics
    pub fn get_metrics(&self) -> Result<Value> {
        let json_ptr = unsafe { msp_manager_get_metrics(self.ptr) };
        
        if json_ptr.is_null() {
            return Ok(Value::Null);
        }
        
        let json_str = unsafe {
            let c_str = CStr::from_ptr(json_ptr);
            let s = c_str.to_string_lossy().to_string();
            msp_free_string(json_ptr);
            s
        };
        
        let metrics: Value = serde_json::from_str(&json_str)?;
        Ok(metrics)
    }
}

#[cfg(feature = "boost_dll")]
impl Drop for BoostPluginManagerHandle {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                msp_manager_destroy(self.ptr);
            }
            info!("Destroyed Boost.DLL Plugin Manager");
        }
    }
}

/// Registry for managing Boost.DLL plugins
#[cfg(feature = "boost_dll")]
#[derive(Debug)]
pub struct BoostPluginRegistry {
    manager: Arc<Mutex<BoostPluginManagerHandle>>,
}

#[cfg(feature = "boost_dll")]
impl BoostPluginRegistry {
    /// Create new registry
    pub fn new() -> Result<Self> {
        let manager = BoostPluginManagerHandle::new()?;
        
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }
    
    /// Load plugin from path
    pub fn load_plugin<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        self.manager.lock().load_plugin(path)
    }
    
    /// Unload plugin
    pub fn unload_plugin(&self, plugin_id: &str) -> bool {
        self.manager.lock().unload_plugin(plugin_id)
    }
    
    /// Execute command with routing
    pub fn execute_command(
        &self,
        plugin_name: Option<&str>,
        command: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        self.manager.lock().execute_command(plugin_name, command, params)
    }
    
    /// Auto-route command
    pub fn execute_command_auto(&self, command: &str, params: Option<Value>) -> Result<Value> {
        self.manager.lock().execute_command_auto(command, params)
    }
    
    /// List plugins
    pub fn list_plugins(&self) -> Result<Vec<Value>> {
        self.manager.lock().list_plugins()
    }
    
    /// Get plugin info for API
    pub fn get_plugin_info(&self) -> Vec<crate::custom_plugin::PluginInfo> {
        let plugins = self.list_plugins().unwrap_or_default();
        
        plugins.into_iter().map(|p| {
            crate::custom_plugin::PluginInfo {
                name: p.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                version: p.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_string(),
                description: p.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                loaded: true,
            }
        }).collect()
    }
}

/// Initialize Boost.DLL plugin system
/// 
/// Only available when compiled with `boost_dll` feature.
#[cfg(feature = "boost_dll")]
pub fn init_boost_plugins() -> Result<BoostPluginRegistry> {
    info!("Initializing Boost.DLL plugin system...");
    
    let registry = BoostPluginRegistry::new()?;
    
    // Try to load plugins from standard directories
    let plugin_dirs = [
        "./plugins",
        "./plugins/cpp/build/plugins",
        "../plugins/cpp/build/plugins",
    ];
    
    for dir in &plugin_dirs {
        let path = Path::new(dir);
        if path.exists() {
            info!("Loading plugins from: {}", path.display());
            registry.manager.lock().load_all_from_directory(path);
        }
    }
    
    let count = registry.manager.lock().plugin_count();
    info!("Boost.DLL plugin system initialized with {} plugins", count);
    
    Ok(registry)
}

// Stub implementation for when boost_dll feature is disabled
#[cfg(not(feature = "boost_dll"))]
pub use stub::*;

#[cfg(not(feature = "boost_dll"))]
mod stub {
    use std::path::Path;
    use serde_json::Value;
    use anyhow::{anyhow, Result};
    
    /// Stub type when Boost.DLL is not available
    pub struct BoostPluginManager;
    pub struct BoostPluginManagerHandle;
    pub struct BoostPluginRegistry;
    
    impl BoostPluginRegistry {
        pub fn new() -> Result<Self> {
            Err(anyhow!("Boost.DLL support not compiled in. Rebuild with C++ libraries."))
        }
        
        pub fn load_plugin<P: AsRef<Path>>(&self, _path: P) -> Result<String> {
            Err(anyhow!("Boost.DLL support not compiled in"))
        }
        
        pub fn unload_plugin(&self, _plugin_id: &str) -> bool {
            false
        }
        
        pub fn execute_command(
            &self,
            _plugin_name: Option<&str>,
            _command: &str,
            _params: Option<Value>,
        ) -> Result<Value> {
            Err(anyhow!("Boost.DLL support not compiled in"))
        }
        
        pub fn execute_command_auto(&self, _command: &str, _params: Option<Value>) -> Result<Value> {
            Err(anyhow!("Boost.DLL support not compiled in"))
        }
        
        pub fn list_plugins(&self) -> Result<Vec<Value>> {
            Ok(vec![])
        }
        
        pub fn get_plugin_info(&self) -> Vec<crate::custom_plugin::PluginInfo> {
            vec![]
        }
    }
    
    pub fn init_boost_plugins() -> Result<BoostPluginRegistry> {
        Err(anyhow!("Boost.DLL support not compiled in. Build C++ libraries first: cd plugins/cpp && cmake -B build -C CMakeLists.txt.boost && cmake --build build"))
    }
}
