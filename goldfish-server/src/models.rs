use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub memory_type: String,
    pub importance: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct StartEpisodeRequest {
    pub title: String,
    pub context: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct MemoryResponse {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub importance: f32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub title: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub thinking_about: Vec<MemoryResponse>,
    pub formatted_context: String,
    pub current_episode: Option<String>,
}
