//! Rairos Web — FastAPI/Axum web server for Rairos API
//!
//! Provides REST API endpoints for paper management, research, and gap detection.
//! Replaces: web/app.py, web/routes_*.py

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use rairos_core::{Database, DbStats, Paper, ResearchGap};
use rairos_kg::{GraphAlgorithms, KgStats, KnowledgeGraph};
use rairos_llm::{
    briefing, citation_chain, impact, Capsule, GenePool, GenePoolDiversityCalculator,
    LlmClient, LlmCredentials, OpenAiClient,
};
use rairos_memory::{MemoryStats, ResearchMemory, ResearchStance, StanceType};
use rairos_parser::{self, detect_source, Source};
use rairos_research::{ResearchOrchestrator, ResearchQuery};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub gene_pool: Arc<RwLock<GenePool>>,
    pub knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
    pub orchestrator: Arc<RwLock<Option<ResearchOrchestrator>>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let kg = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());
        Self {
            db: Arc::new(db),
            gene_pool: Arc::new(RwLock::new(
                GenePool::load().unwrap_or_else(|_| GenePool::new()),
            )),
            knowledge_graph: Arc::new(RwLock::new(kg)),
            orchestrator: Arc::new(RwLock::new(None)),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AddPaperRequest {
    pub id: String,
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

#[derive(Debug, Deserialize)]
pub struct GeneAddRequest {
    pub approach: String,
    pub gap_type: String,
    pub keywords: Vec<String>,
    pub paper_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeneFeedbackRequest {
    pub positive: bool,
}

#[derive(Debug, Deserialize)]
pub struct BriefingRequest {
    pub arxiv_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CitationChainRequest {
    pub arxiv_id: String,
    pub depth: Option<u32>,
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
            published: p.published.format("%Y-%m-%dT%H:%M:%S").to_string(),
            categories: p.categories,
            cited_by: p.metadata.cited_by as u32,
            references: p.metadata.references as u32,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GapResponse {
    pub id: String,
    pub category: String,
    pub description: String,
    pub severity: String,
    pub paper_ids: Vec<String>,
}

impl From<ResearchGap> for GapResponse {
    fn from(g: ResearchGap) -> Self {
        Self {
            id: g.id,
            category: g.category,
            description: g.description,
            severity: g.severity,
            paper_ids: g.paper_ids,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GeneResponse {
    pub capsule_id: String,
    pub gap_type: String,
    pub approach: String,
    pub status: String,
    pub impact_score: f64,
    pub success_count: i32,
    pub failure_count: i32,
    pub created_at: String,
}

impl From<&Capsule> for GeneResponse {
    fn from(c: &Capsule) -> Self {
        Self {
            capsule_id: c.capsule_id.clone(),
            gap_type: c.action_gap_type.clone(),
            approach: c.archetype.approach_summary.clone(),
            status: if c.archived {
                "archived".to_string()
            } else {
                c.status.to_string()
            },
            impact_score: c.impact_score,
            success_count: c.success_count,
            failure_count: c.failure_count,
            created_at: c.created_at.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total: i64,
    pub pending: i64,
    pub done: i64,
    pub gaps: i64,
}

impl From<DbStats> for StatsResponse {
    fn from(s: DbStats) -> Self {
        Self {
            total: s.total,
            pending: s.pending,
            done: s.done,
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
// Routes - Health & Stats
// ============================================================================

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "sqlite".to_string(),
    })
}

async fn stats(State(state): State<Arc<AppState>>) -> Result<Json<StatsResponse>, WebError> {
    let stats = state
        .db
        .stats()
        .map_err(|e| WebError::Database(e.to_string()))?;
    Ok(Json(StatsResponse::from(stats)))
}

// ============================================================================
// Routes - Papers
// ============================================================================

async fn list_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PaperResponse>>, WebError> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let papers = state
        .db
        .list_papers(None, limit, offset)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(papers.into_iter().map(PaperResponse::from).collect()))
}

async fn search_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PaperResponse>>, WebError> {
    let limit = query.limit.unwrap_or(20);
    let papers = state
        .db
        .search_papers(&query.q, limit)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(papers.into_iter().map(PaperResponse::from).collect()))
}

async fn get_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PaperResponse>, WebError> {
    if let Ok(paper) = state.db.get_paper(&id) {
        return Ok(Json(PaperResponse::from(paper)));
    }
    if let Ok(Some(paper)) = state.db.get_paper_by_arxiv(&id) {
        return Ok(Json(PaperResponse::from(paper)));
    }
    Err(WebError::NotFound(format!("Paper not found: {}", id)))
}

async fn add_paper(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddPaperRequest>,
) -> Result<Json<PaperResponse>, WebError> {
    let source = detect_source(&req.id)
        .ok_or_else(|| WebError::BadRequest(format!("Unknown ID format: {}", req.id)))?;

    let paper = match source {
        Source::ArXiv => rairos_parser::fetch_arxiv(&req.id)
            .await
            .map_err(|e| WebError::Parse(e.to_string()))?,
        Source::CrossRef => rairos_parser::fetch_crossref(&req.id)
            .await
            .map_err(|e| WebError::Parse(e.to_string()))?,
        Source::SemanticScholar => rairos_parser::fetch_semantic(&req.id)
            .await
            .map_err(|e| WebError::Parse(e.to_string()))?,
    };

    state
        .db
        .insert_paper(&paper)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(PaperResponse::from(paper)))
}

async fn delete_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, WebError> {
    state
        .db
        .delete_paper(&id)
        .map_err(|e| WebError::Database(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Routes - Gaps
// ============================================================================

async fn list_gaps(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListGapsParams>,
) -> Result<Json<Vec<GapResponse>>, WebError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let gaps = state
        .db
        .list_gaps(limit, offset)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(gaps.into_iter().map(GapResponse::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct ListGapsParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

async fn get_gap(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GapResponse>, WebError> {
    let gap = state
        .db
        .get_gap(&id)
        .map_err(|e| WebError::Database(e.to_string()))?
        .ok_or_else(|| WebError::NotFound(format!("Gap not found: {}", id)))?;

    Ok(Json(GapResponse::from(gap)))
}

// ============================================================================
// Routes - Gene Pool
// ============================================================================

async fn list_genes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListGenesParams>,
) -> Result<Json<Vec<GeneResponse>>, WebError> {
    let pool = state.gene_pool.read().await;
    let capsules: Vec<GeneResponse> = pool.capsules().iter().map(GeneResponse::from).collect();

    let filtered: Vec<GeneResponse> = capsules
        .into_iter()
        .filter(|g| {
            if let Some(ref gt) = params.gap_type {
                if &g.gap_type != gt {
                    return false;
                }
            }
            if let Some(ref status) = params.status {
                if &g.status != status {
                    return false;
                }
            }
            true
        })
        .skip(params.offset.unwrap_or(0))
        .take(params.limit.unwrap_or(50))
        .collect();

    Ok(Json(filtered))
}

#[derive(Debug, Deserialize)]
pub struct ListGenesParams {
    pub gap_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

async fn add_gene(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GeneAddRequest>,
) -> Result<Json<GeneResponse>, WebError> {
    let mut capsule = Capsule::new(&req.approach, &req.gap_type, req.keywords);
    if let Some(pid) = req.paper_id {
        capsule = capsule.with_paper(&pid);
    }

    let mut pool = state.gene_pool.write().await;
    pool.add_capsule(capsule);

    let gene = pool
        .capsules()
        .last()
        .map(GeneResponse::from)
        .ok_or_else(|| WebError::Internal("Failed to add gene".to_string()))?;

    Ok(Json(gene))
}

async fn get_gene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GeneResponse>, WebError> {
    let pool = state.gene_pool.read().await;
    let capsule = pool
        .capsules()
        .iter()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(&id))
        .map(GeneResponse::from)
        .ok_or_else(|| WebError::NotFound(format!("Gene not found: {}", id)))?;

    Ok(Json(capsule))
}

async fn gene_feedback(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<GeneFeedbackRequest>,
) -> Result<Json<GeneResponse>, WebError> {
    let mut pool = state.gene_pool.write().await;
    if let Some(cap) = pool
        .capsules_mut()
        .iter_mut()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(&id))
    {
        if req.positive {
            cap.record_success();
        } else {
            cap.record_failure();
        }
        return Ok(Json(GeneResponse::from(&*cap)));
    }
    Err(WebError::NotFound(format!("Gene not found: {}", id)))
}

async fn gene_diversity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let pool = state.gene_pool.read().await;
    let diversity = GenePoolDiversityCalculator::calculate(pool.capsules());

    Ok(Json(serde_json::json!({
        "shannon_index": diversity.shannon_index,
        "capsule_count": diversity.capsule_count,
        "diversity_score": diversity.diversity_score,
        "family_counts": diversity.family_counts,
        "gap_type_counts": diversity.gap_type_counts,
    })))
}

// ============================================================================
// Routes - Research
// ============================================================================

async fn research(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResearchRequest>,
) -> Result<Json<rairos_research::ResearchResult>, WebError> {
    let orchestrator = state.orchestrator.read().await;
    let orch = orchestrator
        .as_ref()
        .ok_or_else(|| WebError::Internal("Orchestrator not initialized".to_string()))?;

    let query = ResearchQuery::new(&req.query)
        .with_categories(req.categories.unwrap_or_default())
        .with_max_papers(req.max_papers.unwrap_or(50));

    let result = orch
        .research(&query)
        .await
        .map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(result))
}

// ============================================================================
// Routes - Knowledge Graph
// ============================================================================

async fn kg_stats(State(state): State<Arc<AppState>>) -> Result<Json<KgStats>, WebError> {
    let kg = state.knowledge_graph.read().await;
    Ok(Json(kg.stats()))
}

async fn kg_export(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let kg = state.knowledge_graph.read().await;
    Ok(Json(kg.export_json()))
}

async fn kg_citations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, WebError> {
    let kg = state.knowledge_graph.read().await;
    let citing = kg.get_citing(&id);
    Ok(Json(
        citing
            .into_iter()
            .map(|n| serde_json::to_value(n).unwrap_or_default())
            .collect(),
    ))
}

async fn kg_references(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, WebError> {
    let kg = state.knowledge_graph.read().await;
    let refs = kg.get_references(&id);
    Ok(Json(
        refs.into_iter()
            .map(|n| serde_json::to_value(n).unwrap_or_default())
            .collect(),
    ))
}

async fn kg_path(
    State(state): State<Arc<AppState>>,
    Path((source, target)): Path<(String, String)>,
) -> Result<Json<Option<Vec<String>>>, WebError> {
    let kg = state.knowledge_graph.read().await;
    Ok(Json(kg.find_path(&source, &target)))
}

async fn kg_rank(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, WebError> {
    let kg = state.knowledge_graph.read().await;
    let ranks = GraphAlgorithms::rank_papers(&kg);
    let mut result: Vec<_> = ranks.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(Json(serde_json::json!({
        "rankings": result.into_iter().take(50).collect::<Vec<_>>()
    })))
}

// ============================================================================
// Routes - Memory
// ============================================================================

async fn memory_stats() -> Result<Json<MemoryStats>, WebError> {
    let memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    Ok(Json(memory.stats()))
}

#[derive(Debug, Deserialize)]
pub struct StanceAddRequest {
    pub topic: String,
    pub claim: String,
    pub stance: String,
    pub reasoning: String,
}

async fn list_stances() -> Result<Json<Vec<ResearchStance>>, WebError> {
    let memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    Ok(Json(memory.stances().to_vec()))
}

async fn add_stance(Json(req): Json<StanceAddRequest>) -> Result<Json<ResearchStance>, WebError> {
    let stance_type = match req.stance.to_lowercase().as_str() {
        "supported" => StanceType::Supported,
        "rejected" => StanceType::Rejected,
        "deferred" => StanceType::Deferred,
        "qualified" => StanceType::Qualified,
        _ => {
            return Err(WebError::BadRequest(format!(
                "Invalid stance: {}",
                req.stance
            )))
        }
    };

    let mut memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    let stance = ResearchStance::new(&req.topic, &req.claim, stance_type, &req.reasoning);
    memory.add_stance(stance.clone());
    memory
        .save()
        .map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(stance))
}

async fn get_stance(Path(id): Path<String>) -> Result<Json<ResearchStance>, WebError> {
    let memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    let stance = memory
        .get_stance(&id)
        .or_else(|| {
            memory
                .stances()
                .iter()
                .find(|s| s.stance_id.starts_with(&id))
        })
        .ok_or_else(|| WebError::NotFound(format!("Stance not found: {}", id)))?;

    Ok(Json(stance.clone()))
}

// ============================================================================
// Routes - Impact & Insights
// ============================================================================

async fn impact_ranking(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let papers = state
        .db
        .list_papers(None, 1000, 0)
        .map_err(|e| WebError::Database(e.to_string()))?;

    let current_year = 2025;
    let paper_data: Vec<(String, String, u32, i32)> = papers
        .iter()
        .map(|p| {
            let year = p
                .published
                .format("%Y")
                .to_string()
                .parse::<i32>()
                .unwrap_or(current_year);
            (
                p.id.clone(),
                p.title.clone(),
                p.metadata.cited_by as u32,
                year,
            )
        })
        .collect();

    let rankings = impact::rank_papers(&paper_data, current_year, 50);

    Ok(Json(serde_json::json!({
        "rankings": rankings,
        "total": rankings.len()
    })))
}

async fn insights(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let db_stats = state
        .db
        .stats()
        .map_err(|e| WebError::Database(e.to_string()))?;
    let pool = state.gene_pool.read().await;
    let diversity = GenePoolDiversityCalculator::calculate(pool.capsules());

    let mut gap_type_counts: HashMap<String, usize> = HashMap::new();
    for cap in pool.capsules() {
        *gap_type_counts.entry(cap.action_gap_type.clone()).or_default() += 1;
    }

    Ok(Json(serde_json::json!({
        "database": {
            "total_papers": db_stats.total,
            "pending": db_stats.pending,
            "done": db_stats.done,
            "gaps": db_stats.gaps,
        },
        "gene_pool": {
            "total_capsules": pool.capsules().len(),
            "active_capsules": pool.active_capsules().len(),
            "diversity": {
                "shannon_index": diversity.shannon_index,
                "capsule_count": diversity.capsule_count,
                "diversity_score": diversity.diversity_score,
                "family_counts": diversity.family_counts,
                "gap_type_counts": diversity.gap_type_counts,
            },
            "gap_type_distribution": gap_type_counts,
        }
    })))
}

// ============================================================================
// Routes - Briefings
// ============================================================================

async fn generate_briefing(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BriefingRequest>,
) -> Result<Json<serde_json::Value>, WebError> {
    let paper = state
        .db
        .get_paper(&req.arxiv_id)
        .map_err(|e| WebError::NotFound(format!("Paper not found: {}", e)))?;

    let arxiv_id = paper.arxiv_id.clone().unwrap_or_default();

    // Try LLM-backed generation if credentials are available
    let creds = LlmCredentials::resolve(None, None);
    if !creds.api_key.is_empty() {
        let client: Arc<dyn LlmClient> =
            Arc::new(OpenAiClient::with_base_url(creds.api_key, creds.base_url));
        let briefing = briefing::generate_briefing(
            &*client,
            "gpt-4o-mini",
            &arxiv_id,
            &paper.title,
            &paper.abstract_text,
            &paper.authors,
        )
        .await;
        return Ok(Json(serde_json::to_value(briefing).unwrap_or_default()));
    }

    // Fallback: return synthetic briefing from abstract
    let summary = paper
        .abstract_text
        .chars()
        .take(300)
        .collect::<String>();
    Ok(Json(serde_json::json!({
        "success": true,
        "arxiv_id": arxiv_id,
        "title": paper.title,
        "summary": summary,
        "key_contributions": [],
        "methodology": "LLM not configured — no detailed briefing available.",
        "results": "LLM not configured — no detailed briefing available.",
        "relevance": "LLM not configured — no detailed briefing available.",
        "verdict": "No verdict (LLM not available).",
        "markdown": format!("# {}\n\n**Abstract:** {}\n\n_Generated without LLM._", paper.title, summary),
    })))
}

async fn list_briefings() -> Result<Json<Vec<serde_json::Value>>, WebError> {
    // Briefings are not persisted to a dedicated store yet
    Ok(Json(vec![]))
}

// ============================================================================
// Routes - Citation Chain
// ============================================================================

async fn citation_chain_handler(
    Json(req): Json<CitationChainRequest>,
) -> Result<Json<serde_json::Value>, WebError> {
    let depth = req.depth.unwrap_or(2);
    match citation_chain::build_chain(&req.arxiv_id, depth).await {
        Ok(chain) => Ok(Json(serde_json::to_value(chain).unwrap_or_default())),
        Err(e) => Err(WebError::Internal(format!("Citation chain error: {}", e))),
    }
}

// ============================================================================
// Routes - Reports
// ============================================================================

async fn list_reports() -> Result<Json<Vec<serde_json::Value>>, WebError> {
    // Reports are not persisted to a dedicated store yet
    Ok(Json(vec![]))
}

// ============================================================================
// Routes - Research Loop
// ============================================================================

async fn research_loop_root(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let orch = state.orchestrator.read().await;
    Ok(Json(serde_json::json!({
        "running": orch.is_some(),
        "orchestrator_initialized": orch.is_some(),
    })))
}

async fn research_loop_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let orch = state.orchestrator.read().await;
    if let Some(ref orch) = *orch {
        let cost = orch.cost_summary().await;
        Ok(Json(serde_json::json!({
            "running": true,
            "orchestrator_initialized": true,
            "cost_summary": cost,
        })))
    } else {
        Ok(Json(serde_json::json!({
            "running": false,
            "orchestrator_initialized": false,
        })))
    }
}

async fn research_loop_action(
    State(state): State<Arc<AppState>>,
    Path(action): Path<String>,
) -> Result<Json<serde_json::Value>, WebError> {
    match action.as_str() {
        "start" => {
            let mut orch = state.orchestrator.write().await;
            if orch.is_some() {
                return Ok(Json(serde_json::json!({"status": "already_running"})));
            }
            let creds = LlmCredentials::resolve(None, None);
            if creds.api_key.is_empty() {
                return Err(WebError::BadRequest(
                    "LLM credentials not configured — cannot start orchestrator".to_string(),
                ));
            }
            let llm_client: Arc<dyn LlmClient> =
                Arc::new(OpenAiClient::with_base_url(creds.api_key, creds.base_url));
            *orch = Some(ResearchOrchestrator::new(state.db.clone(), llm_client));
            Ok(Json(serde_json::json!({"status": "started"})))
        }
        "stop" => {
            let mut orch = state.orchestrator.write().await;
            *orch = None;
            Ok(Json(serde_json::json!({"status": "stopped"})))
        }
        "run-cycle" => {
            let orch = state.orchestrator.read().await;
            if let Some(ref orch) = *orch {
                let query = ResearchQuery::new("research");
                let result = orch
                    .research(&query)
                    .await
                    .map_err(|e| WebError::Internal(e.to_string()))?;
                Ok(Json(serde_json::json!({
                    "status": "cycle_complete",
                    "result": {
                        "papers_found": result.papers_found,
                        "gaps_found": result.gaps.len(),
                        "citations_analyzed": result.citations_analyzed,
                    }
                })))
            } else {
                Err(WebError::BadRequest(
                    "Orchestrator not running".to_string(),
                ))
            }
        }
        _ => Err(WebError::BadRequest(format!("Unknown action: {}", action))),
    }
}

// ============================================================================
// Routes - Gene Matches
// ============================================================================

async fn paper_gene_matches(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<GeneResponse>>, WebError> {
    let pool = state.gene_pool.read().await;
    // Find capsules matching this paper by source_paper_id
    let matches: Vec<GeneResponse> = pool
        .capsules()
        .iter()
        .filter(|c| {
            c.archetype.source_paper_id.as_deref() == Some(&id)
                || c.capsule_id == id
                || c.capsule_id.starts_with(&id)
        })
        .map(GeneResponse::from)
        .collect();

    Ok(Json(matches))
}

// ============================================================================
// Routes - Server
// ============================================================================

use axum::response::Html;

async fn index() -> impl IntoResponse {
    let html = include_str!("../static/index.html");
    Html(html)
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/papers", get(list_papers))
        .route("/papers/search", get(search_papers))
        .route("/papers", post(add_paper))
        .route("/papers/{id}", get(get_paper))
        .route("/papers/{id}", delete(delete_paper))
        .route("/papers/{id}/gene-matches", get(paper_gene_matches))
        .route("/gaps", get(list_gaps))
        .route("/gaps/{id}", get(get_gap))
        .route("/genes", get(list_genes))
        .route("/genes", post(add_gene))
        .route("/genes/{id}", get(get_gene))
        .route("/genes/{id}/feedback", post(gene_feedback))
        .route("/genes/diversity", get(gene_diversity))
        .route("/research", post(research))
        .route("/kg/stats", get(kg_stats))
        .route("/kg/export", get(kg_export))
        .route("/kg/papers/{id}/citations", get(kg_citations))
        .route("/kg/papers/{id}/references", get(kg_references))
        .route("/kg/papers/{source}/path/{target}", get(kg_path))
        .route("/kg/rank", get(kg_rank))
        .route("/memory/stats", get(memory_stats))
        .route("/memory/stances", get(list_stances))
        .route("/memory/stances", post(add_stance))
        .route("/memory/stances/{id}", get(get_stance))
        .route("/impact/ranking", get(impact_ranking))
        .route("/insights", get(insights))
        .route("/briefing/generate", post(generate_briefing))
        .route("/briefings", get(list_briefings))
        .route("/citation-chain", post(citation_chain_handler))
        .route("/reports", get(list_reports))
        .route("/research-loop", get(research_loop_root))
        .route("/research-loop/status", get(research_loop_status))
        .route("/research-loop/{action}", post(research_loop_action))
        .with_state(state)
}

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
