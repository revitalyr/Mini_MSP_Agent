use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod websocket;
mod nats_client;
mod api;

use websocket::handle_websocket;
use api::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Connect to NATS
    let nats_client = nats_client::NatsClient::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");

    // Shared state
    let app_state = AppState {
        nats: nats_client,
    };

    // CORS settings
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Routes
    let app = Router::new()
        .route("/ws", get(handle_websocket))
        .route("/api/agents", get(api::list_agents))
        .route("/api/agents/:agent_id/metrics", get(api::get_metrics))
        .route("/api/agents/:agent_id/plugins", get(api::list_plugins))
        .route("/api/agents/:agent_id/command", post(api::handle_command))
        .route("/api/agents/:agent_id/files", get(api::list_files))
        .route("/api/agents/:agent_id/files", post(api::upload_file))
        .fallback_service(ServeDir::new("../web-interface/dist"))
        .with_state(app_state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Web interface listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
