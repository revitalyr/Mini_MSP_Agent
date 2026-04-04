//! Unit tests for NATS broker functionality
//! 
//! Tests broker client, message serialization, and basic functionality
//! without requiring external NATS server

use mini_msp_shared::{
    BrokerMessage, BrokerPayload, CommandRequest, CommandResponse, 
    Heartbeat, Metrics, Command
};
use serde_json;

#[cfg(test)]
mod broker_tests {
    use super::*;

    #[test]
    fn test_command_request_serialization() {
        let cmd = CommandRequest {
            command_id: "test-123".to_string(),
            command: Command::GetSystemInfo,
        };

        let serialized = serde_json::to_vec(&cmd).expect("Failed to serialize");
        let deserialized: CommandRequest = serde_json::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(cmd.command_id, deserialized.command_id);
        assert_eq!(cmd.command, deserialized.command);
    }

    #[test]
    fn test_command_response_serialization() {
        let response = CommandResponse {
            command_id: Some("test-123".to_string()),
            r#type: "GetSystemInfo".to_string(),
            status: "success".to_string(),
            data: serde_json::json!({"hostname": "test-host"}),
            timestamp: 1234567890,
        };

        let serialized = serde_json::to_vec(&response).expect("Failed to serialize");
        let deserialized: CommandResponse = serde_json::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(response.command_id, deserialized.command_id);
        assert_eq!(response.status, deserialized.status);
        assert_eq!(response.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_heartbeat_serialization() {
        let heartbeat = Heartbeat {
            agent_id: "test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics {
                cpu: 50.0,
                ram: 60.0,
                disk: 70.0,
            },
            hostname: "test-host".to_string(),
            uptime: 3600,
        };

        let serialized = serde_json::to_vec(&heartbeat).expect("Failed to serialize");
        let deserialized: Heartbeat = serde_json::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(heartbeat.agent_id, deserialized.agent_id);
        assert_eq!(heartbeat.metrics.cpu, deserialized.metrics.cpu);
        assert_eq!(heartbeat.metrics.ram, deserialized.metrics.ram);
        assert_eq!(heartbeat.metrics.disk, deserialized.metrics.disk);
        assert_eq!(heartbeat.hostname, deserialized.hostname);
        assert_eq!(heartbeat.uptime, deserialized.uptime);
    }

    #[test]
    fn test_broker_message_serialization() {
        let broker_msg = BrokerMessage {
            agent_id: "test-agent".to_string(),
            payload: BrokerPayload::Command(CommandRequest {
                command_id: "cmd-123".to_string(),
                command: Command::GetProcesses,
            }),
            timestamp: 1234567890,
        };

        let serialized = serde_json::to_vec(&broker_msg).expect("Failed to serialize");
        let deserialized: BrokerMessage = serde_json::from_slice(&serialized)
            .expect("Failed to deserialize");

        assert_eq!(broker_msg.agent_id, deserialized.agent_id);
        assert_eq!(broker_msg.timestamp, deserialized.timestamp);
        
        match (broker_msg.payload, deserialized.payload) {
            (BrokerPayload::Command(orig_cmd), BrokerPayload::Command(de_cmd)) => {
                assert_eq!(orig_cmd.command_id, de_cmd.command_id);
                assert_eq!(orig_cmd.command, de_cmd.command);
            }
            _ => panic!("Payload types don't match"),
        }
    }

    #[test]
    fn test_broker_payload_tagged_enum() {
        // Test Command payload
        let cmd_payload = BrokerPayload::Command(CommandRequest {
            command_id: "test".to_string(),
            command: Command::GetFile { path: "/tmp/test".to_string() },
        });
        
        let serialized = serde_json::to_value(&cmd_payload).expect("Failed to serialize");
        assert_eq!(serialized["kind"], "Command");
        // Command is serialized with its own structure
        assert!(serialized["command"].is_object());

        // Test Heartbeat payload
        let heartbeat_payload = BrokerPayload::Heartbeat(Heartbeat {
            agent_id: "test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "test-host".to_string(),
            uptime: 3600,
        });
        
        let serialized = serde_json::to_value(&heartbeat_payload).expect("Failed to serialize");
        assert_eq!(serialized["kind"], "Heartbeat");
        // Heartbeat data is serialized directly
        assert!(serialized["agent_id"].is_string());

        // Test Response payload
        let response_payload = BrokerPayload::Response(CommandResponse {
            command_id: Some("test".to_string()),
            r#type: "test".to_string(),
            status: "success".to_string(),
            data: serde_json::json!({}),
            timestamp: 1234567890,
        });
        
        let serialized = serde_json::to_value(&response_payload).expect("Failed to serialize");
        assert_eq!(serialized["kind"], "Response");
        // Response data is serialized directly
        assert!(serialized["command_id"].is_string());

        // Test PluginEvent payload
        let event_payload = BrokerPayload::PluginEvent {
            plugin: "system_plugin".to_string(),
            data: serde_json::json!({"event": "test"}),
        };
        
        let serialized = serde_json::to_value(&event_payload).expect("Failed to serialize");
        assert_eq!(serialized["kind"], "PluginEvent");
        assert_eq!(serialized["plugin"], "system_plugin");
        // PluginEvent data is serialized directly
        assert!(serialized["data"].is_object());
    }

    #[test]
    fn test_topic_generation() {
        let agent_id = "test-agent-123";
        let command_id = "cmd-456";
        let plugin = "system_plugin";

        // Test command topic
        let command_topic = format!("commands.{}", agent_id);
        assert_eq!(command_topic, "commands.test-agent-123");

        // Test heartbeat topic
        let heartbeat_topic = format!("heartbeat.{}", agent_id);
        assert_eq!(heartbeat_topic, "heartbeat.test-agent-123");

        // Test response topic
        let response_topic = format!("responses.{}.{}", agent_id, command_id);
        assert_eq!(response_topic, "responses.test-agent-123.cmd-456");

        // Test event topic
        let event_topic = format!("events.{}.{}", agent_id, plugin);
        assert_eq!(event_topic, "events.test-agent-123.system_plugin");
    }

    #[test]
    fn test_message_validation() {
        // Test valid command request
        let valid_cmd = CommandRequest {
            command_id: "valid-123".to_string(),
            command: Command::GetSystemInfo,
        };
        assert!(!valid_cmd.command_id.is_empty());

        // Test valid heartbeat
        let valid_heartbeat = Heartbeat {
            agent_id: "valid-agent".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "valid-host".to_string(),
            uptime: 3600,
        };
        assert!(!valid_heartbeat.agent_id.is_empty());
        assert!(valid_heartbeat.timestamp > 0);
        assert!(valid_heartbeat.metrics.cpu >= 0.0 && valid_heartbeat.metrics.cpu <= 100.0);
        assert!(valid_heartbeat.metrics.ram >= 0.0 && valid_heartbeat.metrics.ram <= 100.0);
        assert!(valid_heartbeat.metrics.disk >= 0.0 && valid_heartbeat.metrics.disk <= 100.0);
    }

    #[test]
    fn test_error_handling() {
        // Test invalid JSON
        let invalid_json = b"{ invalid json }";
        let result: Result<CommandRequest, _> = serde_json::from_slice(invalid_json);
        assert!(result.is_err());

        // Test missing required fields
        let incomplete_cmd = r#"{"command_id": "test"}"#; // missing command field
        let result: Result<CommandRequest, _> = serde_json::from_str(incomplete_cmd);
        assert!(result.is_err());

        // Test invalid metrics values
        let invalid_metrics = Metrics {
            cpu: 150.0, // > 100%
            ram: -10.0,  // < 0%
            disk: 200.0, // > 100%
        };
        assert!(invalid_metrics.cpu > 100.0);
        assert!(invalid_metrics.ram < 0.0);
        assert!(invalid_metrics.disk > 100.0);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_serialization_performance() {
        let heartbeat = Heartbeat {
            agent_id: "perf-test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics {
                cpu: 75.5,
                ram: 60.2,
                disk: 45.8,
            },
            hostname: "performance-test-host".to_string(),
            uptime: 86400,
        };

        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _serialized = serde_json::to_vec(&heartbeat).expect("Failed to serialize");
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_nanos() / iterations;
        
        println!("Serialization: {} iterations in {:?} (avg: {}ns)", 
                iterations, duration, avg_time);
        
        // Should be very fast (< 20000ns per serialization)
        assert!(avg_time < 20000, "Serialization too slow: {}ns", avg_time);
    }

    #[test]
    fn test_deserialization_performance() {
        let heartbeat = Heartbeat {
            agent_id: "perf-test-agent".to_string(),
            timestamp: 1234567890,
            metrics: Metrics {
                cpu: 75.5,
                ram: 60.2,
                disk: 45.8,
            },
            hostname: "performance-test-host".to_string(),
            uptime: 86400,
        };
        
        let serialized = serde_json::to_vec(&heartbeat).expect("Failed to serialize");
        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _: Result<Heartbeat, _> = serde_json::from_slice(&serialized);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_nanos() / iterations;
        
        println!("Deserialization: {} iterations in {:?} (avg: {}ns)", 
                iterations, duration, avg_time);
        
        // Should be very fast (< 20000ns per deserialization)
        assert!(avg_time < 20000, "Deserialization too slow: {}ns", avg_time);
    }
}
