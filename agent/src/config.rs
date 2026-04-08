use anyhow::{Context, Result};
use mini_msp_shared::AgentConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server_url: String,
    pub ws_url: String,
    pub broker_url: String,
    pub interval: u64,
    pub agent_id: String,
    pub log_level: String,
    pub log_dir: String,
    pub disable_signature_check: bool,
    pub allowed_commands: Vec<String>,
    pub max_file_size: u64,
    pub command_timeout_secs: u64,
}

impl From<AgentConfig> for Config {
    fn from(agent_config: AgentConfig) -> Self {
        Self {
            server_url: agent_config.server_url,
            ws_url: agent_config.ws_url,
            broker_url: "nats://localhost:4222".to_string(),
            interval: agent_config.interval,
            agent_id: agent_config.agent_id,
            log_level: "info".to_string(),
            log_dir: "logs".to_string(),
            disable_signature_check: false,
            allowed_commands: vec![
                "ps".into(), "top".into(), "df".into(), "free".into(), "uptime".into()
            ],
            max_file_size: 1024 * 1024, // 1MB default
            command_timeout_secs: 60, // 60 seconds default
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
            server_url: "http://localhost:8081".to_string(),
            ws_url: "ws://localhost:8081/ws".to_string(),
            broker_url: "nats://localhost:4222".to_string(),
            interval: 30,
            agent_id: uuid::Uuid::new_v4().to_string(),
            log_level: "info".to_string(),
            log_dir: "logs".to_string(),
            disable_signature_check: false,
            allowed_commands: vec![
                "ps".into(), "top".into(), "df".into(), "free".into(), "uptime".into(),
                "whoami".into(), "id".into(), "uname".into(), "date".into(), "ls".into(),
                "cat".into(), "grep".into(), "wc".into(), "head".into(), "tail".into(),
                "netstat".into(), "ss".into(), "ip".into(), "echo".into()
            ],
            max_file_size: 1024 * 1024, // 1MB default
            command_timeout_secs: 60, // 60 seconds default
        }
    }
}
