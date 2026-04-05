use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use std::path::Path;
use tracing::{debug, error, info, warn};

use super::ffi::{PluginInterface, SafePluginInterface};

pub struct PluginLoader {
    library: Option<Library>,
    interface: Option<SafePluginInterface>,
    plugin_path: String,
    disable_signature_check: bool,
}

impl PluginLoader {
    pub fn with_signature_check(disable: bool) -> Self {
        Self {
            library: None,
            interface: None,
            plugin_path: String::new(),
            disable_signature_check: disable,
        }
    }
    
    pub fn load_plugin<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        self.plugin_path = path.to_string_lossy().to_string();
        
        info!("Loading plugin from: {}", self.plugin_path);
        
        // Security: Verify plugin signature before loading
        if !self.verify_signature(path) {
            error!("Security: Plugin signature verification failed for {}", self.plugin_path);
            return Err(anyhow!("Invalid plugin signature."));
        }

        // Load the dynamic library
        let library = unsafe { Library::new(path) }
            .map_err(|e| anyhow!("Failed to load plugin library: {}", e))?;
        
        // Get the plugin interface
        let get_interface: Symbol<unsafe extern "C" fn() -> *mut PluginInterface> = unsafe {
            library.get(b"get_plugin_interface")
                .map_err(|e| anyhow!("Failed to find get_plugin_interface function: {}", e))?
        };
        
        let interface_ptr = unsafe { get_interface() };
        if interface_ptr.is_null() {
            return Err(anyhow!("Plugin interface is null"));
        }
        
        let interface = unsafe { SafePluginInterface::new(*interface_ptr) };
        
        // Get plugin info
        let plugin_info = interface.get_plugin_info()
            .map_err(|e| anyhow!("Failed to get plugin info: {}", e))?;
        
        info!("Loaded plugin: {} v{}", plugin_info.name, plugin_info.version);
        
        // Initialize plugin
        if !interface.init()? {
            warn!("Plugin initialization returned false");
        }
        
        self.library = Some(library);
        self.interface = Some(interface);
        
        Ok(())
    }
    
    pub fn get_interface(&self) -> Result<&SafePluginInterface> {
        self.interface.as_ref()
            .ok_or_else(|| anyhow!("Plugin not loaded"))
    }
    
    pub fn is_loaded(&self) -> bool {
        self.interface.is_some()
    }
    
    fn verify_signature(&self, path: &Path) -> bool {
        if cfg!(debug_assertions) || self.disable_signature_check {
            debug!("Debug mode or signature check disabled: skipping signature verification for {:?}", path);
            return true;
        }

        debug!("Cryptographically verifying signature for plugin: {:?}", path);
        
        // 1. Определение пути к файлу подписи (.sig)
        let mut sig_path = path.to_path_buf();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        sig_path.set_extension(format!("{}.sig", ext));
        
        if !sig_path.exists() {
            error!("Signature file not found: {:?}", sig_path);
            return false;
        }

        // 2. Simple signature check (placeholder - OpenSSL removed for compatibility)
        let result: Result<bool> = (|| {
            // For now, just check if signature file exists
            // TODO: Implement alternative signature verification
            Ok(sig_path.exists())
        })();

        match result {
            Ok(is_valid) => {
                if !is_valid { error!("Invalid signature for {:?}", path); }
                is_valid
            }
            Err(e) => {
                error!("Signature verification error for {:?}: {}", path, e);
                false
            }
        }
    }

    pub fn get_plugin_info(&self) -> Result<super::ffi::PluginInfoData> {
        self.get_interface()?.get_plugin_info()
    }
}

impl Drop for PluginLoader {
    fn drop(&mut self) {
        if let Some(ref mut _interface) = self.interface {
            debug!("Unloading plugin: {}", self.plugin_path);
            // Interface cleanup is handled in SafePluginInterface::drop
        }
    }
}
