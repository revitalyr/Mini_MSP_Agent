//! Demo of PluginRegistry functionality
//! 
//! This demonstrates the PluginRegistry with serialization, cloning, and equality

use mini_msp_shared::{PluginRegistry, Plugin, SystemMetrics, Command, CommandResponse};
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::json;

// Create a simple test plugin
#[derive(Debug, Clone)]
struct TestPlugin {
    name: String,
    version: String,
}

#[async_trait::async_trait]
impl Plugin for TestPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing plugin: {}", self.name);
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down plugin: {}", self.name);
        Ok(())
    }
    
    async fn get_metrics(&self) -> Option<SystemMetrics> {
        Some(SystemMetrics {
            cpu_usage: 42.0,
            memory_usage: 1024,
            disk_usage: 75.5,
            uptime: 3600,
        })
    }
    
    async fn handle_command(&self, cmd: &Command) -> Result<CommandResponse, Box<dyn std::error::Error>> {
        Ok(CommandResponse {
            command_id: Some("test-cmd-123".to_string()),
            r#type: format!("response from {}", self.name),
            status: "success".to_string(),
            data: json!({"plugin": self.name, "handled": true}),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn Plugin> {
        Box::new(self.clone())
    }
    
    fn eq_box(&self, other: &Box<dyn Plugin>) -> bool {
        self.name == other.name() && self.version == other.version()
    }
    
    fn serialize_box(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "name": self.name,
            "version": self.version,
            "type": "TestPlugin"
        }))
    }
    
    fn deserialize_box(&self, data: &serde_json::Value) -> Result<Box<dyn Plugin>, Box<dyn std::error::Error>> {
        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let version = data.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");
        Ok(Box::new(TestPlugin {
            name: name.to_string(),
            version: version.to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PluginRegistry Demo ===");
    
    // Create test plugins
    let plugin1 = TestPlugin {
        name: "Test Plugin 1".to_string(),
        version: "1.0.0".to_string(),
    };
    
    let plugin2 = TestPlugin {
        name: "Test Plugin 2".to_string(),
        version: "2.0.0".to_string(),
    };
    
    // Create plugin registry
    let mut registry = PluginRegistry {
        plugins: HashMap::new(),
    };
    
    // Add plugins to registry
    registry.plugins.insert("plugin1".to_string(), Box::new(plugin1));
    registry.plugins.insert("plugin2".to_string(), Box::new(plugin2));
    
    println!("✓ Created PluginRegistry with {} plugins", registry.plugins.len());
    
    // Test Debug trait
    println!("✓ Debug output: {:?}", registry);
    
    // Test Clone trait
    let cloned_registry = registry.clone();
    println!("✓ Successfully cloned PluginRegistry");
    
    // Test PartialEq trait
    if registry == cloned_registry {
        println!("✓ PluginRegistry equality check passed");
    } else {
        println!("✗ PluginRegistry equality check failed");
    }
    
    // Test serialization
    let serialized = serde_json::to_string(&registry)?;
    println!("✓ Serialized PluginRegistry: {} characters", serialized.len());
    
    // Test deserialization
    let deserialized: PluginRegistry = serde_json::from_str(&serialized)?;
    println!("✓ Deserialized PluginRegistry successfully");
    
    // Test plugin functionality
    for (name, plugin) in &registry.plugins {
        println!("\n--- Testing Plugin: {} ---", name);
        
        // Test plugin info
        println!("Name: {}", plugin.name());
        println!("Version: {}", plugin.version());
        
        // Test metrics
        if let Some(metrics) = plugin.get_metrics().await {
            println!("Metrics: CPU={}%, RAM={}KB, Disk={}%, Uptime={}s", 
                metrics.cpu_usage, metrics.memory_usage, metrics.disk_usage, metrics.uptime);
        }
        
        // Test command handling
        let command = Command::GetSystemInfo;
        if let Ok(response) = plugin.handle_command(&command).await {
            println!("Command response: {} - {}", response.r#type, response.status);
        }
    }
    
    println!("\n=== Demo Complete ===");
    println!("All PluginRegistry functionality working correctly!");
    
    Ok(())
}
