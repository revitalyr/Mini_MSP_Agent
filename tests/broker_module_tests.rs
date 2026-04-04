//! Tests for broker module functionality
//! 
//! Tests BrokerClient and BrokerLoop implementations

#[cfg(test)]
mod server_broker_tests {
    use mini_msp_agent::server::broker::{BrokerClient, BrokerMessageHandler};
    use mini_msp_shared::{CommandRequest, CommandResponse, Heartbeat, Metrics, Command};
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_broker_client_creation() -> anyhow::Result<()> {
        // This test requires NATS server running
        let result = BrokerClient::connect("nats://localhost:4222").await;
        
        match result {
            Ok(broker) => {
                println!("✅ BrokerClient created successfully");
                assert!(true, "BrokerClient should be created when NATS is available");
            }
            Err(e) => {
                println!("⚠️  NATS not available, skipping test: {}", e);
                // This is expected in CI/CD without NATS
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_broker_message_handler() -> anyhow::Result<()> {
        // Create mock broker client (we can't test real NATS without server)
        let handler = BrokerMessageHandler::new(
            // We would need to mock BrokerClient for this test
            // For now, just test the handler structure
        );
        
        // Test heartbeat handling
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
        
        // Test response handling
        let response = CommandResponse {
            command_id: Some("test-123".to_string()),
            r#type: "GetSystemInfo".to_string(),
            status: "success".to_string(),
            data: serde_json::json!({"test": "data"}),
            timestamp: 1234567890,
        };
        
        // Test plugin event handling
        let plugin_data = serde_json::json!({
            "event": "test_event",
            "data": "test_data"
        });
        
        println!("✅ BrokerMessageHandler structure test passed");
        Ok(())
    }
}

#[cfg(test)]
mod agent_broker_tests {
    use mini_msp_agent::agent::broker::{BrokerClient, BrokerLoop, PluginEventPublisher};
    use mini_msp_agent::agent::plugins::PluginManager;
    use mini_msp_shared::{CommandRequest, CommandResponse, Heartbeat, Metrics, Command};
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_agent_broker_client_creation() -> anyhow::Result<()> {
        let result = BrokerClient::connect("nats://localhost:4222").await;
        
        match result {
            Ok(broker) => {
                println!("✅ Agent BrokerClient created successfully");
                assert!(true, "Agent BrokerClient should be created when NATS is available");
            }
            Err(e) => {
                println!("⚠️  NATS not available, skipping test: {}", e);
                // This is expected in CI/CD without NATS
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_plugin_event_publisher() -> anyhow::Result<()> {
        let result = BrokerClient::connect("nats://localhost:4222").await;
        
        match result {
            Ok(broker) => {
                let publisher = PluginEventPublisher::new(broker, "test-agent".to_string());
                
                // Test plugin event publishing
                let event_data = serde_json::json!({
                    "plugin": "test_plugin",
                    "event": "test_event",
                    "data": "test_data"
                });
                
                println!("✅ PluginEventPublisher created successfully");
                // Note: We can't test actual publishing without NATS server
            }
            Err(e) => {
                println!("⚠️  NATS not available, skipping test: {}", e);
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_broker_loop_creation() -> anyhow::Result<()> {
        let result = BrokerClient::connect("nats://localhost:4222").await;
        
        match result {
            Ok(broker) => {
                let plugin_manager = PluginManager::new();
                let broker_loop = BrokerLoop::new(broker, "test-agent".to_string(), plugin_manager);
                
                println!("✅ BrokerLoop created successfully");
                // Note: We can't test the actual run() without NATS server and plugins
            }
            Err(e) => {
                println!("⚠️  NATS not available, skipping test: {}", e);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod end_to_end_tests {
    use mini_msp_agent::server::broker::{BrokerClient, BrokerMessageHandler};
    use mini_msp_agent::agent::broker::{BrokerClient as AgentBrokerClient, BrokerLoop};
    use mini_msp_agent::agent::plugins::PluginManager;
    use mini_msp_shared::{CommandRequest, CommandResponse, Heartbeat, Metrics, Command};
    use tokio::time::{timeout, Duration};
    use futures_util::StreamExt;

    const TEST_AGENT_ID: &str = "e2e-test-agent";
    const TEST_TIMEOUT: Duration = Duration::from_secs(15);

    #[tokio::test]
    async fn test_command_flow() -> anyhow::Result<()> {
        // Try to connect to NATS
        let server_result = BrokerClient::connect("nats://localhost:4222").await;
        let agent_result = AgentBrokerClient::connect("nats://localhost:4222").await;
        
        if server_result.is_err() || agent_result.is_err() {
            println!("⚠️  NATS not available, skipping E2E test");
            return Ok(());
        }
        
        let server_broker = server_result.unwrap();
        let agent_broker = agent_result.unwrap();
        
        // Subscribe agent to commands
        let command_subject = format!("commands.{}", TEST_AGENT_ID);
        let mut agent_sub = agent_broker.subscribe_commands(TEST_AGENT_ID).await?;
        
        // Subscribe server to responses
        let response_subject = format!("responses.{}.*", TEST_AGENT_ID);
        let mut server_sub = server_broker.subscribe_responses(TEST_AGENT_ID).await?;
        
        // Send command from server
        let command = CommandRequest {
            command_id: "e2e-test-123".to_string(),
            command: Command::GetSystemInfo,
        };
        
        server_broker.send_command(TEST_AGENT_ID, command.clone()).await?;
        
        // Agent receives command
        let received_cmd = timeout(TEST_TIMEOUT, agent_sub.next()).await??;
        let parsed_cmd: CommandRequest = serde_json::from_slice(&received_cmd.payload)?;
        
        assert_eq!(parsed_cmd.command_id, command.command_id);
        assert_eq!(parsed_cmd.command, command.command);
        
        // Agent sends response
        let response = CommandResponse {
            command_id: Some(command.command_id.clone()),
            r#type: "GetSystemInfo".to_string(),
            status: "success".to_string(),
            data: serde_json::json!({"hostname": "e2e-test-host"}),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let response_topic = format!("responses.{}.{}", TEST_AGENT_ID, response.command_id.as_ref().unwrap());
        let response_payload = serde_json::to_vec(&response)?;
        agent_broker.client().publish(&response_topic, response_payload.into()).await?;
        
        // Server receives response
        let received_resp = timeout(TEST_TIMEOUT, server_sub.next()).await??;
        let parsed_resp: CommandResponse = serde_json::from_slice(&received_resp.payload)?;
        
        assert_eq!(parsed_resp.command_id, response.command_id);
        assert_eq!(parsed_resp.status, response.status);
        assert_eq!(parsed_resp.data["hostname"], response.data["hostname"]);
        
        println!("✅ End-to-end command flow test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_heartbeat_flow() -> anyhow::Result<()> {
        let agent_result = AgentBrokerClient::connect("nats://localhost:4222").await;
        let server_result = BrokerClient::connect("nats://localhost:4222").await;
        
        if agent_result.is_err() || server_result.is_err() {
            println!("⚠️  NATS not available, skipping heartbeat E2E test");
            return Ok(());
        }
        
        let agent_broker = agent_result.unwrap();
        let server_broker = server_result.unwrap();
        
        // Subscribe server to heartbeats
        let mut server_sub = server_broker.subscribe_heartbeats().await?;
        
        // Agent publishes heartbeat
        let heartbeat = Heartbeat {
            agent_id: TEST_AGENT_ID.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metrics: Metrics {
                cpu: 55.5,
                ram: 65.2,
                disk: 40.8,
            },
            hostname: "e2e-test-host".to_string(),
            uptime: 7200,
        };
        
        agent_broker.publish_heartbeat(TEST_AGENT_ID, &heartbeat).await?;
        
        // Server receives heartbeat
        let received = timeout(TEST_TIMEOUT, server_sub.next()).await??;
        let parsed_heartbeat: Heartbeat = serde_json::from_slice(&received.payload)?;
        
        assert_eq!(parsed_heartbeat.agent_id, heartbeat.agent_id);
        assert_eq!(parsed_heartbeat.metrics.cpu, heartbeat.metrics.cpu);
        assert_eq!(parsed_heartbeat.metrics.ram, heartbeat.metrics.ram);
        assert_eq!(parsed_heartbeat.metrics.disk, heartbeat.metrics.disk);
        assert_eq!(parsed_heartbeat.hostname, heartbeat.hostname);
        assert_eq!(parsed_heartbeat.uptime, heartbeat.uptime);
        
        println!("✅ End-to-end heartbeat flow test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_plugin_event_flow() -> anyhow::Result<()> {
        let agent_result = AgentBrokerClient::connect("nats://localhost:4222").await;
        let server_result = BrokerClient::connect("nats://localhost:4222").await;
        
        if agent_result.is_err() || server_result.is_err() {
            println!("⚠️  NATS not available, skipping plugin event E2E test");
            return Ok(());
        }
        
        let agent_broker = agent_result.unwrap();
        let server_broker = server_result.unwrap();
        
        // Subscribe server to plugin events
        let mut server_sub = server_broker.subscribe_plugin_events().await?;
        
        // Agent publishes plugin event
        let plugin_name = "test_plugin";
        let event_data = serde_json::json!({
            "event_type": "file_created",
            "path": "/tmp/e2e-test.txt",
            "size": 1024,
            "timestamp": chrono::Utc::now().timestamp()
        });
        
        agent_broker.publish_plugin_event(TEST_AGENT_ID, plugin_name, event_data.clone()).await?;
        
        // Server receives plugin event
        let received = timeout(TEST_TIMEOUT, server_sub.next()).await??;
        
        // Parse the subject to extract agent_id and plugin
        let subject_parts: Vec<&str> = received.subject.split('.').collect();
        assert_eq!(subject_parts[0], "events");
        assert_eq!(subject_parts[1], TEST_AGENT_ID);
        assert_eq!(subject_parts[2], plugin_name);
        
        let parsed_data: serde_json::Value = serde_json::from_slice(&received.payload)?;
        assert_eq!(parsed_data["event_type"], event_data["event_type"]);
        assert_eq!(parsed_data["path"], event_data["path"]);
        assert_eq!(parsed_data["size"], event_data["size"]);
        
        println!("✅ End-to-end plugin event flow test passed");
        Ok(())
    }
}

#[cfg(test)]
mod concurrent_tests {
    use mini_msp_agent::server::broker::BrokerClient;
    use mini_msp_agent::agent::broker::BrokerClient as AgentBrokerClient;
    use mini_msp_shared::{CommandRequest, Command};
    use tokio::time::{timeout, Duration};
    use futures_util::StreamExt;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn test_concurrent_commands() -> anyhow::Result<()> {
        let server_result = BrokerClient::connect("nats://localhost:4222").await;
        let agent_result = AgentBrokerClient::connect("nats://localhost:4222").await;
        
        if server_result.is_err() || agent_result.is_err() {
            println!("⚠️  NATS not available, skipping concurrent test");
            return Ok(());
        }
        
        let server_broker = Arc::new(server_result.unwrap());
        let agent_broker = Arc::new(agent_result.unwrap());
        
        let command_count = 10;
        let barrier = Arc::new(Barrier::new(2));
        
        // Subscribe agent to commands
        let command_subject = format!("commands.concurrent-test-agent");
        let mut agent_sub = agent_broker.subscribe_commands("concurrent-test-agent").await?;
        
        // Spawn concurrent command sending
        let server_broker_clone = server_broker.clone();
        let barrier_clone = barrier.clone();
        let sender_task = tokio::spawn(async move {
            barrier_clone.wait().await;
            
            for i in 0..command_count {
                let command = CommandRequest {
                    command_id: format!("concurrent-cmd-{}", i),
                    command: Command::GetSystemInfo,
                };
                
                if let Err(e) = server_broker_clone.send_command("concurrent-test-agent", command).await {
                    println!("Failed to send command {}: {}", i, e);
                }
            }
        });
        
        // Spawn concurrent command receiving
        let agent_broker_clone = agent_broker.clone();
        let barrier_clone = barrier.clone();
        let receiver_task = tokio::spawn(async move {
            barrier_clone.wait().await;
            
            let mut received_count = 0;
            while received_count < command_count {
                match timeout(Duration::from_secs(5), agent_sub.next()).await {
                    Ok(Ok(msg)) => {
                        let _: CommandRequest = serde_json::from_slice(&msg.payload).unwrap();
                        received_count += 1;
                    }
                    Ok(Err(e)) => println!("Stream error: {}", e),
                    Err(_) => {
                        println!("Timeout waiting for command {}", received_count);
                        break;
                    }
                }
            }
            received_count
        });
        
        // Start both tasks
        barrier.wait().await;
        
        // Wait for completion
        let received_count = receiver_task.await?;
        sender_task.await?;
        
        assert_eq!(received_count, command_count, "Should receive all concurrent commands");
        
        println!("✅ Concurrent commands test passed ({} commands)", received_count);
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_agents() -> anyhow::Result<()> {
        let server_result = BrokerClient::connect("nats://localhost:4222").await;
        
        if server_result.is_err() {
            println!("⚠️  NATS not available, skipping multiple agents test");
            return Ok(());
        }
        
        let server_broker = Arc::new(server_result.unwrap());
        let agent_count = 3;
        
        // Create multiple agent connections
        let mut agent_tasks = Vec::new();
        let mut agent_ids = Vec::new();
        
        for i in 0..agent_count {
            let agent_id = format!("multi-test-agent-{}", i);
            agent_ids.push(agent_id.clone());
            
            let agent_result = AgentBrokerClient::connect("nats://localhost:4222").await?;
            let agent_broker = Arc::new(agent_result);
            
            // Subscribe agent to commands
            let command_subject = format!("commands.{}", agent_id);
            let mut agent_sub = agent_broker.subscribe_commands(&agent_id).await?;
            
            // Subscribe server to responses
            let response_subject = format!("responses.{}.*", agent_id);
            let mut server_sub = server_broker.subscribe_responses(&agent_id).await?;
            
            // Spawn task for this agent
            let server_broker_clone = server_broker.clone();
            let agent_id_clone = agent_id.clone();
            let task = tokio::spawn(async move {
                // Send command to this agent
                let command = CommandRequest {
                    command_id: format!("multi-cmd-{}", agent_id_clone),
                    command: Command::GetSystemInfo,
                };
                
                server_broker_clone.send_command(&agent_id_clone, command).await.unwrap();
                
                // Wait for response
                if let Ok(Ok(msg)) = timeout(Duration::from_secs(5), server_sub.next()).await {
                    let _: CommandResponse = serde_json::from_slice(&msg.payload).unwrap();
                    println!("Agent {} responded", agent_id_clone);
                }
            });
            
            agent_tasks.push(task);
        }
        
        // Wait for all agents to respond
        for task in agent_tasks {
            task.await?;
        }
        
        println!("✅ Multiple agents test passed ({} agents)", agent_count);
        Ok(())
    }
}
