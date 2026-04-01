use mini_msp_shared::{Command, Heartbeat, Metrics};

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
}
