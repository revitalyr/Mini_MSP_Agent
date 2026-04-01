//! # Mini MSP Server
//! 
//! A high-performance HTTP and WebSocket server for managing Mini MSP agents.
//! This server provides REST API endpoints and real-time WebSocket communication
//! for fleet management and monitoring.
//! 
//! ## Features
//! 
//! - **REST API**: HTTP endpoints for agent management and commands
//! - **WebSocket Support**: Real-time bidirectional communication with agents
//! - **Agent Registry**: In-memory storage of connected agents
//! - **Command Dispatch**: Send commands to specific agents
//! - **Health Monitoring**: Built-in health checks and metrics
//! - **CORS Support**: Cross-origin resource sharing for web interfaces
//! 
//! ## Architecture
//! 
//! The server consists of several key components:
//! 
//! - **Routes Module**: HTTP endpoint handlers and routing
//! - **WebSocket Module**: WebSocket connection management
//! - **AppState**: Shared state for agent registry
//! - **AgentInfo**: Agent metadata and connection status
//! 
//! ## API Endpoints
//! 
//! ### HTTP Endpoints
//! 
//! - `GET /health` - Health check endpoint
//! - `GET /agents` - List all registered agents
//! - `GET /agents/{id}` - Get specific agent information
//! - `POST /agents/{id}/command` - Send command to agent
//! - `GET /ws` - WebSocket upgrade endpoint
//! 
//! ### WebSocket Events
//! 
//! - **Heartbeat**: Agent status updates
//! - **Command**: Command execution requests
//! - **Response**: Command execution results
//! - **Register**: New agent registration
//! - **Unregister**: Agent disconnection
//! 
//! ## Usage
//! 
//! ```bash
//! mini-msp-server --port 8080
//! ```
//! 
//! ## Configuration
//! 
//! The server accepts command-line arguments:
//! - `--port`: HTTP server port (default: 8080)
//! - `--host`: Bind address (default: 0.0.0.0)

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, IntoResponse},
    routing::{get, post},
    Router,
};
use clap::{Arg, Command as ClapCommand};
use mini_msp_shared::Command;
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};
use tokio::time::{interval, Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::ServeDir};
use tracing::{debug, error, info};

mod routes;
mod websocket;

use routes::{handle_heartbeat, handle_websocket};
use websocket::WebSocketManager;

/// Shared application state for the server
/// 
/// This structure contains all the shared data that needs to be accessed
/// across different HTTP handlers and WebSocket connections.
/// 
/// # Fields
/// 
/// * `agents` - Thread-safe hashmap of registered agents indexed by ID
/// * `ws_manager` - Thread-safe WebSocket connection manager
#[derive(Clone)]
pub struct AppState {
    agents: Arc<tokio::sync::Mutex<HashMap<String, AgentInfo>>>,
    ws_manager: Arc<tokio::sync::Mutex<WebSocketManager>>,
}

/// Information about a connected agent
/// 
/// Contains metadata about each agent that connects to the server,
/// including connection status and system information.
/// 
/// # Fields
/// 
/// * `id` - Unique identifier for the agent
/// * `last_heartbeat` - Timestamp of the last heartbeat received
/// * `hostname` - System hostname reported by the agent
/// * `uptime` - System uptime reported by the agent
#[derive(Debug, Clone)]
pub struct AgentInfo {
    id: String,
    last_heartbeat: Instant,
    hostname: String,
    uptime: u64,
}

/// Main entry point for the Mini MSP Server
/// 
/// This function initializes and starts the server with the following steps:
/// 1. Parse command line arguments (port, host)
/// 2. Initialize shared application state
/// 3. Create WebSocket manager
/// 4. Setup HTTP routes and middleware
/// 5. Start periodic cleanup tasks
/// 6. Bind to specified address and start serving
/// 
/// # Arguments
/// 
/// * `port` - HTTP server port (default: 8080)
/// * `host` - Bind address (default: 0.0.0.0)
/// 
/// # Returns
/// 
/// Returns `Ok(())` on successful execution, `Err(e)` on any failure
/// 
/// # Example
/// 
/// ```bash
/// mini-msp-server --port 8080 --host 127.0.0.1
/// ```
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
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(|| async { axum::response::Redirect::permanent("/static/index.html") }))
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

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

pub async fn list_agents(State(state): State<AppState>) -> impl IntoResponse {
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

pub async fn send_command(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    error!("=== COMMAND RECEIVED === agent: {}, command: {:?}", agent_id, command);
    info!("Sending command to agent {}: {:?}", agent_id, command);

    let mut ws_manager = state.ws_manager.lock().await;
    println!("HTTP: About to send via WebSocket manager");
    match ws_manager.send_to_agent(&agent_id, &command).await {
        Ok(_) => {
            println!("HTTP: Command sent successfully");
            (StatusCode::OK, Json(json!({
                "status": "sent",
                "agent_id": agent_id,
                "command": command
            })))
        },
        Err(e) => {
            println!("HTTP: Failed to send command: {}", e);
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
