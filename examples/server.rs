//! Simple HTTP API for Goldfish - Easy integration like Supermemory
//!
//! Run: cargo run --example server
//!
//! API Endpoints:
//! POST /v1/memory              - Store a memory
//! GET  /v1/memory/:id          - Get a memory
//! POST /v1/search              - Search memories
//! POST /v1/context             - Build context for LLM
//! POST /v1/episodes/start      - Start an episode
//! POST /v1/episodes/:id/end    - End an episode
//! GET  /v1/health              - Health check

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use goldfish::{Memory, MemoryCortex, MemoryType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// API State
pub struct ApiState {
    cortex: Arc<RwLock<MemoryCortex>>,
}

/// Request to store a memory
#[derive(Debug, Deserialize)]
pub struct StoreMemoryRequest {
    pub content: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    #[serde(default)]
    pub importance: Option<f32>,
    #[serde(default)]
    pub source: Option<String>,
}

/// Response for stored memory
#[derive(Debug, Serialize)]
pub struct MemoryResponse {
    pub id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub importance: f32,
    pub created_at: String,
}

/// Request to search
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// Search result
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub score: f32,
    pub why: String, // explainability
}

/// Request to build context
#[derive(Debug, Deserialize)]
pub struct ContextRequest {
    pub query: String,
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
}

fn default_token_budget() -> usize {
    2000
}

/// Context response
#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub context: String,
    pub tokens_used: usize,
    pub memories_included: usize,
    pub citations: Vec<Citation>,
}

/// Citation for context
#[derive(Debug, Serialize)]
pub struct Citation {
    pub id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub memory_type: String,
}

/// Episode request
#[derive(Debug, Deserialize)]
pub struct StartEpisodeRequest {
    pub title: String,
    #[serde(default)]
    pub context: Option<String>,
}

/// Episode response
#[derive(Debug, Serialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub title: String,
    pub started_at: String,
}

/// Health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Store a memory
async fn store_memory(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<StoreMemoryRequest>,
) -> Result<Json<MemoryResponse>, StatusCode> {
    let memory_type = parse_memory_type(&req.memory_type);

    let mut memory = Memory::new(req.content, memory_type);
    if let Some(imp) = req.importance {
        memory.importance = imp;
    }
    if let Some(src) = req.source {
        memory.source = Some(src);
    }

    let cortex = state.cortex.read().await;
    cortex
        .remember(&memory)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MemoryResponse {
        id: memory.id,
        content: memory.content,
        memory_type: format!("{:?}", memory.memory_type),
        importance: memory.importance,
        created_at: memory.created_at.to_rfc3339(),
    }))
}

/// Get a memory by ID
async fn get_memory(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<MemoryResponse>, StatusCode> {
    let cortex = state.cortex.read().await;
    let memory = cortex
        .think_about(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match memory {
        Some(m) => Ok(Json(MemoryResponse {
            id: m.id,
            content: m.content,
            memory_type: format!("{:?}", m.memory_type),
            importance: m.importance,
            created_at: m.created_at.to_rfc3339(),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Search memories
async fn search_memories(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let cortex = state.cortex.read().await;
    let results = cortex
        .recall(&req.query, req.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|r| {
            let why = format!("Matched query '{}' with score {:.2}", req.query, r.score);
            SearchResult {
                id: r.memory.id,
                content: r.memory.content,
                memory_type: format!("{:?}", r.memory.memory_type),
                score: r.score,
                why,
            }
        })
        .collect();

    Ok(Json(search_results))
}

/// Build context for LLM
async fn build_context(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ContextRequest>,
) -> Result<Json<ContextResponse>, StatusCode> {
    let cortex = state.cortex.read().await;

    // Get relevant memories
    let results = cortex
        .recall(&req.query, 20)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build context string with citations
    let mut context_parts = vec!["## Relevant Context\n".to_string()];
    let mut citations = vec![];
    let mut tokens_used = 0;

    for (i, result) in results.iter().take(10).enumerate() {
        let part = format!(
            "{} [{}] {}\n",
            i + 1,
            result.memory.memory_type,
            result.memory.content
        );
        tokens_used += part.split_whitespace().count();

        if tokens_used > req.token_budget {
            break;
        }

        context_parts.push(part);
        citations.push(Citation {
            id: result.memory.id.clone(),
            content: result.memory.content.clone(),
            memory_type: format!("{:?}", result.memory.memory_type),
        });
    }

    let context = context_parts.join("");

    Ok(Json(ContextResponse {
        context,
        tokens_used,
        memories_included: citations.len(),
        citations,
    }))
}

/// Start an episode
async fn start_episode(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<StartEpisodeRequest>,
) -> Result<Json<EpisodeResponse>, StatusCode> {
    let cortex = state.cortex.read().await;
    let id = cortex
        .start_episode(&req.title, &req.context.unwrap_or_default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(EpisodeResponse {
        id,
        title: req.title,
        started_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// End an episode
async fn end_episode(
    State(state): State<Arc<ApiState>>,
    Path(_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let cortex = state.cortex.read().await;
    cortex
        .end_episode()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Health check
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Parse memory type from string
fn parse_memory_type(s: &str) -> MemoryType {
    match s.to_lowercase().as_str() {
        "fact" => MemoryType::Fact,
        "preference" => MemoryType::Preference,
        "goal" => MemoryType::Goal,
        "decision" => MemoryType::Decision,
        "event" => MemoryType::Event,
        "identity" => MemoryType::Identity,
        _ => MemoryType::Fact,
    }
}

/// Create the API router
pub fn create_router(cortex: Arc<RwLock<MemoryCortex>>) -> Router {
    let state = Arc::new(ApiState { cortex });

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/memory", post(store_memory))
        .route("/v1/memory/:id", get(get_memory))
        .route("/v1/search", post(search_memories))
        .route("/v1/context", post(build_context))
        .route("/v1/episodes/start", post(start_episode))
        .route("/v1/episodes/:id/end", post(end_episode))
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🐠 Goldfish Memory API Server");
    println!("==============================\n");

    // Initialize cortex
    let cortex = Arc::new(RwLock::new(
        MemoryCortex::new("./goldfish_server_data").await?,
    ));

    let app = create_router(cortex);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("✅ Server running on http://localhost:3000");
    println!("\nAPI Endpoints:");
    println!("  POST /v1/memory          - Store memory");
    println!("  GET  /v1/memory/:id      - Get memory");
    println!("  POST /v1/search          - Search memories");
    println!("  POST /v1/context         - Build LLM context");
    println!("  POST /v1/episodes/start  - Start episode");
    println!("  GET  /health             - Health check");

    axum::serve(listener, app).await?;

    Ok(())
}
