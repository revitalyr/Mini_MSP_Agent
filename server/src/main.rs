//! Simple modular server for Mini MSP Agent
//!
//! Optimized for fast compilation and modular structure

mod config;
mod broker;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tower_http::{
    cors::{CorsLayer, Any},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, Level};

use crate::handlers::{AppState, health_check, list_agents, handle_heartbeat, handle_plugin_event};
use config::Config;
use crate::broker::BrokerClient;

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

    // Initialize broker client if URL is provided
    let mut broker_client: Option<Arc<BrokerClient>> = None;
    if let Some(url) = &config.broker_url {
        let mut attempts = 0;
        let max_attempts = 5;
        while attempts < max_attempts {
            match BrokerClient::connect(url).await {
                Ok(client) => {
                    broker_client = Some(Arc::new(client));
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    tracing::error!("Failed to connect to NATS (attempt {}/{}): {}", attempts, max_attempts, e);
                    if attempts < max_attempts {
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }

    // Initialize application state
    let app_state = Arc::new(AppState {
        agents: Mutex::new(HashMap::new()),
        broker_client,
    });

    // Build router with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/agents", get(list_agents))
        .route("/heartbeat", post(handle_heartbeat))
        .route("/events", post(handle_plugin_event))
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
