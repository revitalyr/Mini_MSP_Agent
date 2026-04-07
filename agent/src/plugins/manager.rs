use anyhow::{anyhow, Result};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{error, info, warn};

use super::loader::PluginLoader;
use super::ffi::{
    PluginInfoData, SystemMetricsData, ProcessInfoData, CommandResultData, FileContentData, 
    SystemInfoData, EventData, WatchersData, FileReaderData, SensorData, CameraData, 
    ProcessingResults, VideoFrameData, DirectoryInfoData
};

const MAX_SENSOR_QUEUE_SIZE: usize = 1000;

#[derive(Debug, Clone)]
pub enum PluginStatus {
    Unloaded,
    Loading,
    Loaded,
    Active,
    Error,
    Unloading,
}

#[derive(Debug, Clone)]
pub struct PluginRegistryEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub library_path: String,
    pub status: PluginStatus,
    pub status_message: String,
    pub last_loaded: Option<std::time::SystemTime>,
    pub last_unloaded: Option<std::time::SystemTime>,
}

#[derive(Clone)]
pub struct PluginManager {
    plugins: Arc<Mutex<HashMap<String, Arc<PluginLoader>>>>,
    registry: Arc<Mutex<HashMap<String, PluginRegistryEntry>>>,
    sensor_queue: Arc<Mutex<VecDeque<SensorData>>>,
    disable_signature_check: bool,
    system_plugin: Arc<Mutex<Option<String>>>,
    hot_reload_enabled: bool,
    plugin_directory: Arc<Mutex<Option<String>>>,
    event_callback: Option<Arc<dyn Fn(PluginEventType, &str, &str) + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum PluginEventType {
    Loaded,
    Unloaded,
    Error,
    StatusChanged,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            registry: Arc::new(Mutex::new(HashMap::new())),
            sensor_queue: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SENSOR_QUEUE_SIZE))),
            disable_signature_check: false,
            system_plugin: None,
            hot_reload_enabled: false,
            plugin_directory: None,
            event_callback: None,
        }
    }
    
    pub fn with_signature_check(mut self, disable: bool) -> Self {
        self.disable_signature_check = disable;
        self
    }

    pub fn enable_hot_reload(&mut self, enable: bool) {
        self.hot_reload_enabled = enable;
        info!("Hot reload {}", if enable { "enabled" } else { "disabled" });
        
        if enable && self.plugin_directory.lock().unwrap().is_some() {
            self.start_hot_reload_monitor();
        }
    }

    pub fn set_event_callback<F>(&mut self, callback: F) 
    where 
        F: Fn(PluginEventType, &str, &str) + Send + Sync + 'static
    {
        self.event_callback = Some(Arc::new(callback));
    }

    fn notify_event(&self, event_type: PluginEventType, plugin_name: &str, message: &str) {
        if let Some(ref callback) = self.event_callback {
            callback(event_type, plugin_name, message);
        }
    }

    pub fn load_plugin<P: AsRef<Path>>(&self, name: &str, path: P) -> Result<()> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        info!("Loading plugin '{}' from: {}", name, path_str);
        
        let mut loader = PluginLoader::with_signature_check(self.disable_signature_check);
        self.notify_event(PluginEventType::StatusChanged, name, "Loading started");
        
        if let Err(e) = loader.load_plugin(path) {
            self.notify_event(PluginEventType::Error, name, &e.to_string());
            let mut registry = self.registry.lock().unwrap();
            registry.insert(name.to_string(), PluginRegistryEntry {
                name: name.to_string(),
                version: "unknown".to_string(),
                platform: "unknown".to_string(),
                library_path: path_str,
                status: PluginStatus::Error,
                status_message: e.to_string(),
                last_loaded: None,
                last_unloaded: None,
            });
            return Err(e);
        }
        
        // Get plugin info to determine type
        let plugin_info = loader.get_plugin_info()?;
        
        // Update registry
        let mut registry = self.registry.lock().unwrap();
        let entry = PluginRegistryEntry {
            name: name.to_string(),
            version: plugin_info.version.clone(),
            platform: plugin_info.name.clone(), // Simplified
            library_path: path_str,
            status: PluginStatus::Active,
            status_message: "Plugin loaded successfully".to_string(),
            last_loaded: Some(std::time::SystemTime::now()),
            last_unloaded: None,
        };
        
        registry.insert(name.to_string(), entry);
        
        // Check if this is a system plugin
        if plugin_info.name.contains("system") || plugin_info.description.contains("system") {
            let mut sys_plugin = self.system_plugin.lock().unwrap();
            if let Some(ref existing) = *sys_plugin {
                warn!("System plugin already loaded ({}), replacing with {}", existing, name);
            }
            *sys_plugin = Some(name.to_string());
            info!("Registered '{}' as system plugin", name);
        }
        
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(name.to_string(), Arc::new(loader));
        
        self.notify_event(PluginEventType::Loaded, name, "Plugin loaded successfully");
        
        Ok(())
    }

    pub fn unload_plugin(&self, name: &str) -> Result<()> {
        info!("Unloading plugin: {}", name);
        
        // Remove from plugins
        let mut plugins = self.plugins.lock().unwrap();
        if plugins.remove(name).is_none() {
            return Err(anyhow!("Plugin '{}' not found", name));
        }
        
        // Update registry
        let mut registry = self.registry.lock().unwrap();
        if let Some(entry) = registry.get_mut(name) {
            entry.status = PluginStatus::Unloaded;
            entry.status_message = "Plugin unloaded".to_string();
            entry.last_unloaded = Some(SystemTime::now());
        }
        
        // Clear system plugin reference if needed
        let mut sys_plugin = self.system_plugin.lock().unwrap();
        if let Some(ref sp_name) = *sys_plugin {
            if sp_name == name {
                *sys_plugin = None;
                info!("System plugin unloaded");
            }
        }
        
        self.notify_event(PluginEventType::Unloaded, name, "Plugin unloaded");
        
        Ok(())
    }

    pub fn reload_plugin(&self, name: &str) -> Result<()> {
        info!("Reloading plugin: {}", name);
        
        let library_path = {
            let registry = self.registry.lock().unwrap();
            registry.get(name)
                .ok_or_else(|| anyhow!("Plugin '{}' not found", name))?
                .library_path
                .clone()
        };
        
        // Set unloading status
        self.set_plugin_status(name, PluginStatus::Unloading)?;
        
        // Unload first
        self.unload_plugin(name)?;
        
        // Set loading status
        self.set_plugin_status(name, PluginStatus::Loading)?;
        
        // Load again
        self.load_plugin(name, &library_path)?;
        
        info!("Plugin '{}' reloaded successfully", name);
        Ok(())
    }

    pub fn get_system_plugin(&self) -> Result<Arc<PluginLoader>> {
        let sys_lock = self.system_plugin.lock().unwrap();
        let plugin_name = sys_lock.as_ref()
            .ok_or_else(|| anyhow!("No system plugin loaded"))?;
        
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_name)
            .cloned()
            .ok_or_else(|| anyhow!("System plugin not found in registry"))
    }
    
    /// Set plugin status with timestamp tracking
    pub fn set_plugin_status(&self, name: &str, status: PluginStatus) -> Result<()> {
        let mut registry = self.registry.lock().unwrap();
        if let Some(entry) = registry.get_mut(name) {
            entry.status = status.clone();
            entry.status_message = match &status {
                PluginStatus::Loading => "Plugin is loading...".to_string(),
                PluginStatus::Loaded => "Plugin loaded successfully".to_string(),
                PluginStatus::Unloading => "Plugin is unloading...".to_string(),
                PluginStatus::Active => "Plugin is active and processing".to_string(),
                PluginStatus::Error => "Plugin encountered errors".to_string(),
                PluginStatus::Unloaded => "Plugin is unloaded".to_string(),
            };
            
            // Update timestamps
            match status {
                PluginStatus::Loaded => {
                    entry.last_loaded = Some(std::time::SystemTime::now());
                    entry.last_unloaded = None;
                }
                PluginStatus::Unloaded => {
                    entry.last_unloaded = Some(std::time::SystemTime::now());
                }
                _ => {}
            }
            
            info!("Plugin '{}' status changed to: {:?}", name, status);
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' not found in registry", name))
        }
    }
    
    /// Asynchronously load a plugin with status tracking
    pub async fn load_plugin_async(&self, name: &str, library_path: &str) -> Result<()> {
        info!("Starting async load of plugin: {}", name);
        
        // Set loading status
        self.set_plugin_status(name, PluginStatus::Loading)?;
        
        // Simulate async loading (in real implementation, this could be I/O bound)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Load the plugin
        self.load_plugin(name, library_path)?;
        
        // Set loaded status
        self.set_plugin_status(name, PluginStatus::Loaded)?;
        
        info!("Plugin '{}' loaded asynchronously", name);
        Ok(())
    }
    
    /// Gracefully unload a plugin with cleanup
    pub fn unload_plugin_graceful(&self, name: &str) -> Result<()> {
        info!("Starting graceful unload of plugin: {}", name);
        
        // Set unloading status
        self.set_plugin_status(name, PluginStatus::Unloading)?;
        
        // Give plugin time to cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Perform actual unload
        self.unload_plugin(name)?;
        
        info!("Plugin '{}' unloaded gracefully", name);
        Ok(())
    }

    pub fn get_plugin(&self, name: &str) -> Result<Arc<PluginLoader>> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))
    }

    pub fn list_plugins(&self) -> Vec<PluginInfoData> {
        let mut plugins_info = Vec::new();
        
        for (_name, loader) in self.plugins.lock().unwrap().iter() {
            if let Ok(info) = loader.get_plugin_info() {
                plugins_info.push(info);
            }
        }
        
        plugins_info
    }
    
    /// Get detailed registry entry for a plugin
    pub fn get_registry_entry(&self, name: &str) -> Result<PluginRegistryEntry> {
        let registry = self.registry.lock().unwrap();
        registry.get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Plugin '{}' not found in registry", name))
    }

    pub fn get_plugin_registry(&self) -> Vec<PluginRegistryEntry> {
        self.registry.lock().unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub fn get_loaded_plugins(&self) -> Vec<String> {
        self.registry.lock().unwrap()
            .values()
            .filter(|entry| matches!(entry.status, PluginStatus::Active))
            .map(|entry| entry.name.clone())
            .collect()
    }

    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.registry.lock().unwrap()
            .get(name)
            .map(|entry| matches!(entry.status, PluginStatus::Active))
            .unwrap_or(false)
    }

    pub fn get_plugin_status(&self, name: &str) -> PluginStatus {
        self.registry.lock().unwrap()
            .get(name)
            .map(|entry| entry.status.clone())
            .unwrap_or(PluginStatus::Unloaded)
    }

    // Convenience methods that delegate to the system plugin
    pub fn get_system_metrics(&self) -> Result<SystemMetricsData> {
        self.get_system_plugin()?.get_interface()?.get_system_metrics()
    }

    pub fn get_processes(&self) -> Result<Vec<ProcessInfoData>> {
        self.get_system_plugin()?.get_interface()?.get_processes()
    }

    pub fn execute_command(&self, command: &str) -> Result<CommandResultData> {
        self.get_system_plugin()?.get_interface()?.execute_command(command)
    }

    pub fn read_file(&self, path: &str) -> Result<FileContentData> {
        self.get_system_plugin()?.get_interface()?.read_file(path)
    }

    pub fn get_system_info(&self) -> Result<SystemInfoData> {
        self.get_system_plugin()?.get_interface()?.get_system_info()
    }

    pub fn get_directory_info_data(&self, path: &str, recursive: bool, show_hidden: bool, max_depth: u32) -> Result<DirectoryInfoData> {
        self.get_system_plugin()?.get_interface()?.get_directory_info_data(path, recursive, show_hidden, max_depth)
    }

    pub fn get_event_data(&self, path: &str) -> Result<EventData> {
        self.get_system_plugin()?.get_interface()?.get_event_data(path)
    }

    pub fn get_watchers_data(&self) -> Result<WatchersData> {
        self.get_system_plugin()?.get_interface()?.get_watchers_data()
    }

    pub fn get_file_reader_data(&self, path: &str) -> Result<FileReaderData> {
        self.get_system_plugin()?.get_interface()?.get_file_reader_data(path)
    }

    pub fn get_sensor_data(&self) -> Result<SensorData> {
        self.get_system_plugin()?.get_interface()?.get_sensor_data()
    }

    /// Добавляет данные в очередь (с вытеснением старых данных)
    pub fn push_sensor_data(&self, data: SensorData) {
        let mut queue = self.sensor_queue.lock().unwrap();
        if queue.len() >= MAX_SENSOR_QUEUE_SIZE {
            queue.pop_front();
        }
        queue.push_back(data);
    }

    /// Возвращает историю данных из очереди
    pub fn get_sensor_history(&self) -> Vec<SensorData> {
        let queue = self.sensor_queue.lock().unwrap();
        queue.iter().cloned().collect()
    }

    /// Запускает фоновый опрос датчиков на высокой частоте
    pub fn start_sensor_polling(&self, interval_ms: u64) {
        let pm = self.clone();
        let frequency = Duration::from_millis(interval_ms);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(frequency);
            loop {
                interval.tick().await;
                if let Ok(data) = pm.get_sensor_data() {
                    pm.push_sensor_data(data);
                }
            }
        });
    }

    pub fn get_camera_data(&self) -> Result<CameraData> {
        self.get_system_plugin()?.get_interface()?.get_camera_data()
    }

    pub fn get_processing_results(&self) -> Result<ProcessingResults> {
        self.get_system_plugin()?.get_interface()?.get_processing_results()
    }

    pub fn get_video_frame(&self) -> Result<VideoFrameData> {
        self.get_system_plugin()?.get_interface()?.get_video_frame()
    }

    pub fn is_system_plugin_loaded(&self) -> bool {
        self.system_plugin.is_some()
    }

    pub fn load_plugins_from_directory<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let dir = dir.as_ref();
        *self.plugin_directory.lock().unwrap() = Some(dir.to_string_lossy().to_string());
        
        if !dir.exists() || !dir.is_dir() {
            warn!("Plugin directory does not exist: {:?}", dir);
            return Ok(());
        }
        
        info!("Loading plugins from directory: {:?}", dir);
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Check for common plugin extensions
                if file_name.ends_with(".dll") || file_name.ends_with(".so") || file_name.ends_with(".dylib") {
                    let plugin_name = file_name.split('.')
                        .next()
                        .unwrap_or(file_name)
                        .trim_start_matches("lib"); // Remove 'lib' prefix for .so files
                    
                    match self.load_plugin(plugin_name, &path) {
                        Ok(_) => info!("Successfully loaded plugin: {}", plugin_name),
                        Err(e) => error!("Failed to load plugin {}: {}", plugin_name, e),
                    }
                }
            }
        }
        
        // Start hot-reload monitor if enabled
        if self.hot_reload_enabled {
            self.start_hot_reload_monitor();
        }
        
        Ok(())
    }

    fn start_hot_reload_monitor(&self) {
        if !self.hot_reload_enabled {
            return;
        }

        let plugin_dir = self.plugin_directory.lock().unwrap().clone().unwrap();
        let pm = self.clone();
        let registry = Arc::clone(&self.registry);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            
                            if path.is_file() {
                                let file_name = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                
                                if file_name.ends_with(".dll") || file_name.ends_with(".so") || file_name.ends_with(".dylib") {
                                    // Check file modification time
                                    if let Ok(metadata) = std::fs::metadata(&path) {
                                        if let Ok(modified) = metadata.modified() {
                                            let plugin_name = file_name.split('.')
                                                .next()
                                                .unwrap_or(file_name)
                                                .trim_start_matches("lib"); // Remove 'lib' prefix for .so files
                                            
                                            let mut registry = registry.lock().unwrap();
                                            
                                            if let Some(entry) = registry.get_mut(plugin_name) {
                                                // Check if file is newer than last load time
                                                if let Some(last_loaded) = entry.last_loaded {
                                                    if let Ok(duration) = modified.duration_since(last_loaded) {
                                                        if duration > Duration::from_secs(2) {
                                                            tracing::info!("Hot-reload: Detected modification for plugin {}", plugin_name);
                                                            drop(registry); // Release lock before reload
                                                            if let Err(e) = pm.reload_plugin(plugin_name) {
                                                                tracing::error!("Hot-reload failed for {}: {}", plugin_name, e);
                                                            }
                                                            return; // Exit loop to re-acquire locks next tick
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        info!("Shutting down plugin manager");
        
        // Don't unload plugins automatically to avoid double free issues
        // Let the system cleanup handle it naturally
        info!("Plugin manager shutdown complete (plugins will be cleaned up naturally)");
    }
}
