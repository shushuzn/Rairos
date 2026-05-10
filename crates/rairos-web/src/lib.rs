//! Rairos Web — FastAPI/Axum web server for Rairos API
//!
//! Provides REST API endpoints for paper management, research, and gap detection.
//! Replaces: web/app.py, web/routes_*.py

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use rairos_core::{Database, Paper, DbStats};
use rairos_parser::{self, detect_source, Source};
use rairos_research::{ResearchQuery, ResearchOrchestrator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum WebError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            WebError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            WebError::Parse(e) => (StatusCode::BAD_REQUEST, e.clone()),
            WebError::NotFound(e) => (StatusCode::NOT_FOUND, e.clone()),
            WebError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.clone()),
            WebError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };

        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16()
        });

        (status, Json(body)).into_response()
    }
}

// ============================================================================
// App State
// ============================================================================

pub struct AppState {
    pub db: Arc<Database>,
    pub orchestrator: Arc<RwLock<Option<ResearchOrchestrator>>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(db),
            orchestrator: Arc::new(RwLock::new(None)),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AddPaperRequest {
    pub id: String, // arXiv ID, DOI, or Semantic Scholar ID
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ResearchRequest {
    pub query: String,
    pub categories: Option<Vec<String>>,
    pub max_papers: Option<usize>,
    pub include_citations: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PaperResponse {
    pub id: String,
    pub arxiv_id: Option<String>,
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub published: String,
    pub categories: Vec<String>,
    pub cited_by: u32,
    pub references: u32,
}

impl From<Paper> for PaperResponse {
    fn from(p: Paper) -> Self {
        Self {
            id: p.id,
            arxiv_id: p.arxiv_id,
            title: p.title,
            abstract_text: p.abstract_text,
            authors: p.authors,
            published: p.published,
            categories: p.categories,
            cited_by: p.metadata.cited_by,
            references: p.metadata.references,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total: usize,
    pub pending: usize,
    pub parsing: usize,
    pub done: usize,
    pub failed: usize,
    pub gaps: usize,
}

impl From<DbStats> for StatsResponse {
    fn from(s: DbStats) -> Self {
        Self {
            total: s.total,
            pending: s.pending,
            parsing: s.parsing,
            done: s.done,
            failed: s.failed,
            gaps: s.gaps,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
}

// ============================================================================
// Routes
// ============================================================================

// Health check
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "sqlite".to_string(),
    })
}

// Get database statistics
async fn stats(State(state): State<Arc<AppState>>) -> Result<Json<StatsResponse>, WebError> {
    let stats = state.db.stats().map_err(|e| WebError::Database(e.to_string()))?;
    Ok(Json(StatsResponse::from(stats)))
}

// List papers
async fn list_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PaperResponse>>, WebError> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let papers = state.db.list_papers(None, limit, offset)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(papers.into_iter().map(PaperResponse::from).collect()))
}

// Search papers
async fn search_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PaperResponse>>, WebError> {
    let papers = state.db.search_papers(&query.q, query.limit.unwrap_or(20))
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(papers.into_iter().map(PaperResponse::from).collect()))
}

// Get single paper
async fn get_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PaperResponse>, WebError> {
    // Try by ID first
    if let Ok(paper) = state.db.get_paper(&id) {
        return Ok(Json(PaperResponse::from(paper)));
    }

    // Try by arXiv ID
    if let Ok(Some(paper)) = state.db.get_paper_by_arxiv(&id) {
        return Ok(Json(PaperResponse::from(paper)));
    }

    Err(WebError::NotFound(format!("Paper not found: {}", id)))
}

// Add paper (fetch from external source)
async fn add_paper(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddPaperRequest>,
) -> Result<Json<PaperResponse>, WebError> {
    let source = detect_source(&req.id)
        .ok_or_else(|| WebError::BadRequest(format!("Unknown ID format: {}", req.id)))?;

    let paper = match source {
        Source::ArXiv => {
            rairos_parser::fetch_arxiv(&req.id)
                .await
                .map_err(|e| WebError::Parse(e.to_string()))?
        }
        Source::CrossRef => {
            rairos_parser::fetch_crossref(&req.id)
                .await
                .map_err(|e| WebError::Parse(e.to_string()))?
        }
        Source::SemanticScholar => {
            rairos_parser::fetch_semantic(&req.id)
                .await
                .map_err(|e| WebError::Parse(e.to_string()))?
        }
    };

    state.db.insert_paper(&paper)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(PaperResponse::from(paper)))
}

// Delete paper
async fn delete_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, WebError> {
    state.db.delete_paper(&id)
        .map_err(|e| WebError::Database(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// Run research query
async fn research(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResearchRequest>,
) -> Result<Json<rairos_research::ResearchResult>, WebError> {
    let orchestrator = state.orchestrator.read().await;
    let orch = orchestrator.as_ref()
        .ok_or_else(|| WebError::Internal("Orchestrator not initialized".to_string()))?;

    let query = ResearchQuery::new(&req.query)
        .with_categories(req.categories.unwrap_or_default())
        .with_max_papers(req.max_papers.unwrap_or(50));

    let result = orch.research(&query)
        .await
        .map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(result))
}

// ============================================================================
// Server
// ============================================================================

/// Build the Axum router
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/papers", get(list_papers))
        .route("/papers/search", get(search_papers))
        .route("/papers", post(add_paper))
        .route("/papers/:id", get(get_paper))
        .route("/papers/:id", axum::routing::delete(delete_paper))
        .route("/research", post(research))
        .with_state(state)
}

/// Start the server
pub async fn start(addr: &str, state: Arc<AppState>) -> Result<(), WebError> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to bind: {}", e)))?;

    tracing::info!("Starting Rairos web server on {}", addr);

    axum::serve(listener, build_router(state))
        .await
        .map_err(|e| WebError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_response_from_paper() {
        let paper = Paper::new(
            Some("2301.00001".to_string()),
            "Test Paper".to_string(),
            "Abstract here".to_string(),
        );

        let response = PaperResponse::from(paper.clone());
        assert_eq!(response.arxiv_id, Some("2301.00001".to_string()));
        assert_eq!(response.title, "Test Paper");
    }

    #[tokio::test]
    async fn test_build_router() {
        // This would need a real database to fully test
        // Just verify router builds without panic
        // let state = Arc::new(AppState::new());
        // let _router = build_router(state);
    }
}
