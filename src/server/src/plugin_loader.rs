//! Platform-specific C++ plugin loader
//!
//! This module provides dynamic loading of the correct forensic plugin
//! based on the running operating system:
//! - Windows: ForensicPlugin.dll (registry, event logs, processes)
//! - Linux: ForensicPlugin.so (/proc, systemd, kernel modules)
//! - macOS: ForensicPlugin.dylib (LaunchAgents, kexts, Mach APIs)

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use tracing::{info, warn, error};

use crate::ffi::{PluginInterface, SafePluginInterface};

/// Plugin file extension based on platform
#[cfg(target_os = "windows")]
const PLUGIN_EXT: &str = "dll";

#[cfg(target_os = "linux")]
const PLUGIN_EXT: &str = "so";

#[cfg(target_os = "macos")]
const PLUGIN_EXT: &str = "dylib";

/// Default plugin search directories
const PLUGIN_DIRS: &[&str] = &[
    "./plugins",
    "../plugins",
    "./build/plugins",
    "../src/plugins/build",
    "/opt/mini-msp/plugins",
];

/// Plugin names to try loading (in order of preference)
const PLUGIN_NAMES: &[&str] = &[
    "ForensicPlugin",
    "ModernSystemPlugin",
    "SystemPlugin",
];

/// Platform-specific plugin loader
pub struct PluginLoader {
    library: Library,
    interface: SafePluginInterface,
    plugin_name: String,
    plugin_version: String,
}

impl PluginLoader {
    /// Detect current platform and load appropriate plugin
    pub fn load() -> Result<Self> {
        let platform = detect_platform();
        info!("Detected platform: {}", platform);

        let plugin_path = find_plugin_file(platform)?;
        info!("Loading plugin from: {}", plugin_path.display());

        Self::load_from_path(&plugin_path)
    }

    /// Load plugin from specific path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        unsafe {
            // Load the library
            let library = Library::new(path)
                .map_err(|e| anyhow!("Failed to load plugin library '{}': {}", path.display(), e))?;

            // Get the plugin interface function
            type GetPluginInterfaceFn = unsafe extern "C" fn() -> *mut PluginInterface;
            let get_interface: Symbol<GetPluginInterfaceFn> = library
                .get(b"get_plugin_interface")
                .map_err(|e| anyhow!("Failed to find 'get_plugin_interface' symbol: {}", e))?;

            // Call the function to get the interface
            let interface_ptr = get_interface();
            if interface_ptr.is_null() {
                return Err(anyhow!("Plugin returned null interface"));
            }

            // Copy the interface data
            let interface = (*interface_ptr).clone();
            
            // Create safe wrapper
            let safe_interface = SafePluginInterface::new(interface);

            // Get plugin info
            let plugin_info = safe_interface.get_plugin_info()
                .map_err(|e| anyhow!("Failed to get plugin info: {}", e))?;

            info!(
                "Successfully loaded plugin: {} v{} - {}",
                plugin_info.name, plugin_info.version, plugin_info.description
            );

            // Initialize the plugin
            if let Err(e) = safe_interface.init() {
                warn!("Plugin initialization warning: {}", e);
            }

            Ok(Self {
                library,
                interface: safe_interface,
                plugin_name: plugin_info.name,
                plugin_version: plugin_info.version,
            })
        }
    }

    /// Get reference to the plugin interface
    pub fn interface(&self) -> &SafePluginInterface {
        &self.interface
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.plugin_name
    }

    /// Get plugin version
    pub fn version(&self) -> &str {
        &self.plugin_version
    }
}

impl Drop for PluginLoader {
    fn drop(&mut self) {
        info!("Cleaning up plugin: {}", self.plugin_name);
        self.interface.cleanup();
    }
}

/// Detect current platform
fn detect_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    { "windows" }
    
    #[cfg(target_os = "linux")]
    { "linux" }
    
    #[cfg(target_os = "macos")]
    { "macos" }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    { "unknown" }
}

/// Find plugin file in search directories
fn find_plugin_file(platform: &str) -> Result<PathBuf> {
    for dir in PLUGIN_DIRS {
        for name in PLUGIN_NAMES {
            let filename = format!("{}.{}.{}", name, platform, PLUGIN_EXT);
            let path = Path::new(dir).join(&filename);
            
            if path.exists() {
                return Ok(path);
            }

            // Try without platform suffix
            let filename_simple = format!("{}.{}.{}", name, PLUGIN_EXT, PLUGIN_EXT);
            let path_simple = Path::new(dir).join(&filename_simple);
            if path_simple.exists() {
                return Ok(path_simple);
            }
            
            // Try base name
            let filename_base = format!("{}.{}.{}", name, platform, PLUGIN_EXT);
            let path_base = Path::new(dir).join(&filename_base);
            if path_base.exists() {
                return Ok(path_base);
            }

            // Try direct name
            let filename_direct = format!("lib{}.{}.{}", name, platform, PLUGIN_EXT);
            let path_direct = Path::new(dir).join(&filename_direct);
            if path_direct.exists() {
                return Ok(path_direct);
            }
        }
    }

    // Try environment variable
    if let Ok(plugin_path) = std::env::var("MINI_MSP_PLUGIN_PATH") {
        let path = PathBuf::from(plugin_path);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "Could not find plugin file for platform '{}' with extension '{}' in any of: {:?}",
        platform, PLUGIN_EXT, PLUGIN_DIRS
    ))
}

/// Get the expected plugin filename for current platform
pub fn expected_plugin_filename() -> String {
    format!("ForensicPlugin.{}", PLUGIN_EXT)
}

/// Get all possible plugin paths for debugging
pub fn debug_plugin_paths() -> Vec<PathBuf> {
    let platform = detect_platform();
    let mut paths = Vec::new();
    
    for dir in PLUGIN_DIRS {
        for name in PLUGIN_NAMES {
            let filename = format!("{}.{}.{}", name, platform, PLUGIN_EXT);
            paths.push(Path::new(dir).join(&filename));
            
            let filename_base = format!("{}.{}.{}", name, PLUGIN_EXT, PLUGIN_EXT);
            paths.push(Path::new(dir).join(&filename_base));
            
            let filename_direct = format!("lib{}.{}.{}", name, platform, PLUGIN_EXT);
            paths.push(Path::new(dir).join(&filename_direct));
        }
    }
    
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let platform = detect_platform();
        assert!(["windows", "linux", "macos", "unknown"].contains(&platform));
    }

    #[test]
    fn test_expected_plugin_filename() {
        let filename = expected_plugin_filename();
        assert!(filename.ends_with(PLUGIN_EXT));
        assert!(filename.contains("ForensicPlugin"));
    }
}
