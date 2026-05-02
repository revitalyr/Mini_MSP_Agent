use anyhow::{Context, Result};
use async_nats;
use clap::{Arg, Command};
use mini_msp_shared::{AgentInfo, EventMessage};
use gethostname;
use serde_json;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod orchestrator;
mod broker;
mod config;

use orchestrator::Orchestrator;
use broker::BrokerClient;
use config::ConfigManager;

// Build information constants
pub const BUILD_INFO: &str = "unknown build";

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("mini-msp-agent")
        .version("1.0.0")
        .about("Mini MSP Agent - Modular System Monitoring Agent")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config.toml"),
        )
        .arg(
            Arg::new("broker-url")
                .short('b')
                .long("broker-url")
                .value_name("URL")
                .help("NATS broker URL")
                .default_value("nats://localhost:4222"),
        )
        .arg(
            Arg::new("agent-id")
                .short('i')
                .long("agent-id")
                .value_name("ID")
                .help("Agent identifier")
                .env("MSP_AGENT_ID"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info")
                .env("MSP_LOG_LEVEL"),
        )
        .arg(
            Arg::new("log-file")
                .long("log-file")
                .value_name("FILE")
                .help("Log file path"),
        )
        .arg(
            Arg::new("daemon")
                .short('d')
                .long("daemon")
                .help("Run as daemon")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging
    initialize_logging(&matches)?;

    // Print startup banner
    print_startup_banner();

    // Load configuration
    let config = load_configuration(&matches).await?;

    // Connect to broker
    let broker_client = connect_to_broker(&matches, &config).await?;

    // Initialize orchestrator
    let (mut orchestrator, mut event_receiver) = Orchestrator::new(
        config.clone(),
        Arc::new(broker_client),
    );

    // Initialize orchestrator and load plugins
    orchestrator.initialize().await?;

    // Start event processing
    let event_task = tokio::spawn(process_events(
        orchestrator.clone(),
        event_receiver
    ));

    // Start command processing
    let command_task = tokio::spawn(process_commands(
        orchestrator.clone(),
        broker_client.clone()
    ));

    // Start heartbeat task
    let heartbeat_task = tokio::spawn(send_heartbeats(
        orchestrator.get_agent_info().id.clone(),
        broker_client.clone()
    ));

    info!("Mini MSP Agent started successfully");
    info!("Agent ID: {}", orchestrator.get_agent_info().id);
    info!("Broker URL: {}", config.broker.url);
    info!("Loaded plugins: {}", orchestrator.list_plugins().await.len());

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
    }

    // Graceful shutdown
    info!("Shutting down gracefully...");
    
    // Cancel tasks
    event_task.abort();
    command_task.abort();
    heartbeat_task.abort();

    // Unregister from broker
    if let Err(e) = broker_client.publish_agent_unregistration(&orchestrator.get_agent_info().id).await {
        error!("Failed to unregister agent: {}", e);
    }

    info!("Mini MSP Agent stopped");
    Ok(())
}

fn initialize_logging(matches: &clap::ArgMatches) -> Result<()> {
    let log_level = matches.get_one::<String>("log-level").unwrap();
    let log_file = matches.get_one::<String>("log-file");

    let mut builder = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(log_level)));

    if let Some(file) = log_file {
        // File logging
        let file_appender = tracing_appender::rolling::daily(file, "agent.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        builder = builder.with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking));
    }

    builder = builder.with(tracing_subscriber::fmt::layer().json());

    builder.init();
    Ok(())
}

fn print_startup_banner() {
    println!(
        r#"
{} Mini MSP Agent v{} {}
"#,
        "=".repeat(60),
        env!("CARGO_PKG_VERSION"),
        "=".repeat(60),
    );
}

async fn load_configuration(matches: &clap::ArgMatches) -> Result<mini_msp_shared::AgentConfig> {
    let config_path = matches.get_one::<String>("config").unwrap();
    
    // Try to load from file first
    let mut config_manager = if std::path::Path::new(config_path).exists() {
        ConfigManager::load_from_file(config_path)?
    } else {
        warn!("Configuration file not found: {}, using defaults", config_path);
        ConfigManager::load_with_defaults()?
    };

    // Override with command line arguments
    if let Some(agent_id) = matches.get_one::<String>("agent-id") {
        config_manager.get_config_mut().agent.id = agent_id;
    }

    if let Some(broker_url) = matches.get_one::<String>("broker-url") {
        config_manager.get_config_mut().broker.url = broker_url;
    }

    if let Some(log_level) = matches.get_one::<String>("log-level") {
        config_manager.get_config_mut().logging.level = log_level;
    }

    if let Some(log_file) = matches.get_one::<String>("log-file") {
        config_manager.get_config_mut().logging.file = Some(log_file);
    }

    Ok(config_manager.get_config().clone())
}

async fn connect_to_broker(matches: &clap::ArgMatches, config: &mini_msp_shared::AgentConfig) -> Result<BrokerClient> {
    let broker_url = matches.get_one::<String>("broker-url")
        .unwrap_or(&config.broker.url);

    let agent_id = &config.agent.id;

    let broker_client = BrokerClient::connect(broker_url, agent_id.clone()).await?;

    // Register with broker
    broker_client.publish_agent_registration(&mini_msp_shared::AgentInfo {
        id: agent_id.clone(),
        hostname: config.agent.hostname.clone().unwrap_or_else(|| {
            gethostname::gethostname().to_string_lossy().to_string()
        }),
        version: config.agent.version.clone(),
        platform: config.agent.platform.clone(),
        architecture: std::env::consts::ARCH.to_string(),
        start_time: chrono::Utc::now(),
    }).await?;

    info!("Connected to broker and registered as: {}", agent_id);
    
    Ok(broker_client)
}

async fn process_events(
    orchestrator: Orchestrator,
    mut event_receiver: tokio::sync::mpsc::UnboundedReceiver<mini_msp_shared::EventMessage>,
) {
    info!("Starting event processing task");

    while let Some(event) = event_receiver.recv().await {
        info!("Received event: {} from {}", event.event_type, event.source);
        
        // Process events based on type
        match event.event_type {
            mini_msp_shared::EventType::PluginLoaded => {
                info!("Plugin loaded: {}", event.source);
            }
            mini_msp_shared::EventType::PluginUnloaded => {
                info!("Plugin unloaded: {}", event.source);
            }
            mini_msp_shared::EventType::PluginError => {
                error!("Plugin error in {}: {:?}", event.source, event.data);
            }
            mini_msp_shared::EventType::CommandExecuted => {
                info!("Command executed by {}: {:?}", event.source, event.data);
            }
            mini_msp_shared::EventType::SystemAlert => {
                warn!("System alert from {}: {:?}", event.source, event.data);
            }
            mini_msp_shared::EventType::NetworkEvent => {
                info!("Network event from {}: {:?}", event.source, event.data);
            }
            mini_msp_shared::EventType::FileSystemEvent => {
                info!("File system event from {}: {:?}", event.source, event.data);
            }
        }
    }

    info!("Event processing task ended");
}

async fn process_commands(
    orchestrator: Orchestrator,
    broker_client: BrokerClient,
) {
    info!("Starting command processing task");

    // Subscribe to commands
    let mut command_subscriber = match broker_client.subscribe_to_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to commands: {}", e);
            return;
        }
    };

    // Subscribe to metrics requests
    let mut metrics_subscriber = match broker_client.subscribe_to_metrics_requests().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to metrics requests: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            Some(message) = command_subscriber.next() => {
                if let Err(e) = handle_command_message(&orchestrator, &broker_client, message).await {
                    error!("Error handling command: {}", e);
                }
            }
            Some(message) = metrics_subscriber.next() => {
                if let Err(e) = handle_metrics_request(&orchestrator, &broker_client, message).await {
                    error!("Error handling metrics request: {}", e);
                }
            }
        }
    }
}

async fn handle_command_message(
    orchestrator: &Orchestrator,
    broker_client: &BrokerClient,
    message: async_nats::Message,
) -> Result<()> {
    let request: serde_json::Value = serde_json::from_slice(&message.payload)?;
    
    let command = request.get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("unknown");
    
    let request_id = request.get("request_id")
        .and_then(|id| serde_json::from_value(id.clone()))
        .unwrap_or_else(|_| uuid::Uuid::new_v4());
    
    let parameters = request.get("parameters")
        .and_then(|p| serde_json::from_value(p.clone()))
        .unwrap_or_else(|_| serde_json::Map::default());
    
    info!("Executing command: {} with parameters: {:?}", command, parameters);
    
    match orchestrator.execute_command(command, parameters).await {
        Ok(response) => {
            broker_client.publish_command_response(
                &orchestrator.get_agent_info().id,
                request_id,
                response.data,
            ).await?;
        }
        Err(e) => {
            let error_response = serde_json::json!({
                "error": e.to_string(),
                "success": false
            });
            
            broker_client.publish_command_response(
                &orchestrator.get_agent_info().id,
                request_id,
                error_response,
            ).await?;
        }
    }
    
    Ok(())
}

async fn handle_metrics_request(
    orchestrator: &Orchestrator,
    broker_client: &BrokerClient,
    message: async_nats::Message,
) -> Result<()> {
    let request: serde_json::Value = serde_json::from_slice(&message.payload)?;
    
    let request_id = request.get("request_id")
        .and_then(|id| serde_json::from_value(id.clone()))
        .unwrap_or_else(|_| uuid::Uuid::new_v4());
    
    match orchestrator.collect_metrics().await {
        Ok(metrics) => {
            let response = serde_json::json!({
                "request_id": request_id,
                "metrics": metrics,
                "success": true
            });
            
            let subject = format!("metrics_response.{}", orchestrator.get_agent_info().id);
            let payload = serde_json::to_vec(&response)?;
            
            broker_client.client.publish(subject, payload.into()).await?;
        }
        Err(e) => {
            let error_response = serde_json::json!({
                "request_id": request_id,
                "error": e.to_string(),
                "success": false
            });
            
            let subject = format!("metrics_response.{}", orchestrator.get_agent_info().id);
            let payload = serde_json::to_vec(&error_response)?;
            
            broker_client.client.publish(subject, payload.into()).await?;
        }
    }
    
    Ok(())
}

async fn send_heartbeats(
    agent_id: String,
    broker_client: BrokerClient,
) {
    info!("Starting heartbeat task");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        interval.tick().await;
        
        if let Err(e) = broker_client.publish_heartbeat().await {
            error!("Failed to publish heartbeat: {}", e);
        }
    }
}
