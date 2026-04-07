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
use tracing::{info, debug, Level};
use anyhow::Context;

use config::Config;
use broker::BrokerClient;
use mini_msp_shared::{Heartbeat, Metrics, CommandResponse};

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
                .default_value("8080"),
        )
        .get_matches();

    // Load configuration
    let config_path = matches
        .get_one::<String>("config")
        .map(|s| s.as_str())
        .unwrap_or("configs/server.toml");
    let config = Config::load(config_path)?;

    // Initialize logging
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&format!("{}/server.log", config.log_dir))
        .with_context(|| format!("Failed to create log file: {}/server.log", config.log_dir))?;

    // Parse log level from config
    let log_level = match config.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO, // Default to INFO if invalid
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false) // Disable ANSI colors for file output
        .init();

    let port: u16 = matches.get_one::<String>("port")
        .map(|p| p.parse().unwrap_or(config.port))
        .unwrap_or(config.port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port)); // Listen on all interfaces

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
        let _handler_task = tokio::spawn(async move {
            // Example: handle heartbeats in background
            // This would normally subscribe to broker topics
            info!("Broker message handler task started");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                
                // Log current agent count
                let agent_count = {
                    let agents = app_state_clone.agents.lock().unwrap();
                    agents.len()
                };
                
                info!("Current registered agents: {}", agent_count);
                
                // Use handler to process any pending messages (placeholder)
                // In real implementation, this would subscribe to NATS topics
                // and call handler.handle_heartbeat(), handler.handle_response(), etc.
                // For now, just log that handler is available
                debug!("Handler available for {} agents", agent_count);
                
                // Test handler methods with dummy data
                if agent_count > 0 {
                    let dummy_heartbeat = Heartbeat {
                        agent_id: "test".to_string(),
                        timestamp: chrono::Utc::now().timestamp(),
                        metrics: Metrics {
                            cpu: 50.0,
                            ram: 60.0,
                            disk: 70.0,
                        },
                        hostname: "test".to_string(),
                        uptime: 3600,
                    };
                    
                    // Use the handler methods
                    if let Err(e) = handler_arc.handle_heartbeat("test", dummy_heartbeat).await {
                        debug!("Heartbeat handling test failed: {}", e);
                    }
                    
                    // Test command response handling
                    let dummy_response = CommandResponse {
                        command_id: Some("test_cmd".to_string()),
                        r#type: "test".to_string(),
                        status: "success".to_string(),
                        data: serde_json::json!({"output": "Test output"}),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    
                    if let Err(e) = handler_arc.handle_response("test", dummy_response).await {
                        debug!("Response handling test failed: {}", e);
                    }
                    
                    // Test plugin event handling
                    let dummy_plugin_data = serde_json::json!({
                        "event": "test_event",
                        "data": "test_data"
                    });
                    
                    if let Err(e) = handler_arc.handle_plugin_event("test", "test_plugin", dummy_plugin_data).await {
                        debug!("Plugin event handling test failed: {}", e);
                    }
                    
                    // Use broker getter method
                    let _broker_client = handler_arc.broker();
                    debug!("Broker client available");
                }
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
        .route("/agents/:id/command", post(api::agents::send_command))
        .route("/heartbeat", post(simple_handlers::handle_heartbeat))
        .route("/system-info", get(api::system::get_system_info))
        
        // WebSocket
        .route("/ws", get(websocket::handle_websocket))
        
        // Static files
        .nest_service("/static", ServeDir::new("server/static"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Start server
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
