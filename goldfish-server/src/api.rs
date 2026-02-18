use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use goldfish::{ContextWindow, Memory, MemoryType};
use crate::models::{CreateMemoryRequest, MemoryResponse, SearchRequest, ContextResponse};
use crate::state::AppState;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

// Fix Create Memory
pub async fn create_memory(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateMemoryRequest>,
) -> Result<Json<MemoryResponse>, StatusCode> {
    // 1. Convert string type to enum
    let mem_type = match payload.memory_type.to_lowercase().as_str() {
        "fact" => MemoryType::Fact,
        "goal" => MemoryType::Goal,
        "preference" => MemoryType::Preference,
        "experience" => MemoryType::Event, 
        "decision" => MemoryType::Decision,
        _ => MemoryType::Fact,
    };

    // 2. Create Memory object
    let mut memory = Memory::new(payload.content.clone(), mem_type);
    if let Some(imp) = payload.importance {
        memory.importance = imp;
    }
    
    // Capture ID before moving memory into remember (if remember consumes it, but it takes reference)
    let id = memory.id.clone();
    let created_at = memory.created_at;

    // 3. Save to Cortex
    match state.cortex.remember(&memory).await {
        Ok(_) => {
            let response = MemoryResponse {
                id,
                content: payload.content,
                memory_type: format!("{:?}", mem_type),
                importance: memory.importance,
                created_at: created_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to save memory: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<Vec<MemoryResponse>>, StatusCode> {
    let limit = params.limit.unwrap_or(10);
    
    // Use cortex.recall instead of search
    match state.cortex.recall(&params.q, limit).await {
        Ok(results) => {
            let responses = results
                .into_iter()
                .map(|m| MemoryResponse {
                    id: m.memory.id,
                    content: m.memory.content,
                    memory_type: format!("{:?}", m.memory.memory_type),
                    importance: m.memory.importance, // Or m.score? Return importance for now
                    created_at: m.memory.created_at.to_rfc3339(),
                })
                .collect();
            Ok(Json(responses))
        }
        Err(e) => {
            tracing::error!("Search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_context(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ContextResponse>, StatusCode> {
    // 1. Get structured working memory items
    let active_items = state.cortex.get_context().await;
    
    let responses = active_items.iter().map(|m| MemoryResponse {
        id: m.memory_id.clone(),
        content: m.content.clone(),
        memory_type: format!("{:?}", m.memory_type),
        importance: m.attention_score, // Mapping attention to importance for view
        created_at: m.accessed_at.to_rfc3339(), // Using accessed_at for WM items
    }).collect();

    // 2. Build formatted string for LLM
    let context_window = ContextWindow::new(2000);
    let formatted_context = context_window.build(&state.cortex).await.unwrap_or_default();
    
    // 3. Get current episode ID if any
    let episode_id = state.cortex.get_current_experience().await.map(|e| e.id);

    Ok(Json(ContextResponse {
        thinking_about: responses,
        formatted_context,
        current_episode: episode_id,
    }))
}
