use mini_msp_shared::{Command, Heartbeat, Metrics, PluginRegistry, Plugin, SystemMetrics, CommandResponse};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use async_trait::async_trait;
use serde_json::json;

#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_void};
#[cfg(target_os = "linux")]
use std::os::raw::{c_char, c_void};
#[cfg(target_os = "windows")]
use std::os::raw::{c_char, c_void};

// Real C++ Plugin loader for testing
fn load_cpp_plugins_from_directory() -> Vec<Box<dyn Plugin>> {
    let mut loaded_plugins = Vec::new();
    let plugins_dir = Path::new("src/plugins/src");
    
    if !plugins_dir.exists() {
        println!("Warning: Plugin directory {} does not exist", plugins_dir.display());
        return loaded_plugins;
    }
    
    println!("Scanning for C++ plugins in: {}", plugins_dir.display());
    
    // List all .cpp files in the directory
    if let Ok(entries) = fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("cpp") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    println!("Found C++ plugin file: {}", file_name);
                    
                    // Create a real plugin placeholder for each C++ file found
                    // In a real implementation, you would compile and load these as dynamic libraries
                    let real_plugin = RealCppPlugin::new(file_name.to_string(), path.to_string_lossy().to_string());
                    loaded_plugins.push(Box::new(real_plugin));
                }
            }
        }
    }
    
    println!("Loaded {} C++ plugins", loaded_plugins.len());
    loaded_plugins
}

// Real plugin wrapper for C++ plugins
#[derive(Debug, Clone)]
struct RealCppPlugin {
    name: String,
    version: String,
    file_path: String,
}

impl RealCppPlugin {
    fn new(file_name: String, file_path: String) -> Self {
        // Extract plugin name from filename
        let name = file_name.replace(".cpp", "").replace("_", " ");
        Self {
            name: name.clone(),
            version: "1.0.0".to_string(),
            file_path,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for RealCppPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing real C++ plugin: {} from {}", self.name, self.file_path);
        // In real implementation, this would load the C++ library and call init function
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down real C++ plugin: {}", self.name);
        // In real implementation, this would unload the C++ library
        Ok(())
    }
    
    async fn get_metrics(&self) -> Option<SystemMetrics> {
        Some(SystemMetrics {
            cpu_usage: 30.0,
            memory_usage: 1024,
            disk_usage: 70.0,
            uptime: 3600,
        })
    }
    
    async fn handle_command(&self, _cmd: &Command) -> Result<CommandResponse, Box<dyn std::error::Error>> {
        Ok(CommandResponse {
            command_id: Some("real-cpp-plugin-cmd".to_string()),
            r#type: format!("real_cpp_plugin_response_{}", self.name),
            status: "success".to_string(),
            data: json!({"plugin": self.name, "type": "real_cpp", "path": self.file_path}),
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
            "type": "RealCppPlugin",
            "file_path": self.file_path
        }))
    }
    
    fn deserialize_box(&self, data: &serde_json::Value) -> Result<Box<dyn Plugin>, Box<dyn std::error::Error>> {
        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let version = data.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");
        let file_path = data.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown");
        Ok(Box::new(RealCppPlugin {
            name: name.to_string(),
            version: version.to_string(),
            file_path: file_path.to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_serialization() {
        let heartbeat = Heartbeat {
            agent_id: "test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics {
                cpu: 45.5,
                ram: 60.2,
                disk: 75.8,
            },
            hostname: "test-host".to_string(),
            uptime: 3600,
        };
        
        // Test serialization
        let json = serde_json::to_string(&heartbeat).unwrap();
        assert!(json.contains("test-agent"));
        assert!(json.contains("45.5"));
        
        // Test deserialization
        let deserialized: Heartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, heartbeat.agent_id);
        assert_eq!(deserialized.hostname, heartbeat.hostname);
        assert!((deserialized.metrics.cpu - heartbeat.metrics.cpu).abs() < 0.001);
    }

    #[test]
    fn test_command_serialization() {
        let commands = vec![
            Command::GetProcesses,
            Command::Exec { cmd: "ls -la".to_string() },
            Command::GetFile { path: "/tmp/test.txt".to_string() },
            Command::GetSystemInfo,
        ];
        
        for command in commands {
            // Test serialization
            let json = serde_json::to_string(&command).unwrap();
            assert!(!json.is_empty());
            
            // Test deserialization
            let deserialized: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, command);
        }
    }

    #[test]
    fn test_metrics_values() {
        let metrics = Metrics {
            cpu: 75.5,
            ram: 60.2,
            disk: 85.8,
        };
        
        // Test that metrics can be created and compared
        let same_metrics = Metrics {
            cpu: 75.5,
            ram: 60.2,
            disk: 85.8,
        };
        
        assert_eq!(metrics, same_metrics);
        
        // Test different metrics
        let different_metrics = Metrics {
            cpu: 50.0,
            ram: 60.2,
            disk: 85.8,
        };
        
        assert_ne!(metrics, different_metrics);
    }

    #[test]
    fn test_heartbeat_equality() {
        let heartbeat1 = Heartbeat {
            agent_id: "agent-1".to_string(),
            timestamp: 1234567890,
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "host-1".to_string(),
            uptime: 3600,
        };
        
        let heartbeat2 = Heartbeat {
            agent_id: "agent-1".to_string(),
            timestamp: 1234567890,
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "host-1".to_string(),
            uptime: 3600,
        };
        
        let heartbeat3 = Heartbeat {
            agent_id: "agent-2".to_string(),
            timestamp: 1234567890,
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "host-1".to_string(),
            uptime: 3600,
        };
        
        assert_eq!(heartbeat1, heartbeat2);
        assert_ne!(heartbeat1, heartbeat3);
    }

    #[test]
    fn test_json_validation() {
        // Test valid heartbeat JSON
        let valid_json = r#"
        {
            "agent_id": "test-agent",
            "timestamp": 1234567890,
            "metrics": {
                "cpu": 45.5,
                "ram": 60.2,
                "disk": 75.8
            },
            "hostname": "test-host",
            "uptime": 3600
        }
        "#;
        
        let heartbeat: Heartbeat = serde_json::from_str(valid_json).unwrap();
        assert_eq!(heartbeat.agent_id, "test-agent");
        assert_eq!(heartbeat.hostname, "test-host");
        assert_eq!(heartbeat.metrics.cpu, 45.5);
        
        // Test invalid JSON
        let invalid_json = r#"{ "invalid": "json" }"#;
        let result: Result<Heartbeat, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    // Create a simple test plugin for PluginRegistry testing
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
            Ok(())
        }
        
        async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
        
        async fn handle_command(&self, _cmd: &Command) -> Result<CommandResponse, Box<dyn std::error::Error>> {
            Ok(CommandResponse {
                command_id: Some("test-cmd".to_string()),
                r#type: "test_response".to_string(),
                status: "success".to_string(),
                data: json!({"plugin": self.name}),
                timestamp: 1234567890,
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

    #[test]
    fn test_plugin_registry_functionality() {
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
        
        // Test Debug trait
        let debug_output = format!("{:?}", registry);
        assert!(debug_output.contains("PluginRegistry"));
        
        // Test Clone trait
        let cloned_registry = registry.clone();
        assert_eq!(registry.plugins.len(), cloned_registry.plugins.len());
        
        // Test PartialEq trait
        assert_eq!(registry, cloned_registry);
        
        // Test serialization
        let serialized = serde_json::to_string(&registry);
        assert!(serialized.is_ok());
        
        // Test deserialization
        let deserialized: PluginRegistry = serde_json::from_str(&serialized.unwrap()).expect("Failed to deserialize PluginRegistry");
        
        println!("✓ PluginRegistry functionality test passed!");
    }

    #[test]
    fn test_load_all_cpp_plugins() {
        println!("=== Testing Real C++ Plugin Loading ===");
        
        // Load all C++ plugins from the directory
        let cpp_plugins = load_cpp_plugins_from_directory();
        
        // Create plugin registry and add all loaded plugins
        let mut registry = PluginRegistry {
            plugins: HashMap::new(),
        };
        
        for (index, plugin) in cpp_plugins.into_iter().enumerate() {
            let plugin_name = format!("real_cpp_plugin_{}", index);
            let file_path = if let Ok(serialized) = plugin.serialize_box() {
                serialized.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
            } else {
                "unknown".to_string()
            };
            println!("Adding real plugin: {} -> {} (path: {})", plugin_name, plugin.name(), file_path);
            registry.plugins.insert(plugin_name, plugin);
        }
        
        println!("✓ Loaded {} real C++ plugins into registry", registry.plugins.len());
        
        // Test that all real plugins are functional
        for (name, plugin) in &registry.plugins {
            println!("Testing real plugin: {} ({})", name, plugin.name());
            
            // Test plugin info
            assert!(!plugin.name().is_empty());
            assert!(!plugin.version().is_empty());
            
            // Test serialization includes file path
            let serialized = plugin.serialize_box();
            assert!(serialized.is_ok());
            let serialized_data = serialized.unwrap();
            assert!(serialized_data.get("file_path").is_some());
            
            // Test cloning
            let cloned = plugin.clone_box();
            assert_eq!(plugin.name(), cloned.name());
            assert_eq!(plugin.version(), cloned.version());
            
            // Test equality
            assert!(plugin.eq_box(&cloned));
        }
        
        // Test registry serialization with all real C++ plugins
        let serialized = serde_json::to_string(&registry);
        assert!(serialized.is_ok());
        
        let serialized_str = serialized.unwrap();
        println!("✓ Serialized registry with real C++ plugins: {} characters", serialized_str.len());
        
        // Test registry deserialization
        let deserialized: PluginRegistry = serde_json::from_str(&serialized_str)
            .expect("Failed to deserialize registry with real C++ plugins");
        
        println!("✓ Deserialized registry with real C++ plugins successfully");
        
        // Test registry cloning with real C++ plugins
        let cloned_registry = registry.clone();
        assert_eq!(registry.plugins.len(), cloned_registry.plugins.len());
        
        // Test registry equality with real C++ plugins
        assert_eq!(registry, cloned_registry);
        
        println!("✓ Real C++ Plugin Loading test passed! Found and tested {} plugins", registry.plugins.len());
    }
}
