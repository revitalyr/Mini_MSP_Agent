use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub log_level: String,
    pub log_dir: String,
    pub broker_url: Option<String>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading config from: {}", path.display());
        
        if !path.exists() {
            warn!("Config file not found: {}, creating default", path.display());
            // Create default config if it doesn't exist
            let default_config = Config::default();
            default_config.save(path)?;
            info!("Created default config file: {}", path.display());
            return Ok(default_config);
        }

        debug!("Reading config file: {}", path.display());
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        debug!("Parsing TOML config");
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
            
        info!("Config loaded successfully: port={}, log_level={}, log_dir={}, broker_url={:?}", 
               config.port, config.log_level, config.log_dir, config.broker_url);

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
            port: 8080,
            log_level: "info".to_string(),
            log_dir: "logs".to_string(),
            broker_url: None, // Optional broker
        }
    }
}
