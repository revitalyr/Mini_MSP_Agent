use anyhow::Result;
use axum::{
    extract::{ws::WebSocket, Path, State, WebSocketUpgrade},
    http::{StatusCode, HeaderValue},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use clap::{Arg, Command as ClapCommand};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::Command;
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};
use tokio::time::{interval, Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info};

mod routes;
mod websocket;

use routes::{handle_heartbeat, handle_websocket};
use websocket::WebSocketManager;

#[derive(Clone)]
struct AppState {
    agents: Arc<tokio::sync::Mutex<HashMap<String, AgentInfo>>>,
    ws_manager: Arc<tokio::sync::Mutex<WebSocketManager>>,
}

#[derive(Debug, Clone)]
struct AgentInfo {
    id: String,
    last_heartbeat: Instant,
    hostname: String,
    uptime: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = ClapCommand::new("mini-msp-server")
        .version("0.1.0")
        .about("Backend server for Mini MSP Agent")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the server port")
                .default_value("8080"),
        )
        .get_matches();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .json()
        .init();

    let port: u16 = matches.get_one::<String>("port").unwrap().parse()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("Starting Mini MSP Server on {}", addr);

    // Initialize application state
    let app_state = AppState {
        agents: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        ws_manager: Arc::new(tokio::sync::Mutex::new(WebSocketManager::new())),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/heartbeat", post(handle_heartbeat))
        .route("/ws", get(handle_websocket))
        .route("/agents", get(list_agents))
        .route("/agents/:id/command", post(send_command))
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:3000".parse::<axum::http::HeaderValue>().unwrap())
                .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap())
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                ]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state.clone());

    // Spawn cleanup task for inactive agents
    let cleanup_state = app_state.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_inactive_agents(&cleanup_state).await;
        }
    });

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

async fn list_agents(State(state): State<AppState>) -> impl IntoResponse {
    let agents = state.agents.lock().await;
    let agent_list: Vec<_> = agents
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

async fn send_command(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    debug!("Sending command to agent {}: {:?}", agent_id, command);

    let mut ws_manager = state.ws_manager.lock().await;
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => (StatusCode::OK, Json(json!({
            "status": "sent",
            "agent_id": agent_id,
            "command": command
        }))),
        Err(e) => {
            error!("Failed to send command to agent {}: {}", agent_id, e);
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("Agent not connected: {}", e)
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, Method},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let app = create_app().await;
        
        let request = Request::builder()
            .uri("/health")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        
        assert!(body_str.contains("ok"));
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let app = create_app().await;
        
        let request = Request::builder()
            .uri("/agents")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        
        assert!(body_str.contains("\"count\":0"));
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let app = create_app().await;
        
        let heartbeat = mini_msp_shared::Heartbeat {
            agent_id: "test-agent".to_string(),
            timestamp: 1234567890,
            metrics: mini_msp_shared::Metrics {
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

        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_send_command_agent_not_found() {
        let app = create_app().await;
        
        let command = mini_msp_shared::Command::GetSystemInfo;
        
        let request = Request::builder()
            .uri("/agents/non-existent-agent/command")
            .method(Method::POST)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&command).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    async fn create_app() -> axum::Router<AppState> {
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
}

async fn cleanup_inactive_agents(state: &AppState) {
    let mut agents = state.agents.lock().await;
    let mut ws_manager = state.ws_manager.lock().await;
    
    let now = Instant::now();
    let timeout = Duration::from_secs(120); // 2 minutes timeout
    
    let mut to_remove = Vec::new();
    
    for (id, agent) in agents.iter() {
        if now.duration_since(agent.last_heartbeat) > timeout {
            to_remove.push(id.to_string());
        }
    }
    
    for id in to_remove {
        info!("Removing inactive agent: {}", id);
        agents.remove(&id);
        ws_manager.remove_agent(&id).await;
    }
}
