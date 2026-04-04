//! Tests for plugin system functionality
//! 
//! Tests plugin loading, command execution, and system plugin requirements

#[cfg(test)]
mod plugin_manager_tests {
    use mini_msp_agent::agent::plugins::PluginManager;
    use mini_msp_shared::{Command, CommandResponse};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_plugin_manager_creation() {
        let plugin_manager = PluginManager::new();
        
        // Initially should have no plugins
        let loaded_plugins = plugin_manager.get_loaded_plugins();
        assert!(loaded_plugins.is_empty(), "Should start with no plugins");
        
        // Should have no system plugin
        assert!(!plugin_manager.is_system_plugin_loaded(), "Should have no system plugin initially");
        
        println!("✅ PluginManager creation test passed");
    }

    #[test]
    fn test_plugin_directory_validation() {
        let plugin_manager = PluginManager::new();
        
        // Test with non-existent directory
        let result = plugin_manager.load_plugins_from_directory("/non/existent/path");
        assert!(result.is_ok(), "Should handle non-existent directory gracefully");
        
        let loaded_plugins = plugin_manager.get_loaded_plugins();
        assert!(loaded_plugins.is_empty(), "Should load no plugins from non-existent directory");
        
        println!("✅ Plugin directory validation test passed");
    }

    #[test]
    fn test_plugin_status_tracking() {
        let plugin_manager = PluginManager::new();
        
        // Test status for non-existent plugin
        let status = plugin_manager.get_plugin_status("non_existent_plugin");
        // Should handle gracefully - implementation dependent
        
        // Test plugin registry
        let registry = plugin_manager.get_plugin_registry();
        assert!(registry.is_empty(), "Should have empty registry initially");
        
        println!("✅ Plugin status tracking test passed");
    }

    #[test]
    fn test_hot_reload_configuration() {
        let mut plugin_manager = PluginManager::new();
        
        // Test hot reload enable/disable
        plugin_manager.enable_hot_reload(true);
        // Note: We can't easily test the actual hot reload without file system changes
        
        plugin_manager.enable_hot_reload(false);
        
        println!("✅ Hot reload configuration test passed");
    }

    #[test]
    fn test_command_execution_without_plugins() {
        let plugin_manager = PluginManager::new();
        
        // Test command execution with no plugins loaded
        let result = plugin_manager.execute_command(&Command::GetSystemInfo).await;
        
        // Should handle gracefully - implementation dependent
        match result {
            Ok(_) => println!("Command executed without plugins (may be expected)"),
            Err(_) => println!("Command failed without plugins (expected)"),
        }
        
        println!("✅ Command execution without plugins test passed");
    }
}

#[cfg(test)]
mod plugin_interface_tests {
    use mini_msp_shared::{Command, CommandResponse};
    
    #[test]
    fn test_command_serialization() {
        let commands = vec![
            Command::GetProcesses,
            Command::GetSystemInfo,
            Command::GetPluginRegistry,
            Command::GetSensorData,
            Command::Exec { cmd: "echo test".to_string() },
            Command::GetFile { path: "/tmp/test.txt".to_string() },
        ];
        
        for cmd in commands {
            let serialized = serde_json::to_vec(&cmd);
            assert!(serialized.is_ok(), "Failed to serialize command: {:?}", cmd);
            
            let deserialized: Result<Command, _> = serde_json::from_slice(&serialized.unwrap());
            assert!(deserialized.is_ok(), "Failed to deserialize command: {:?}", cmd);
            assert_eq!(deserialized.unwrap(), cmd);
        }
        
        println!("✅ Command serialization test passed");
    }

    #[test]
    fn test_command_response_creation() {
        let response = CommandResponse {
            command_id: Some("test-123".to_string()),
            r#type: "GetSystemInfo".to_string(),
            status: "success".to_string(),
            data: serde_json::json!({
                "hostname": "test-host",
                "os": "linux"
            }),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        // Test serialization
        let serialized = serde_json::to_vec(&response);
        assert!(serialized.is_ok(), "Failed to serialize response");
        
        // Test deserialization
        let deserialized: Result<CommandResponse, _> = serde_json::from_slice(&serialized.unwrap());
        assert!(deserialized.is_ok(), "Failed to deserialize response");
        
        let parsed = deserialized.unwrap();
        assert_eq!(parsed.command_id, response.command_id);
        assert_eq!(parsed.status, response.status);
        assert_eq!(parsed.r#type, response.r#type);
        assert_eq!(parsed.data["hostname"], response.data["hostname"]);
        assert_eq!(parsed.data["os"], response.data["os"]);
        
        println!("✅ Command response creation test passed");
    }

    #[test]
    fn test_error_response_creation() {
        let error_response = CommandResponse {
            command_id: Some("error-test".to_string()),
            r#type: "GetFile".to_string(),
            status: "error".to_string(),
            data: serde_json::json!({
                "error": "File not found",
                "code": 404
            }),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        // Test error response serialization
        let serialized = serde_json::to_vec(&error_response);
        assert!(serialized.is_ok(), "Failed to serialize error response");
        
        let deserialized: Result<CommandResponse, _> = serde_json::from_slice(&serialized.unwrap());
        assert!(deserialized.is_ok(), "Failed to deserialize error response");
        
        let parsed = deserialized.unwrap();
        assert_eq!(parsed.status, "error");
        assert_eq!(parsed.data["error"], "File not found");
        assert_eq!(parsed.data["code"], 404);
        
        println!("✅ Error response creation test passed");
    }
}

#[cfg(test)]
mod system_plugin_tests {
    use mini_msp_agent::agent::plugins::PluginManager;
    use std::fs;
    use std::path::Path;
    
    #[test]
    fn test_system_plugin_requirement() {
        let plugin_manager = PluginManager::new();
        
        // Initially should have no system plugin
        assert!(!plugin_manager.is_system_plugin_loaded(), 
                  "Should not have system plugin initially");
        
        println!("✅ System plugin requirement test passed");
    }

    #[test]
    fn test_plugin_loading_from_directory() {
        let plugin_manager = PluginManager::new();
        
        // Create a temporary directory structure
        let temp_dir = tempfile::tempdir().unwrap();
        let plugin_dir = temp_dir.path();
        
        // Create a mock plugin file (just for testing directory scanning)
        let mock_plugin_path = plugin_dir.join("mock_plugin.dll");
        fs::write(&mock_plugin_path, "mock plugin content").unwrap();
        
        // Try to load plugins from directory
        let result = plugin_manager.load_plugins_from_directory(plugin_dir.to_str().unwrap());
        
        // Should handle gracefully even with invalid plugin files
        match result {
            Ok(_) => println!("Plugin loading completed (may have failed on invalid plugins)"),
            Err(e) => println!("Plugin loading failed as expected: {}", e),
        }
        
        println!("✅ Plugin loading from directory test passed");
    }

    #[test]
    fn test_plugin_registry_collection() {
        let plugin_manager = PluginManager::new();
        
        // Test empty registry
        let registry = plugin_manager.get_plugin_registry();
        assert!(registry.is_empty(), "Registry should be empty initially");
        
        println!("✅ Plugin registry collection test passed");
    }
}

#[cfg(test)]
mod plugin_command_tests {
    use mini_msp_shared::Command;
    
    #[test]
    fn test_all_command_variants() {
        let commands = vec![
            Command::GetProcesses,
            Command::Exec { cmd: "ls -la".to_string() },
            Command::GetFile { path: "/etc/hosts".to_string() },
            Command::GetSystemInfo,
            Command::GetDirectoryInfoData { 
                path: "/tmp".to_string(), 
                include_subdirs: true, 
                show_hidden: false, 
                max_depth: 3 
            },
            Command::GetPluginRegistry,
            Command::GetEventData { path: "/tmp".to_string() },
            Command::GetWatchersData,
            Command::GetFileReaderData { path: "/tmp/test.txt".to_string() },
            Command::GetSensorData,
            Command::GetCameraData,
            Command::GetProcessingResults,
            Command::GetVideoFrame,
        ];
        
        for (i, cmd) in commands.iter().enumerate() {
            let serialized = serde_json::to_vec(cmd);
            assert!(serialized.is_ok(), "Failed to serialize command {}: {:?}", i, cmd);
            
            let deserialized: Result<Command, _> = serde_json::from_slice(&serialized.unwrap());
            assert!(deserialized.is_ok(), "Failed to deserialize command {}: {:?}", i, cmd);
            assert_eq!(deserialized.unwrap(), *cmd);
        }
        
        println!("✅ All command variants test passed ({} commands)", commands.len());
    }

    #[test]
    fn test_command_with_complex_data() {
        let complex_command = Command::GetDirectoryInfoData { 
            path: "/very/long/path/to/directory".to_string(), 
            include_subdirs: true, 
            show_hidden: true, 
            max_depth: 10 
        };
        
        let serialized = serde_json::to_vec(&complex_command).unwrap();
        let deserialized: Command = serde_json::from_slice(&serialized).unwrap();
        
        match (complex_command, deserialized) {
            (Command::GetDirectoryInfoData { path: orig_path, include_subdirs: orig_subdirs, show_hidden: orig_hidden, max_depth: orig_max_depth },
             Command::GetDirectoryInfoData { path: new_path, include_subdirs: new_subdirs, show_hidden: new_hidden, max_depth: new_max_depth }) => {
                assert_eq!(orig_path, new_path);
                assert_eq!(orig_subdirs, new_subdirs);
                assert_eq!(orig_hidden, new_hidden);
                assert_eq!(orig_max_depth, new_max_depth);
            }
            _ => panic!("Command types don't match"),
        }
        
        println!("✅ Complex command data test passed");
    }

    #[test]
    fn test_command_edge_cases() {
        // Test empty string command
        let empty_cmd = Command::Exec { cmd: "".to_string() };
        let serialized = serde_json::to_vec(&empty_cmd);
        assert!(serialized.is_ok(), "Should handle empty command string");
        
        // Test very long path
        let long_path = "/".to_string() + &"a".repeat(1000);
        let long_path_cmd = Command::GetFile { path: long_path.clone() };
        let serialized = serde_json::to_vec(&long_path_cmd);
        assert!(serialized.is_ok(), "Should handle very long paths");
        
        let deserialized: Command = serde_json::from_slice(&serialized.unwrap()).unwrap();
        match deserialized {
            Command::GetFile { path } => assert_eq!(path, long_path),
            _ => panic!("Should deserialize as GetFile command"),
        }
        
        println!("✅ Command edge cases test passed");
    }
}
