use anyhow::{Context, Result};
use mini_msp_shared::AgentConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub ws_url: String,
    pub interval: u64,
    pub agent_id: String,
    pub log_level: String,
}

impl From<AgentConfig> for Config {
    fn from(agent_config: AgentConfig) -> Self {
        Self {
            server_url: agent_config.server_url,
            ws_url: agent_config.ws_url,
            interval: agent_config.interval,
            agent_id: agent_config.agent_id,
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // Create default config if it doesn't exist
            let default_config = Config::default();
            default_config.save(path)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;

        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration")?;

        fs::write(path, content)
            .with_context(|| "Failed to write config file")?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            ws_url: "ws://localhost:8080/ws".to_string(),
            interval: 30,
            agent_id: uuid::Uuid::new_v4().to_string(),
            log_level: "info".to_string(),
        }
    }
}
