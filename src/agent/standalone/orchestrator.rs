use anyhow::{Context, Result};
use chrono::Utc;
use mini_msp_shared::{
    Plugin, PluginRegistry, PluginStatus, CommandRequest, CommandResponse,
    EventMessage, EventType, AgentConfig, AgentInfo, SystemMetrics
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::broker::BrokerClient;
use crate::config::ConfigManager;

/// Core orchestrator that manages plugins and coordinates operations
pub struct Orchestrator {
    plugins: Arc<RwLock<PluginRegistry>>,
    config: AgentConfig,
    broker_client: Arc<BrokerClient>,
    event_sender: mpsc::UnboundedSender<EventMessage>,
    agent_info: AgentInfo,
}

impl Orchestrator {
    pub fn new(
        config: AgentConfig,
        broker_client: Arc<BrokerClient>,
    ) -> (Self, mpsc::UnboundedReceiver<EventMessage>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let agent_info = AgentInfo {
            id: config.agent.id.clone(),
            hostname: config.agent.hostname.clone().unwrap_or_else(|| gethostname::gethostname().to_string_lossy().to_string()),
            version: config.agent.version.clone(),
            platform: config.agent.platform.clone(),
            architecture: std::env::consts::ARCH.to_string(),
            start_time: Utc::now(),
        };
        
        let orchestrator = Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
            broker_client,
            event_sender,
            agent_info,
        };
        
        (orchestrator, event_receiver)
    }

    /// Initialize the orchestrator and load all plugins
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing orchestrator for agent: {}", self.agent_info.id);
        
        // Load built-in plugins
        self.load_builtin_plugins().await?;
        
        // Load external plugins if configured
        if !self.config.plugins.enabled_plugins.is_empty() {
            self.load_external_plugins().await?;
        }
        
        // Start background tasks
        self.start_background_tasks().await;
        
        self.emit_event(EventType::SystemAlert, "orchestrator", json!({
            "message": "Orchestrator initialized successfully",
            "agent_id": self.agent_info.id,
            "plugins_loaded": self.plugins.read().await.len()
        })).await;
        
        Ok(())
    }

    /// Load built-in plugins
    async fn load_builtin_plugins(&mut self) -> Result<()> {
        info!("Loading built-in plugins");
        
        // TODO: Implement system, file and network plugins
        // For now, skip loading built-in plugins
        // self.load_plugin(Box::new(system_plugin::SystemPlugin::new())).await?;
        // self.load_plugin(Box::new(file_plugin::FilePlugin::new())).await?;
        // self.load_plugin(Box::new(network_plugin::NetworkPlugin::new())).await?;
        
        Ok(())
    }

    /// Load external plugins from configured directories
    async fn load_external_plugins(&mut self) -> Result<()> {
        info!("Loading external plugins from: {:?}", self.config.plugins.plugin_dirs);
        
        for plugin_dir in &self.config.plugins.plugin_dirs {
            let path = std::path::Path::new(plugin_dir);
            if !path.exists() {
                warn!("Plugin directory does not exist: {}", plugin_dir);
                continue;
            }
            
            // For now, we'll implement basic plugin loading
            // In a full implementation, this would load dynamic libraries (.so/.dll/.dylib)
            self.scan_plugin_directory(path).await?;
        }
        
        Ok(())
    }

    /// Scan a directory for plugin files
    async fn scan_plugin_directory(&self, dir_path: &std::path::Path) -> Result<()> {
        info!("Scanning plugin directory: {}", dir_path.display());
        
        let mut loaded_count = 0;
        
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            // Look for plugin libraries
            if let Some(extension) = path.extension() {
                if extension.to_str() == Some("so") || extension.to_str() == Some("dll") || extension.to_str() == Some("dylib") {
                    info!("Found plugin library: {}", path.display());
                    // TODO: Implement dynamic plugin loading
                    loaded_count += 1;
                }
            }
        }
        
        info!("Scanned {} plugin files", loaded_count);
        Ok(())
    }

    /// Load a plugin into the registry
    pub async fn load_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let plugin_name = plugin.name().to_string();
        let plugin_version = plugin.version().to_string();
        let plugin_description = plugin.description().to_string();
        
        info!("Loading plugin: {} v{}", plugin_name, plugin_version);
        
        // Check if plugin is already loaded
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&plugin_name) {
                warn!("Plugin {} is already loaded", plugin_name);
                return Ok(());
            }
        }
        
        // Initialize the plugin
        let mut plugin = plugin;
        match plugin.initialize().await {
            Ok(_) => {
                info!("Plugin {} initialized successfully", plugin_name);
                
                // Add to registry
                {
                    let mut plugins = self.plugins.write().await;
                    plugins.insert(plugin_name.clone(), plugin);
                }
                
                // Emit event
                self.emit_event(EventType::PluginLoaded, &plugin_name, json!({
                    "version": plugin_version,
                    "description": plugin_description,
                })).await;
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to initialize plugin {}: {}", plugin_name, e);
                
                // Emit error event
                self.emit_event(EventType::PluginError, &plugin_name, json!({
                    "error": e.to_string(),
                })).await;
                
                Err(e)
            }
        }
    }

    /// Unload a plugin
    pub async fn unload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        info!("Unloading plugin: {}", plugin_name);
        
        let mut plugins = self.plugins.write().await;
        
        if let Some(mut plugin) = plugins.remove(plugin_name) {
            // Shutdown the plugin
            match plugin.shutdown().await {
                Ok(_) => {
                    info!("Plugin {} unloaded successfully", plugin_name);
                    
                    // Emit event
                    drop(plugins); // Release lock before emitting event
                    self.emit_event(EventType::PluginUnloaded, plugin_name, json!({})).await;
                    
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to shutdown plugin {}: {}", plugin_name, e);
                    
                    // Re-add plugin to registry
                    plugins.insert(plugin_name.to_string(), plugin);
                    
                    Err(e)
                }
            }
        } else {
            warn!("Plugin {} not found in registry", plugin_name);
            Err(anyhow::anyhow!("Plugin not found: {}", plugin_name))
        }
    }

    /// Execute a command on a specific plugin
    pub async fn execute_command(&self, command: &str, params: HashMap<String, serde_json::Value>) -> Result<CommandResponse> {
        let request_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();
        
        debug!("Executing command {} with params: {:?}", command, params);
        
        // Find the appropriate plugin for this command
        let plugins = self.plugins.read().await;
        
        // Try to find a plugin that can handle this command
        let plugin = plugins.values().find(|p| {
            // For now, we'll try each plugin
            // In a more sophisticated implementation, plugins would register their capabilities
            true
        });
        
        if let Some(plugin) = plugin {
            let plugin_name = plugin.name().to_string();
            
            match plugin.handle_command(command, params).await {
                Ok(data) => {
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    
                    let response = CommandResponse {
                        request_id,
                        success: true,
                        data,
                        error: None,
                        timestamp: Utc::now(),
                        execution_time,
                    };
                    
                    // Emit command executed event
                    drop(plugins); // Release lock before emitting event
                    self.emit_event(EventType::CommandExecuted, &plugin_name, json!({
                        "command": command,
                        "success": true,
                        "execution_time_ms": execution_time,
                    })).await;
                    
                    Ok(response)
                }
                Err(e) => {
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    
                    let response = CommandResponse {
                        request_id,
                        success: false,
                        data: json!({}),
                        error: Some(e.to_string()),
                        timestamp: Utc::now(),
                        execution_time,
                    };
                    
                    // Emit command executed event
                    drop(plugins); // Release lock before emitting event
                    self.emit_event(EventType::CommandExecuted, &plugin_name, json!({
                        "command": command,
                        "success": false,
                        "error": e.to_string(),
                        "execution_time_ms": execution_time,
                    })).await;
                    
                    Ok(response)
                }
            }
        } else {
            Err(anyhow::anyhow!("No plugin available to handle command: {}", command))
        }
    }

    /// Collect metrics from all plugins
    pub async fn collect_metrics(&self) -> Result<SystemMetrics> {
        let plugins = self.plugins.read().await;
        
        // Try to get metrics from system plugin first
        if let Some(system_plugin) = plugins.get("system_plugin") {
            system_plugin.get_metrics().await
        } else {
            Err(anyhow::anyhow!("System plugin not available for metrics"))
        }
    }

    /// Get list of loaded plugins
    pub async fn list_plugins(&self) -> Vec<mini_msp_shared::PluginInfo> {
        let plugins = self.plugins.read().await;
        
        plugins.values().map(|plugin| {
            mini_msp_shared::PluginInfo {
                name: plugin.name().to_string(),
                version: plugin.version().to_string(),
                description: plugin.description().to_string(),
                author: "MSP Agent Team".to_string(), // This could be part of the plugin trait
                status: PluginStatus::Loaded, // Would need to track actual status
                loaded_at: Some(Utc::now()), // Would need to track actual load time
                last_error: None,
            }
        }).collect()
    }

    /// Get agent information
    pub fn get_agent_info(&self) -> &AgentInfo {
        &self.agent_info
    }

    /// Emit an event
    async fn emit_event(&self, event_type: EventType, source: &str, data: serde_json::Value) {
        let event = EventMessage {
            id: Uuid::new_v4(),
            event_type,
            source: source.to_string(),
            data,
            timestamp: Utc::now(),
        };
        
        // Send to internal event channel
        if let Err(e) = self.event_sender.send(event.clone()) {
            error!("Failed to send event to internal channel: {}", e);
        }
        
        // Send to broker
        if let Err(e) = self.broker_client.publish_event(&self.agent_info.id, event).await {
            error!("Failed to publish event to broker: {}", e);
        }
    }

    /// Start background tasks
    async fn start_background_tasks(&self) {
        // Start metrics collection task
        let orchestrator = self.clone();
        tokio::spawn(async move {
            orchestrator.metrics_collection_task().await;
        });
        
        // Start health check task
        let orchestrator = self.clone();
        tokio::spawn(async move {
            orchestrator.health_check_task().await;
        });
    }

    /// Background task for collecting metrics
    async fn metrics_collection_task(&self) {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(self.config.agent.metrics_interval)
        );
        
        loop {
            interval.tick().await;
            
            match self.collect_metrics().await {
                Ok(metrics) => {
                    // Send metrics to broker
                    if let Err(e) = self.broker_client.publish_metrics(&self.agent_info.id, metrics).await {
                        error!("Failed to publish metrics: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                }
            }
        }
    }

    /// Background task for health checking plugins
    async fn health_check_task(&self) {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(60) // Check every minute
        );
        
        loop {
            interval.tick().await;
            
            let plugins = self.plugins.read().await;
            for (name, plugin) in plugins.iter() {
                if let Err(e) = plugin.health_check() {
                    error!("Health check failed for plugin {}: {}", name, e);
                    
                    // Emit health check event
                    self.emit_event(EventType::SystemAlert, name, json!({
                        "health_check": "failed",
                        "error": e.to_string(),
                    })).await;
                }
            }
        }
    }
}

impl Clone for Orchestrator {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            config: self.config.clone(),
            broker_client: Arc::clone(&self.broker_client),
            event_sender: self.event_sender.clone(),
            agent_info: self.agent_info.clone(),
        }
    }
}
