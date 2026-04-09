use anyhow::{Context, Result};
use core_shared::AgentConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

/// Configuration manager for the agent
pub struct ConfigManager {
    config: AgentConfig,
}

impl ConfigManager {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        info!("Loading configuration from: {}", path.display());
        
        let content = std::fs::read_to_string(path)
            .context("Failed to read configuration file")?;
        
        let config: AgentConfig = toml::from_str(&content)
            .context("Failed to parse configuration")?;
        
        // Validate configuration
        Self::validate_config(&config)?;
        
        info!("Configuration loaded successfully for agent: {}", config.agent.id);
        
        Ok(Self { config })
    }

    /// Load configuration with defaults
    pub fn load_with_defaults() -> Result<Self> {
        let config = AgentConfig::default();
        
        info!("Using default configuration for agent: {}", config.agent.id);
        
        Ok(Self { config })
    }

    /// Load configuration from environment variables
    pub fn load_from_env() -> Result<Self> {
        let mut config = AgentConfig::default();
        
        // Override with environment variables
        if let Ok(agent_id) = std::env::var("MSP_AGENT_ID") {
            config.agent.id = agent_id;
        }
        
        if let Ok(broker_url) = std::env::var("MSP_BROKER_URL") {
            config.broker.url = broker_url;
        }
        
        if let Ok(log_level) = std::env::var("MSP_LOG_LEVEL") {
            config.logging.level = log_level;
        }
        
        if let Ok(plugins_str) = std::env::var("MSP_ENABLED_PLUGINS") {
            config.plugins.enabled_plugins = plugins_str.split(',').map(|s| s.trim().to_string()).collect();
        }
        
        info!("Configuration loaded from environment variables");
        
        Ok(Self { config })
    }

    /// Get configuration
    pub fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn get_config_mut(&mut self) -> &mut AgentConfig {
        &mut self.config
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(&self.config)?;
        std::fs::write(path, content)?;
        
        info!("Configuration saved to: {}", path.display());
        
        Ok(())
    }

    /// Validate configuration
    fn validate_config(config: &AgentConfig) -> Result<()> {
        // Validate agent ID
        if config.agent.id.is_empty() {
            return Err(anyhow::anyhow!("Agent ID cannot be empty"));
        }
        
        // Validate broker URL
        if config.broker.url.is_empty() {
            return Err(anyhow::anyhow!("Broker URL cannot be empty"));
        }
        
        // Validate heartbeat interval
        if config.agent.heartbeat_interval == 0 {
            warn!("Heartbeat interval is 0, this may cause issues");
        }
        
        // Validate metrics interval
        if config.agent.metrics_interval == 0 {
            warn!("Metrics interval is 0, this may cause issues");
        }
        
        // Validate log level
        if !["trace", "debug", "info", "warn", "error"].contains(&config.logging.level.as_str()) {
            return Err(anyhow::anyhow!("Invalid log level: {}", config.logging.level));
        }
        
        // Validate log format
        if !["json", "text"].contains(&config.logging.format.as_str()) {
            return Err(anyhow::anyhow!("Invalid log format: {}", config.logging.format));
        }
        
        info!("Configuration validation passed");
        
        Ok(())
    }

    /// Merge with another configuration
    pub fn merge(&mut self, other: &AgentConfig) {
        // Simple merge strategy: override non-empty values
        if !other.agent.id.is_empty() {
            self.config.agent.id = other.agent.id.clone();
        }
        
        if !other.agent.hostname.is_none() {
            self.config.agent.hostname = other.agent.hostname.clone();
        }
        
        if !other.broker.url.is_empty() {
            self.config.broker.url = other.broker.url.clone();
        }
        
        if other.agent.heartbeat_interval > 0 {
            self.config.agent.heartbeat_interval = other.agent.heartbeat_interval;
        }
        
        if other.agent.metrics_interval > 0 {
            self.config.agent.metrics_interval = other.agent.metrics_interval;
        }
        
        if !other.logging.level.is_empty() {
            self.config.logging.level = other.logging.level.clone();
        }
        
        if !other.logging.format.is_empty() {
            self.config.logging.format = other.logging.format.clone();
        }
        
        // Merge plugin configurations
        if !other.plugins.enabled_plugins.is_empty() {
            self.config.plugins.enabled_plugins = other.plugins.enabled_plugins.clone();
        }
        
        if !other.plugins.plugin_dirs.is_empty() {
            self.config.plugins.plugin_dirs = other.plugins.plugin_dirs.clone();
        }
        
        info!("Configuration merged successfully");
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfigSection {
                id: "default-agent".to_string(),
                hostname: None,
                version: "1.0.0".to_string(),
                platform: std::env::consts::OS.to_string(),
                heartbeat_interval: 30,
                metrics_interval: 10,
            },
            broker: BrokerConfig {
                url: "nats://localhost:4222".to_string(),
                client_id: format!("agent-{}", gethostname::gethostname().to_string_lossy()),
                max_reconnect_attempts: 5,
                reconnect_delay: 5000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file: None,
                max_file_size: Some(10 * 1024 * 1024), // 10MB
                max_files: Some(5),
            },
            plugins: PluginConfig {
                enabled_plugins: vec![
                    "system_plugin".to_string(),
                    "file_plugin".to_string(),
                    "network_plugin".to_string(),
                ],
                plugin_dirs: vec![
                    "./plugins".to_string(),
                    "/opt/msp-agent/plugins".to_string(),
                ],
                auto_reload: false,
                hot_reload: false,
            },
            security: SecurityConfig {
                allowed_commands: vec![
                    "get_system_info".to_string(),
                    "get_processes".to_string(),
                    "get_disk_info".to_string(),
                    "get_memory_info".to_string(),
                    "get_cpu_info".to_string(),
                    "get_network_info".to_string(),
                    "list_directory".to_string(),
                    "get_file_info".to_string(),
                    "read_file".to_string(),
                    "get_interfaces".to_string(),
                    "get_routes".to_string(),
                    "get_connections".to_string(),
                ],
                max_file_size: 100 * 1024 * 1024, // 100MB
                sandbox_enabled: false,
                require_signature: false,
            },
        }
    }
}

// Configuration structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigSection {
    pub id: String,
    pub hostname: Option<String>,
    pub version: String,
    pub platform: String,
    pub heartbeat_interval: u64,
    pub metrics_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerConfig {
    pub url: String,
    pub client_id: String,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file: Option<String>,
    pub max_file_size: Option<u64>,
    pub max_files: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_dirs: Vec<String>,
    pub auto_reload: bool,
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub allowed_commands: Vec<String>,
    pub max_file_size: u64,
    pub sandbox_enabled: bool,
    pub require_signature: bool,
}
