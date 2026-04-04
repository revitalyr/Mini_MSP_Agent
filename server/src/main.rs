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
    extract::State,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Instant, Duration},
};
use tracing::info;
use tracing_subscriber::{self, EnvFilter, prelude::*};
use tokio::time::interval;
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::ServeDir};
use clap::{Arg, Command as ClapCommand};

mod routes;
mod websocket;
mod browse;
mod config;

use routes::{handle_heartbeat, handle_websocket, send_command as send_command_handler, get_directory_info};
use websocket::WebSocketManager;
use config::Config;

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
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("configs/server.toml"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the server port")
                .default_value("8081"),
        )
        .get_matches();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap_or(&"configs/server.toml".to_string());
    let config = Config::load(config_path)?;

    // Initialize logging с уровнем из конфига и записью в файл
    let log_level = config.log_level.as_str(); // "debug" | "info" | "warn" | "error"
    
    // Лог в файл с rotation по дням в настраиваемую директорию
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()           // RUST_LOG имеет приоритет
                .unwrap_or_else(|_| EnvFilter::new(log_level))
        )
        .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking))  // файл
        .with(tracing_subscriber::fmt::layer().json())                            // stdout
        .init();

    let port: u16 = matches.get_one::<String>("port").unwrap().parse()
        .unwrap_or(config.port);
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
        .route("/login", post(routes::login))
        .route("/refresh", post(routes::refresh))
        .route("/heartbeat", post(handle_heartbeat))
        .route("/ws", get(handle_websocket))
        .route("/agents", get(list_agents))
        .route("/agents/:id/command", post(send_command_handler))
        .route("/agents/:id/data/directory_info", get(routes::get_directory_info_data))
        .route("/agents/:id/data/event_data", get(routes::get_event_data_endpoint))
        .route("/agents/:id/data/watchers_data", get(routes::get_watchers_data_endpoint))
        .route("/agents/:id/data/file_reader_data", get(routes::get_file_reader_data_endpoint))
        .route("/agents/:id/data/plugin_registry", get(routes::get_plugin_registry_data))
        .route("/agents/:id/data/sensors", get(routes::get_sensor_data_endpoint))
        .route("/agents/ws", get(list_ws_agents))
        .route("/directory/:path", get(get_directory_info))
        .route("/api/browse/directory", post(browse::browse_directory))
        .route("/api/browse/file",      post(browse::browse_file))
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(|| async { axum::response::Redirect::permanent("/static/plugin_control.html") }))
        .with_state(app_state.clone())
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap())
                .allow_origin("http://127.0.0.1:8080".parse::<axum::http::HeaderValue>().unwrap())
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                ])
        )
        .layer(TraceLayer::new_for_http());

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


async fn cleanup_inactive_agents(state: &AppState) {
    let timeout = Duration::from_secs(120);

    let mut agents = state.agents.lock().await;
    let to_remove: Vec<_> = agents.iter()
        .filter(|(_, agent)| Instant::now().duration_since(agent.last_heartbeat) > timeout)
        .map(|(id, _)| id.clone())
        .collect();
    for id in &to_remove {
        info!("Removing inactive agent: {}", id);
        agents.remove(id);
    }
    drop(agents);

    let mut ws_manager = state.ws_manager.lock().await;
    for id in &to_remove {
        ws_manager.remove_agent(id).await;  // <-- вернули
    }
    ws_manager.cleanup_inactive(timeout);
}

async fn list_ws_agents(State(state): State<AppState>) -> impl IntoResponse {
    let ws_manager = state.ws_manager.lock().await;
    let agents = ws_manager.get_connected_agents(); // <-- вот он
    Json(json!({ "agents": agents, "count": agents.len() }))
}
