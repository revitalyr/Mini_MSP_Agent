use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
};
use mini_msp_shared::{Heartbeat, Metrics, Command};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

// Import the server modules
use mini_msp_agent::server::{AppState, WebSocketManager};
use mini_msp_agent::server::{health_check, handle_heartbeat, list_agents, send_command};

/// Integration tests for the Mini MSP Server
/// These tests verify the complete API functionality

#[tokio::test]
async fn test_complete_agent_workflow() {
    let app = create_test_app().await;
    
    // 1. Check initial state - no agents
    let response = get_request(&app, "/agents").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body(response).await;
    assert!(body.contains("\"count\":0"));
    
    // 2. Send heartbeat from agent
    let heartbeat = Heartbeat {
        agent_id: "test-agent-001".to_string(),
        timestamp: 1234567890,
        metrics: Metrics {
            cpu: 45.5,
            ram: 60.2,
            disk: 75.8,
        },
        hostname: "test-host".to_string(),
        uptime: 3600,
    };
    
    let response = post_request(&app, "/heartbeat", &heartbeat).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    // 3. Verify agent is now registered
    let response = get_request(&app, "/agents").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body(response).await;
    assert!(body.contains("\"count\":1"));
    assert!(body.contains("test-agent-001"));
    
    // 4. Send command to agent (will fail since no WebSocket connection)
    let command = Command::GetSystemInfo;
    let response = post_request(&app, "/agents/test-agent-001/command", &command).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response_body(response).await.contains("not connected"));
}

#[tokio::test]
async fn test_multiple_agents() {
    let app = create_test_app().await;
    
    // Register multiple agents
    let agents = vec![
        Heartbeat {
            agent_id: "agent-1".to_string(),
            timestamp: 1234567890,
            metrics: Metrics { cpu: 30.0, ram: 50.0, disk: 60.0 },
            hostname: "host-1".to_string(),
            uptime: 1000,
        },
        Heartbeat {
            agent_id: "agent-2".to_string(),
            timestamp: 1234567891,
            metrics: Metrics { cpu: 40.0, ram: 55.0, disk: 65.0 },
            hostname: "host-2".to_string(),
            uptime: 2000,
        },
        Heartbeat {
            agent_id: "agent-3".to_string(),
            timestamp: 1234567892,
            metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
            hostname: "host-3".to_string(),
            uptime: 3000,
        },
    ];
    
    for agent in &agents {
        let response = post_request(&app, "/heartbeat", agent).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    // Verify all agents are registered
    let response = get_request(&app, "/agents").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body(response).await;
    assert!(body.contains("\"count\":3"));
    
    for agent in &agents {
        assert!(body.contains(&agent.agent_id));
        assert!(body.contains(&agent.hostname));
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;
    
    let response = get_request(&app, "/health").await;
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response_body(response).await;
    assert!(body.contains("ok"));
    assert!(body.contains("timestamp"));
}

#[tokio::test]
async fn test_invalid_endpoints() {
    let app = create_test_app().await;
    
    // Test non-existent endpoint
    let response = get_request(&app, "/non-existent").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    // Test invalid method on existing endpoint
    let response = Request::builder()
        .uri("/health")
        .method(Method::POST)
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(response).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_command_types() {
    let app = create_test_app().await;
    
    // Register an agent first
    let heartbeat = Heartbeat {
        agent_id: "test-agent".to_string(),
        timestamp: 1234567890,
        metrics: Metrics { cpu: 50.0, ram: 60.0, disk: 70.0 },
        hostname: "test-host".to_string(),
        uptime: 3600,
    };
    
    let response = post_request(&app, "/heartbeat", &heartbeat).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test different command types
    let commands = vec![
        Command::GetProcesses,
        Command::Exec { cmd: "ls -la".to_string() },
        Command::GetFile { path: "/tmp/test.txt".to_string() },
        Command::GetSystemInfo,
    ];
    
    for command in commands {
        let response = post_request(&app, "/agents/test-agent/command", &command).await;
        // Should fail because no WebSocket connection, but the endpoint should be valid
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_malformed_requests() {
    let app = create_test_app().await;
    
    // Test malformed JSON in heartbeat
    let response = Request::builder()
        .uri("/heartbeat")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(Body::from("{invalid json"))
        .unwrap();
    
    let response = app.oneshot(response).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    // Test malformed JSON in command
    let response = Request::builder()
        .uri("/agents/test-agent/command")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(Body::from("{invalid json"))
        .unwrap();
    
    let response = app.oneshot(response).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Helper functions

async fn create_test_app() -> axum::Router<AppState> {
    let app_state = AppState {
        agents: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        ws_manager: Arc::new(tokio::sync::Mutex::new(WebSocketManager::new())),
    };

    axum::Router::new()
        .route("/health", get(health_check))
        .route("/heartbeat", post(handle_heartbeat))
        .route("/agents", get(list_agents))
        .route("/agents/:id/command", post(send_command))
        .with_state(app_state)
}

async fn get_request(app: &axum::Router<AppState>, uri: &str) -> axum::response::Response {
    let request = Request::builder()
        .uri(uri)
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    app.clone().oneshot(request).await.unwrap()
}

async fn post_request<T: serde::Serialize>(
    app: &axum::Router<AppState>, 
    uri: &str, 
    body: &T
) -> axum::response::Response {
    let json_body = serde_json::to_string(body).unwrap();
    
    let request = Request::builder()
        .uri(uri)
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(Body::from(json_body))
        .unwrap();

    app.clone().oneshot(request).await.unwrap()
}

async fn response_body(response: axum::response::Response) -> String {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}
