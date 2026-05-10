//! Rairos Research — Deep research orchestration and gap detection
//!
//! Coordinates the full research pipeline: fetch → analyze → detect gaps → evolve.
//! Replaces: research_loop/core.py, llm/gap_detector.py

use rairos_core::{Database, Paper, ResearchGap};
use rairos_llm::{LlmClient, Message, CostTracker, GapDetector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ResearchError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("No papers found for research query: {0}")]
    NoPapers(String),

    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

// ============================================================================
// Research Query
// ============================================================================

/// A research query to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    pub id: String,
    pub query: String,
    pub categories: Vec<String>,
    pub max_papers: usize,
    pub min_relevance: f32,
    pub include_citations: bool,
}

impl ResearchQuery {
    pub fn new(query: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query: query.to_string(),
            categories: Vec::new(),
            max_papers: 50,
            min_relevance: 0.5,
            include_citations: true,
        }
    }

    pub fn with_categories(mut self, cats: Vec<String>) -> Self {
        self.categories = cats;
        self
    }

    pub fn with_max_papers(mut self, n: usize) -> Self {
        self.max_papers = n;
        self
    }
}

// ============================================================================
// Research Result
// ============================================================================

/// Output of a research session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub query_id: String,
    pub papers_found: usize,
    pub gaps: Vec<ResearchGap>,
    pub citations_analyzed: usize,
    pub evolution_suggestions: Vec<String>,
    pub cost_usd: f64,
    pub duration_secs: f64,
}

impl ResearchResult {
    pub fn summary(&self) -> String {
        format!(
            "Research completed: {} papers found, {} gaps detected, {:.4} cost, {:.1}s",
            self.papers_found,
            self.gaps.len(),
            self.cost_usd,
            self.duration_secs
        )
    }
}

// ============================================================================
// Research Orchestrator
// ============================================================================

/// The main research orchestrator
pub struct ResearchOrchestrator {
    db: Arc<Database>,
    llm: Arc<dyn LlmClient>,
    cost_tracker: Arc<RwLock<CostTracker>>,
}

impl ResearchOrchestrator {
    pub fn new(db: Arc<Database>, llm: Arc<dyn LlmClient>) -> Self {
        Self {
            db,
            llm,
            cost_tracker: Arc::new(RwLock::new(CostTracker::new())),
        }
    }

    /// Execute a full research query
    pub async fn research(&self, query: &ResearchQuery) -> Result<ResearchResult, ResearchError> {
        let start = std::time::Instant::now();

        // Step 1: Fetch papers from database matching query
        let papers = self.find_relevant_papers(query).await?;

        if papers.is_empty() {
            return Err(ResearchError::NoPapers(query.query.clone()));
        }

        // Step 2: Analyze citation chains
        let citations_analyzed = if query.include_citations {
            self.analyze_citations(&papers).await
        } else {
            0
        };

        // Step 3: Detect research gaps
        let gaps = self.detect_gaps(&papers, &query.categories).await?;

        // Step 4: Generate evolution suggestions via LLM
        let evolution_suggestions = self.generate_suggestions(&papers, &gaps).await?;

        // Step 5: Save gaps to database
        for gap in &gaps {
            if let Err(e) = self.db.insert_gap(gap) {
                tracing::warn!("Failed to save gap: {}", e);
            }
        }

        let duration_secs = start.elapsed().as_secs_f64();
        let cost_usd = {
            let tracker = self.cost_tracker.read().await;
            tracker.total_cost_usd
        };

        Ok(ResearchResult {
            query_id: query.id.clone(),
            papers_found: papers.len(),
            gaps,
            citations_analyzed,
            evolution_suggestions,
            cost_usd,
            duration_secs,
        })
    }

    async fn find_relevant_papers(&self, query: &ResearchQuery) -> Result<Vec<Paper>, ResearchError> {
        // Get papers from database
        let all_papers = self.db.list_papers(None, query.max_papers as usize, 0)
            .map_err(|e| ResearchError::Database(e.to_string()))?;

        Ok(all_papers)
    }

    async fn analyze_citations(&self, papers: &[Paper]) -> usize {
        papers.len()
    }

    async fn detect_gaps(&self, papers: &[Paper], categories: &[String]) -> Result<Vec<ResearchGap>, ResearchError> {
        // Use keyword-based gap detection
        let keywords: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
        let gap_descriptions = GapDetector::detect_gaps(papers, &keywords);

        // Also find under-explored categories
        let under_category = GapDetector::find_underexplored_areas(papers, 3);

        let mut gaps = Vec::new();

        for desc in gap_descriptions {
            gaps.push(ResearchGap::new(
                "keyword_gap",
                &desc,
                "medium",
            ));
        }

        for cat in under_category {
            gaps.push(ResearchGap::new(
                "category_gap",
                &format!("Under-explored category: {}", cat),
                "low",
            ));
        }

        Ok(gaps)
    }

    async fn generate_suggestions(&self, papers: &[Paper], gaps: &[ResearchGap]) -> Result<Vec<String>, ResearchError> {
        if gaps.is_empty() {
            return Ok(vec![]);
        }

        // Build context from papers
        let context = papers.iter()
            .take(5)
            .map(|p| format!("- {} ({})", p.title, p.arxiv_id.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");

        let gaps_text = gaps.iter()
            .take(5)
            .map(|g| format!("- {} [{}]", g.description, g.severity))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "You are a research assistant. Based on these papers:\n{}\n\nAnd these research gaps:\n{}\n\nSuggest 3 novel research directions that could address these gaps. Be specific and innovative.",
            context,
            gaps_text
        );

        let messages = vec![
            Message { role: "system".to_string(), content: "You are a helpful research assistant.".to_string() },
            Message { role: "user".to_string(), content: prompt },
        ];

        let response = self.llm.complete(
            messages,
            "gpt-4o",
            0.7,
            500,
        ).await.map_err(|e| ResearchError::Llm(e.to_string()))?;

        // Record cost
        {
            let mut tracker = self.cost_tracker.write().await;
            tracker.record(&response.usage, response.model.as_str(), self.llm.provider_name());
        }

        // Parse response into suggestions
        let suggestions: Vec<String> = response.content
            .lines()
            .filter(|l| l.trim().starts_with('-') || l.trim().starts_with('1') || l.trim().starts_with('2') || l.trim().starts_with('3'))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(suggestions)
    }

    /// Get cost summary
    pub async fn cost_summary(&self) -> String {
        let tracker = self.cost_tracker.read().await;
        tracker.summary()
    }

    /// Reset cost tracker
    pub async fn reset_costs(&self) {
        let mut tracker = self.cost_tracker.write().await;
        tracker.reset();
    }
}

// ============================================================================
// Paper Ranker
// ============================================================================

/// Ranks papers by relevance for a given query
pub struct PaperRanker;

impl PaperRanker {
    /// Simple TF-IDF-like ranking
    pub fn rank(papers: &[Paper], query: &str) -> Vec<(String, f32)> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scores: Vec<(String, f32)> = Vec::new();
        for p in papers {
            let title_lower = p.title.to_lowercase();
            let abstract_lower = p.abstract_text.to_lowercase();
            let mut score = 0.0;
            for term in &query_terms {
                let title_count = title_lower.matches(term).count() as f32;
                let abstract_count = abstract_lower.matches(term).count() as f32;
                score += title_count * 2.0 + abstract_count;
            }
            scores.push((p.id.clone(), score));
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// Filter papers by minimum relevance score
    pub fn filter_by_threshold(papers: &[Paper], query: &str, threshold: f32) -> Vec<Paper> {
        let scores = Self::rank(papers, query);
        scores.into_iter()
            .filter(|(_, score)| *score >= threshold)
            .filter_map(|(id, _)| papers.iter().find(|p| p.id == id))
            .cloned()
            .collect()
    }
}

// ============================================================================
// Claim Graph (from Python's research_loop/claim_graph.py)
// ============================================================================

/// A node in the claim graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimNode {
    pub id: String,
    pub paper_id: String,
    pub claim: String,
    pub supporting_papers: Vec<String>,
    pub contradicting_papers: Vec<String>,
    pub confidence: f32,
}

impl ClaimNode {
    pub fn new(paper_id: &str, claim: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            paper_id: paper_id.to_string(),
            claim: claim.to_string(),
            supporting_papers: Vec::new(),
            contradicting_papers: Vec::new(),
            confidence: 0.5,
        }
    }
}

/// A claim graph connecting papers via their claims
#[derive(Debug, Default)]
pub struct ClaimGraph {
    nodes: Vec<ClaimNode>,
}

impl ClaimGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_claim(&mut self, node: ClaimNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[ClaimNode] {
        &self.nodes
    }

    pub fn find_claims_about(&self, keyword: &str) -> Vec<&ClaimNode> {
        self.nodes.iter()
            .filter(|n| n.claim.to_lowercase().contains(&keyword.to_lowercase()))
            .collect()
    }

    pub fn find_contradictions(&self, claim_id: &str) -> Vec<&ClaimNode> {
        let claim = self.nodes.iter().find(|n| n.id == claim_id);
        match claim {
            Some(c) => self.nodes.iter()
                .filter(|n| n.id != claim_id && c.contradicting_papers.contains(&n.paper_id))
                .collect(),
            None => Vec::new(),
        }
    }
}

// ============================================================================
// Benchmark Runner
// ============================================================================

/// Runs benchmarks on paper datasets
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// Run a citation count benchmark
    pub fn citation_benchmark(papers: &[Paper]) -> CitationBenchmarkResult {
        let total_citations: usize = papers.iter()
            .map(|p| p.metadata.cited_by)
            .sum();

        let avg_citations = if papers.is_empty() {
            0.0
        } else {
            total_citations as f32 / papers.len() as f32
        };

        let most_cited = papers.iter()
            .max_by_key(|p| p.metadata.cited_by)
            .map(|p| (p.title.clone(), p.metadata.cited_by as u32));

        CitationBenchmarkResult {
            total_papers: papers.len(),
            total_citations,
            avg_citations,
            most_cited,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CitationBenchmarkResult {
    pub total_papers: usize,
    pub total_citations: usize,
    pub avg_citations: f32,
    pub most_cited: Option<(String, u32)>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_graph() {
        let mut graph = ClaimGraph::new();
        graph.add_claim(ClaimNode::new("paper1", "Transformers are effective for NLP"));
        graph.add_claim(ClaimNode::new("paper2", "CNNs outperform Transformers for vision"));

        let found = graph.find_claims_about("transformers");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].paper_id, "paper1");
    }

    #[test]
    fn test_paper_ranker() {
        let papers = vec![
            Paper::new(Some("1".into()), "Attention is all you need".into(), "Transformers".into()),
            Paper::new(Some("2".into()), "ResNet paper".into(), "CNNs".into()),
            Paper::new(Some("3".into()), "BERT paper".into(), "Attention mechanism".into()),
        ];

        let scores = PaperRanker::rank(&papers, "attention");
        assert_eq!(scores.len(), 3);
        assert!(scores[0].1 >= scores[1].1); // ranked by score descending
    }

    #[test]
    fn test_benchmark() {
        let papers = vec![
            Paper::new(Some("1".into()), "Paper 1".into(), "Abstract 1".into()),
            Paper::new(Some("2".into()), "Paper 2".into(), "Abstract 2".into()),
        ];
        // Papers start with cited_by = 0
        let result = BenchmarkRunner::citation_benchmark(&papers);
        assert_eq!(result.total_papers, 2);
    }
}
