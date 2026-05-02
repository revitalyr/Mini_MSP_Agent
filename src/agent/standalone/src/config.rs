use anyhow::{Context, Result};
use mini_msp_shared::{AgentConfig, AgentConfigSection, BrokerConfig, LoggingConfig, PluginConfig, SecurityConfig};
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

