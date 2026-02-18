use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use goldfish::MemoryCortex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod models;
mod state;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,goldfish=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Goldfish Cortex...");

    // Initialize Cortex
    let cortex = MemoryCortex::new("./goldfish_data").await.expect("Failed to initialize Cortex");
    let state = Arc::new(AppState {
        cortex: Arc::new(cortex),
    });

    // Build Router
    let app = Router::new()
        .route("/health", get(api::health_check))
        .route("/v1/memory", post(api::create_memory))
        .route("/v1/search", get(api::search_memories))
        .route("/v1/context", get(api::get_context))
        .with_state(state);

    // Run Server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Goldfish Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
