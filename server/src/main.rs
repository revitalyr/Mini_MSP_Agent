use anyhow::Result;
use axum::{
    extract::{ws::WebSocket, Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use clap::{Arg, Command};
use futures_util::{SinkExt, StreamExt};
use mini_msp_shared::{Command, Heartbeat};
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::time::{interval, Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info, warn};

mod routes;
mod websocket;

use routes::{handle_heartbeat, handle_websocket};
use websocket::WebSocketManager;

#[derive(Clone)]
struct AppState {
    agents: Arc<Mutex<HashMap<String, AgentInfo>>>,
    ws_manager: Arc<Mutex<WebSocketManager>>,
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
    let matches = Command::new("mini-msp-server")
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
        agents: Arc::new(Mutex::new(HashMap::new())),
        ws_manager: Arc::new(Mutex::new(WebSocketManager::new())),
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
                .allow_origin("*".parse::<http::HeaderValue>().unwrap())
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers("*".parse::<http::HeaderValue>().unwrap()),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

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
    let agents = state.agents.lock().unwrap();
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

    let mut ws_manager = state.ws_manager.lock().unwrap();
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

async fn cleanup_inactive_agents(state: &AppState) {
    let mut agents = state.agents.lock().unwrap();
    let mut ws_manager = state.ws_manager.lock().unwrap();
    
    let now = Instant::now();
    let timeout = Duration::from_secs(120); // 2 minutes timeout
    
    let mut to_remove = Vec::new();
    
    for (id, agent) in agents.iter() {
        if now.duration_since(agent.last_heartbeat) > timeout {
            to_remove.push(id.clone());
        }
    }
    
    for id in to_remove {
        info!("Removing inactive agent: {}", id);
        agents.remove(&id);
        ws_manager.remove_agent(&id).await;
    }
}
