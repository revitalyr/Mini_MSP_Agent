//! Simple modular server for Mini MSP Agent
//!
//! Optimized for fast compilation and modular structure

mod simple_handlers;
mod api;
mod websocket;
mod config;
mod broker;
mod ffi;
mod plugin_loader;
mod custom_plugin;

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
use tracing::{info, error, warn, Level};
use anyhow::Context;

use config::Config;
use broker::{BrokerClient, BrokerMessageHandler};
use custom_plugin::CustomPluginRegistry;
use plugin_loader::PluginLoader;
use mini_msp_shared::{AgentInfo, CommandResponse, Heartbeat};
use futures_util::StreamExt;

/// Complete application state with all integrated components
#[derive(Clone)]
pub struct AppState {
    /// Registered agents: agent_id -> AgentInfo
    pub agents: Arc<Mutex<HashMap<String, AgentInfo>>>,
    /// NATS broker client for agent communication
    pub broker_client: Option<Arc<BrokerClient>>,
    /// Custom plugin registry for extensible functionality
    pub plugin_registry: Arc<Mutex<CustomPluginRegistry>>,
    /// C++ forensic plugin loader (SystemPluginV3 + ForensicPlugin)
    pub forensic_plugin: Arc<Mutex<Option<PluginLoader>>>,
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

    // Initialize custom plugin registry
    let plugin_registry = Arc::new(Mutex::new(CustomPluginRegistry::new()));
    
    // Load custom plugins from directory
    {
        let mut registry = plugin_registry.lock().unwrap();
        match registry.load_from_directory("./plugins") {
            Ok(plugins) => {
                for plugin in &plugins {
                    info!("Auto-loaded custom plugin: {} v{}", plugin.name, plugin.version);
                }
            }
            Err(e) => {
                warn!("Failed to load custom plugins from directory: {}", e);
            }
        }
    }
    
    // Load C++ forensic plugin (SystemPluginV3 + ForensicPlugin)
    let forensic_plugin = Arc::new(Mutex::new(None));
    match PluginLoader::load() {
        Ok(loader) => {
            info!("Loaded forensic plugin: {} v{}", loader.name(), loader.version());
            *forensic_plugin.lock().unwrap() = Some(loader);
        }
        Err(e) => {
            warn!("Failed to load forensic plugin: {}", e);
            info!("Continuing without forensic plugin functionality");
        }
    }
    
    // Initialize NATS broker client if broker URL is configured
    let mut broker_client: Option<Arc<BrokerClient>> = None;
    if let Some(ref broker_url) = config.broker_url {
        if !broker_url.is_empty() {
            match BrokerClient::connect(broker_url).await {
                Ok(client) => {
                    info!("Connected to NATS broker at {}", broker_url);
                    broker_client = Some(Arc::new(client));
                }
                Err(e) => {
                    warn!("Failed to connect to NATS broker: {}", e);
                    info!("Continuing without broker functionality");
                }
            }
        }
    }
    
    // Create application state
    let app_state = Arc::new(AppState {
        agents: Arc::new(Mutex::new(HashMap::new())),
        broker_client: broker_client.clone(),
        plugin_registry,
        forensic_plugin,
    });

    // Start broker message processing if broker is connected
    if let Some(ref broker) = broker_client {
        // Spawn heartbeat processor
        let handler = BrokerMessageHandler::new(broker.clone());
        let app_state_clone = app_state.clone();
        let broker_clone = broker.clone();
        
        tokio::spawn(async move {
            info!("Starting NATS heartbeat processor...");
            
            // Subscribe to heartbeats
            let mut heartbeat_sub = match broker_clone.subscribe_heartbeats().await {
                Ok(sub) => {
                    info!("Subscribed to agent heartbeats");
                    sub
                }
                Err(e) => {
                    error!("Failed to subscribe to heartbeats: {}", e);
                    return;
                }
            };
            
            // Process incoming messages
            while let Some(msg) = heartbeat_sub.next().await {
                if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                    if let Ok(heartbeat) = serde_json::from_str::<Heartbeat>(payload) {
                        // Extract agent_id from heartbeat payload
                        let agent_id = heartbeat.agent_id.clone();
                        let now = chrono::Utc::now().timestamp() as u64;
                        
                        // Update agent info with last_seen
                        {
                            let mut agents = app_state_clone.agents.lock().unwrap();
                            let agent = agents.entry(agent_id.clone()).or_insert_with(|| AgentInfo {
                                id: agent_id.clone(),
                                hostname: heartbeat.hostname.clone(),
                                version: "1.0.0".to_string(),
                                platform: heartbeat.platform.clone(),
                                last_seen: now,
                            });
                            // Update last_seen for existing agents
                            agent.last_seen = now;
                            info!("Agent heartbeat: {} ({} - {}) at {}", agent_id, heartbeat.hostname, heartbeat.platform, now);
                        }
                        
                        // Handle heartbeat via broker handler
                        if let Err(e) = handler.handle_heartbeat(&agent_id, heartbeat).await {
                            warn!("Failed to handle heartbeat from {}: {}", agent_id, e);
                        }
                    }
                }
            }
        });
        
        // Spawn command response processor
        let handler = BrokerMessageHandler::new(broker.clone());
        let app_state_clone = app_state.clone();
        let broker_clone = broker.clone();
        
        tokio::spawn(async move {
            info!("Starting NATS response processor...");
            
            // Subscribe to all agent responses
            let mut response_sub = match broker_clone.subscribe_all_responses().await {
                Ok(sub) => {
                    info!("Subscribed to agent responses");
                    sub
                }
                Err(e) => {
                    error!("Failed to subscribe to responses: {}", e);
                    return;
                }
            };
            
            while let Some(msg) = response_sub.next().await {
                if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                    if let Ok(response) = serde_json::from_str::<CommandResponse>(payload) {
                        let subject = msg.subject.as_str();
                        let parts: Vec<&str> = subject.split('.').collect();
                        if parts.len() >= 2 {
                            let agent_id = parts[1];
                            let cmd_id = response.command_id.as_deref().unwrap_or("unknown");
                            let status = response.status.clone();
                            info!("Received response from {}: command_id={} status={}", 
                                  agent_id, cmd_id, status);
                            
                            // Update agent activity tracking before moving response
                            {
                                let mut agents = app_state_clone.agents.lock().unwrap();
                                if let Some(_agent) = agents.get_mut(agent_id) {
                                    info!("Agent {} executed command {} with status {}", 
                                          agent_id, cmd_id, status);
                                }
                            }
                            
                            // Process response through broker handler
                            if let Err(e) = handler.handle_response(agent_id, response).await {
                                warn!("Failed to handle response from {}: {}", agent_id, e);
                            }
                        }
                    }
                }
            }
        });
        
        // Spawn plugin event processor
        let handler = BrokerMessageHandler::new(broker.clone());
        let broker_clone = broker.clone();
        
        tokio::spawn(async move {
            info!("Starting NATS plugin event processor...");
            
            // Subscribe to plugin events
            let mut event_sub = match broker_clone.subscribe_plugin_events().await {
                Ok(sub) => {
                    info!("Subscribed to plugin events");
                    sub
                }
                Err(e) => {
                    error!("Failed to subscribe to plugin events: {}", e);
                    return;
                }
            };
            
            while let Some(msg) = event_sub.next().await {
                if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                        let subject = msg.subject.as_str();
                        let parts: Vec<&str> = subject.split('.').collect();
                        if parts.len() >= 3 {
                            let agent_id = parts[1];
                            let plugin_name = parts[2];
                            
                            if let Err(e) = handler.handle_plugin_event(agent_id, plugin_name, data).await {
                                warn!("Failed to handle plugin event from {}: {}", agent_id, e);
                            }
                        }
                    }
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
        .route("/agents/:id/command/simple", post(simple_handlers::send_command))
        .route("/heartbeat", post(simple_handlers::handle_heartbeat))
        .route("/system-info", get(api::system::get_system_info))
        .route("/forensic/metrics", get(api::system::get_forensic_metrics))
        .route("/forensic/data", get(api::system::get_forensic_data))
        
        // WebSocket
        .route("/ws", get(websocket::handle_websocket))
        
        // Plugin management
        .route("/plugins", get(api::plugins::list_plugins))
        .route("/plugins/load", post(api::plugins::load_plugin))
        .route("/plugins/:name/unload", post(api::plugins::unload_plugin))
        .route("/plugins/:name/metrics", get(api::plugins::get_plugin_metrics))
        .route("/plugins/:name/health", get(api::plugins::plugin_health))
        .route("/plugins/execute", post(api::plugins::execute_command))
        
        // Static files
        .nest_service("/static", ServeDir::new("server/static"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Start server with detailed logging
    info!("Attempting to bind server to {}", addr);
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            info!("Successfully bound to {}", addr);
            listener
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            return Err(e.into());
        }
    };
    
    info!("Starting axum server on {}", addr);
    info!("Available routes:");
    info!("  GET  /health - Health check");
    info!("  GET  /health/simple - Simple health check");
    info!("  POST /login - Authentication");
    info!("  GET  /agents - List agents");
    info!("  POST /agents/:id/command - Send command to agent");
    info!("  GET  /ws - WebSocket endpoint");
    info!("  GET  /plugins - List loaded plugins");
    info!("  POST /plugins/load - Load a plugin");
    info!("  POST /plugins/:name/execute - Execute plugin command");
    info!("  GET  /plugins/:name/metrics - Get plugin metrics");
    info!("  GET  /static/* - Static files");
    
    match axum::serve(listener, app).await {
        Ok(_) => info!("Server shutdown completed"),
        Err(e) => error!("Server error: {}", e),
    }

    Ok(())
}
