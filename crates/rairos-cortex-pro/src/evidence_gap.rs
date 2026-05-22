//! Evidence Gap Tracker — tracks evidence collection progress for research questions.
//!
//! Based on MemR3 (arxiv:2512.20237) which introduces:
//! - Router that selects among retrieve, reflect, and answer actions
//! - Global evidence-gap tracker that makes the answering process transparent
//!
//! This module tracks what evidence has been collected vs. what gaps remain,
//! enabling the agent to know when it has enough evidence to answer.

// ============================================================================
// Data Structures
// ============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a research question or sub-question being investigated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    /// Unique query ID
    pub id: String,
    /// The original question
    pub question: String,
    /// When the query was created
    pub created_at: DateTime<Utc>,
    /// Current status
    pub status: QueryStatus,
    /// Evidence items collected for this query
    pub evidence: Vec<EvidenceItem>,
    /// Sub-questions derived from this query
    pub sub_questions: Vec<String>,
    /// Confidence score (0.0 - 1.0) based on evidence
    pub confidence: f32,
}

impl ResearchQuery {
    /// Create a new research query.
    pub fn new(id: &str, question: &str) -> Self {
        Self {
            id: id.to_string(),
            question: question.to_string(),
            created_at: Utc::now(),
            status: QueryStatus::Open,
            evidence: Vec::new(),
            sub_questions: Vec::new(),
            confidence: 0.0,
        }
    }

    /// Add evidence to this query.
    pub fn add_evidence(&mut self, evidence: EvidenceItem) {
        self.evidence.push(evidence);
        self.update_confidence();
    }

    /// Update confidence based on evidence.
    fn update_confidence(&mut self) {
        if self.evidence.is_empty() {
            self.confidence = 0.0;
            return;
        }

        // Confidence increases with evidence count and quality
        let evidence_score: f32 = self.evidence.iter()
            .map(|e| e.quality * e.relevance)
            .sum::<f32>() / self.evidence.len() as f32;

        // Also factor in recency
        let now = Utc::now();
        let freshness: f32 = self.evidence.iter()
            .map(|e| {
                let age_secs = (now - e.collected_at).num_seconds() as f32;
                // Decay factor: 1.0 for recent, 0.5 for 1 hour old
                (1.0 - (age_secs / 3600.0).min(0.5))
            })
            .sum::<f32>() / self.evidence.len() as f32;

        self.confidence = (evidence_score * 0.7 + freshness * 0.3).min(1.0);
    }

    /// Check if query is ready to answer.
    pub fn is_ready(&self) -> bool {
        self.confidence >= 0.6 || self.evidence.len() >= 3
    }
}

/// Status of a research query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    /// Initial state, no evidence collected
    Open,
    /// Actively collecting evidence
    Investigating,
    /// Evidence being synthesized
    Synthesizing,
    /// Sufficient evidence gathered
    Ready,
    /// Query has been answered
    Answered,
    /// Query abandoned or unanswerable
    Abandoned,
}

impl std::fmt::Display for QueryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryStatus::Open => write!(f, "Open"),
            QueryStatus::Investigating => write!(f, "Investigating"),
            QueryStatus::Synthesizing => write!(f, "Synthesizing"),
            QueryStatus::Ready => write!(f, "Ready"),
            QueryStatus::Answered => write!(f, "Answered"),
            QueryStatus::Abandoned => write!(f, "Abandoned"),
        }
    }
}

/// An evidence item collected for a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Unique evidence ID
    pub id: String,
    /// Source of the evidence (paper ID, URL, etc.)
    pub source_id: String,
    /// Source name/title for display
    pub source_name: String,
    /// The actual evidence content
    pub content: String,
    /// Relevance to the query (0.0 - 1.0)
    pub relevance: f32,
    /// Quality of the evidence (0.0 - 1.0)
    pub quality: f32,
    /// When this evidence was collected
    pub collected_at: DateTime<Utc>,
    /// Evidence type
    pub evidence_type: EvidenceType,
}

/// Type of evidence collected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Direct answer to the query
    Direct,
    /// Supporting evidence
    Supporting,
    /// Contradicting evidence
    Contradicting,
    /// Contextual information
    Contextual,
}

impl EvidenceItem {
    /// Create a new evidence item.
    pub fn new(
        id: &str,
        source_id: &str,
        source_name: &str,
        content: &str,
        evidence_type: EvidenceType,
    ) -> Self {
        Self {
            id: id.to_string(),
            source_id: source_id.to_string(),
            source_name: source_name.to_string(),
            content: content.to_string(),
            relevance: 0.5,
            quality: 0.5,
            collected_at: Utc::now(),
            evidence_type,
        }
    }

    /// Set relevance score.
    pub fn with_relevance(mut self, relevance: f32) -> Self {
        self.relevance = relevance.clamp(0.0, 1.0);
        self
    }

    /// Set quality score.
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 1.0);
        self
    }
}

/// The action recommended by the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouterAction {
    /// Need to retrieve more evidence
    Retrieve,
    /// Need to reflect on current evidence
    Reflect,
    /// Ready to generate answer
    Answer,
    /// Insufficient evidence, need more investigation
    Investigate,
}

impl std::fmt::Display for RouterAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterAction::Retrieve => write!(f, "Retrieve"),
            RouterAction::Reflect => write!(f, "Reflect"),
            RouterAction::Answer => write!(f, "Answer"),
            RouterAction::Investigate => write!(f, "Investigate"),
        }
    }
}

/// Evidence Gap Tracker — main orchestrator for evidence-centered research.
// ============================================================================

/// Evidence Gap Tracker following the MemR3 approach.
///
/// Tracks research queries, collects evidence, identifies gaps,
/// and recommends actions via a router.
#[derive(Debug, Clone)]
pub struct EvidenceGapTracker {
    /// All research queries being tracked
    queries: Vec<ResearchQuery>,
    /// Maximum queries to track
    max_queries: usize,
    /// Evidence collection history
    collection_history: Vec<CollectionEvent>,
    /// Query ID generator
    next_query_id: usize,
}

impl Default for EvidenceGapTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EvidenceGapTracker {
    /// Create a new evidence gap tracker.
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            max_queries: 100,
            collection_history: Vec::new(),
            next_query_id: 1,
        }
    }

    /// Create a new research query.
    pub fn create_query(&mut self, question: &str) -> &ResearchQuery {
        let id = format!("q{:04}", self.next_query_id);
        self.next_query_id += 1;

        let query = ResearchQuery::new(&id, question);
        self.queries.push(query);

        // Trim if over max
        if self.queries.len() > self.max_queries {
            self.queries.remove(0);
        }

        self.queries.last().unwrap()
    }

    /// Add evidence to a query.
    pub fn add_evidence(
        &mut self,
        query_id: &str,
        evidence: EvidenceItem,
    ) -> Option<&ResearchQuery> {
        let query = self.queries.iter_mut().find(|q| q.id == query_id)?;
        query.add_evidence(evidence.clone());

        // Record collection event
        self.collection_history.push(CollectionEvent {
            query_id: query_id.to_string(),
            action: CollectionAction::AddEvidence,
            evidence_id: Some(evidence.id),
            timestamp: Utc::now(),
        });

        Some(query)
    }

    /// Get a query by ID.
    pub fn get_query(&self, query_id: &str) -> Option<&ResearchQuery> {
        self.queries.iter().find(|q| q.id == query_id)
    }

    /// Get a mutable query by ID.
    pub fn get_query_mut(&mut self, query_id: &str) -> Option<&mut ResearchQuery> {
        self.queries.iter_mut().find(|q| q.id == query_id)
    }

    /// Get all open queries.
    pub fn get_open_queries(&self) -> Vec<&ResearchQuery> {
        self.queries
            .iter()
            .filter(|q| matches!(q.status, QueryStatus::Open | QueryStatus::Investigating))
            .collect()
    }

    /// Get queries that need more evidence.
    pub fn get_gaps(&self) -> Vec<GapInfo> {
        self.queries
            .iter()
            .filter(|q| !q.is_ready())
            .map(|q| {
                let gap_description = if q.evidence.is_empty() {
                    "No evidence collected yet".to_string()
                } else {
                    format!(
                        "Confidence {:.0}% - need {} more evidence items",
                        q.confidence * 100.0,
                        (3 - q.evidence.len()).max(0)
                    )
                };

                GapInfo {
                    query_id: q.id.clone(),
                    question: q.question.clone(),
                    current_confidence: q.confidence,
                    evidence_count: q.evidence.len(),
                    gap_description,
                }
            })
            .collect()
    }

    /// Decide the next action for a query using the router logic.
    ///
    /// Based on MemR3's router that selects among retrieve, reflect, and answer.
    pub fn decide_action(&self, query_id: &str) -> Option<RouterAction> {
        let query = self.get_query(query_id)?;

        // Decision tree based on evidence and confidence
        if query.evidence.is_empty() {
            return Some(RouterAction::Retrieve);
        }

        if query.confidence >= 0.8 {
            return Some(RouterAction::Answer);
        }

        if query.confidence >= 0.5 && query.evidence.len() >= 2 {
            return Some(RouterAction::Reflect);
        }

        if query.confidence < 0.3 {
            return Some(RouterAction::Investigate);
        }

        Some(RouterAction::Retrieve)
    }

    /// Get the next recommended action across all queries.
    pub fn get_next_global_action(&self) -> Option<(String, RouterAction)> {
        // Prioritize by confidence (lowest first)
        let mut sorted: Vec<_> = self.queries.iter()
            .filter(|q| !matches!(q.status, QueryStatus::Answered | QueryStatus::Abandoned))
            .collect();
        sorted.sort_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());

        sorted.first().and_then(|q| {
            self.decide_action(&q.id).map(|action| (q.id.clone(), action))
        })
    }

    /// Mark a query as answered.
    pub fn mark_answered(&mut self, query_id: &str) {
        if let Some(query) = self.get_query_mut(query_id) {
            query.status = QueryStatus::Answered;
        }
        self.collection_history.push(CollectionEvent {
            query_id: query_id.to_string(),
            action: CollectionAction::MarkAnswered,
            evidence_id: None,
            timestamp: Utc::now(),
        });
    }

    /// Get evidence summary for a query.
    pub fn get_evidence_summary(&self, query_id: &str) -> Option<EvidenceSummary> {
        let query = self.get_query(query_id)?;

        let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &query.evidence {
            *by_type.entry(format!("{:?}", e.evidence_type)).or_insert(0) += 1;
        }

        let avg_relevance: f32 = if query.evidence.is_empty() {
            0.0
        } else {
            query.evidence.iter().map(|e| e.relevance).sum::<f32>() / query.evidence.len() as f32
        };

        Some(EvidenceSummary {
            query_id: query.id.clone(),
            total_evidence: query.evidence.len(),
            by_type,
            avg_relevance,
            confidence: query.confidence,
            sources: query.evidence.iter().map(|e| e.source_name.clone()).collect(),
        })
    }

    /// Get collection statistics.
    pub fn get_stats(&self) -> TrackerStats {
        let status_counts: std::collections::HashMap<String, usize> = self
            .queries
            .iter()
            .map(|q| (q.status.to_string(), 1))
            .fold(std::collections::HashMap::new(), |mut acc, (k, v)| {
                *acc.entry(k).or_insert(0) += v;
                acc
            });

        let total_evidence: usize = self.queries.iter().map(|q| q.evidence.len()).sum();

        TrackerStats {
            total_queries: self.queries.len(),
            status_counts,
            total_evidence_collected: total_evidence,
            collection_events: self.collection_history.len(),
        }
    }
}

/// Information about a gap in evidence.
#[derive(Debug, Clone)]
pub struct GapInfo {
    pub query_id: String,
    pub question: String,
    pub current_confidence: f32,
    pub evidence_count: usize,
    pub gap_description: String,
}

/// Summary of evidence for a query.
#[derive(Debug, Clone)]
pub struct EvidenceSummary {
    pub query_id: String,
    pub total_evidence: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub avg_relevance: f32,
    pub confidence: f32,
    pub sources: Vec<String>,
}

/// Statistics for the tracker.
#[derive(Debug, Clone)]
pub struct TrackerStats {
    pub total_queries: usize,
    pub status_counts: std::collections::HashMap<String, usize>,
    pub total_evidence_collected: usize,
    pub collection_events: usize,
}

/// Event in the evidence collection history.
#[derive(Debug, Clone)]
pub struct CollectionEvent {
    pub query_id: String,
    pub action: CollectionAction,
    pub evidence_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Action taken in evidence collection.
#[derive(Debug, Clone, Copy)]
pub enum CollectionAction {
    CreateQuery,
    AddEvidence,
    RemoveEvidence,
    MarkAnswered,
    AbandonQuery,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_query() {
        let mut tracker = EvidenceGapTracker::new();
        let query = tracker.create_query("What is the capital of France?");
        assert_eq!(query.question, "What is the capital of France?");
        assert_eq!(query.status, QueryStatus::Open);
    }

    #[test]
    fn test_add_evidence() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("What is AI?");
        let query_id = tracker.queries[0].id.clone();

        let evidence = EvidenceItem::new("e1", "paper1", "AI Paper", "AI is artificial intelligence", EvidenceType::Direct);
        tracker.add_evidence(&query_id, evidence);

        let query = tracker.get_query(&query_id).unwrap();
        assert_eq!(query.evidence.len(), 1);
    }

    #[test]
    fn test_confidence_update() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test question");
        let query_id = tracker.queries[0].id.clone();

        // Add evidence with high relevance
        let evidence = EvidenceItem::new("e1", "s1", "Source", "Content", EvidenceType::Direct)
            .with_relevance(0.9)
            .with_quality(0.9);
        tracker.add_evidence(&query_id, evidence);

        let query = tracker.get_query(&query_id).unwrap();
        assert!(query.confidence > 0.0);
    }

    #[test]
    fn test_router_decision_empty() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        let action = tracker.decide_action(&query_id).unwrap();
        assert_eq!(action, RouterAction::Retrieve);
    }

    #[test]
    fn test_router_decision_high_confidence() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        // Add multiple high-quality evidence
        for i in 0..5 {
            let evidence = EvidenceItem::new(&format!("e{}", i), "s1", "Source", "Content", EvidenceType::Direct)
                .with_relevance(0.9)
                .with_quality(0.9);
            tracker.add_evidence(&query_id, evidence);
        }

        let action = tracker.decide_action(&query_id).unwrap();
        assert_eq!(action, RouterAction::Answer);
    }

    #[test]
    fn test_gaps() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Question 1");
        tracker.create_query("Question 2");

        let gaps = tracker.get_gaps();
        assert_eq!(gaps.len(), 2);
    }

    #[test]
    fn test_mark_answered() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        tracker.mark_answered(&query_id);
        let query = tracker.get_query(&query_id).unwrap();
        assert_eq!(query.status, QueryStatus::Answered);
    }

    #[test]
    fn test_evidence_summary() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        let evidence = EvidenceItem::new("e1", "paper1", "Paper 1", "Content 1", EvidenceType::Direct);
        tracker.add_evidence(&query_id, evidence);

        let summary = tracker.get_evidence_summary(&query_id).unwrap();
        assert_eq!(summary.total_evidence, 1);
    }

    #[test]
    fn test_stats() {
        let tracker = EvidenceGapTracker::new();
        let stats = tracker.get_stats();
        assert_eq!(stats.total_queries, 0);
    }

    #[test]
    fn test_global_action() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Q1");
        tracker.create_query("Q2");

        let global_action = tracker.get_next_global_action();
        assert!(global_action.is_some());
    }
}