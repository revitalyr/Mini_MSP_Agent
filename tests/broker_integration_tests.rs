//! Integration tests for NATS broker functionality
//! 
//! Tests broker communication with real NATS server
//! Requires NATS server running on localhost:4222

use mini_msp_shared::{
    BrokerMessage, BrokerPayload, CommandRequest, CommandResponse, 
    Heartbeat, Metrics, Command
};
use async_nats::Client;
use tokio::time::{timeout, Duration};
use futures_util::StreamExt;

/// Test configuration
const NATS_URL: &str = "nats://localhost:4222";
const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_AGENT_ID: &str = "test-integration-agent";

/// Helper to connect to NATS
async fn connect_to_nats() -> anyhow::Result<Client> {
    let client = async_nats::connect(NATS_URL).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to NATS: {}", e))?;
    Ok(client)
}

/// Helper to create test heartbeat
fn create_test_heartbeat() -> Heartbeat {
    Heartbeat {
        agent_id: TEST_AGENT_ID.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        metrics: Metrics {
            cpu: 45.5,
            ram: 60.2,
            disk: 30.8,
        },
        hostname: "test-host".to_string(),
        uptime: 3600,
    }
}

/// Helper to create test command
fn create_test_command() -> CommandRequest {
    CommandRequest {
        command_id: "test-cmd-123".to_string(),
        command: Command::GetSystemInfo,
    }
}

/// Helper to create test response
fn create_test_response() -> CommandResponse {
    CommandResponse {
        command_id: Some("test-cmd-123".to_string()),
        r#type: "GetSystemInfo".to_string(),
        status: "success".to_string(),
        data: serde_json::json!({
            "hostname": "test-host",
            "os": "linux",
            "version": "5.15.0"
        }),
        timestamp: chrono::Utc::now().timestamp(),
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_connection() -> anyhow::Result<()> {
        // Test basic NATS connection
        let client = connect_to_nats().await?;
        
        // Test simple publish/subscribe
        let subject = "test.connection";
        let mut subscriber = client.subscribe(subject).await?;
        
        let message = b"hello world";
        client.publish(subject, message.into()).await?;
        
        // Wait for message
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        assert_eq!(received.payload, message);
        
        println!("✅ NATS connection test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_command_publish_subscribe() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to commands for test agent
        let command_subject = format!("commands.{}", TEST_AGENT_ID);
        let mut subscriber = client.subscribe(&command_subject).await?;
        
        // Publish command
        let command = create_test_command();
        let payload = serde_json::to_vec(&command)?;
        client.publish(&command_subject, payload.into()).await?;
        
        // Wait for command
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        let received_cmd: CommandRequest = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(received_cmd.command_id, command.command_id);
        assert_eq!(received_cmd.command, command.command);
        
        println!("✅ Command publish/subscribe test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_heartbeat_publish_subscribe() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to heartbeats for test agent
        let heartbeat_subject = format!("heartbeat.{}", TEST_AGENT_ID);
        let mut subscriber = client.subscribe(&heartbeat_subject).await?;
        
        // Publish heartbeat
        let heartbeat = create_test_heartbeat();
        let payload = serde_json::to_vec(&heartbeat)?;
        client.publish(&heartbeat_subject, payload.into()).await?;
        
        // Wait for heartbeat
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        let received_heartbeat: Heartbeat = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(received_heartbeat.agent_id, heartbeat.agent_id);
        assert_eq!(received_heartbeat.metrics.cpu, heartbeat.metrics.cpu);
        assert_eq!(received_heartbeat.metrics.ram, heartbeat.metrics.ram);
        assert_eq!(received_heartbeat.metrics.disk, heartbeat.metrics.disk);
        assert_eq!(received_heartbeat.hostname, heartbeat.hostname);
        
        println!("✅ Heartbeat publish/subscribe test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_response_publish_subscribe() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to responses for test agent
        let response_subject = format!("responses.{}.*", TEST_AGENT_ID);
        let mut subscriber = client.subscribe(&response_subject).await?;
        
        // Publish response
        let response = create_test_response();
        let response_topic = format!("responses.{}.{}", TEST_AGENT_ID, response.command_id.as_ref().unwrap());
        let payload = serde_json::to_vec(&response)?;
        client.publish(&response_topic, payload.into()).await?;
        
        // Wait for response
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        let received_response: CommandResponse = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(received_response.command_id, response.command_id);
        assert_eq!(received_response.status, response.status);
        assert_eq!(received_response.r#type, response.r#type);
        
        println!("✅ Response publish/subscribe test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_plugin_event_publish_subscribe() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to plugin events for test agent
        let event_subject = format!("events.{}.*", TEST_AGENT_ID);
        let mut subscriber = client.subscribe(&event_subject).await?;
        
        // Publish plugin event
        let plugin_name = "test_plugin";
        let event_data = serde_json::json!({
            "event_type": "file_changed",
            "path": "/tmp/test.txt",
            "timestamp": chrono::Utc::now().timestamp()
        });
        
        let event_topic = format!("events.{}.{}", TEST_AGENT_ID, plugin_name);
        let payload = serde_json::to_vec(&event_data)?;
        client.publish(&event_topic, payload.into()).await?;
        
        // Wait for event
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        let received_data: serde_json::Value = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(received_data["event_type"], event_data["event_type"]);
        assert_eq!(received_data["path"], event_data["path"]);
        
        println!("✅ Plugin event publish/subscribe test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_broker_message_round_trip() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to all topics for test agent
        let command_subject = format!("commands.{}", TEST_AGENT_ID);
        let response_subject = format!("responses.{}.*", TEST_AGENT_ID);
        
        let mut cmd_sub = client.subscribe(&command_subject).await?;
        let mut resp_sub = client.subscribe(&response_subject).await?;
        
        // Send broker message
        let broker_msg = BrokerMessage {
            agent_id: TEST_AGENT_ID.to_string(),
            payload: BrokerPayload::Command(create_test_command()),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let msg_payload = serde_json::to_vec(&broker_msg)?;
        client.publish(&command_subject, msg_payload.into()).await?;
        
        // Receive and verify
        let received = timeout(TEST_TIMEOUT, cmd_sub.next()).await??;
        let received_msg: BrokerMessage = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(received_msg.agent_id, broker_msg.agent_id);
        assert_eq!(received_msg.timestamp, broker_msg.timestamp);
        
        match (received_msg.payload, broker_msg.payload) {
            (BrokerPayload::Command(received_cmd), BrokerPayload::Command(original_cmd)) => {
                assert_eq!(received_cmd.command_id, original_cmd.command_id);
                assert_eq!(received_cmd.command, original_cmd.command);
            }
            _ => panic!("Payload types don't match"),
        }
        
        println!("✅ Broker message round-trip test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_subscribers() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Create multiple subscribers for the same topic
        let subject = format!("commands.{}", TEST_AGENT_ID);
        let mut sub1 = client.subscribe(&subject).await?;
        let mut sub2 = client.subscribe(&subject).await?;
        let mut sub3 = client.subscribe(&subject).await?;
        
        // Publish message
        let command = create_test_command();
        let payload = serde_json::to_vec(&command)?;
        client.publish(&subject, payload.into()).await?;
        
        // All subscribers should receive the message
        let received1 = timeout(TEST_TIMEOUT, sub1.next()).await??;
        let received2 = timeout(TEST_TIMEOUT, sub2.next()).await??;
        let received3 = timeout(TEST_TIMEOUT, sub3.next()).await??;
        
        let cmd1: CommandRequest = serde_json::from_slice(&received1.payload)?;
        let cmd2: CommandRequest = serde_json::from_slice(&received2.payload)?;
        let cmd3: CommandRequest = serde_json::from_slice(&received3.payload)?;
        
        assert_eq!(cmd1.command_id, command.command_id);
        assert_eq!(cmd2.command_id, command.command_id);
        assert_eq!(cmd3.command_id, command.command_id);
        
        println!("✅ Multiple subscribers test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_wildcard_subscriptions() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Subscribe to wildcard topics
        let mut agent_sub = client.subscribe("heartbeat.*").await?;
        let mut response_sub = client.subscribe("responses.*.*").await?;
        
        // Publish to specific topics
        let heartbeat_subject = format!("heartbeat.{}", TEST_AGENT_ID);
        let response_subject = format!("responses.{}.test-123", TEST_AGENT_ID);
        
        // Publish heartbeat
        let heartbeat = create_test_heartbeat();
        let hb_payload = serde_json::to_vec(&heartbeat)?;
        client.publish(&heartbeat_subject, hb_payload.into()).await?;
        
        // Publish response
        let response = create_test_response();
        let resp_payload = serde_json::to_vec(&response)?;
        client.publish(&response_subject, resp_payload.into()).await?;
        
        // Verify wildcard subscriptions work
        let hb_received = timeout(TEST_TIMEOUT, agent_sub.next()).await??;
        let resp_received = timeout(TEST_TIMEOUT, response_sub.next()).await??;
        
        let received_hb: Heartbeat = serde_json::from_slice(&hb_received.payload)?;
        let received_resp: CommandResponse = serde_json::from_slice(&resp_received.payload)?;
        
        assert_eq!(received_hb.agent_id, heartbeat.agent_id);
        assert_eq!(received_resp.command_id, response.command_id);
        
        println!("✅ Wildcard subscription test passed");
        Ok(())
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_high_throughput_publishing() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        let subject = format!("commands.{}", TEST_AGENT_ID);
        let message_count = 1000;
        
        // Create test messages
        let messages: Vec<Vec<u8>> = (0..message_count)
            .map(|i| {
                let cmd = CommandRequest {
                    command_id: format!("perf-test-{}", i),
                    command: Command::GetSystemInfo,
                };
                serde_json::to_vec(&cmd).unwrap()
            })
            .collect();
        
        // Measure publishing performance
        let start = Instant::now();
        
        for message in messages {
            client.publish(&subject, message.into()).await?;
        }
        
        let duration = start.elapsed();
        let msgs_per_sec = message_count as f64 / duration.as_secs_f64();
        
        println!("Published {} messages in {:?} ({:.2} msgs/sec)", 
                message_count, duration, msgs_per_sec);
        
        // Should handle at least 100 msgs/sec
        assert!(msgs_per_sec >= 100.0, "Publishing too slow: {:.2} msgs/sec", msgs_per_sec);
        
        println!("✅ High throughput publishing test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_large_message_handling() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        // Create large message (simulating big system info)
        let large_data = serde_json::json!({
            "processes": (0..1000).map(|i| {
                serde_json::json!({
                    "pid": i,
                    "name": format!("process_{}", i),
                    "cpu": (i % 100) as f64,
                    "memory": (i * 1024) as u64
                })
            }).collect::<Vec<_>>(),
            "files": (0..1000).map(|i| format!("/path/to/file_{}.txt", i)).collect::<Vec<_>>(),
            "network_connections": (0..500).map(|i| {
                serde_json::json!({
                    "local": format!("192.168.1.1:{}", i),
                    "remote": format!("10.0.0.1:{}", i),
                    "state": "ESTABLISHED"
                })
            }).collect::<Vec<_>>()
        });
        
        let response = CommandResponse {
            command_id: Some("large-response".to_string()),
            r#type: "GetSystemInfo".to_string(),
            status: "success".to_string(),
            data: large_data,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let response_subject = format!("responses.{}.large-response", TEST_AGENT_ID);
        let payload = serde_json::to_vec(&response)?;
        
        println!("Large message size: {} bytes", payload.len());
        
        // Subscribe to response
        let mut subscriber = client.subscribe(&response_subject).await?;
        
        // Publish large message
        let start = Instant::now();
        client.publish(&response_subject, payload.into()).await?;
        let publish_time = start.elapsed();
        
        // Receive large message
        let start = Instant::now();
        let received = timeout(Duration::from_secs(30), subscriber.next()).await??;
        let receive_time = start.elapsed();
        
        let received_response: CommandResponse = serde_json::from_slice(&received.payload)?;
        
        // Verify data integrity
        assert_eq!(received_response.command_id, response.command_id);
        assert_eq!(received_response.data["processes"].as_array().unwrap().len(), 1000);
        assert_eq!(received_response.data["files"].as_array().unwrap().len(), 1000);
        assert_eq!(received_response.data["network_connections"].as_array().unwrap().len(), 500);
        
        println!("Publish time: {:?}, Receive time: {:?}", publish_time, receive_time);
        
        // Large messages should be handled reasonably quickly (< 5 seconds total)
        assert!(publish_time + receive_time < Duration::from_secs(5));
        
        println!("✅ Large message handling test passed");
        Ok(())
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_failure_handling() -> anyhow::Result<()> {
        // Try to connect to non-existent NATS server
        let result = async_nats::connect("nats://localhost:9999").await;
        
        assert!(result.is_err(), "Should fail to connect to non-existent server");
        
        println!("✅ Connection failure handling test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_message_handling() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        let subject = format!("commands.{}", TEST_AGENT_ID);
        let mut subscriber = client.subscribe(&subject).await?;
        
        // Publish invalid JSON
        let invalid_payload = b"{ invalid json message }";
        client.publish(subject, invalid_payload.into()).await?;
        
        // Try to receive and deserialize (should fail gracefully)
        let received = timeout(TEST_TIMEOUT, subscriber.next()).await??;
        let result: Result<CommandRequest, _> = serde_json::from_slice(&received.payload);
        
        assert!(result.is_err(), "Should fail to deserialize invalid JSON");
        
        println!("✅ Invalid message handling test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_timeout_handling() -> anyhow::Result<()> {
        let client = connect_to_nats().await?;
        
        let subject = "test.timeout";
        let mut subscriber = client.subscribe(subject).await?;
        
        // Wait for message that will never come
        let start = Instant::now();
        let result = timeout(Duration::from_millis(100), subscriber.next()).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_err(), "Should timeout waiting for message");
        assert!(elapsed >= Duration::from_millis(100), "Should wait at least timeout duration");
        
        println!("✅ Timeout handling test passed");
        Ok(())
    }
}
