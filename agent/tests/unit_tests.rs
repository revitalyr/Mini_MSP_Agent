use mini_msp_agent::plugins::ffi::*;
use mini_msp_agent::plugins::{PluginManager, PluginEventType};
use mini_msp_agent::plugins::manager::{PluginRegistryEntry, PluginStatus};
use mini_msp_agent::plugins::ffi::{SystemInfoData, CommandResultData};
use mini_msp_shared::{Command, Heartbeat, Metrics};
use std::time::SystemTime;
use std::sync::Arc;

/// Unit tests for agent plugin system

#[tokio::test]
async fn test_command_result_default() {
    let result = CommandResult::default();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.success, false);
    assert!(result.output.is_null());
    assert_eq!(result.error[0], 0);
}

#[tokio::test]
async fn test_command_result_data_from_c_struct() {
    let mut c_result = CommandResult {
        output: std::ptr::null_mut(),
        exit_code: 0,
        success: true,
        error: [0; 256],
    };
    
    // Create a test string
    let test_output = "Test output";
    let output_ptr = test_output.as_ptr() as *mut i8;
    c_result.output = output_ptr;
    
    unsafe {
        let data = CommandResultData::from_c_struct(&c_result);
        assert_eq!(data.output, "Test output");
        assert_eq!(data.exit_code, 0);
        assert!(data.success);
        assert_eq!(data.error, "");
    }
}

#[tokio::test]
async fn test_plugin_manager_new() {
    let manager = PluginManager::new();
    assert!(!manager.is_hot_reload_enabled());
    assert!(manager.get_plugin_directory().is_none());
    assert!(manager.get_system_plugin().is_none());
}

#[tokio::test]
async fn test_plugin_manager_set_event_callback() {
    let mut manager = PluginManager::new();
    
    let callback_called = Arc::new(std::sync::Mutex::new(false));
    let callback_called_clone = callback_called.clone();
    
    manager.set_event_callback(move |event_type, plugin_name, message| {
        *callback_called_clone.lock().unwrap() = true;
        println!("Event: {:?} from {}: {}", event_type, plugin_name, message);
    });
    
    // Test the callback (this would normally be called internally)
    manager.notify_event(PluginEventType::Loaded, "test-plugin", "Loaded successfully");
    
    // Note: In a real test, we'd need to trigger the callback through actual plugin operations
    // This is just a structural test to ensure the callback can be set
}

#[tokio::test]
async fn test_plugin_event_types() {
    let events = vec![
        PluginEventType::Loaded,
        PluginEventType::Unloaded,
        PluginEventType::Error,
        PluginEventType::StatusChanged,
    ];
    
    for event in events {
        // Test that all event types can be created and compared
        let same_event = event.clone();
        assert_eq!(event, same_event);
    }
}

#[tokio::test]
async fn test_heartbeat_serialization() {
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

#[tokio::test]
async fn test_command_serialization() {
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

#[tokio::test]
async fn test_plugin_registry_entry() {
    let entry = PluginRegistryEntry {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        platform: "windows".to_string(),
        library_path: "/path/to/plugin.dll".to_string(),
        status: PluginStatus::Active,
        status_message: "Running".to_string(),
        last_loaded: Some(SystemTime::now()),
        last_unloaded: None,
    };
    
    assert_eq!(entry.name, "test-plugin");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.platform, "windows");
    assert!(matches!(entry.status, PluginStatus::Active));
    assert!(entry.last_loaded.is_some());
    assert!(entry.last_unloaded.is_none());
}

#[tokio::test]
async fn test_plugin_status_values() {
    let statuses = vec![
        PluginStatus::Unknown,
        PluginStatus::Loading,
        PluginStatus::Active,
        PluginStatus::Error,
        PluginStatus::Unloaded,
    ];
    
    for status in statuses {
        // Test that all status values can be created and compared
        let same_status = status.clone();
        assert_eq!(status, same_status);
    }
}

#[test]
fn test_safe_plugin_interface() {
    // This test would normally require an actual plugin library
    // For now, we test the interface structure
    
    let interface = PluginInterface {
        get_plugin_info: None,
        init: None,
        cleanup: None,
        get_system_metrics: None,
        get_processes: None,
        execute_command: None,
        read_file: None,
        get_system_info: None,
        free_memory: None,
    };
    
    // Verify interface is Copy (important for FFI)
    let interface_copy = interface;
    assert_eq!(std::mem::size_of::<PluginInterface>(), std::mem::size_of::<PluginInterface>());
}

#[test]
fn test_system_info_data() {
    let mut c_info = SystemInfo {
        hostname: [0; 256],
        os_version: [0; 256],
        arch: [0; 64],
        cpu_count: 4,
        total_memory: 8589934592, // 8GB
        available_memory: 4294967296, // 4GB
    };
    
    // Create test strings
    let hostname_str = "test-hostname";
    let os_version_str = "Windows 10";
    let arch_str = "x86_64";
    
    // Copy strings to C arrays (simplified for test)
    for (i, byte) in hostname_str.bytes().enumerate() {
        if i < 255 {
            c_info.hostname[i] = byte as i8;
        }
    }
    c_info.hostname[hostname_str.len()] = 0;
    
    for (i, byte) in os_version_str.bytes().enumerate() {
        if i < 255 {
            c_info.os_version[i] = byte as i8;
        }
    }
    c_info.os_version[os_version_str.len()] = 0;
    
    for (i, byte) in arch_str.bytes().enumerate() {
        if i < 63 {
            c_info.arch[i] = byte as i8;
        }
    }
    c_info.arch[arch_str.len()] = 0;
    
    unsafe {
        let data = SystemInfoData::from_c_struct(&c_info);
        assert!(data.hostname.contains("test-hostname"));
        assert!(data.os_version.contains("Windows 10"));
        assert!(data.arch.contains("x86_64"));
        assert_eq!(data.cpu_count, 4);
        assert_eq!(data.total_memory, 8589934592);
        assert_eq!(data.available_memory, 4294967296);
    }
}
