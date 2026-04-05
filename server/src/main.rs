//! Simple modular server for Mini MSP Agent
//!
//! Optimized for fast compilation and modular structure

mod simple_handlers;
mod api;
mod websocket;
mod config;
mod broker;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::{
    cors::{CorsLayer, Any},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, Level};

use config::Config;
use broker::BrokerClient;

// Unified AppState for all modules
#[derive(Clone)]
pub struct AppState {
    pub agents: Arc<Mutex<HashMap<String, String>>>,
    pub broker_client: Option<Arc<BrokerClient>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matches = clap::Command::new("mini-msp-server")
        .version("0.1.0")
        .about("Backend server for Mini MSP Agent")
        .arg(
            clap::Arg::new("config")
                .short('c')
                .long("config")
                .help("Configuration file path")
                .default_value("configs/server.toml"),
        )
        .arg(
            clap::Arg::new("port")
                .short('p')
                .long("port")
                .help("Sets the server port")
                .default_value("8081"),
        )
        .get_matches();

    // Load configuration
    let config_path = matches
        .get_one::<String>("config")
        .map(|s| s.as_str())
        .unwrap_or("configs/server.toml");
    let config = Config::load(config_path)?;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let port: u16 = matches.get_one::<String>("port").unwrap().parse()
        .unwrap_or(config.port);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    info!("Starting Mini MSP Server on {}", addr);

    // Initialize application state
    let broker_client = None; // Will be initialized later if needed
    let app_state = Arc::new(AppState {
        agents: Arc::new(Mutex::new(HashMap::new())),
        broker_client,
    });

    // Initialize broker message handler if broker is available
    if let Some(ref broker) = app_state.broker_client {
        let handler = broker::BrokerMessageHandler::new(broker.clone());
        info!("Broker message handler initialized");
        
        // Start background task to handle broker messages
        let handler_arc = Arc::new(handler);
        let app_state_clone = app_state.clone();
        tokio::spawn(async move {
            // Example: handle heartbeats in background
            // This would normally subscribe to broker topics
            info!("Broker message handler task started");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                
                // Log current agent count
                let agents = app_state_clone.agents.lock().unwrap();
                info!("Current registered agents: {}", agents.len());
                
                // Use handler to process any pending messages (placeholder)
                // In real implementation, this would subscribe to NATS topics
                // and call handler.handle_heartbeat(), handler.handle_response(), etc.
                // For now, just log that handler is available
                debug!("Handler available for {} agents", agents.len());
            }
        });
    }

    // Build router with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        // Health checks
        .route("/health", get(api::health_check))
        .route("/health/simple", get(simple_handlers::health_check))
        
        // Authentication
        .route("/login", post(api::auth::login))
        .route("/refresh", post(api::auth::refresh_token))
        
        // Agent management
        .route("/agents", get(api::agents::list_agents))
        .route("/agents/simple", get(simple_handlers::list_agents))
        .route("/heartbeat", post(simple_handlers::handle_heartbeat))
        
        // WebSocket
        .route("/ws", get(websocket::handle_websocket))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Start server
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
