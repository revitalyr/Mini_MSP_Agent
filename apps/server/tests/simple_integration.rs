use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
    response::Json,
    routing::{get, post},
    Router,
};
use tower::ServiceExt;
use mini_msp_shared::{Heartbeat, Metrics, Command};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

/// Simple integration tests for the Mini MSP Server

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;
    
    let request = Request::builder()
        .uri("/health")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    
    assert!(body_str.contains("ok"));
}

#[tokio::test]
async fn test_agents_list_empty() {
    let app = create_test_app().await;
    
    let request = Request::builder()
        .uri("/agents")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    
    assert!(body_str.contains("\"count\":0"));
}

#[tokio::test]
async fn test_heartbeat_endpoint() {
    let app = create_test_app().await;
    
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

    let request = Request::builder()
        .uri("/heartbeat")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&heartbeat).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_command_to_nonexistent_agent() {
    let app = create_test_app().await;
    
    let command = Command::GetSystemInfo;
    let request = Request::builder()
        .uri("/agents/non-existent-agent/command")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&command).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test app setup
async fn create_test_app() -> Router {
    let agents: Arc<Mutex<HashMap<String, AgentInfo>>> = Arc::new(Mutex::new(HashMap::new()));
    
    Router::new()
        .route("/health", get(health_check_handler))
        .route("/heartbeat", post(heartbeat_handler))
        .route("/agents", get(list_agents_handler))
        .route("/agents/:id/command", post(send_command_handler))
        .with_state(agents)
}

// Mock handlers for testing
async fn health_check_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

async fn heartbeat_handler(
    axum::extract::State(agents): axum::extract::State<Arc<Mutex<HashMap<String, AgentInfo>>>>,
    axum::extract::Json(heartbeat): axum::extract::Json<Heartbeat>,
) -> Json<serde_json::Value> {
    let agent_id = heartbeat.agent_id.clone();
    let mut agents_map = agents.lock().await;
    agents_map.insert(agent_id.clone(), AgentInfo {
        id: agent_id.clone(),
        last_heartbeat: Instant::now(),
        hostname: heartbeat.hostname,
        uptime: heartbeat.uptime,
    });
    
    Json(json!({
        "status": "received",
        "agent_id": agent_id
    }))
}

async fn list_agents_handler(
    axum::extract::State(agents): axum::extract::State<Arc<Mutex<HashMap<String, AgentInfo>>>>,
) -> Json<serde_json::Value> {
    let agents_map = agents.lock().await;
    let agent_list: Vec<_> = agents_map
        .values()
        .map(|agent| {
            json!({
                "id": agent.id,
                "hostname": agent.hostname,
                "uptime": agent.uptime,
                "last_heartbeat": agent.last_heartbeat.duration_since(Instant::now()).as_secs()
            })
        })
        .collect();

    Json(json!({
        "agents": agent_list,
        "count": agent_list.len()
    }))
}

async fn send_command_handler(
    axum::extract::State(agents): axum::extract::State<Arc<Mutex<HashMap<String, AgentInfo>>>>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
    axum::extract::Json(command): axum::extract::Json<Command>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agents_map = agents.lock().await;
    
    if agents_map.contains_key(&agent_id) {
        Ok(Json(json!({
            "status": "sent",
            "agent_id": agent_id,
            "command": command
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Clone)]
struct AgentInfo {
    id: String,
    last_heartbeat: Instant,
    hostname: String,
    uptime: u64,
}
