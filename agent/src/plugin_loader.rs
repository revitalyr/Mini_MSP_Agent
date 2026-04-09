//! Enhanced plugin loader with hot-reload and security features

use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use std::time::{SystemTime, UNIX_EPOCH};

/// C ABI interface for plugins
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInterface {
    pub name: *const u8,
    pub version: *const u8,
    pub handle_command: extern "C" fn(*const u8, usize, *mut u8, usize) -> usize,
    pub get_metrics: extern "C" fn(*mut u8, usize) -> usize,
    pub cleanup: extern "C" fn(),
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub loaded_at: SystemTime,
}

/// Loaded plugin instance
pub struct LoadedPlugin {
    pub info: PluginInfo,
    pub library: Library,
    pub interface: PluginInterface,
}

impl LoadedPlugin {
    /// Execute command through plugin
    pub fn execute_command(&self, command: &str) -> Result<Value> {
        let cmd_bytes = command.as_bytes();
        let mut response_buf = vec![0u8; 8192];
        
        let response_len = (self.interface.handle_command)(
            cmd_bytes.as_ptr(),
            cmd_bytes.len(),
            response_buf.as_mut_ptr(),
            response_buf.len(),
        );
        
        if response_len > 0 && response_len < response_buf.len() {
            let response = serde_json::from_slice(&response_buf[..response_len])
                .context("Failed to parse plugin response")?;
            Ok(response)
        } else {
            Err(anyhow::anyhow!("Plugin returned empty or oversized response"))
        }
    }

    /// Get metrics from plugin
    pub fn get_metrics(&self) -> Result<Value> {
        let mut metrics_buf = vec![0u8; 4096];
        
        let metrics_len = (self.interface.get_metrics)(metrics_buf.as_mut_ptr(), metrics_buf.len());
        
        if metrics_len > 0 && metrics_len < metrics_buf.len() {
            let metrics = serde_json::from_slice(&metrics_buf[..metrics_len])
                .context("Failed to parse plugin metrics")?;
            Ok(metrics)
        } else {
            Err(anyhow::anyhow!("Plugin returned empty metrics"))
        }
    }

    /// Cleanup plugin resources
    pub fn cleanup(&self) {
        (self.interface.cleanup)();
    }
}

/// Enhanced plugin manager with hot-reload support
#[derive(Clone)]
pub struct EnhancedPluginManager {
    plugins: Arc<Mutex<Vec<LoadedPlugin>>>,
    plugin_dir: PathBuf,
    hot_reload_enabled: bool,
    signature_check: bool,
}

impl EnhancedPluginManager {
    pub fn new(plugin_dir: PathBuf, hot_reload: bool, signature_check: bool) -> Self {
        Self {
            plugins: Arc::new(Mutex::new(Vec::new())),
            plugin_dir,
            hot_reload_enabled: hot_reload,
            signature_check: signature_check,
        }
    }

    /// Load all plugins from directory
    pub async fn load_all_plugins(&mut self) -> Result<usize> {
        info!("Loading plugins from: {}", self.plugin_dir.display());
        
        if !self.plugin_dir.exists() {
            warn!("Plugin directory does not exist: {}", self.plugin_dir.display());
            return Ok(0);
        }

        let mut loaded_count = 0;
        
        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(ext) = path.extension() {
                if ["so", "dll", "dylib"].contains(&ext.to_str().unwrap_or("")) {
                    match self.load_plugin(&path).await {
                        Ok(_) => {
                            loaded_count += 1;
                        }
                        Err(e) => {
                            error!("Failed to load plugin {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        info!("Loaded {} plugins successfully", loaded_count);
        Ok(loaded_count)
    }

    /// Load single plugin
    pub async fn load_plugin(&mut self, path: &Path) -> Result<PluginInfo> {
        info!("Loading plugin: {}", path.display());

        // Check signature if enabled
        if self.signature_check {
            self.verify_plugin_signature(path)
                .context("Plugin signature verification failed")?;
        }

        // Load library
        let library = unsafe {
            Library::new(path)
                .with_context(|| format!("Failed to load library: {:?}", path))?
        };

        // Get plugin init symbol
        let init: Symbol<unsafe extern "C" fn() -> *mut PluginInterface> = 
            unsafe {
                library
                    .get(b"plugin_init")
                    .context("Plugin missing 'plugin_init' symbol")?
            };

        let interface_ptr = unsafe { init() };
        if interface_ptr.is_null() {
            return Err(anyhow::anyhow!("Plugin init returned null pointer"));
        }

        let interface = unsafe { *interface_ptr };

        // Extract plugin metadata
        let name = unsafe {
            std::ffi::CStr::from_ptr(interface.name as *const _)
                .to_string_lossy()
                .to_string()
        };

        let version = unsafe {
            std::ffi::CStr::from_ptr(interface.version as *const _)
                .to_string_lossy()
                .to_string()
        };

        // Check for duplicates
        {
            let plugins = self.plugins.lock().await;
            if plugins.iter().any(|p| p.info.name == name) {
                if self.hot_reload_enabled {
                    warn!("Hot-reloading plugin: {}", name);
                } else {
                    return Err(anyhow::anyhow!("Plugin already loaded: {}", name));
                }
            }
        }

        // Get file metadata
        let metadata = std::fs::metadata(path)?;
        let last_modified = metadata.modified()?;
        let loaded_at = SystemTime::now();

        let plugin_info = PluginInfo {
            name: name.clone(),
            version: version.clone(),
            path: path.to_path_buf(),
            last_modified,
            loaded_at,
        };

        let plugin = LoadedPlugin {
            info: plugin_info.clone(),
            library,
            interface,
        };

        // Add to plugins list
        {
            let mut plugins = self.plugins.lock().await;
            plugins.push(plugin);
        }

        info!("Successfully loaded plugin: {} v{}", name, version);
        Ok(plugin_info)
    }

    /// Execute command through appropriate plugin
    pub async fn execute_command(&self, plugin_name: &str, command: &str) -> Result<Value> {
        let plugins = self.plugins.lock().await;
        
        // Find the plugin and clone its interface
        let plugin_interface = plugins.iter()
            .find(|p| p.info.name == plugin_name)
            .map(|p| p.interface);
        
        drop(plugins); // Release lock before execution
        
        if let Some(interface) = plugin_interface {
            let cmd_bytes = command.as_bytes();
            let mut response_buf = vec![0u8; 8192];
            
            let response_len = unsafe {
                (interface.handle_command)(
                    cmd_bytes.as_ptr(),
                    cmd_bytes.len(),
                    response_buf.as_mut_ptr(),
                    response_buf.len(),
                )
            };
            
            if response_len > 0 && response_len < response_buf.len() {
                let response = serde_json::from_slice(&response_buf[..response_len])
                    .context("Failed to parse plugin response")?;
                Ok(response)
            } else {
                Err(anyhow::anyhow!("Plugin returned empty or oversized response"))
            }
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", plugin_name))
        }
    }

    /// Collect metrics from all plugins
    pub async fn collect_all_metrics(&self) -> Result<Vec<(String, Value)>> {
        let plugins = self.plugins.lock().await;
        let mut all_metrics = Vec::new();

        for plugin in plugins.iter() {
            match plugin.get_metrics() {
                Ok(metrics) => {
                    all_metrics.push((plugin.info.name.clone(), metrics));
                }
                Err(e) => {
                    warn!("Failed to get metrics from {}: {}", plugin.info.name, e);
                }
            }
        }

        Ok(all_metrics)
    }

    /// Get list of loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.lock().await;
        plugins.iter().map(|p| p.info.clone()).collect()
    }

    /// Check if plugin needs reload
    pub async fn check_reload_needed(&self, plugin_name: &str) -> Result<bool> {
        if !self.hot_reload_enabled {
            return Ok(false);
        }

        let plugins = self.plugins.lock().await;
        
        if let Some(plugin) = plugins.iter().find(|p| p.info.name == plugin_name) {
            let current_modified = std::fs::metadata(&plugin.info.path)?.modified()?;
            Ok(current_modified != plugin.info.last_modified)
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", plugin_name))
        }
    }

    /// Reload specific plugin
    pub async fn reload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        let plugin_path = {
            let plugins = self.plugins.lock().await;
            plugins.iter()
                .find(|p| p.info.name == plugin_name)
                .map(|p| p.info.path.clone())
                .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?
        };

        // Unload plugin
        self.unload_plugin(plugin_name).await?;
        
        // Reload plugin
        self.load_plugin(&plugin_path).await?;
        
        info!("Successfully reloaded plugin: {}", plugin_name);
        Ok(())
    }

    /// Unload plugin
    async fn unload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().await;
        
        let index = plugins.iter().position(|p| p.info.name == plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;
        
        let plugin = plugins.remove(index);
        drop(plugins);
        
        // Cleanup will be called automatically when plugin is dropped
        plugin.cleanup();
        
        info!("Unloaded plugin: {}", plugin_name);
        Ok(())
    }

    /// Get plugin count
    pub async fn plugin_count(&self) -> usize {
        self.plugins.lock().await.len()
    }

    /// Verify plugin signature (placeholder implementation)
    fn verify_plugin_signature(&self, path: &Path) -> Result<()> {
        // TODO: Implement GPG signature verification
        // For now, just check if .sig file exists
        let sig_path = path.with_extension("sig");
        if !sig_path.exists() {
            warn!("No signature file found for: {:?}", path);
            if cfg!(debug_assertions) {
                info!("Debug mode: skipping signature verification");
                return Ok(());
            } else {
                return Err(anyhow::anyhow!("Missing signature file"));
            }
        }
        
        info!("Signature file found for: {:?}", path);
        // TODO: Actually verify the signature
        Ok(())
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        self.cleanup();
    }
}
