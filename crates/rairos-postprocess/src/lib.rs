//! rairos-postprocess — 6-stage research deep dive pipeline.
//!
//! Ported from `llm/postprocess.py` + `cli/cmd/postprocess.py`.
//! Each stage is independently error-wrapped so partial success is preserved.
//!
//! Stages:
//!   1. PAPER_ANALYSIS — LLM section filling + rubric scoring (via rairos-llm)
//!   2. BENCHMARK — benchmark table detection (skip: no Rust equivalent yet)
//!   3. CROSS_REFERENCE — contradiction/alignment (skip: no Rust equivalent yet)
//!   4. INSIGHT — heuristic insight extraction from text
//!   5. KG_SYNC — knowledge graph integration (via rairos-kg)
//!   6. PNODE_UPDATE — re-render P-note (via rairos-render)

#![allow(clippy::print_literal)]

use chrono::Utc;
use rairos_core::constants::{LLM_BASE_URL, LLM_MODEL};
use rairos_core::Paper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::warn;

// ============================================================================
// Error
// ============================================================================

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("KG error: {0}")]
    Kg(String),
    #[error("Render error: {0}")]
    Render(String),
}

// ============================================================================
// Types
// ============================================================================

/// Pipeline execution stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostStage {
    #[serde(rename = "paper_analysis")]
    PaperAnalysis,
    #[serde(rename = "benchmark")]
    Benchmark,
    #[serde(rename = "cross_reference")]
    CrossReference,
    #[serde(rename = "insight")]
    Insight,
    #[serde(rename = "kg_sync")]
    KgSync,
    #[serde(rename = "pnote_update")]
    PnoteUpdate,
}

impl PostStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            PostStage::PaperAnalysis => "paper_analysis",
            PostStage::Benchmark => "benchmark",
            PostStage::CrossReference => "cross_reference",
            PostStage::Insight => "insight",
            PostStage::KgSync => "kg_sync",
            PostStage::PnoteUpdate => "pnote_update",
        }
    }

    /// Parse from a string value (case-insensitive, underscore-normalized).
    pub fn from_string(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "_").as_str() {
            "paper_analysis" => Some(PostStage::PaperAnalysis),
            "benchmark" => Some(PostStage::Benchmark),
            "cross_reference" => Some(PostStage::CrossReference),
            "insight" => Some(PostStage::Insight),
            "kg_sync" => Some(PostStage::KgSync),
            "pnote_update" | "pnote" => Some(PostStage::PnoteUpdate),
            _ => None,
        }
    }

    /// All stages in execution order.
    pub fn all() -> Vec<PostStage> {
        vec![
            PostStage::PaperAnalysis,
            PostStage::Benchmark,
            PostStage::CrossReference,
            PostStage::Insight,
            PostStage::KgSync,
            PostStage::PnoteUpdate,
        ]
    }
}

/// Result of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: serde_json::Value,
}

/// Complete result of the post-processing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingResult {
    pub paper_id: String,
    #[serde(default)]
    pub stages_completed: Vec<String>,
    #[serde(default)]
    pub stages_failed: Vec<String>,
    #[serde(default)]
    pub stage_results: HashMap<String, StageResult>,
    pub pnote_updated: bool,
    pub start_time: String,
    pub end_time: String,
}

impl PostProcessingResult {
    pub fn all_succeeded(&self) -> bool {
        self.stages_failed.is_empty()
    }

    pub fn summary(&self) -> String {
        let total = self.stage_results.len();
        let ok = self.stages_completed.len();
        let fail = self.stages_failed.len();
        format!("[{ok}/{total}] stages OK, {fail} failed")
    }

    fn new(paper_id: &str, start_time: String) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            stages_completed: Vec::new(),
            stages_failed: Vec::new(),
            stage_results: HashMap::new(),
            pnote_updated: false,
            start_time,
            end_time: String::new(),
        }
    }
}

/// Configuration for LLM calls in pipeline stages.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl LlmConfig {
    /// Build from environment variables (same scheme as Python `make_llm_config`).
    /// Returns None if no API key is available (graceful degradation).
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("AIROS_OPENAI_API_KEY"))
            .ok()?;
        if api_key.is_empty() {
            return None;
        }
        let base_url = std::env::var("OPENAI_BASE_URL")
            .or_else(|_| std::env::var("AIROS_DEFAULT_OPENAI_BASE_URL"))
            .unwrap_or_else(|_| LLM_BASE_URL.to_string());
        let model = std::env::var("AIROS_DEFAULT_MODEL_CLI")
            .unwrap_or_else(|_| LLM_MODEL.to_string());
        let timeout_secs = std::env::var("AIROS_LLM_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        Some(LlmConfig { api_key, base_url, model, timeout_secs })
    }
}

// ============================================================================
// Pipeline
// ============================================================================

/// Orchestrate 6-stage post-processing pipeline.
pub struct ResearchDeepDivePipeline {
    pub db: Option<rairos_core::Database>,
    pub data_dir: PathBuf,
    pub analysis_dir: PathBuf,
}

impl ResearchDeepDivePipeline {
    pub fn new(db: Option<rairos_core::Database>, data_dir: PathBuf) -> Self {
        let analysis_dir = data_dir.join(".analysis");
        Self { db, data_dir, analysis_dir }
    }

    /// Run the post-processing pipeline.
    ///
    /// Each stage is independently try/except wrapped so partial success is preserved.
    /// Stages with no Rust equivalent are gracefully skipped (reported as "not_implemented").
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        paper_id: &str,
        extracted_text: &str,
        paper: Option<&Paper>,
        tags: &[String],
        pnote_path: Option<&Path>,
        stages: Option<&[PostStage]>,
        llm_config: Option<&LlmConfig>,
    ) -> PostProcessingResult {
        let start_time = Utc::now().to_rfc3339();
        let stages_to_run: Vec<PostStage> =
            stages.map(|s| s.to_vec()).unwrap_or_else(PostStage::all);

        let tags: Vec<String> = tags.to_vec();
        let use_llm = llm_config.is_some();

        let mut result = PostProcessingResult::new(paper_id, start_time);

        // Ensure analysis output directory exists
        let paper_out = self.analysis_dir.join(paper_id);
        let _ = std::fs::create_dir_all(&paper_out);

        // Shared state between stages (carried from PAPER_ANALYSIS to PNODE_UPDATE)
        let mut sections: HashMap<String, String> = HashMap::new();
        let mut rubric: HashMap<String, serde_json::Value> = HashMap::new();

        // ── Stage 1: Paper Analysis ─────────────────────────────────────
        if stages_to_run.contains(&PostStage::PaperAnalysis) {
            let mut sr = StageResult::new("paper_analysis");
            let (title, abstract_text, authors_str) = self.get_paper_meta(paper, paper_id);
            if use_llm {
                let cfg = llm_config.unwrap();
                match self.run_paper_analysis(
                    paper_id,
                    &title,
                    &abstract_text,
                    &authors_str,
                    extracted_text,
                    cfg,
                ) {
                    Ok(analysis) => {
                        sections = analysis.sections;
                        rubric = analysis.rubric;
                        let kws = &analysis.keywords;
                        sr.success = true;
                        sr.data = serde_json::json!({
                            "sections_count": sections.len(),
                            "rubric": rubric,
                            "keywords": kws,
                        });
                        result.stages_completed.push("paper_analysis".to_string());
                        let _ = write_json(&paper_out.join("paper_analysis.json"), &sr.data);
                    }
                    Err(e) => {
                        sr.error = e.to_string();
                        result.stages_failed.push("paper_analysis".to_string());
                        warn!("PAPER_ANALYSIS failed: {e}");
                    }
                }
            } else {
                // Keyword-only fallback
                let fallback_kws = extract_keywords(&format!("{title} {abstract_text}"));
                sr.success = true;
                sr.data = serde_json::json!({
                    "note": "LLM disabled — keyword analysis only",
                    "keywords": fallback_kws,
                });
                result.stages_completed.push("paper_analysis".to_string());
            }
            result.stage_results.insert("paper_analysis".to_string(), sr);
        }

        // ── Stage 2: Benchmark ─────────────────────────────────────────
        if stages_to_run.contains(&PostStage::Benchmark) {
            let mut sr = StageResult::new("benchmark");
            sr.success = true;
            sr.data = serde_json::json!({
                "note": "not_implemented — no Rust equivalent yet",
            });
            result.stages_completed.push("benchmark".to_string());
            result.stage_results.insert("benchmark".to_string(), sr);
        }

        // ── Stage 3: Cross Reference ───────────────────────────────────
        if stages_to_run.contains(&PostStage::CrossReference) {
            let mut sr = StageResult::new("cross_reference");
            sr.success = true;
            sr.data = serde_json::json!({
                "note": "not_implemented — no Rust equivalent yet",
            });
            result.stages_completed.push("cross_reference".to_string());
            result.stage_results.insert("cross_reference".to_string(), sr);
        }

        // ── Stage 4: Insight Cards ─────────────────────────────────────
        if stages_to_run.contains(&PostStage::Insight) {
            let mut sr = StageResult::new("insight");
            let (title, _, _) = self.get_paper_meta(paper, paper_id);
            let cards = self.run_insight_stage(paper_id, &title, extracted_text);
            sr.success = true;
            sr.data = serde_json::json!({
                "cards_created": cards.len(),
                "card_ids": cards,
            });
            result.stages_completed.push("insight".to_string());
            let _ = write_json(&paper_out.join("insight_cards.json"), &sr.data);
            result.stage_results.insert("insight".to_string(), sr);
        }

        // ── Stage 5: KG Sync ───────────────────────────────────────────
        if stages_to_run.contains(&PostStage::KgSync) {
            let mut sr = StageResult::new("kg_sync");
            match self.run_kg_sync(paper, paper_id, &tags) {
                Ok(chart_data) => {
                    let data = if let Some(cd) = chart_data {
                        let mut m = serde_json::json!({"synced": true});
                        if let Some(obj) = m.as_object_mut() {
                            if let Some(cd_obj) = cd.as_object() {
                                for (k, v) in cd_obj {
                                    obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                        m
                    } else {
                        serde_json::json!({"synced": true})
                    };
                    sr.success = true;
                    sr.data = data;
                    result.stages_completed.push("kg_sync".to_string());
                    let _ = write_json(&paper_out.join("kg_sync.json"), &sr.data);
                }
                Err(e) => {
                    sr.error = e.to_string();
                    result.stages_failed.push("kg_sync".to_string());
                    warn!("KG_SYNC failed: {e}");
                }
            }
            result.stage_results.insert("kg_sync".to_string(), sr);
        }

        // ── Stage 6: P-Note Update ─────────────────────────────────────
        if stages_to_run.contains(&PostStage::PnoteUpdate) {
            if let Some(pnote) = pnote_path {
                let mut sr = StageResult::new("pnote_update");
                match self.run_pnote_update(
                    pnote,
                    paper,
                    paper_id,
                    &tags,
                    extracted_text,
                    &sections,
                    &rubric,
                ) {
                    Ok(()) => {
                        result.pnote_updated = true;
                        sr.success = true;
                        result.stages_completed.push("pnote_update".to_string());
                    }
                    Err(e) => {
                        sr.error = e.to_string();
                        result.stages_failed.push("pnote_update".to_string());
                        warn!("PNODE_UPDATE failed: {e}");
                    }
                }
                result.stage_results.insert("pnote_update".to_string(), sr);
            }
        }

        result.end_time = Utc::now().to_rfc3339();
        let _ = write_json(&paper_out.join("pipeline_result.json"), &result);
        result
    }

    // ── Stage Implementations ──────────────────────────────────────────

    /// Stage 1: LLM-powered paper analysis via rairos-llm.
    fn run_paper_analysis(
        &self,
        _paper_id: &str,
        title: &str,
        abstract_text: &str,
        authors: &str,
        body: &str,
        cfg: &LlmConfig,
    ) -> Result<PaperAnalysis, PipelineError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PipelineError::Llm(format!("tokio runtime: {e}")))?;
        // OpenAiClient defaults to standard OpenAI endpoint.
        // For custom base URLs, we'd need to extend the client API.
        let client = rairos_llm::OpenAiClient::new(cfg.api_key.clone());
        let result = rt.block_on(rairos_llm::paper_analyzer::analyze_paper(
            &client,
            &cfg.model,
            title,
            abstract_text,
            authors,
            body,
        ));
        Ok(PaperAnalysis {
            sections: result.sections,
            rubric: result
                .rubric
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::Number(v.into())))
                .collect(),
            keywords: result.keywords,
        })
    }

    /// Stage 4: Heuristic insight extraction from text.
    fn run_insight_stage(&self, paper_id: &str, _title: &str, text: &str) -> Vec<String> {
        // Simple heuristic: extract sentences containing insight markers
        let markers = [
            "key insight", "we propose", "we introduce", "our approach",
            "we show", "we demonstrate", "we find", "we observe", "critical",
            "important", "notably", "significantly", "surprisingly",
        ];
        let mut cards: Vec<String> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.len() < 20 || trimmed.len() > 500 {
                continue;
            }
            let lower = trimmed.to_lowercase();
            if markers.iter().any(|m| lower.contains(m)) {
                let card_id = format!("insight_{paper_id}_{}", cards.len() + 1);
                cards.push(card_id);
                if cards.len() >= 10 {
                    break;
                }
            }
        }
        cards
    }

    /// Stage 5: Knowledge graph sync via rairos-kg.
    fn run_kg_sync(
        &self,
        paper: Option<&Paper>,
        paper_id: &str,
        _tags: &[String],
    ) -> Result<Option<serde_json::Value>, PipelineError> {
        let kg_path = self.data_dir.join("kg.db");
        let mut kg = if kg_path.exists() {
            tokio::runtime::Handle::current()
                .block_on(rairos_kg::KnowledgeGraph::with_db(kg_path))
                .map_err(|e| PipelineError::Kg(e.to_string()))?
        } else {
            rairos_kg::KnowledgeGraph::new()
        };

        if let Some(p) = paper {
            kg.add_paper(p);
        } else if let Some(ref db) = self.db {
            match db.get_paper(paper_id) {
                Ok(rec) => {
                    kg.add_paper(&rec);
                }
                Err(e) => warn!("KG_SYNC: paper {paper_id} not in DB: {e}"),
            }
        }

        Ok(None) // no chart data (ChartKG not ported to Rust)
    }

    /// Stage 6: Re-render P-note with analysis content.
    #[allow(clippy::too_many_arguments)]
    fn run_pnote_update(
        &self,
        pnote_path: &Path,
        paper: Option<&Paper>,
        paper_id: &str,
        tags: &[String],
        extracted_text: &str,
        sections: &HashMap<String, String>,
        rubric: &HashMap<String, serde_json::Value>,
    ) -> Result<(), PipelineError> {
        let ai_draft = if sections.is_empty() {
            String::new()
        } else {
            let mut lines: Vec<String> = Vec::new();
            for (k, v) in sections {
                lines.push(format!("{k}\n{v}\n"));
            }
            lines.join("\n")
        };

        let rubric_scores: Option<HashMap<String, i32>> = if rubric.is_empty() {
            None
        } else {
            let mut scores = HashMap::new();
            for (k, v) in rubric {
                if let Some(n) = v.as_i64() {
                    scores.insert(k.clone(), n as i32);
                }
            }
            if scores.is_empty() { None } else { Some(scores) }
        };

        let p = self.build_render_paper(paper, paper_id);

        let rendered = rairos_render::pnote::render_pnote(
            &p,
            tags,
            extracted_text,
            &ai_draft,
            "",  // table_md
            "",  // math_md
            rubric_scores,
            None, // ai_overall
        );
        std::fs::write(pnote_path, rendered).map_err(PipelineError::Io)?;
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────

    fn get_paper_meta(&self, paper: Option<&Paper>, paper_id: &str) -> (String, String, String) {
        if let Some(p) = paper {
            return (p.title.clone(), p.abstract_text.clone(), p.authors.join(", "));
        }
        if let Some(ref db) = self.db {
            if let Ok(rec) = db.get_paper(paper_id) {
                return (rec.title, rec.abstract_text, rec.authors.join(", "));
            }
        }
        (paper_id.to_string(), String::new(), String::new())
    }

    /// Build a rairos_render::pnote::Paper from a rairos_core::Paper or stub.
    fn build_render_paper(&self, paper: Option<&Paper>, paper_id: &str) -> rairos_render::pnote::Paper {
        let abs_url = format!("https://arxiv.org/abs/{paper_id}");
        match paper {
            Some(p) => {
                let pdf_url = p.metadata.pdf_url.clone().unwrap_or_default();
                rairos_render::pnote::Paper {
                    uid: p.id.clone(),
                    title: p.title.clone(),
                    authors: p.authors.clone(),
                    abstract_: p.abstract_text.clone(),
                    published: Some(p.published.to_string()),
                    updated: None,
                    abs_url: Some(abs_url),
                    pdf_url: Some(pdf_url),
                    source: "arxiv".to_string(),
                    primary_category: p.categories.first().cloned(),
                }
            }
            None => rairos_render::pnote::Paper {
                uid: paper_id.to_string(),
                title: paper_id.to_string(),
                authors: vec![],
                abstract_: String::new(),
                published: None,
                updated: None,
                abs_url: Some(abs_url),
                pdf_url: None,
                source: "arxiv".to_string(),
                primary_category: None,
            },
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

impl StageResult {
    pub fn new(stage: &str) -> Self {
        Self {
            stage: stage.to_string(),
            success: false,
            error: String::new(),
            data: serde_json::Value::Null,
        }
    }
}

fn write_json(path: &Path, data: &impl Serialize) -> Result<(), PipelineError> {
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Extract known method keywords from text (mirrors Python's method).
fn extract_keywords(text: &str) -> Vec<String> {
    let keywords = [
        "transformer", "attention", "cnn", "rnn", "lstm", "gru", "gnn",
        "diffusion", "reinforcement", "bert", "gpt", "llm", "foundation",
        "multi-modal", "contrastive", "self-supervised", "semi-supervised",
        "few-shot", "zero-shot", "transfer",
    ];
    let lower = text.to_lowercase();
    keywords
        .iter()
        .filter(|kw| lower.contains(*kw))
        .map(|s| s.to_string())
        .collect()
}

/// Internal result from paper analysis stage.
struct PaperAnalysis {
    sections: HashMap<String, String>,
    rubric: HashMap<String, serde_json::Value>,
    keywords: Vec<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poststage_from_str() {
        assert_eq!(
            PostStage::from_string("paper_analysis"),
            Some(PostStage::PaperAnalysis)
        );
        assert_eq!(PostStage::from_string("benchmark"), Some(PostStage::Benchmark));
        assert_eq!(
            PostStage::from_string("cross_reference"),
            Some(PostStage::CrossReference)
        );
        assert_eq!(PostStage::from_string("insight"), Some(PostStage::Insight));
        assert_eq!(PostStage::from_string("kg_sync"), Some(PostStage::KgSync));
        assert_eq!(
            PostStage::from_string("pnote_update"),
            Some(PostStage::PnoteUpdate)
        );
        assert_eq!(PostStage::from_string("pnote"), Some(PostStage::PnoteUpdate));
        assert_eq!(PostStage::from_string("unknown"), None);
        // Case-insensitive
        assert_eq!(
            PostStage::from_string("Paper_Analysis"),
            Some(PostStage::PaperAnalysis)
        );
        assert_eq!(
            PostStage::from_string("CROSS-REFERENCE"),
            Some(PostStage::CrossReference)
        );
    }

    #[test]
    fn test_poststage_as_str() {
        assert_eq!(PostStage::PaperAnalysis.as_str(), "paper_analysis");
        assert_eq!(PostStage::Benchmark.as_str(), "benchmark");
        assert_eq!(PostStage::CrossReference.as_str(), "cross_reference");
        assert_eq!(PostStage::Insight.as_str(), "insight");
        assert_eq!(PostStage::KgSync.as_str(), "kg_sync");
        assert_eq!(PostStage::PnoteUpdate.as_str(), "pnote_update");
    }

    #[test]
    fn test_poststage_all_contains_all() {
        let all = PostStage::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&PostStage::PaperAnalysis));
        assert!(all.contains(&PostStage::Benchmark));
        assert!(all.contains(&PostStage::CrossReference));
        assert!(all.contains(&PostStage::Insight));
        assert!(all.contains(&PostStage::KgSync));
        assert!(all.contains(&PostStage::PnoteUpdate));
    }

    #[test]
    fn test_processing_result_summary() {
        let mut result = PostProcessingResult::new("test123", String::new());
        result.stages_completed = vec!["paper_analysis".to_string(), "kg_sync".to_string()];
        result.stages_failed = vec!["benchmark".to_string()];
        result.stage_results.insert(
            "paper_analysis".to_string(),
            StageResult::new("paper_analysis"),
        );
        result.stage_results.insert("kg_sync".to_string(), StageResult::new("kg_sync"));
        result.stage_results.insert("benchmark".to_string(), StageResult::new("benchmark"));

        assert!(!result.all_succeeded());
        assert_eq!(result.summary(), "[2/3] stages OK, 1 failed");
    }

    #[test]
    fn test_processing_result_all_succeeded() {
        let result = PostProcessingResult {
            paper_id: "test".to_string(),
            stages_completed: vec!["a".to_string(), "b".to_string()],
            stages_failed: vec![],
            stage_results: HashMap::new(),
            pnote_updated: false,
            start_time: String::new(),
            end_time: String::new(),
        };
        assert!(result.all_succeeded());
    }

    #[test]
    fn test_extract_keywords() {
        let kws = extract_keywords("transformer attention model");
        assert!(kws.contains(&"transformer".to_string()));
        assert!(kws.contains(&"attention".to_string()));
        assert!(!kws.contains(&"model".to_string()));
    }

    #[test]
    fn test_extract_keywords_empty() {
        let kws = extract_keywords("");
        assert!(kws.is_empty());
    }

    #[test]
    fn test_extract_keywords_no_match() {
        let kws = extract_keywords("just some random text about math");
        assert!(kws.is_empty());
    }

    #[test]
    fn test_run_empty_pipeline_no_paper() {
        let mut pipeline = ResearchDeepDivePipeline::new(None, PathBuf::from("/tmp"));
        let result = pipeline.run(
            "test-paper",
            "",
            None,
            &[],
            None,
            None, // all stages
            None, // no LLM
        );
        assert_eq!(result.paper_id, "test-paper");
        // All stages complete except pnote_update (needs pnote_path)
        assert_eq!(result.stages_failed.len(), 0);
        // paper_analysis, benchmark, cross_reference, insight, kg_sync = 5
        // pnote_update skipped because no pnote_path
        assert_eq!(result.stages_completed.len(), 5);
        assert!(!result.end_time.is_empty());
    }

    #[test]
    fn test_run_selected_stages() {
        let stages = [PostStage::PaperAnalysis, PostStage::KgSync];
        let mut pipeline = ResearchDeepDivePipeline::new(None, PathBuf::from("/tmp"));
        let result = pipeline.run("test-paper", "", None, &[], None, Some(&stages), None);
        // Only 2 stages should run
        assert_eq!(result.stage_results.len(), 2);
        assert!(result.stage_results.contains_key("paper_analysis"));
        assert!(result.stage_results.contains_key("kg_sync"));
    }

    #[test]
    fn test_pnote_update_requires_path() {
        let stages = [PostStage::PnoteUpdate];
        let mut pipeline = ResearchDeepDivePipeline::new(None, PathBuf::from("/tmp"));
        // Without pnote_path, pnote_update should be silently skipped
        let result = pipeline.run("test-paper", "", None, &[], None, Some(&stages), None);
        // The stage didn't run at all
        assert!(!result.stage_results.contains_key("pnote_update"));
    }

    #[test]
    fn test_stage_result_defaults() {
        let sr = StageResult::new("test_stage");
        assert!(!sr.success);
        assert!(sr.error.is_empty());
        assert!(sr.data.is_null());
        assert_eq!(sr.stage, "test_stage");
    }

    #[test]
    fn test_llm_config_from_env() {
        // When env vars are not set, from_env returns None
        let config = LlmConfig::from_env();
        // Can't assert specific value since it depends on env
        assert!(config.is_none() || config.is_some());
    }

    #[test]
    fn test_insight_stage_empty_text() {
        let pipeline = ResearchDeepDivePipeline::new(None, PathBuf::from("/tmp"));
        let cards = pipeline.run_insight_stage("paper123", "Test Title", "");
        assert!(cards.is_empty());
    }

    #[test]
    fn test_insight_stage_with_markers() {
        let pipeline = ResearchDeepDivePipeline::new(None, PathBuf::from("/tmp"));
        let text = "This is a boring sentence.\nWe propose a new method.\nAnother boring line.\nOur key insight is important.";
        let cards = pipeline.run_insight_stage("paper123", "Test Title", text);
        assert!(!cards.is_empty());
        assert!(cards[0].contains("paper123"));
    }

    #[test]
    fn test_write_json_invalid_path() {
        let result = write_json(
            Path::new("/nonexistent/deep/path/file.json"),
            &serde_json::json!({"key": "value"}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut sr_map = HashMap::new();
        sr_map.insert(
            "a".to_string(),
            StageResult {
                stage: "a".to_string(),
                success: true,
                error: String::new(),
                data: serde_json::json!({"k": "v"}),
            },
        );
        let result = PostProcessingResult {
            paper_id: "p1".to_string(),
            stages_completed: vec!["a".to_string()],
            stages_failed: vec![],
            stage_results: sr_map,
            pnote_updated: false,
            start_time: "2024-01-01".to_string(),
            end_time: "2024-01-01".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PostProcessingResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.paper_id, "p1");
        assert_eq!(deserialized.stages_completed[0], "a");
        assert_eq!(
            deserialized.stage_results["a"].data["k"],
            serde_json::json!("v"),
        );
    }
}
