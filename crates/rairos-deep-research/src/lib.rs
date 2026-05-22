//! Deep Research Agent — iterative research with gap detection and archetype-aware refinement.

#![allow(clippy::default_constructed_unit_structs)]
#![allow(dead_code)]
//!
//! Architecture inspired by:
//! - gpt-researcher: multi-agent research with planning
//! - deer-flow: sandbox + memory + tool use
//! - snapstate: session persistence for pause/resume

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use parking_lot::RwLock;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum DeepResearchError {
    #[error("Session not initialized")]
    NoSession,
    #[error("Database error: {0}")]
    Database(String),
    #[error("MCP tool error: {0}")]
    McpTool(String),
    #[error("Search failed: {0}")]
    SearchFailed(String),
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Checkpoint error: {0}")]
    Checkpoint(String),
    #[error("Signal handler error: {0}")]
    Signal(String),
}

// ============================================================================
// Data Structures
// ============================================================================

/// A single reasoning step in the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThought {
    pub iteration: usize,
    pub role: String,
    pub content: String,
    pub timestamp: f64,
}

impl AgentThought {
    pub fn new(iteration: usize, role: &str, content: &str) -> Self {
        Self {
            iteration,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        }
    }
}

/// Final result of a deep research agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub session_id: String,
    pub query: String,
    pub iterations: usize,
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub thoughts: Vec<AgentThought>,
    pub report: String,
    pub duration_seconds: f64,
    pub status: String,
}

/// Paper snapshot for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSnapshot {
    pub arxiv_id: String,
    pub title: String,
    pub abstract_text: String,
    pub url: String,
    pub extracted_text: String,
    #[serde(default)]
    pub gaps_found: usize,
}

/// Gap snapshot for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSnapshot {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub matched_papers: Vec<String>,
    #[serde(default)]
    pub archetype_match: f64,
    #[serde(default)]
    pub accepted: bool,
}

/// Research session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    pub session_id: String,
    pub query: String,
    pub iteration: usize,
    pub max_iterations: usize,
    pub status: String,
    pub archetype: HashMap<String, f64>,
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub findings: Vec<String>,
    pub search_history: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ResearchSession {
    pub fn new(session_id: &str, query: &str, max_iterations: usize) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.to_string(),
            query: query.to_string(),
            iteration: 0,
            max_iterations,
            status: "active".to_string(),
            archetype: HashMap::new(),
            papers: Vec::new(),
            gaps: Vec::new(),
            findings: Vec::new(),
            search_history: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Paper data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub uid: String,
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub source: String,
    pub pdf_url: String,
    pub published: String,
    pub updated: String,
    pub abs_url: String,
    #[serde(default)]
    pub primary_category: Option<String>,
    #[serde(default)]
    pub categories: Option<String>,
    /// References cited by this paper
    #[serde(default)]
    pub references: Vec<String>,
    /// Citing papers (populated during citation chain analysis)
    #[serde(default)]
    pub cited_by: Vec<String>,
}

// ============================================================================
// Adaptive Query Strategy
// ============================================================================

/// Adaptive query planning: evolve search strategy based on gap coverage.
pub struct AdaptiveQueryStrategy {
    topic: String,
    /// query → list of gap_type found
    query_gap_types: HashMap<String, Vec<String>>,
    /// gap_type → how many times it appeared
    gap_type_counts: HashMap<String, usize>,
    total_gaps: usize,
}

impl AdaptiveQueryStrategy {
    const ALL_GAP_TYPES: &'static [&'static str] = &[
        "capability",
        "improvement",
        "contradiction",
        "assumption",
        "extension",
        "baseline_gap",
        "evaluation_gap",
        "reproducibility_gap",
        "embodied_planning",
        "rl_pretraining",
        "scaling_laws",
        "reasoning",
    ];

    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            query_gap_types: HashMap::new(),
            gap_type_counts: HashMap::new(),
            total_gaps: 0,
        }
    }

    /// Record gap types found from a search result.
    pub fn record_search_result(&mut self, query: &str, gaps: &[GapSnapshot]) {
        if gaps.is_empty() {
            return;
        }
        let mut found_types = HashSet::new();
        for g in gaps {
            let gt = &g.gap_type;
            *self.gap_type_counts.entry(gt.clone()).or_insert(0) += 1;
            self.total_gaps += 1;
            found_types.insert(gt.clone());
        }
        self.query_gap_types
            .insert(query.to_string(), found_types.into_iter().collect());
    }

    /// Return coverage ratio for each gap type (0.0–1.0).
    pub fn gap_type_coverage(&self) -> HashMap<String, f64> {
        if self.total_gaps == 0 {
            return Self::ALL_GAP_TYPES
                .iter()
                .map(|&gt| (gt.to_string(), 0.0))
                .collect();
        }
        let mut result: HashMap<String, f64> = self
            .gap_type_counts
            .iter()
            .map(|(gt, &count)| (gt.clone(), count as f64 / self.total_gaps as f64))
            .collect();
        for &gt in Self::ALL_GAP_TYPES {
            result.entry(gt.to_string()).or_insert(0.0);
        }
        result
    }

    /// Return gap types that appear in < threshold of all gaps.
    pub fn under_represented_types(&self, threshold: f64) -> Vec<String> {
        self.gap_type_coverage()
            .into_iter()
            .filter(|(_, ratio)| *ratio > 0.0 && *ratio < threshold)
            .map(|(gt, _)| gt)
            .collect()
    }

    /// Return queries that produced the most diverse gap types.
    pub fn most_productive_queries(&self, top_k: usize) -> Vec<String> {
        let mut scored: Vec<(String, usize)> = self
            .query_gap_types
            .iter()
            .map(|(q, types)| (q.clone(), types.len()))
            .collect();
        scored.sort_by_key(|x| std::cmp::Reverse(x.1));
        scored.into_iter().take(top_k).map(|(q, _)| q).collect()
    }

    /// Build next search query adaptively.
    pub fn build_adaptive_query(
        &self,
        iteration: usize,
        latest_gap_title: &str,
        latest_gap_type: &str,
        gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let under_rep = self.under_represented_types(0.15);

        if iteration == 0 {
            return self.topic.clone();
        }

        // Case 1: have under-represented types → target them
        if let Some(target) = under_rep.first() {
            let productive = self.most_productive_queries(1);
            let base = productive
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&self.topic);
            return format!("{} {}", base, target);
        }

        // Case 2: high-confidence GenePool hint
        if !gene_pool_hint.is_empty() && confidence >= 0.4 {
            if !latest_gap_title.is_empty() {
                return format!("{} {}", gene_pool_hint, latest_gap_title);
            }
            return gene_pool_hint.to_string();
        }

        // Case 3: latest gap context
        match latest_gap_type {
            "Contradiction" => format!("{} {} disagreement", self.topic, latest_gap_title),
            "improvement" | "capability" | "extension" | "Missing" | "Unknown" | "" => {
                format!("{} {} improvement", self.topic, latest_gap_title)
            }
            _ => {
                if !latest_gap_title.is_empty() {
                    format!("{} {}", self.topic, latest_gap_title)
                } else {
                    self.topic.clone()
                }
            }
        }
    }

    /// Simple word-overlap similarity between two queries (0.0–1.0).
    pub fn query_similarity(&self, q1: &str, q2: &str) -> f64 {
        let words1: HashSet<String> = q1
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let words2: HashSet<String> = q2
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }
        let intersection: HashSet<_> = words1.intersection(&words2).collect();
        let union: HashSet<_> = words1.union(&words2).collect();
        intersection.len() as f64 / union.len() as f64
    }
}

// ============================================================================
// Gap Analyzer (Pattern-based)
// ============================================================================

/// Gap detection patterns for rule-based analysis
struct GapPattern {
    gap_type: &'static str,
    label: &'static str,
    patterns: &'static [&'static str],
}

const GAP_PATTERNS: &[GapPattern] = &[
    GapPattern {
        gap_type: "method_limitation",
        label: "Method Limitation",
        patterns: &[
            "limitation", "drawback", "however", "but ", "not suitable",
            "not efficient", "poor performance", "high latency", "high cost",
            "low accuracy", "not scalable", "bottleneck",
        ],
    },
    GapPattern {
        gap_type: "unexplored_application",
        label: "Unexplored Application",
        patterns: &[
            "future work", "open question", "not explore", "remains unexplored",
            "beyond the scope", "left for future", "not covered", "out of scope",
        ],
    },
    GapPattern {
        gap_type: "contradiction",
        label: "Contradiction",
        patterns: &[
            "inconsistent", "contradict", "debate", "disagree",
            "conflicting", "opposing", "mixed results",
        ],
    },
    GapPattern {
        gap_type: "evaluation_gap",
        label: "Evaluation Gap",
        patterns: &[
            "no benchmark", "lack evaluation", "not compare", "no standard",
            "not evaluated", "no metric", "hard to evaluate",
        ],
    },
    GapPattern {
        gap_type: "scalability_issue",
        label: "Scalability Issue",
        patterns: &[
            "scalab", "large scale", "computational cost", "memory footprint",
            "not efficient", "complexity", "expensive", "resource intensive",
        ],
    },
    GapPattern {
        gap_type: "dataset_gap",
        label: "Dataset Gap",
        patterns: &[
            "dataset lack", "no data", "limited data", "small dataset",
            "not enough data", "data scarcity", "lack of dataset",
        ],
    },
];

/// Gap analyzer using pattern matching
pub struct GapAnalyzerV2;

impl Default for GapAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl GapAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        topic: &str,
        _use_insights: bool,
        _min_papers: usize,
        _use_llm: bool,
    ) -> GapAnalysisResult {
        let text = topic.to_lowercase();
        let mut gaps = Vec::new();

        for pattern in GAP_PATTERNS {
            for pat in pattern.patterns {
                if text.contains(&pat.to_lowercase()) {
                    gaps.push(Gap {
                        gap_type: GapType::Improvement,
                        title: pattern.label.to_string(),
                        description: format!("Found '{}' in topic: {}", pat, topic),
                    });
                    break;
                }
            }
        }

        GapAnalysisResult { gaps }
    }
}

pub struct GapAnalysisResult {
    pub gaps: Vec<Gap>,
}

pub struct Gap {
    pub gap_type: GapType,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GapType {
    Capability,
    Improvement,
    Contradiction,
    Assumption,
    Extension,
    BaselineGap,
    EvaluationGap,
    ReproducibilityGap,
    EmbodiedPlanning,
    RlPretraining,
    ScalingLaws,
    Reasoning,
    Unknown,
}

impl GapType {
    pub fn value(&self) -> &str {
        match self {
            GapType::Capability => "capability",
            GapType::Improvement => "improvement",
            GapType::Contradiction => "contradiction",
            GapType::Assumption => "assumption",
            GapType::Extension => "extension",
            GapType::BaselineGap => "baseline_gap",
            GapType::EvaluationGap => "evaluation_gap",
            GapType::ReproducibilityGap => "reproducibility_gap",
            GapType::EmbodiedPlanning => "embodied_planning",
            GapType::RlPretraining => "rl_pretraining",
            GapType::ScalingLaws => "scaling_laws",
            GapType::Reasoning => "reasoning",
            GapType::Unknown => "unknown",
        }
    }
}

// ============================================================================
// Citation Chain Analyzer
// ============================================================================

/// A citation relationship between two papers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationEdge {
    pub from: String,  // citing paper
    pub to: String,    // cited paper
    pub context: String, // surrounding text mentioning the citation
}

/// A citation chain for traversing paper relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationChain {
    /// All papers in the chain
    pub papers: Vec<Paper>,
    /// Citation edges (from -> to)
    pub edges: Vec<CitationEdge>,
    /// Paper UID -> index in papers vector
    paper_index: std::collections::HashMap<String, usize>,
}

impl CitationChain {
    /// Create a new empty citation chain.
    pub fn new() -> Self {
        Self {
            papers: Vec::new(),
            edges: Vec::new(),
            paper_index: std::collections::HashMap::new(),
        }
    }

    /// Add a paper to the chain.
    pub fn add_paper(&mut self, paper: Paper) {
        let uid = paper.uid.clone();
        if !self.paper_index.contains_key(&uid) {
            let idx = self.papers.len();
            self.paper_index.insert(uid.clone(), idx);
            self.papers.push(paper);
        }
    }

    /// Add a citation edge.
    pub fn add_citation(&mut self, from: &str, to: &str, context: &str) {
        // Ensure both papers exist in the chain
        if !self.paper_index.contains_key(from) {
            return;
        }
        if !self.paper_index.contains_key(to) {
            return;
        }
        self.edges.push(CitationEdge {
            from: from.to_string(),
            to: to.to_string(),
            context: context.to_string(),
        });
    }

    /// Get papers that a given paper cites.
    pub fn get_references(&self, paper_uid: &str) -> Vec<&Paper> {
        let Some(&idx) = self.paper_index.get(paper_uid) else {
            return Vec::new();
        };
        self.edges
            .iter()
            .filter(|e| e.from == paper_uid)
            .filter_map(|e| self.paper_index.get(&e.to))
            .filter_map(|&idx| self.papers.get(idx))
            .collect()
    }

    /// Get papers that cite a given paper.
    pub fn get_cited_by(&self, paper_uid: &str) -> Vec<&Paper> {
        let Some(&idx) = self.paper_index.get(paper_uid) else {
            return Vec::new();
        };
        self.edges
            .iter()
            .filter(|e| e.to == paper_uid)
            .filter_map(|e| self.paper_index.get(&e.from))
            .filter_map(|&idx| self.papers.get(idx))
            .collect()
    }

    /// Find the shortest path between two papers via citations.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        use std::collections::{HashSet, VecDeque};

        if !self.paper_index.contains_key(from) || !self.paper_index.contains_key(to) {
            return None;
        }

        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        queue.push_back((from.to_string(), vec![from.to_string()]));
        visited.insert(from.to_string());

        while let Some((current, path)) = queue.pop_front() {
            if current == to {
                return Some(path);
            }

            for cited in self.get_references(&current) {
                let uid = &cited.uid;
                if !visited.contains(uid) {
                    visited.insert(uid.clone());
                    let mut new_path = path.clone();
                    new_path.push(uid.clone());
                    queue.push_back((uid.clone(), new_path));
                }
            }
        }

        None
    }

    /// Get citation depth (how many papers deep does this paper influence).
    pub fn citation_depth(&self, paper_uid: &str) -> usize {
        use std::collections::{HashSet, VecDeque};

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((paper_uid.to_string(), 0));
        visited.insert(paper_uid.to_string());

        let mut max_depth = 0;

        while let Some((current, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);
            for cited in self.get_references(&current) {
                if !visited.contains(&cited.uid) {
                    visited.insert(cited.uid.clone());
                    queue.push_back((cited.uid.clone(), depth + 1));
                }
            }
        }

        max_depth
    }
}

impl Default for CitationChain {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory Context for Agents
// ============================================================================

/// A memory entry in the agent's context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub timestamp: f64,
    pub importance: f64,
    pub tags: Vec<String>,
    /// References to paper UIDs related to this memory
    #[serde(default)]
    pub related_papers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Fact learned from a paper
    Fact,
    /// Gap identified during research
    Gap,
    /// Research finding or conclusion
    Finding,
    /// Question to explore further
    Question,
    /// User preference or instruction
    Preference,
    /// Error or failure encountered
    Error,
}

impl MemoryEntry {
    pub fn new(content: &str, memory_type: MemoryType) -> Self {
        Self {
            id: uuid_v4(),
            content: content.to_string(),
            memory_type,
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
            importance: 0.5,
            tags: Vec::new(),
            related_papers: Vec::new(),
        }
    }

    /// Create a fact memory.
    pub fn fact(content: &str) -> Self {
        Self::new(content, MemoryType::Fact)
    }

    /// Create a gap memory.
    pub fn gap(content: &str) -> Self {
        Self::new(content, MemoryType::Gap)
    }

    /// Create a finding memory.
    pub fn finding(content: &str) -> Self {
        Self::new(content, MemoryType::Finding)
    }

    /// Create a question memory.
    pub fn question(content: &str) -> Self {
        Self::new(content, MemoryType::Question)
    }

    /// Set importance score (0.0 to 1.0).
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add tags to this memory.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add related paper.
    pub fn with_paper(mut self, paper_uid: &str) -> Self {
        self.related_papers.push(paper_uid.to_string());
        self
    }
}

/// Simple UUID generator (minimal implementation).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        now.as_secs() as u32,
        (now.subsec_nanos() >> 16) as u16,
        rand_u16() & 0x0fff,
        (rand_u16() & 0x3fff) | 0x8000,
        now.as_nanos()
    )
}

fn rand_u16() -> u16 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish() as u16
}

/// Memory context for the research agent.
///
/// Provides semantic memory storage and retrieval, allowing the agent
/// to remember facts, gaps, and findings across research sessions.
pub struct MemoryContext {
    /// All memory entries
    entries: Vec<MemoryEntry>,
    /// Index for fast lookup by type
    type_index: std::collections::HashMap<MemoryType, Vec<usize>>,
    /// Tag inverted index
    tag_index: std::collections::HashMap<String, Vec<usize>>,
}

impl MemoryContext {
    /// Create a new empty memory context.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            type_index: std::collections::HashMap::new(),
            tag_index: std::collections::HashMap::new(),
        }
    }

    /// Add a memory entry.
    pub fn add(&mut self, entry: MemoryEntry) {
        let idx = self.entries.len();
        self.entries.push(entry.clone());

        // Index by type
        self.type_index
            .entry(entry.memory_type.clone())
            .or_default()
            .push(idx);

        // Index by tags
        for tag in &entry.tags {
            self.tag_index.entry(tag.clone()).or_default().push(idx);
        }
    }

    /// Add a fact memory.
    pub fn add_fact(&mut self, content: &str) {
        self.add(MemoryEntry::fact(content));
    }

    /// Add a gap memory.
    pub fn add_gap(&mut self, content: &str) {
        self.add(MemoryEntry::gap(content));
    }

    /// Add a finding memory.
    pub fn add_finding(&mut self, content: &str) {
        self.add(MemoryEntry::finding(content));
    }

    /// Add a question memory.
    pub fn add_question(&mut self, content: &str) {
        self.add(MemoryEntry::question(content));
    }

    /// Get all memories of a specific type.
    pub fn get_by_type(&self, memory_type: &MemoryType) -> Vec<&MemoryEntry> {
        self.type_index
            .get(memory_type)
            .map(|indices| indices.iter().filter_map(|&i| self.entries.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get all facts.
    pub fn facts(&self) -> Vec<&MemoryEntry> {
        self.get_by_type(&MemoryType::Fact)
    }

    /// Get all gaps.
    pub fn gaps(&self) -> Vec<&MemoryEntry> {
        self.get_by_type(&MemoryType::Gap)
    }

    /// Get all findings.
    pub fn findings(&self) -> Vec<&MemoryEntry> {
        self.get_by_type(&MemoryType::Finding)
    }

    /// Get all questions.
    pub fn questions(&self) -> Vec<&MemoryEntry> {
        self.get_by_type(&MemoryType::Question)
    }

    /// Get memories by tag.
    pub fn get_by_tag(&self, tag: &str) -> Vec<&MemoryEntry> {
        self.tag_index
            .get(tag)
            .map(|indices| indices.iter().filter_map(|&i| self.entries.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get memories related to a paper.
    pub fn get_by_paper(&self, paper_uid: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.related_papers.contains(&paper_uid.to_string()))
            .collect()
    }

    /// Semantic search over memories using keyword matching.
    ///
    /// Returns memories whose content contains any of the query keywords,
    /// sorted by importance and recency.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&MemoryEntry> {
        let keywords: HashSet<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|s| s.to_string())
            .collect();

        if keywords.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(&MemoryEntry, f64)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let match_count: usize = keywords
                    .iter()
                    .filter(|kw| content_lower.contains(*kw))
                    .count();

                if match_count > 0 {
                    // Score = keyword matches * importance + recency bonus
                    let recency = (entry.timestamp - oldest_timestamp(&self.entries))
                        .max(1.0)
                        .log(1.0 + entry.timestamp)
                        .min(1.0);
                    let score = (match_count as f64) * entry.importance + recency * 0.1;
                    Some((entry, score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(limit).map(|(e, _)| e).collect()
    }

    /// Get recent memories, sorted by timestamp descending.
    pub fn recent(&self, limit: usize) -> Vec<&MemoryEntry> {
        let mut sorted = self.entries.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.timestamp.partial_cmp(&a.timestamp).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(limit).collect()
    }

    /// Get most important memories.
    pub fn important(&self, limit: usize) -> Vec<&MemoryEntry> {
        let mut sorted = self.entries.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(limit).collect()
    }

    /// Get memory count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Merge another memory context into this one (avoiding duplicates by id).
    pub fn merge(&mut self, other: &MemoryContext) {
        let existing_ids: HashSet<String> = self.entries.iter().map(|e| e.id.clone()).collect();
        for entry in &other.entries {
            if !existing_ids.contains(&entry.id) {
                self.add(entry.clone());
            }
        }
    }

    /// Clear all memories.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.type_index.clear();
        self.tag_index.clear();
    }

    /// Export memories as JSON string.
    pub fn export(&self) -> String {
        serde_json::to_string_pretty(&self.entries).unwrap_or_default()
    }

    /// Import memories from JSON string.
    pub fn import(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let entries: Vec<MemoryEntry> = serde_json::from_str(json)?;
        for entry in entries {
            self.add(entry);
        }
        Ok(())
    }
}

impl Default for MemoryContext {
    fn default() -> Self {
        Self::new()
    }
}

fn oldest_timestamp(entries: &[MemoryEntry]) -> f64 {
    entries
        .iter()
        .map(|e| e.timestamp)
        .fold(f64::MAX, |a, b| a.min(b))
}

// ============================================================================
// MCP Tool Types
// ============================================================================

/// MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({}),
        }
    }
}

/// MCP tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub papers: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Progress Tracking
// ============================================================================

/// Progress tracking for observability.
#[derive(Debug, Default)]
pub struct Progress {
    pub papers_found: usize,
    pub papers_extracted: usize,
    pub gaps_found: usize,
    pub searches_done: usize,
    pub iterations_done: usize,
}

// ============================================================================
// Deep Research Agent Configuration
// ============================================================================

/// Configuration for the Deep Research Agent.
#[derive(Debug, Clone)]
pub struct DeepResearchConfig {
    pub query: String,
    pub max_iterations: usize,
    pub max_papers_per_iteration: usize,
    pub verbose: bool,
    pub mode: String,
    pub auto_checkpoint: bool,
    pub checkpoint_every_n_steps: usize,
    pub checkpoint_interval_seconds: u64,
    pub use_streaming_reasoning: bool,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            query: String::new(),
            max_iterations: 3,
            max_papers_per_iteration: 5,
            verbose: false,
            mode: "agent".to_string(),
            auto_checkpoint: true,
            checkpoint_every_n_steps: 1,
            checkpoint_interval_seconds: 60,
            use_streaming_reasoning: false,
        }
    }
}

// ============================================================================
// Deep Research Agent
// ============================================================================

/// Deep Research Agent with iterative gap-aware refinement loop.
///
/// Loop:
/// 1. PLANNER: decide search strategy based on gaps found
/// 2. SEARCHER: fetch papers from arXiv
/// 3. EXTRACTOR: pull abstracts/full text
/// 4. ANALYZER: detect gaps via GapAnalyzerV2
/// 5. REFLECTOR: assess progress, decide to iterate or stop
/// 6. GENETIC: encode accepted gaps into Gene Pool
pub struct DeepResearchAgent {
    config: DeepResearchConfig,
    session: Option<ResearchSession>,
    thoughts: Vec<AgentThought>,
    gap_analyzer: GapAnalyzerV2,
    adaptive_strategy: RwLock<AdaptiveQueryStrategy>,
    progress: Progress,
    stop_requested: AtomicBool,
    checkpoint_counter: AtomicUsize,
    last_checkpoint_time: RwLock<f64>,
    citation_chain: RwLock<CitationChain>,
    memory: RwLock<MemoryContext>,
}

impl DeepResearchAgent {
    /// Create a new DeepResearchAgent with configuration.
    pub fn new(config: DeepResearchConfig) -> Self {
        Self {
            config,
            session: None,
            thoughts: Vec::new(),
            gap_analyzer: GapAnalyzerV2::new(),
            adaptive_strategy: RwLock::new(AdaptiveQueryStrategy::new("")),
            progress: Progress::default(),
            stop_requested: AtomicBool::new(false),
            checkpoint_counter: AtomicUsize::new(0),
            last_checkpoint_time: RwLock::new(0.0),
            citation_chain: RwLock::new(CitationChain::new()),
            memory: RwLock::new(MemoryContext::new()),
        }
    }

    /// Create with default configuration.
    pub fn with_query(query: &str) -> Self {
        let config = DeepResearchConfig { query: query.to_string(), ..Default::default() };
        Self::new(config)
    }

    // -------------------------------------------------------------------------
    // Session lifecycle
    // -------------------------------------------------------------------------

    /// Start a new research session.
    pub fn start(&mut self) -> Result<ResearchSession, DeepResearchError> {
        let session_id = generate_session_id();
        let strategy = AdaptiveQueryStrategy::new(&self.config.query);

        *self.adaptive_strategy.write() = strategy;

        let session =
            ResearchSession::new(&session_id, &self.config.query, self.config.max_iterations);

        self.session = Some(session.clone());
        Ok(session)
    }

    /// Resume an existing session.
    #[allow(dead_code)]
    pub fn resume(
        &mut self,
        _session_id: &str,
    ) -> Result<Option<ResearchSession>, DeepResearchError> {
        // In a real implementation, this would load from snapstate
        Ok(self.session.clone())
    }

    /// Pause and persist current session state.
    #[allow(dead_code)]
    pub fn pause(&mut self) -> Result<(), DeepResearchError> {
        if let Some(ref mut session) = self.session {
            session.status = "paused".to_string();
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Thought recording
    // -------------------------------------------------------------------------

    fn record_thought(&mut self, role: &str, content: &str, iteration: usize) {
        let thought = AgentThought::new(iteration, role, content);
        self.thoughts.push(thought.clone());

        if let Some(ref mut session) = self.session {
            session
                .findings
                .push(format!("[{}] {}", role.to_uppercase(), content));
        }

        if self.config.verbose {
            println!(
                "[DeepResearchAgent] [{}] iter{} | {}",
                role.to_uppercase(),
                iteration,
                content
            );
        }
    }

    // -------------------------------------------------------------------------
    // Core iteration steps
    // -------------------------------------------------------------------------

    /// Get search guidance based on gap analysis.
    #[allow(dead_code)]
    fn get_search_guidance(
        &self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
    ) -> (Option<String>, f64) {
        let hint = match gap_type {
            t if t.contains("method_limitation") => {
                Some(format!("{} improvements beyond current limitations", topic))
            }
            t if t.contains("unexplored_application") => {
                Some(format!("{} applications in new domains", topic))
            }
            t if t.contains("evaluation_gap") => {
                Some(format!("{} benchmarks and evaluation", topic))
            }
            t if t.contains("scalability") => {
                Some(format!("{} at scale", topic))
            }
            t if t.contains("dataset") => {
                Some(format!("{} datasets and data collection", topic))
            }
            _ => Some(format!("{} {}", topic, gap_title)),
        };

        (hint, 0.7)
    }

    /// PLANNER: decide next search query using adaptive strategy + GenePool.
    fn plan_next_search(&self, iteration: usize) -> String {
        let gaps = self
            .session
            .as_ref()
            .map(|s| s.gaps.clone())
            .unwrap_or_default();
        let search_history = self
            .session
            .as_ref()
            .map(|s| s.search_history.clone())
            .unwrap_or_default();

        let planned = if iteration == 0 {
            self.config.query.clone()
        } else if let Some(latest_gap) = gaps.last() {
            // Get GenePool guidance
            let (hint, confidence) = self.get_search_guidance(
                &self.config.query,
                &latest_gap.gap_type,
                &latest_gap.title,
            );

            // Use adaptive strategy to build query
            let strategy = self.adaptive_strategy.read();
            let planned = strategy.build_adaptive_query(
                iteration,
                &latest_gap.title,
                &latest_gap.gap_type,
                hint.as_deref().unwrap_or(""),
                confidence,
            );

            // Semantic deduplication
            for prev_q in &search_history {
                let sim = strategy.query_similarity(&planned, prev_q);
                if sim > 0.75 {
                    return format!("{} variant{}", planned, iteration);
                }
            }

            planned
        } else {
            self.config.query.clone()
        };

        // Fallback duplicate guard
        if search_history.contains(&planned) {
            return format!("{} exploration{}", self.config.query, iteration);
        }

        planned
    }

    /// SEARCHER: fetch papers from arXiv API.
    fn search_papers(&self, search_query: &str, _iteration: usize) -> Vec<Paper> {
        let max_results = self.config.max_papers_per_iteration;
        if max_results == 0 {
            return Vec::new();
        }

        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            urlencode(search_query),
            max_results,
        );

        tracing::debug!("[DeepResearchAgent] Searching arXiv: {}", url);

        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Rairos/0.1")
            .danger_accept_invalid_certs(false)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[DeepResearchAgent] Failed to create HTTP client: {}", e);
                return Vec::new();
            }
        };

        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("[DeepResearchAgent] arXiv search failed: {}", e);
                return Vec::new();
            }
        };

        tracing::debug!("[DeepResearchAgent] arXiv status: {}", resp.status());

        let text = match resp.text() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[DeepResearchAgent] Failed to read arXiv response: {}", e);
                return Vec::new();
            }
        };

        tracing::debug!("[DeepResearchAgent] arXiv response length: {} bytes", text.len());

        let papers = parse_arxiv_atom(&text, max_results, search_query);
        tracing::debug!("[DeepResearchAgent] Parsed {} papers from arXiv", papers.len());
        papers
    }

    /// EXTRACTOR: extract text from papers and build snapshots.
    #[allow(dead_code)]
    fn extract_papers(&self, papers: &[Paper], _iteration: usize) -> Vec<PaperSnapshot> {
        papers
            .iter()
            .map(|p| PaperSnapshot {
                arxiv_id: p.uid.clone(),
                title: p.title.clone(),
                abstract_text: p.abstract_text.clone(),
                url: p.pdf_url.clone(),
                extracted_text: String::new(), // TODO: extract from PDF
                gaps_found: 0,
            })
            .collect()
    }

    /// ANALYZER: detect research gaps using pattern matching.
    #[allow(dead_code)]
    fn analyze_gaps(&self, snapshots: &[PaperSnapshot], _iteration: usize) -> Vec<GapSnapshot> {
        if snapshots.is_empty() {
            return Vec::new();
        }

        let topic = snapshots
            .iter()
            .map(|s| s.title.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let gap_result = self.gap_analyzer.analyze(&topic, false, snapshots.len(), false);

        gap_result
            .gaps
            .into_iter()
            .map(|gap| {
                let title_lower = gap.title.to_lowercase();
                let matched: Vec<String> = snapshots
                    .iter()
                    .filter(|s| {
                        let abstract_lower = s.abstract_text.to_lowercase();
                        abstract_lower.contains(&title_lower)
                    })
                    .take(3)
                    .map(|s| s.arxiv_id.clone())
                    .collect();
                GapSnapshot {
                    gap_type: format!("{:?}", gap.gap_type),
                    title: gap.title,
                    description: gap.description,
                    matched_papers: matched,
                    archetype_match: 0.5,
                    accepted: false,
                }
            })
            .collect()
    }

    /// REFLECTOR: decide whether to continue iterating or stop.
    fn reflect(&self, iteration: usize) -> (bool, String) {
        let Some(ref session) = self.session else {
            return (false, "no session".to_string());
        };

        let gaps = &session.gaps;
        let papers = &session.papers;

        // Stop conditions
        if iteration >= self.config.max_iterations {
            return (
                false,
                format!("max iterations ({}) reached", self.config.max_iterations),
            );
        }

        if papers.len() >= self.config.max_iterations * self.config.max_papers_per_iteration {
            return (false, "max papers reached".to_string());
        }

        if gaps.is_empty() && iteration > 1 {
            return (false, "no gaps found after thorough search".to_string());
        }

        // Continue conditions
        let recent_gaps: Vec<_> = gaps.iter().filter(|g| g.accepted).collect();
        if !recent_gaps.is_empty() {
            return (
                false,
                format!("{} gaps accepted, stopping", recent_gaps.len()),
            );
        }

        // Check archetype alignment
        if !gaps.is_empty() {
            let avg_match: f64 =
                gaps.iter().map(|g| g.archetype_match).sum::<f64>() / gaps.len() as f64;
            if avg_match < 0.3 && iteration >= 2 {
                // Broadening search would happen on next iteration
            }
        }

        (true, "continue iterating".to_string())
    }

    /// GENETIC: encode all accepted gaps into the Gene Pool.
    #[allow(dead_code)]
    fn encode_accepted_gaps(&self) -> usize {
        let Some(ref session) = self.session else {
            return 0;
        };

        let accepted: Vec<_> = session.gaps.iter().filter(|g| g.accepted).collect();
        let count = accepted.len();

        if count > 0 {
            tracing::info!(
                "[DeepResearchAgent] Encoding {} accepted gaps into GenePool",
                count
            );
            for gap in &accepted {
                tracing::debug!(
                    "[DeepResearchAgent] Accepted gap: {} - {}",
                    gap.gap_type,
                    gap.title
                );
            }
        }

        count
    }

    // -------------------------------------------------------------------------
    // Citation Chain
    // -------------------------------------------------------------------------

    /// Build citation chain from collected papers.
    ///
    /// This extracts citation relationships based on shared references
    /// and builds a traversable citation graph.
    #[allow(dead_code)]
    pub fn build_citation_chain(&self) {
        let Some(ref session) = self.session else {
            return;
        };

        let mut chain = CitationChain::new();

        // Add all papers to the chain
        for snapshot in &session.papers {
            let paper = Paper {
                uid: snapshot.arxiv_id.clone(),
                title: snapshot.title.clone(),
                abstract_text: snapshot.abstract_text.clone(),
                authors: Vec::new(),
                source: "arxiv".to_string(),
                pdf_url: snapshot.url.clone(),
                published: String::new(),
                updated: String::new(),
                abs_url: format!("https://arxiv.org/abs/{}", snapshot.arxiv_id),
                primary_category: None,
                categories: None,
                references: Vec::new(),
                cited_by: Vec::new(),
            };
            chain.add_paper(paper);
        }

        // Build edges based on abstract similarity (papers in same niche cite each other)
        for i in 0..session.papers.len() {
            for j in 0..session.papers.len() {
                if i != j {
                    let paper_i = &session.papers[i];
                    let paper_j = &session.papers[j];

                    // Check if paper_i's title keywords appear in paper_j's abstract
                    // (indicating paper_i likely cites paper_j)
                    let title_words: HashSet<String> = paper_i
                        .title
                        .to_lowercase()
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();

                    let abstract_lower = paper_j.abstract_text.to_lowercase();
                    let overlap: Vec<&String> = title_words
                        .iter()
                        .filter(|w| w.len() > 4 && abstract_lower.contains(*w))
                        .collect();

                    if overlap.len() >= 2 {
                        chain.add_citation(
                            &paper_i.arxiv_id,
                            &paper_j.arxiv_id,
                            &format!("Shared topic: {}", overlap.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
                        );
                    }
                }
            }
        }

        *self.citation_chain.write() = chain;
    }

    /// Get papers related to a given paper through citation chain.
    #[allow(dead_code)]
    pub fn get_related_papers(&self, paper_uid: &str, depth: usize) -> Vec<String> {
        let chain = self.citation_chain.read();
        let mut related = Vec::new();

        // Get direct references
        for paper in chain.get_references(paper_uid) {
            related.push(paper.uid.clone());
        }

        // Get direct citations
        for paper in chain.get_cited_by(paper_uid) {
            related.push(paper.uid.clone());
        }

        if depth > 1 {
            // Get transitive references
            let mut visited: HashSet<String> = related.iter().cloned().collect();
            let mut queue: VecDeque<String> = related.iter().cloned().collect();

            let mut current_depth = 1;
            while current_depth < depth && !queue.is_empty() {
                let level_size = queue.len();
                for _ in 0..level_size {
                    if let Some(uid) = queue.pop_front() {
                        for paper in chain.get_references(&uid) {
                            if !visited.contains(&paper.uid) {
                                visited.insert(paper.uid.clone());
                                related.push(paper.uid.clone());
                                queue.push_back(paper.uid.clone());
                            }
                        }
                    }
                }
                current_depth += 1;
            }
        }

        related
    }

    /// Get the citation path between two papers if it exists.
    #[allow(dead_code)]
    pub fn get_citation_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        self.citation_chain.read().find_path(from, to)
    }

    /// Get papers sorted by citation influence (most cited first).
    #[allow(dead_code)]
    pub fn get_influential_papers(&self) -> Vec<(String, usize)> {
        let chain = self.citation_chain.read();
        let mut influence: Vec<(String, usize)> = chain
            .papers
            .iter()
            .map(|p| {
                let depth = chain.citation_depth(&p.uid);
                (p.uid.clone(), depth)
            })
            .collect();
        influence.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        influence
    }

    // -------------------------------------------------------------------------
    // Memory Context Methods
    // -------------------------------------------------------------------------

    /// Add a fact to memory.
    #[allow(dead_code)]
    pub fn remember_fact(&self, content: &str, paper_uid: Option<&str>) {
        let mut entry = MemoryEntry::fact(content);
        if let Some(uid) = paper_uid {
            entry = entry.with_paper(uid);
        }
        self.memory.write().add(entry);
    }

    /// Add a finding to memory.
    #[allow(dead_code)]
    pub fn remember_finding(&self, content: &str, paper_uid: Option<&str>) {
        let mut entry = MemoryEntry::finding(content);
        if let Some(uid) = paper_uid {
            entry = entry.with_paper(uid);
        }
        self.memory.write().add(entry);
    }

    /// Add a gap to memory.
    #[allow(dead_code)]
    pub fn remember_gap(&self, content: &str, paper_uid: Option<&str>) {
        let mut entry = MemoryEntry::gap(content);
        if let Some(uid) = paper_uid {
            entry = entry.with_paper(uid);
        }
        self.memory.write().add(entry);
    }

    /// Add a question to memory.
    #[allow(dead_code)]
    pub fn remember_question(&self, content: &str) {
        self.memory.write().add(MemoryEntry::question(content));
    }

    /// Search memory for relevant entries.
    #[allow(dead_code)]
    pub fn recall(&self, query: &str, limit: usize) -> Vec<String> {
        self.memory
            .read()
            .search(query, limit)
            .iter()
            .map(|e| e.content.clone())
            .collect()
    }

    /// Get all facts from memory.
    #[allow(dead_code)]
    pub fn get_facts(&self) -> Vec<String> {
        self.memory.read().facts().iter().map(|e| e.content.clone()).collect()
    }

    /// Get all findings from memory.
    #[allow(dead_code)]
    pub fn get_findings(&self) -> Vec<String> {
        self.memory.read().findings().iter().map(|e| e.content.clone()).collect()
    }

    /// Get all gaps from memory.
    #[allow(dead_code)]
    pub fn get_memory_gaps(&self) -> Vec<String> {
        self.memory.read().gaps().iter().map(|e| e.content.clone()).collect()
    }

    /// Get recent memories.
    #[allow(dead_code)]
    pub fn get_recent_memories(&self, limit: usize) -> Vec<String> {
        self.memory.read().recent(limit).iter().map(|e| e.content.clone()).collect()
    }

    /// Get memories related to a specific paper.
    #[allow(dead_code)]
    pub fn get_memories_for_paper(&self, paper_uid: &str) -> Vec<String> {
        self.memory
            .read()
            .get_by_paper(paper_uid)
            .iter()
            .map(|e| e.content.clone())
            .collect()
    }

    /// Store key findings from papers into memory.
    #[allow(dead_code)]
    pub fn store_paper_findings(&self) {
        let Some(ref session) = self.session else {
            return;
        };

        let mut memory = self.memory.write();

        for paper in &session.papers {
            // Store title and abstract as a finding
            memory.add_finding(&format!(
                "Paper {}: {} - {}",
                paper.arxiv_id, paper.title, paper.abstract_text
            ));
        }

        // Store gaps as memory entries
        for gap in &session.gaps {
            memory.add_gap(&format!("[{}] {}: {}", gap.gap_type, gap.title, gap.description));
        }
    }

    /// Export memory context.
    #[allow(dead_code)]
    pub fn export_memory(&self) -> String {
        self.memory.read().export()
    }

    /// Import memory context.
    #[allow(dead_code)]
    pub fn import_memory(&self, json: &str) -> Result<(), serde_json::Error> {
        self.memory.write().import(json)
    }

    // -------------------------------------------------------------------------
    // Checkpointing
    // -------------------------------------------------------------------------

    /// Create checkpoint of current session state.
    #[allow(dead_code)]
    pub fn checkpoint(&self) -> Result<String, DeepResearchError> {
        let session = self.session.as_ref().ok_or(DeepResearchError::NoSession)?;
        serde_json::to_string(session).map_err(|e| DeepResearchError::Checkpoint(e.to_string()))
    }

    // -------------------------------------------------------------------------
    // Report building
    // -------------------------------------------------------------------------

    fn build_report(&self) -> String {
        let Some(ref session) = self.session else {
            return "No session".to_string();
        };

        let mut lines = vec![
            format!("# Deep Research Report: {}", session.query),
            String::new(),
            format!(
                "**Session**: {} | **Iterations**: {} | **Status**: {}",
                session.session_id, session.iteration, session.status
            ),
            String::new(),
            "## Papers Analyzed".to_string(),
        ];

        for p in &session.papers {
            lines.push(format!("- [{}] {}", p.arxiv_id, p.title));
        }

        lines.push(String::new());
        lines.push("## Research Gaps".to_string());

        for g in &session.gaps {
            let status = if g.accepted { "✅" } else { "⬜" };
            lines.push(format!("- {} [{}] {}", status, g.gap_type, g.title));
            if !g.description.is_empty() {
                lines.push(format!(
                    "  {}",
                    &g.description[..g.description.len().min(100)]
                ));
            }
        }

        lines.push(String::new());
        lines.push("## Findings".to_string());

        for f in session.findings.iter().rev().take(10) {
            lines.push(format!("- {}", f));
        }

        lines.join("\n")
    }

    // -------------------------------------------------------------------------
    // Main run loop
    // -------------------------------------------------------------------------

    /// Run the deep research agent synchronously.
    pub fn run(&mut self) -> Result<DeepResearchResult, DeepResearchError> {
        if self.session.is_none() {
            self.start()?;
        }

        let start_time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut iteration = self
            .session
            .as_ref()
            .ok_or(DeepResearchError::NoSession)?
            .iteration;

        self.record_thought(
            "planner",
            &format!(
                "topic={:?}, max_iter={}",
                self.config.query, self.config.max_iterations
            ),
            iteration,
        );

        while iteration < self.config.max_iterations && !self.stop_requested.load(Ordering::SeqCst)
        {
            if let Some(ref mut session) = self.session {
                session.iteration = iteration;
            }

            // Step 1: Plan
            let search_query = self.plan_next_search(iteration);
            self.record_thought(
                "planner",
                &format!("Planned search: {}", search_query),
                iteration,
            );

            // Step 2: Search
            if self.config.verbose {
                println!("[DR] [SEARCH] q={:?}", search_query);
            }

            let papers = self.search_papers(&search_query, iteration);

            let has_papers = !papers.is_empty();
            if !has_papers {
                // Fallback to topic query
                let fallback = self.search_papers(&self.config.query, iteration);
                if fallback.is_empty() {
                    iteration += 1;
                    continue;
                }
            }

            if let Some(ref mut session) = self.session {
                session.search_history.push(search_query);
            }

            // Step 3: Extract
            if self.config.verbose {
                println!("[DR] [EXTRACT] {} papers", papers.len());
            }

            let snapshots = self.extract_papers(&papers, iteration);

            if let Some(ref mut session) = self.session {
                session.papers.extend(snapshots.clone());
            }

            // Step 4: Analyze gaps
            if self.config.verbose {
                println!("[DR] [ANALYZE] {} snapshots", snapshots.len());
            }

            let gap_snapshots = self.analyze_gaps(&snapshots, iteration);

            if let Some(ref mut session) = self.session {
                session.gaps.extend(gap_snapshots.clone());
            }

            // Step 5: Reflect
            let (should_continue, reason) = self.reflect(iteration);
            self.record_thought("reflector", &reason, iteration);

            if self.config.verbose {
                println!(
                    "[DeepResearchAgent] [iter {}] Reflect: {}",
                    iteration, reason
                );
            }

            if !should_continue {
                break;
            }

            iteration += 1;

            if let Some(ref mut session) = self.session {
                session.updated_at = Utc::now();
            }
        }

        // Encode accepted gaps into Gene Pool
        self.encode_accepted_gaps();

        // Build citation chain for paper relationships
        self.build_citation_chain();

        // Finalize session
        let duration = chrono::Utc::now().timestamp_millis() as f64 / 1000.0 - start_time;

        if let Some(ref mut session) = self.session {
            session.status = if self.stop_requested.load(Ordering::SeqCst) {
                "paused".to_string()
            } else {
                "completed".to_string()
            };
            session.iteration = iteration;
        }

        let result = DeepResearchResult {
            session_id: self
                .session
                .as_ref()
                .map(|s| s.session_id.clone())
                .unwrap_or_default(),
            query: self.config.query.clone(),
            iterations: iteration + 1,
            papers: self
                .session
                .as_ref()
                .map(|s| s.papers.clone())
                .unwrap_or_default(),
            gaps: self
                .session
                .as_ref()
                .map(|s| s.gaps.clone())
                .unwrap_or_default(),
            thoughts: self.thoughts.clone(),
            report: self.build_report(),
            duration_seconds: duration,
            status: self
                .session
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "failed".to_string()),
        };

        if self.config.verbose {
            self.print_summary(&result);
        }

        Ok(result)
    }

    #[allow(dead_code)]
    fn print_summary(&self, result: &DeepResearchResult) {
        let papers = result.papers.len();
        let gaps = result.gaps.len();
        let accepted = result.gaps.iter().filter(|g| g.accepted).count();
        let duration = result.duration_seconds;

        println!();
        println!("{}", "=".repeat(60));
        println!("  DeepResearch Complete — {}", result.status.to_uppercase());
        println!("  Iterations : {}", result.iterations);
        println!("  Papers     : {} found, {} extracted", papers, papers);
        println!("  Gaps       : {} found, {} accepted", gaps, accepted);
        println!("  Duration   : {:.1}s", duration);
        println!("{}", "=".repeat(60));
    }

    /// Request the agent to stop at next reflection point.
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.stop_requested.store(true, Ordering::SeqCst);
        if let Some(ref mut session) = self.session {
            session.status = "paused".to_string();
        }
    }

    /// Get current session.
    #[allow(dead_code)]
    pub fn get_session(&self) -> Option<&ResearchSession> {
        self.session.as_ref()
    }

    /// Get thoughts recorded so far.
    #[allow(dead_code)]
    pub fn get_thoughts(&self) -> &[AgentThought] {
        &self.thoughts
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("session_{}_{}", duration.as_secs(), duration.subsec_nanos())
}

// ─── arXiv Atom XML Parser ───────────────────────────────────────────────────

/// URL-encode a query string for arXiv API.
fn urlencode(s: &str) -> String {
    s.replace(' ', "+")
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '_' | '.' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Parse an arXiv Atom XML response into Paper structs.
fn parse_arxiv_atom(xml: &str, max_results: usize, _query: &str) -> Vec<Paper> {
    let mut papers = Vec::new();

    // Simple XML parser — extract entry sections
    for entry in xml.split("<entry>").skip(1) {
        if papers.len() >= max_results {
            break;
        }

        let uid = extract_xml_tag(entry, "id")
            .map(|s| s.trim().trim_start_matches("http://arxiv.org/abs/").trim_start_matches("https://arxiv.org/abs/").to_string())
            .unwrap_or_default();

        let title = extract_xml_tag(entry, "title")
            .map(|s| s.trim().replace('\n', " ").trim().to_string())
            .unwrap_or_default();

        let abstract_text = extract_xml_tag(entry, "summary")
            .map(|s| s.trim().replace('\n', " ").trim().to_string())
            .unwrap_or_default();

        let published = extract_xml_tag(entry, "published").unwrap_or_default();

        let updated = extract_xml_tag(entry, "updated").unwrap_or_default();

        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", uid);
        let abs_url = format!("https://arxiv.org/abs/{}", uid);

        // Extract authors
        let mut authors = Vec::new();
        for author_entry in entry.split("<author>").skip(1) {
            if let Some(name) = extract_xml_tag(author_entry, "name") {
                authors.push(name.trim().to_string());
            }
        }

        // Extract categories
        let mut categories = Vec::new();
        for cat in entry.split("<category") {
            if let Some(term) = extract_attr(cat, "term") {
                categories.push(term);
            }
        }

        let primary_category = categories.first().cloned();

        papers.push(Paper {
            uid,
            title,
            abstract_text,
            authors,
            source: "arxiv".to_string(),
            pdf_url,
            published,
            updated,
            abs_url,
            primary_category,
            categories: Some(categories.join(",")),
            references: Vec::new(),
            cited_by: Vec::new(),
        });
    }

    papers
}

/// Extract text content of an XML tag.
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)?;
    let content_start = xml[start..].find('>')? + start + 1;
    let end = xml[content_start..].find(&close)?;
    Some(xml[content_start..content_start + end].to_string())
}

/// Extract an attribute value from an XML tag fragment.
fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    let start = xml.find(&search)?;
    let value_start = start + search.len();
    let end = xml[value_start..].find('"')?;
    Some(xml[value_start..value_start + end].to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_query_strategy_new() {
        let strategy = AdaptiveQueryStrategy::new("machine learning");
        assert_eq!(strategy.topic, "machine learning");
    }

    #[test]
    fn test_adaptive_query_strategy_gap_coverage() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let coverage = strategy.gap_type_coverage();
        assert!(coverage.values().all(|&v| v == 0.0));
    }

    #[test]
    fn test_adaptive_query_strategy_under_represented() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let under_rep = strategy.under_represented_types(0.15);
        // When no gaps, should be empty
        assert!(under_rep.is_empty());
    }

    #[test]
    fn test_query_similarity() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let sim = strategy.query_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < f64::EPSILON);

        let sim2 = strategy.query_similarity("hello world", "world hello");
        assert!((sim2 - 1.0).abs() < f64::EPSILON);

        let sim3 = strategy.query_similarity("hello", "world");
        assert!(sim3 < 0.5);
    }

    #[test]
    fn test_build_adaptive_query_iteration_0() {
        let strategy = AdaptiveQueryStrategy::new("machine learning");
        let query = strategy.build_adaptive_query(0, "", "", "", 0.0);
        assert_eq!(query, "machine learning");
    }

    #[test]
    fn test_build_adaptive_query_with_under_represented() {
        let mut strategy = AdaptiveQueryStrategy::new("machine learning");
        // Add some gaps to make capability under-represented
        strategy.record_search_result(
            "MLP training",
            &[GapSnapshot {
                gap_type: "improvement".to_string(),
                title: "test".to_string(),
                description: "".to_string(),
                matched_papers: vec![],
                archetype_match: 0.5,
                accepted: false,
            }],
        );
        let query = strategy.build_adaptive_query(1, "Neural architecture", "improvement", "", 0.0);
        assert!(query.contains("improvement"));
    }

    #[test]
    fn test_deep_research_agent_new() {
        let config = DeepResearchConfig {
            query: "test query".to_string(),
            max_iterations: 3,
            ..Default::default()
        };
        let agent = DeepResearchAgent::new(config);
        assert_eq!(agent.config.query, "test query");
        assert_eq!(agent.config.max_iterations, 3);
    }

    #[test]
    fn test_agent_thought_new() {
        let thought = AgentThought::new(1, "planner", "test content");
        assert_eq!(thought.iteration, 1);
        assert_eq!(thought.role, "planner");
        assert_eq!(thought.content, "test content");
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("session_"));
    }

    #[test]
    fn test_gap_analyzer_new() {
        let analyzer = GapAnalyzerV2::new();
        let result = analyzer.analyze("test", false, 3, false);
        assert!(result.gaps.is_empty());
    }

    #[test]
    fn test_paper_snapshot_serde() {
        let snapshot = PaperSnapshot {
            arxiv_id: "2301.00001".to_string(),
            title: "Test Paper".to_string(),
            abstract_text: "Abstract".to_string(),
            url: "https://arxiv.org/abs/2301.00001".to_string(),
            extracted_text: "Full text".to_string(),
            gaps_found: 2,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: PaperSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.arxiv_id, snapshot.arxiv_id);
        assert_eq!(deserialized.title, snapshot.title);
    }

    #[test]
    fn test_gap_snapshot_serde() {
        let gap = GapSnapshot {
            gap_type: "improvement".to_string(),
            title: "Test Gap".to_string(),
            description: "Description".to_string(),
            matched_papers: vec!["2301.00001".to_string()],
            archetype_match: 0.7,
            accepted: true,
        };
        let json = serde_json::to_string(&gap).unwrap();
        let deserialized: GapSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.gap_type, gap.gap_type);
        assert_eq!(deserialized.archetype_match, gap.archetype_match);
    }
}
