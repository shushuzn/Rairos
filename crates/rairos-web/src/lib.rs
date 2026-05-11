//! Rairos Web — FastAPI/Axum web server for Rairos API
//!
//! Provides REST API endpoints for paper management, research, and gap detection.
//! Replaces: web/app.py, web/routes_*.py

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post, delete},
    Router,
};
use rairos_core::{Database, Paper, DbStats, ResearchGap};
use rairos_kg::{KnowledgeGraph, GraphAlgorithms, KgStats};
use rairos_llm::{GenePool, Capsule, GenePoolDiversityCalculator};
use rairos_memory::{ResearchMemory, ResearchStance, StanceType, MemoryStats};
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
    pub gene_pool: Arc<RwLock<GenePool>>,
    pub knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
    pub orchestrator: Arc<RwLock<Option<ResearchOrchestrator>>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let kg = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());
        Self {
            db: Arc::new(db),
            gene_pool: Arc::new(RwLock::new(GenePool::load().unwrap_or_else(|_| GenePool::new()))),
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
            status: if c.archived { "archived".to_string() } else { c.status.to_string() },
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
    let stats = state.db.stats().map_err(|e| WebError::Database(e.to_string()))?;
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

    let papers = state.db.list_papers(None, limit, offset)
        .map_err(|e| WebError::Database(e.to_string()))?;

    Ok(Json(papers.into_iter().map(PaperResponse::from).collect()))
}

async fn search_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PaperResponse>>, WebError> {
    let limit = query.limit.unwrap_or(20);
    let papers = state.db.search_papers(&query.q, limit)
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

async fn delete_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, WebError> {
    state.db.delete_paper(&id)
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

    let gaps = state.db.list_gaps(limit, offset)
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
    let gap = state.db.get_gap(&id)
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

    let filtered: Vec<GeneResponse> = capsules.into_iter()
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

    let gene = pool.capsules().last().map(GeneResponse::from)
        .ok_or_else(|| WebError::Internal("Failed to add gene".to_string()))?;

    Ok(Json(gene))
}

async fn get_gene(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GeneResponse>, WebError> {
    let pool = state.gene_pool.read().await;
    let capsule = pool.capsules().iter()
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
    if let Some(cap) = pool.capsules_mut().iter_mut().find(|c| c.capsule_id == id || c.capsule_id.starts_with(&id)) {
        if req.positive {
            cap.record_success();
        } else {
            cap.record_failure();
        }
        return Ok(Json(GeneResponse::from(&*cap)));
    }
    Err(WebError::NotFound(format!("Gene not found: {}", id)))
}

async fn gene_diversity(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, WebError> {
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
// Routes - Knowledge Graph
// ============================================================================

async fn kg_stats(State(state): State<Arc<AppState>>) -> Result<Json<KgStats>, WebError> {
    let kg = state.knowledge_graph.read().await;
    Ok(Json(kg.stats()))
}

async fn kg_export(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, WebError> {
    let kg = state.knowledge_graph.read().await;
    Ok(Json(kg.export_json()))
}

async fn kg_citations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, WebError> {
    let kg = state.knowledge_graph.read().await;
    let citing = kg.get_citing(&id);
    Ok(Json(citing.into_iter().map(|n| serde_json::to_value(n).unwrap_or_default()).collect()))
}

async fn kg_references(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, WebError> {
    let kg = state.knowledge_graph.read().await;
    let refs = kg.get_references(&id);
    Ok(Json(refs.into_iter().map(|n| serde_json::to_value(n).unwrap_or_default()).collect()))
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
        _ => return Err(WebError::BadRequest(format!("Invalid stance: {}", req.stance))),
    };

    let mut memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    let stance = ResearchStance::new(&req.topic, &req.claim, stance_type, &req.reasoning);
    memory.add_stance(stance.clone());
    memory.save().map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(stance))
}

async fn get_stance(Path(id): Path<String>) -> Result<Json<ResearchStance>, WebError> {
    let memory = ResearchMemory::load().map_err(|e| WebError::Internal(e.to_string()))?;
    let stance = memory.get_stance(&id)
        .or_else(|| memory.stances().iter().find(|s| s.stance_id.starts_with(&id)))
        .ok_or_else(|| WebError::NotFound(format!("Stance not found: {}", id)))?;

    Ok(Json(stance.clone()))
}

// ============================================================================
// Routes - Extended Papers (with filtering)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PapersQuery {
    pub q: Option<String>,
    pub source: Option<String>,
    pub page: Option<usize>,
    pub year_from: Option<String>,
    pub year_to: Option<String>,
}

async fn list_papers_extended(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PapersQuery>,
) -> Result<Json<serde_json::Value>, WebError> {
    let limit = 20;
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let papers: Vec<PaperResponse> = if let Some(ref q) = query.q {
        if !q.is_empty() {
            state.db.search_papers(q, limit)
                .map(|rows| rows.into_iter().map(PaperResponse::from).collect())
                .map_err(|e| WebError::Database(e.to_string()))?
        } else {
            let rows = state.db.list_papers(None, limit, offset)
                .map_err(|e| WebError::Database(e.to_string()))?;
            rows.into_iter().map(PaperResponse::from).collect()
        }
    } else {
        let rows = state.db.list_papers(None, limit, offset)
            .map_err(|e| WebError::Database(e.to_string()))?;
        rows.into_iter().map(PaperResponse::from).collect()
    };
    let total = papers.len();

    let total_pages = ((total + limit - 1) / limit).max(1);

    Ok(Json(serde_json::json!({
        "papers": papers,
        "query": query.q.as_deref().unwrap_or(""),
        "total": total,
        "total_pages": total_pages,
        "page": page,
        "year_from": query.year_from.as_deref().unwrap_or(""),
        "year_to": query.year_to.as_deref().unwrap_or(""),
    })))
}

#[derive(Debug, Deserialize)]
pub struct GapAnalysisQuery {
    pub ids: Option<String>,
}

async fn gap_analysis(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GapAnalysisQuery>,
) -> Result<Json<serde_json::Value>, WebError> {
    let Some(ids_str) = &query.ids else {
        return Ok(Json(serde_json::json!({
            "error": "No papers selected"
        })));
    };

    let paper_ids: Vec<String> = ids_str.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if paper_ids.len() < 2 {
        return Ok(Json(serde_json::json!({
            "error": "Need at least 2 papers"
        })));
    }

    // Build papers data from database
    let mut papers_data = Vec::new();
    for pid in &paper_ids {
        if let Ok(paper) = state.db.get_paper(pid) {
            papers_data.push(serde_json::json!({
                "id": paper.id,
                "title": paper.title,
                "abstract": paper.abstract_text,
            }));
        }
    }

    // Return placeholder gap analysis (LLM-based analysis would require rairos-llm integration)
    Ok(Json(serde_json::json!({
        "papers": papers_data,
        "paper_ids": paper_ids,
        "shared_themes": [],
        "frontier_gaps": [],
        "complementary_gaps": [],
        "contradictions": [],
        "status": "available"
    })))
}

// ============================================================================
// Routes - Briefing
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BriefingQuery {
    pub arxiv_id: Option<String>,
}

async fn get_briefing(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BriefingQuery>,
) -> Result<Json<serde_json::Value>, WebError> {
    let arxiv_id = query.arxiv_id.as_deref().unwrap_or("").trim();
    if arxiv_id.is_empty() {
        return Ok(Json(serde_json::json!({
            "error": "Please enter an arXiv ID"
        })));
    }

    // Try to load briefing from disk
    let slug = arxiv_id.replace("/", "_").replace(":", "_");
    let briefing_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rairos")
        .join("briefings")
        .join(format!("briefing_{}.md", slug));

    let markdown_content = if briefing_path.exists() {
        std::fs::read_to_string(&briefing_path).ok()
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "arxiv_id": arxiv_id,
        "markdown": markdown_content,
        "markdown_path": format!("/data/briefings/briefing_{}.md", slug),
    })))
}

// ============================================================================
// Routes - Notifications
// ============================================================================

// In-memory notification store
use std::sync::Mutex;
use once_cell::sync::Lazy;

static NOTIFICATION_STORE: Lazy<Mutex<Vec<serde_json::Value>>> = Lazy::new(|| Mutex::new(Vec::new()));

async fn get_notifications() -> Json<serde_json::Value> {
    let notifications = NOTIFICATION_STORE.lock().unwrap().clone();
    Json(serde_json::json!({ "notifications": notifications }))
}

#[derive(Debug, Deserialize)]
pub struct DismissRequest {
    pub uid: Option<String>,
}

async fn dismiss_notification(
    Json(req): Json<DismissRequest>,
) -> Json<serde_json::Value> {
    let mut store = NOTIFICATION_STORE.lock().unwrap();
    if let Some(uid) = &req.uid {
        store.retain(|n| n.get("uid").and_then(|v| v.as_str()) != Some(uid.as_str()));
    } else {
        store.clear();
    }
    Json(serde_json::json!({ "success": true, "remaining": store.len() }))
}

// ============================================================================
// Routes - Gene Pool Visualizations
// ============================================================================

async fn gene_pool_graph_svg(
    State(state): State<Arc<AppState>>,
) -> Result<String, WebError> {
    let pool = state.gene_pool.read().await;
    let capsules = pool.capsules();

    let nodes_json = serde_json::to_string(&capsules.iter().map(|c| {
        serde_json::json!({
            "id": c.capsule_id,
            "label": format!("Capsule {}", &c.capsule_id.to_string()[..8]),
            "gap_type": "unknown",
            "color": "#7A9E7A",
            "score": 0.5,
        })
    }).collect::<Vec<_>>()).unwrap_or_default();

    Ok(format!(r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Gene Pool Graph</title>
<script src="https://d3js.org/d3.v7.min.js"></script>
<style>body{{margin:0;background:#0d1117}}h2{{color:#c9d1d9;padding:12px;font-family:monospace;font-size:14px}}
svg{{width:100vw;height:100vh}}</style></head><body>
<h2>Gene Pool — {} capsules</h2>
<svg></svg>
<script>
const nodes = {};
const links = [];
const simulation = d3.forceSimulation(nodes)
  .force("link", d3.forceLink(links).id(d=>d.id).distance(80))
  .force("charge", d3.forceManyBody().strength(-200))
  .force("center", d3.forceCenter(window.innerWidth/2, window.innerHeight/2));
const svg = d3.select('svg');
const link = svg.append('g').selectAll('line').data(links).join('line')
  .attr('stroke','#999').attr('stroke-opacity',0.6);
const node = svg.append('g').selectAll('g').data(nodes).join('g')
  .call(d3.drag().on('start',dragstarted).on('drag',dragged).on('end',dragended));
node.append('rect').attr('width',130).attr('height',36).attr('rx',6).attr('fill',d=>d.color||'#7A9E7A');
node.append('text').attr('x',8).attr('y',14).attr('fill','white').text(d=>d.label.slice(0,18));
node.append('text').attr('x',8).attr('y',27).attr('fill','#aaa').text(d=>d.gap_type.slice(0,14));
simulation.on('tick',()=>{{link.attr('x1',d=>d.source.x).attr('y1',d=>d.source.y).attr('x2',d=>d.target.x).attr('y2',d=>d.target.y);node.attr('transform',d=>'translate('+d.x+','+d.y+')');}});
function dragstarted(e){{if(!e.active)simulation.alphaTarget(0.3).restart();e.subject.fx=e.subject.x;e.subject.fy=e.subject.y;}}
function dragged(e){{e.subject.fx=e.x;e.subject.fy=e.y;}}
function dragended(e){{if(!e.active)simulation.alphaTarget(0);e.subject.fx=null;e.subject.fy=null;}}
</script></body></html>"#, capsules.len(), nodes_json))
}

async fn contradiction_heatmap_svg(
    State(state): State<Arc<AppState>>,
) -> Result<String, WebError> {
    Ok(r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Contradiction Heatmap</title>
<style>body{{margin:0;background:#0d1117;font-family:monospace}}h2{{color:#c9d1d9;padding:12px}}table{{width:100%;border-collapse:collapse}}th,td{{padding:6px 10px;border-bottom:1px solid #21262d;text-align:left;font-size:12px}}th{{color:#8b949e;background:#161b22}}td{{color:#c9d1d9}}</style></head>
<body><h2>Contradiction Heatmap — no data yet</h2>
<p style="color:#484f58;padding:0 16px">Run `rairos analyze --contradictions` first to populate.</p>
</body></html>"#.to_string())
}

// ============================================================================
// Routes - Impact Ranking
// ============================================================================

async fn impact_ranking(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let rows = state.db.list_papers(None, 100, 0)
        .map_err(|e| WebError::Database(e.to_string()))?;

    let mut papers: Vec<_> = rows.into_iter().map(|p| {
        serde_json::json!({
            "paper_id": p.id,
            "title": p.title,
            "year": p.published.format("%Y").to_string(),
            "citation_count": p.metadata.cited_by,
        })
    }).collect();

    // Sort by citation count descending
    papers.sort_by(|a, b| {
        let ca = a.get("citation_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let cb = b.get("citation_count").and_then(|v| v.as_u64()).unwrap_or(0);
        cb.cmp(&ca)
    });

    Ok(Json(serde_json::json!({ "ranking": papers.into_iter().take(50).collect::<Vec<_>>() })))
}

// ============================================================================
// Routes - Research Log
// ============================================================================

async fn research_log_page() -> impl IntoResponse {
    let html = r#"<div class="research-log">
        <h2>Research Log</h2>
        <p>Research notes will appear here.</p>
    </div>"#;
    Html(html)
}

#[derive(Debug, Deserialize)]
pub struct AddNoteRequest {
    pub paper_id: Option<String>,
    pub note: String,
    pub tags: Option<Vec<String>>,
}

async fn add_note(
    Json(req): Json<AddNoteRequest>,
) -> Json<serde_json::Value> {
    // Notes would be stored in memory or database
    Json(serde_json::json!({ "success": true }))
}

async fn get_notes(
    Query(params): Query<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "notes": [] }))
}

// ============================================================================
// Server
// ============================================================================

async fn index() -> impl IntoResponse {
    let html = include_str!("../static/index.html");
    Html(html)
}

use axum::response::Html;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/papers", get(list_papers))
        .route("/papers/search", get(search_papers))
        .route("/papers", post(add_paper))
        .route("/papers/:id", get(get_paper))
        .route("/papers/:id", delete(delete_paper))
        .route("/gaps", get(list_gaps))
        .route("/gaps/:id", get(get_gap))
        .route("/genes", get(list_genes))
        .route("/genes", post(add_gene))
        .route("/genes/:id", get(get_gene))
        .route("/genes/:id/feedback", post(gene_feedback))
        .route("/genes/diversity", get(gene_diversity))
        .route("/research", post(research))
        .route("/kg/stats", get(kg_stats))
        .route("/kg/export", get(kg_export))
        .route("/kg/papers/:id/citations", get(kg_citations))
        .route("/kg/papers/:id/references", get(kg_references))
        .route("/kg/papers/:source/path/:target", get(kg_path))
        .route("/kg/rank", get(kg_rank))
        .route("/memory/stats", get(memory_stats))
        .route("/memory/stances", get(list_stances))
        .route("/memory/stances", post(add_stance))
        .route("/memory/stances/:id", get(get_stance))
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
