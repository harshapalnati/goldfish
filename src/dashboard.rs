//! # Web Dashboard API
//!
//! HTTP REST API for memory system management and visualization.
//!
//! ## Endpoints
//!
//! ### Memories
//! - `GET /api/memories` - List memories
//! - `POST /api/memories` - Create memory
//! - `GET /api/memories/{id}` - Get memory
//! - `PUT /api/memories/{id}` - Update memory
//! - `DELETE /api/memories/{id}` - Delete memory
//!
//! ### Search
//! - `GET /api/search?q={query}` - Search memories
//! - `POST /api/search` - Advanced search
//!
//! ### Associations
//! - `GET /api/memories/{id}/associations` - Get associations
//! - `POST /api/memories/{id}/associations` - Create association
//!
//! ### Temporal
//! - `GET /api/temporal/today` - Today's memories
//! - `GET /api/temporal/episode` - Episodes
//!
//! ### Administration
//! - `GET /api/stats` - System statistics
//! - `POST /api/maintenance` - Run maintenance
//! - `GET /api/dashboard` - Dashboard data
//!
//! ## Example
//!
//! ```rust,no_run
//! use goldfish::dashboard::DashboardServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let server = DashboardServer::new("./data", "127.0.0.1:8080").await?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

use crate::{
    error::{MemoryError, Result},
    types::{Memory, MemoryId, MemoryType, RelationType, Association, CreateMemoryInput},
    search::{SearchConfig, SearchMode},
    MemorySystem,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

/// Dashboard configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Bind address
    pub bind_address: String,
    /// Enable CORS
    pub enable_cors: bool,
    /// Data directory
    pub data_dir: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            enable_cors: true,
            data_dir: "./data".to_string(),
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    memory: Arc<MemorySystem>,
    stats: Arc<RwLock<SystemStats>>,
}

/// Dashboard server
pub struct DashboardServer {
    state: AppState,
    config: DashboardConfig,
}

impl DashboardServer {
    /// Create new dashboard server
    pub async fn new(data_dir: impl Into<String>, bind_address: impl Into<String>) -> Result<Self> {
        let data_dir = data_dir.into();
        let bind_address = bind_address.into();
        
        let memory = Arc::new(MemorySystem::new(&data_dir).await?);
        let stats = Arc::new(RwLock::new(SystemStats::default()));
        
        let state = AppState { memory, stats };
        let config = DashboardConfig {
            data_dir,
            bind_address,
            ..Default::default()
        };
        
        Ok(Self { state, config })
    }
    
    /// Run the server
    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = self.config.bind_address.parse()
            .map_err(|e| MemoryError::Configuration(format!("Invalid bind address: {}", e)))?;
        
        let app = create_router(self.state, self.config.enable_cors);
        
        println!("Goldfish dashboard running on http://{}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| MemoryError::Configuration(format!("Failed to bind: {}", e)))?;
        
        axum::serve(listener, app).await
            .map_err(|e| MemoryError::Configuration(format!("Server error: {}", e)))?;
        
        Ok(())
    }
}

/// Create API router
fn create_router(state: AppState, enable_cors: bool) -> Router {
    let mut router = Router::new()
        // Memory endpoints
        .route("/api/memories", get(list_memories).post(create_memory))
        .route("/api/memories/:id", get(get_memory).put(update_memory).delete(delete_memory))
        .route("/api/memories/:id/associations", get(get_associations).post(create_association))
        
        // Search endpoints
        .route("/api/search", get(search_memories))
        .route("/api/search/advanced", post(advanced_search))
        
        // Temporal endpoints
        .route("/api/temporal/today", get(get_today))
        .route("/api/temporal/yesterday", get(get_yesterday))
        .route("/api/temporal/recent/:days", get(get_recent))
        
        // Stats and dashboard
        .route("/api/stats", get(get_stats))
        .route("/api/dashboard", get(get_dashboard))
        .route("/api/maintenance", post(run_maintenance))
        
        // Health check
        .route("/health", get(health_check))
        
        .with_state(state);
    
    if enable_cors {
        router = router.layer(CorsLayer::permissive());
    }
    
    router
}

// ============ Request/Response Types ============

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub memory_type: MemoryType,
    pub tags: Vec<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMemoryRequest {
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub memory_type: Option<MemoryType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdvancedSearchRequest {
    pub query: String,
    pub mode: SearchMode,
    pub limit: usize,
    pub filters: SearchFilters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilters {
    pub memory_type: Option<MemoryType>,
    pub tags: Vec<String>,
    pub min_priority: Option<f32>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAssociationRequest {
    pub target_id: String,
    pub relation_type: RelationType,
    pub weight: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub priority: f32,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultResponse {
    pub memories: Vec<MemoryWithScore>,
    pub total: usize,
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryWithScore {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub score: f32,
    pub rank: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssociationResponse {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub weight: f32,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SystemStats {
    pub total_memories: u64,
    pub memories_by_type: HashMap<String, u64>,
    pub total_associations: u64,
    pub avg_priority: f32,
    pub storage_size_bytes: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub stats: SystemStats,
    pub recent_memories: Vec<MemoryResponse>,
    pub high_priority_memories: Vec<MemoryResponse>,
    pub trending_tags: Vec<TrendingTag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrendingTag {
    pub tag: String,
    pub count: u64,
    pub trend: String, // "up", "down", "stable"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceRequest {
    pub full: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceResponse {
    pub success: bool,
    pub pruned_count: u64,
    pub consolidated_count: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

// ============ Handler Functions ============

async fn list_memories(State(state): State<AppState>) -> impl IntoResponse {
    match state.memory.store().list_all(1000).await {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter()
                .map(memory_to_response)
                .collect();
            Json(responses).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn create_memory(
    State(state): State<AppState>,
    Json(req): Json<CreateMemoryRequest>,
) -> impl IntoResponse {
    let memory = Memory::new(&req.content, req.memory_type);
    
    match state.memory.save(&memory).await {
        Ok(_) => {
            let response = memory_to_response(memory);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.memory.load(&id).await {
        Ok(Some(memory)) => {
            Json(memory_to_response(memory)).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, format!("Memory {} not found", id)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMemoryRequest>,
) -> impl IntoResponse {
    match state.memory.load(&id).await {
        Ok(Some(mut memory)) => {
            if let Some(content) = req.content {
                memory.content = content;
            }
            if let Some(tags) = req.tags {
                memory.tags = tags;
            }
            if let Some(metadata) = req.metadata {
                memory.metadata = metadata;
            }
            
            match state.memory.update(&memory).await {
                Ok(_) => Json(memory_to_response(memory)).into_response(),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            }
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, format!("Memory {} not found", id)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.memory.forget(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, format!("Memory {} not found", id)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn search_memories(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let config = SearchConfig {
        limit: query.limit.unwrap_or(10),
        ..Default::default()
    };
    
    match state.memory.search_with_config(&query.q, &config).await {
        Ok(results) => {
            let memories: Vec<MemoryWithScore> = results.into_iter()
                .map(|r| MemoryWithScore {
                    id: r.memory.id.to_string(),
                    content: r.memory.content.clone(),
                    memory_type: format!("{:?}", r.memory.memory_type),
                    score: r.score,
                    rank: r.rank,
                })
                .collect();
            
            Json(SearchResultResponse {
                memories,
                total: memories.len(),
                query: query.q,
            }).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn advanced_search(
    State(state): State<AppState>,
    Json(req): Json<AdvancedSearchRequest>,
) -> impl IntoResponse {
    let config = SearchConfig {
        limit: req.limit,
        mode: req.mode,
        ..Default::default()
    };
    
    match state.memory.search_with_config(&req.query, &config).await {
        Ok(results) => {
            let memories: Vec<MemoryWithScore> = results.into_iter()
                .map(|r| MemoryWithScore {
                    id: r.memory.id.to_string(),
                    content: r.memory.content.clone(),
                    memory_type: format!("{:?}", r.memory.memory_type),
                    score: r.score,
                    rank: r.rank,
                })
                .collect();
            
            Json(SearchResultResponse {
                memories,
                total: memories.len(),
                query: req.query,
            }).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_associations(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.memory.get_associations(&id).await {
        Ok(associations) => {
            let responses: Vec<AssociationResponse> = associations.into_iter()
                .map(association_to_response)
                .collect();
            Json(responses).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn create_association(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateAssociationRequest>,
) -> impl IntoResponse {
    match state.memory.associate(&id, &req.target_id, req.relation_type).await {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_today(State(state): State<AppState>) -> impl IntoResponse {
    match state.memory.get_today().await {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter()
                .map(memory_to_response)
                .collect();
            Json(responses).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_yesterday(State(state): State<AppState>) -> impl IntoResponse {
    match state.memory.get_yesterday().await {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter()
                .map(memory_to_response)
                .collect();
            Json(responses).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_recent(
    State(state): State<AppState>,
    Path(days): Path<i64>,
) -> impl IntoResponse {
    match state.memory.get_last_days(days).await {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter()
                .map(memory_to_response)
                .collect();
            Json(responses).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.stats.read().await.clone();
    Json(stats).into_response()
}

async fn get_dashboard(State(state): State<AppState>) -> impl IntoResponse {
    // Collect dashboard data
    let recent = match state.memory.get_today().await {
        Ok(memories) => memories.into_iter().take(10).map(memory_to_response).collect(),
        Err(_) => vec![],
    };
    
    let high_priority = match state.memory.get_high_importance(0.7, 10).await {
        Ok(memories) => memories.into_iter().map(memory_to_response).collect(),
        Err(_) => vec![],
    };
    
    let stats = state.stats.read().await.clone();
    
    Json(DashboardData {
        stats,
        recent_memories: recent,
        high_priority_memories: high_priority,
        trending_tags: vec![], // Would implement trend analysis
    }).into_response()
}

async fn run_maintenance(
    State(state): State<AppState>,
    Json(_req): Json<MaintenanceRequest>,
) -> impl IntoResponse {
    use crate::maintenance::MaintenanceConfig;
    
    let config = MaintenanceConfig::default();
    
    match state.memory.run_maintenance(&config).await {
        Ok(report) => {
            Json(MaintenanceResponse {
                success: true,
                pruned_count: report.pruned as u64,
                consolidated_count: report.consolidated as u64,
                errors: report.errors.clone(),
            }).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// ============ Helper Functions ============

fn memory_to_response(memory: Memory) -> MemoryResponse {
    MemoryResponse {
        id: memory.id.to_string(),
        content: memory.content,
        memory_type: format!("{:?}", memory.memory_type),
        priority: memory.priority,
        tags: memory.tags,
        created_at: memory.created_at.to_rfc3339(),
        updated_at: memory.updated_at.map(|d| d.to_rfc3339()),
        metadata: memory.metadata,
    }
}

fn association_to_response(assoc: Association) -> AssociationResponse {
    AssociationResponse {
        id: assoc.id.to_string(),
        source_id: assoc.source_id.to_string(),
        target_id: assoc.target_id.to_string(),
        relation_type: format!("{:?}", assoc.relation_type),
        weight: assoc.weight,
        created_at: assoc.created_at.to_rfc3339(),
    }
}

fn error_response(status: StatusCode, message: String) -> axum::response::Response {
    let error = ErrorResponse {
        error: message,
        code: status.as_u16(),
    };
    (status, Json(error)).into_response()
}

/// Start dashboard server standalone
pub async fn start_dashboard(data_dir: &str, bind_address: &str) -> Result<()> {
    let server = DashboardServer::new(data_dir, bind_address).await?;
    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config_default() {
        let config = DashboardConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1:8080");
        assert!(config.enable_cors);
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        // Should not panic
    }
}
