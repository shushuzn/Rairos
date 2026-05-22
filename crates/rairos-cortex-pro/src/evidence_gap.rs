//! Evidence Gap Tracker — tracks evidence collection progress for research questions.
//!
//! Based on MemR3 (arxiv:2512.20237) which introduces:
//! - Router that selects among retrieve, reflect, and answer actions
//! - Global evidence-gap tracker that makes the answering process transparent
//!
//! This module tracks what evidence has been collected vs. what gaps remain,
//! enabling the agent to know when it has enough evidence to answer.
//!
//! ## Bayesian Confidence Updating (Mnemo-style)
//!
//! Confidence is represented as a Beta distribution, updated via Bayesian inference:
//! - Beta(α, β) represents belief strength
//! - New evidence updates the distribution: α += success_weight, β += failure_weight
//! - Expected confidence = α / (α + β)
//!
//! Based on Mnemo (HN #47691109) and MuninnDB (HN #47236100).

// ============================================================================
// Data Structures
// ============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ============================================================================
// Beta Distribution for Bayesian Confidence
// ============================================================================

/// Beta distribution for Bayesian belief representation.
///
/// Represents confidence as Beta(α, β) where:
/// - α (alpha) = number of confirming observations + 1
/// - β (beta) = number of contradicting observations + 1
/// - Expected value = α / (α + β)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BetaDistribution {
    /// Alpha parameter (confirmation strength)
    pub alpha: f64,
    /// Beta parameter (contradiction strength)
    pub beta: f64,
}

impl BetaDistribution {
    /// Create a new Beta distribution with given parameters.
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    /// Create a neutral Beta(1, 1) distribution (uniform prior).
    pub fn uniform() -> Self {
        Self { alpha: 1.0, beta: 1.0 }
    }

    /// Create a skeptical prior (Beta(1, 1) but leaning toward low confidence).
    pub fn skeptical() -> Self {
        Self { alpha: 0.5, beta: 0.5 }
    }

    /// Expected value of the distribution (mean).
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Variance of the distribution.
    pub fn variance(&self) -> f64 {
        let denom = (self.alpha + self.beta).powi(2) * (self.alpha + self.beta + 1.0);
        if denom == 0.0 {
            0.25 // Maximum variance for Beta(0,0)
        } else {
            (self.alpha * self.beta) / denom
        }
    }

    /// Standard deviation.
    pub fn std(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Upper bound of credible interval (default 95%).
    pub fn credible_interval_upper(&self, probability: f64) -> f64 {
        // Using approximation: for Beta(α, β), use mean + z * std
        // For 95% CI, z ≈ 1.96
        let z = 1.96;
        (self.mean() + z * self.std()).clamp(0.0, 1.0)
    }

    /// Lower bound of credible interval (default 95%).
    pub fn credible_interval_lower(&self, probability: f64) -> f64 {
        let z = 1.96;
        (self.mean() - z * self.std()).clamp(0.0, 1.0)
    }

    /// Update with confirming evidence (increases confidence).
    ///
    /// Based on Mnemo's Bayesian belief updating:
    /// - α' = α + weight * reliability
    pub fn update_with_confirming(&mut self, weight: f64, reliability: f64) {
        self.alpha += weight * reliability;
        self.beta += weight * (1.0 - reliability);
    }

    /// Update with contradicting evidence (decreases confidence).
    pub fn update_with_contradicting(&mut self, weight: f64, reliability: f64) {
        self.beta += weight * reliability;
        self.alpha += weight * (1.0 - reliability);
    }

    /// Apply time-based decay (evidence becomes less reliable over time).
    ///
    /// Based on ACT-R decay formula from MuninnDB:
    /// Decay factor = e^(-λ * time)
    pub fn apply_decay(&mut self, half_life_days: f64, current_age_days: f64) {
        if half_life_days <= 0.0 {
            return; // No decay for eternal evidence (e.g., theorems)
        }
        let decay = (-current_age_days.ln() / half_life_days).exp();
        let strength = decay.clamp(0.1, 1.0);
        self.alpha *= strength;
        self.beta *= strength;
    }

    /// Merge with another Beta distribution (combine evidence from multiple sources).
    ///
    /// Uses conjugate prior property: product of Beta distributions is also Beta.
    pub fn merge(&mut self, other: &BetaDistribution) {
        // Approximation: weighted average of parameters
        let total = self.alpha + self.beta + other.alpha + other.beta;
        if total == 0.0 {
            return;
        }
        let new_alpha = (self.alpha * (self.alpha + self.beta) + other.alpha * (other.alpha + other.beta)) / total;
        let new_beta = (self.beta * (self.alpha + self.beta) + other.beta * (other.alpha + other.beta)) / total;
        self.alpha = new_alpha.max(0.1);
        self.beta = new_beta.max(0.1);
    }
}

impl Default for BetaDistribution {
    fn default() -> Self {
        Self::uniform()
    }
}

// ============================================================================
// Evidence Source with Reliability Tracking
// ============================================================================

/// Source type for evidence with reliability weight and decay rate.
///
/// Based on evidence hierarchy research:
/// - Paper: high reliability, slow decay (citations indicate enduring significance)
/// - Web: lower reliability, fast decay (information changes rapidly)
/// - Reasoning/Theorem: highest reliability, no decay (eternal truth)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceSource {
    /// Peer-reviewed paper (reliability: 0.85, half-life: 730 days)
    Paper,
    /// Preprint or working paper (reliability: 0.7, half-life: 365 days)
    Preprint,
    /// Web source (reliability: 0.6, half-life: 30 days)
    Web,
    /// User-provided information (reliability: 0.5, half-life: 90 days)
    User,
    /// Experimental result (reliability: 0.9, half-life: none - eternal)
    Experiment,
    /// Mathematical reasoning or theorem (reliability: 1.0, half-life: none)
    Reasoning,
    /// Other or unknown source (reliability: 0.4, half-life: 180 days)
    Other,
}

impl EvidenceSource {
    /// Get the base reliability weight for this source type.
    pub fn reliability_weight(&self) -> f64 {
        match self {
            EvidenceSource::Paper => 0.85,
            EvidenceSource::Preprint => 0.7,
            EvidenceSource::Web => 0.6,
            EvidenceSource::User => 0.5,
            EvidenceSource::Experiment => 0.9,
            EvidenceSource::Reasoning => 1.0,
            EvidenceSource::Other => 0.4,
        }
    }

    /// Get the half-life in days for evidence from this source.
    ///
    /// Returns None for sources that don't decay (e.g., theorems).
    pub fn half_life_days(&self) -> Option<f64> {
        match self {
            EvidenceSource::Paper => Some(730.0),    // ~2 years
            EvidenceSource::Preprint => Some(365.0), // ~1 year
            EvidenceSource::Web => Some(30.0),       // 1 month
            EvidenceSource::User => Some(90.0),      // 3 months
            EvidenceSource::Experiment => None,        // Eternal
            EvidenceSource::Reasoning => None,        // Eternal
            EvidenceSource::Other => Some(180.0),    // 6 months
        }
    }

    /// Check if evidence from this source decays over time.
    pub fn decays_over_time(&self) -> bool {
        self.half_life_days().is_some()
    }

    /// Get effective reliability after time-based decay.
    pub fn effective_reliability(&self, age_days: f64) -> f64 {
        let half_life = match self.half_life_days() {
            Some(hl) => hl,
            None => return self.reliability_weight(), // No decay
        };

        if age_days <= 0.0 {
            return self.reliability_weight();
        }

        // Exponential decay: reliability(t) = reliability_0 * e^(-ln(2) * t / half_life)
        let decay = (-0.69314718 * age_days / half_life).exp();
        let base = self.reliability_weight();

        // Minimum reliability is 20% of original
        (base * decay).max(base * 0.2)
    }
}

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
    /// Beta distribution for Bayesian confidence (alpha, beta)
    /// Kept separate for serialization compatibility
    #[serde(skip)]
    beta_distribution: BetaDistribution,
    /// Confidence uncertainty (std dev of Beta distribution)
    #[serde(skip)]
    confidence_uncertainty: f32,
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
            beta_distribution: BetaDistribution::uniform(),
            confidence_uncertainty: 0.5,
        }
    }

    /// Add evidence to this query.
    pub fn add_evidence(&mut self, evidence: EvidenceItem) {
        self.evidence.push(evidence);
        self.update_confidence();
    }

    /// Get the Beta distribution for Bayesian analysis.
    pub fn get_beta_distribution(&self) -> BetaDistribution {
        self.beta_distribution
    }

    /// Get confidence uncertainty (standard deviation).
    pub fn confidence_uncertainty(&self) -> f32 {
        self.confidence_uncertainty
    }

    /// Get confidence as a tuple (mean, lower_95, upper_95).
    pub fn confidence_interval(&self) -> (f32, f32, f32) {
        let mean = self.beta_distribution.mean() as f32;
        let lower = self.beta_distribution.credible_interval_lower(0.95) as f32;
        let upper = self.beta_distribution.credible_interval_upper(0.95) as f32;
        (mean, lower, upper)
    }

    /// Update confidence using Bayesian inference.
    ///
    /// Based on Mnemo's approach:
    /// 1. For each evidence item, calculate effective reliability
    /// 2. Update Beta distribution based on evidence type
    /// 3. Apply time-based decay
    /// 4. Set confidence to expected value (mean of Beta)
    fn update_confidence(&mut self) {
        if self.evidence.is_empty() {
            self.confidence = 0.0;
            self.confidence_uncertainty = 0.5;
            self.beta_distribution = BetaDistribution::uniform();
            return;
        }

        // Start with skeptical prior
        let mut beta = BetaDistribution::skeptical();
        let now = Utc::now();

        for evidence in &self.evidence {
            // Calculate effective reliability with time decay
            let age_days = (now - evidence.collected_at).num_seconds() as f64 / 86400.0;
            let source_reliability = evidence.source.effective_reliability(age_days);

            // Weight by quality and relevance (modulates evidence strength)
            let weight = (evidence.quality as f64) * (evidence.relevance as f64);

            // Update Beta distribution based on evidence type
            // weight modulates the count, source_reliability is the confirmation probability
            match evidence.evidence_type {
                EvidenceType::Direct | EvidenceType::Supporting => {
                    // Confirming evidence increases alpha
                    beta.update_with_confirming(weight, source_reliability);
                }
                EvidenceType::Contradicting => {
                    // Contradicting evidence increases beta
                    beta.update_with_contradicting(weight, source_reliability);
                }
                EvidenceType::Contextual => {
                    // Contextual evidence has partial effect
                    beta.update_with_confirming(weight * 0.5, source_reliability);
                }
            }
        }

        // Note: Evidence decay is already accounted for in effective_reliability()
        // called above, so no additional decay application needed.

        // Update stored values
        self.beta_distribution = beta;
        self.confidence = beta.mean() as f32;
        self.confidence_uncertainty = beta.std() as f32;
    }

    /// Apply time-based decay to all evidence.
    ///
    /// Based on ACT-R decay formula from MuninnDB.
    fn apply_evidence_decay(&self, beta: &mut BetaDistribution) {
        let now = Utc::now();
        let mut total_decay_strength = 0.0;
        let mut decayed_alpha = 0.0;
        let mut decayed_beta = 0.0;

        for evidence in &self.evidence {
            let age_days = (now - evidence.collected_at).num_seconds() as f64 / 86400.0;
            if let Some(half_life) = evidence.source.half_life_days() {
                if half_life > 0.0 {
                    let decay = (-0.69314718 * age_days / half_life).exp();
                    let strength = decay.min(1.0).max(0.1);
                    let weight = (evidence.quality as f64) * (evidence.relevance as f64);

                    match evidence.evidence_type {
                        EvidenceType::Direct | EvidenceType::Supporting => {
                            decayed_alpha += strength * weight;
                        }
                        EvidenceType::Contradicting => {
                            decayed_beta += strength * weight;
                        }
                        EvidenceType::Contextual => {
                            decayed_alpha += strength * weight * 0.5;
                            decayed_beta += strength * weight * 0.5;
                        }
                    }
                    total_decay_strength += weight;
                }
            }
        }

        if total_decay_strength > 0.0 {
            // Blend decayed evidence into beta
            beta.alpha += decayed_alpha * 0.1;
            beta.beta += decayed_beta * 0.1;
        }
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
    /// Source type for reliability tracking (default: Paper)
    pub source: EvidenceSource,
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
            source: EvidenceSource::Paper, // Default to paper
        }
    }

    /// Create evidence from a paper source.
    pub fn from_paper(id: &str, paper_id: &str, title: &str, content: &str, evidence_type: EvidenceType) -> Self {
        Self {
            id: id.to_string(),
            source_id: paper_id.to_string(),
            source_name: title.to_string(),
            content: content.to_string(),
            relevance: 0.5,
            quality: 0.5,
            collected_at: Utc::now(),
            evidence_type,
            source: EvidenceSource::Paper,
        }
    }

    /// Create evidence from web source.
    pub fn from_web(id: &str, url: &str, title: &str, content: &str, evidence_type: EvidenceType) -> Self {
        Self {
            id: id.to_string(),
            source_id: url.to_string(),
            source_name: title.to_string(),
            content: content.to_string(),
            relevance: 0.5,
            quality: 0.5,
            collected_at: Utc::now(),
            evidence_type,
            source: EvidenceSource::Web,
        }
    }

    /// Create evidence from reasoning/theory.
    pub fn from_reasoning(id: &str, theorem: &str, content: &str, evidence_type: EvidenceType) -> Self {
        Self {
            id: id.to_string(),
            source_id: theorem.to_string(),
            source_name: theorem.to_string(),
            content: content.to_string(),
            relevance: 0.5,
            quality: 0.5,
            collected_at: Utc::now(),
            evidence_type,
            source: EvidenceSource::Reasoning,
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

    /// Set source type.
    pub fn with_source(mut self, source: EvidenceSource) -> Self {
        self.source = source;
        self
    }

    /// Get effective reliability after time-based decay.
    pub fn effective_reliability(&self) -> f32 {
        let age_days = (Utc::now() - self.collected_at).num_seconds() as f64 / 86400.0;
        self.source.effective_reliability(age_days) as f32
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

        if query.confidence >= 0.7 {
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
    pub fn get_stats(&self) -> GapTrackerStats {
        let status_counts: std::collections::HashMap<String, usize> = self
            .queries
            .iter()
            .map(|q| (q.status.to_string(), 1))
            .fold(std::collections::HashMap::new(), |mut acc, (k, v)| {
                *acc.entry(k).or_insert(0) += v;
                acc
            });

        let total_evidence: usize = self.queries.iter().map(|q| q.evidence.len()).sum();

        GapTrackerStats {
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
pub struct GapTrackerStats {
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

    // =====================================================================
    // Bayesian Update Tests (Mnemo/MuninnDB style)
    // =====================================================================

    #[test]
    fn test_beta_distribution_creation() {
        let beta = BetaDistribution::new(2.0, 1.0);
        assert!((beta.mean() - 2.0/3.0).abs() < 0.001);
    }

    #[test]
    fn test_beta_distribution_uniform() {
        let beta = BetaDistribution::uniform();
        assert!((beta.mean() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_beta_distribution_skeptical() {
        let beta = BetaDistribution::skeptical();
        // Skeptical starts with Beta(0.5, 0.5) which is centered on 0.5 but with high uncertainty
        assert!(beta.mean() > 0.4 && beta.mean() < 0.6);
        assert!(beta.std() > 0.3);
    }

    #[test]
    fn test_beta_confirming_update() {
        let mut beta = BetaDistribution::uniform();
        beta.update_with_confirming(1.0, 0.8);
        // Alpha should increase, mean should shift toward confirming evidence
        assert!(beta.alpha > 1.0);
        assert!(beta.mean() > 0.5);
    }

    #[test]
    fn test_beta_contradicting_update() {
        let mut beta = BetaDistribution::uniform();
        beta.update_with_contradicting(1.0, 0.8);
        // Beta should increase, mean should shift toward lower values
        assert!(beta.beta > 1.0);
        assert!(beta.mean() < 0.5);
    }

    #[test]
    fn test_beta_decay() {
        let mut beta = BetaDistribution::new(4.0, 2.0);
        let initial_mean = beta.mean();
        beta.apply_decay(30.0, 60.0); // 60 days with 30 day half-life
        // After 2 half-lives, strength should be reduced
        assert!(beta.alpha < 4.0);
        assert!(beta.beta < 2.0);
    }

    #[test]
    fn test_beta_no_decay_for_eternal() {
        let mut beta = BetaDistribution::new(4.0, 2.0);
        beta.apply_decay(0.0, 1000.0); // 0 half-life = eternal
        // Nothing should change
        assert!((beta.alpha - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_evidence_source_reliability() {
        assert_eq!(EvidenceSource::Paper.reliability_weight(), 0.85);
        assert_eq!(EvidenceSource::Reasoning.reliability_weight(), 1.0);
        assert_eq!(EvidenceSource::Web.reliability_weight(), 0.6);
    }

    #[test]
    fn test_evidence_source_decay() {
        // Paper decays
        assert!(EvidenceSource::Paper.decays_over_time());
        // Reasoning doesn't
        assert!(!EvidenceSource::Reasoning.decays_over_time());
    }

    #[test]
    fn test_evidence_source_effective_reliability() {
        // Fresh paper = base reliability
        let fresh = EvidenceSource::Paper.effective_reliability(0.0);
        assert!((fresh - 0.85).abs() < 0.001);

        // Old paper decays
        let old = EvidenceSource::Paper.effective_reliability(730.0); // 2 years = 2 half-lives
        assert!(old < 0.85);
        assert!(old > 0.85 * 0.2); // But not below 20% of base
    }

    #[test]
    fn test_evidence_item_from_paper() {
        let evidence = EvidenceItem::from_paper("e1", "paper1", "Test Paper", "Content", EvidenceType::Direct);
        assert_eq!(evidence.source, EvidenceSource::Paper);
        assert_eq!(evidence.evidence_type, EvidenceType::Direct);
    }

    #[test]
    fn test_evidence_item_from_reasoning() {
        let evidence = EvidenceItem::from_reasoning("e1", "Theorem 1", "Content", EvidenceType::Supporting);
        assert_eq!(evidence.source, EvidenceSource::Reasoning);
        // Reasoning should have high effective reliability even when old
        let old = evidence.effective_reliability();
        assert!((old - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bayesian_confidence_with_multiple_evidence() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        // Add multiple high-quality supporting evidence
        for i in 0..3 {
            let evidence = EvidenceItem::from_paper(&format!("e{}", i), &format!("p{}", i), "Paper", "Content", EvidenceType::Direct)
                .with_relevance(0.9)
                .with_quality(0.9);
            tracker.add_evidence(&query_id, evidence);
        }

        let query = tracker.get_query(&query_id).unwrap();
        // With 3 strong supporting evidence, confidence should be high
        assert!(query.confidence > 0.7);
        // Uncertainty should decrease with more evidence
        assert!(query.confidence_uncertainty < 0.3);
    }

    #[test]
    fn test_bayesian_confidence_with_contradicting() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        // Add strong supporting evidence
        let supporting = EvidenceItem::from_paper("e1", "p1", "Paper", "Content", EvidenceType::Direct)
            .with_relevance(0.9)
            .with_quality(0.9);
        tracker.add_evidence(&query_id, supporting);

        // Add contradicting evidence
        let contradicting = EvidenceItem::from_web("e2", "url", "Web", "Content", EvidenceType::Contradicting)
            .with_relevance(0.8)
            .with_quality(0.8);
        tracker.add_evidence(&query_id, contradicting);

        let query = tracker.get_query(&query_id).unwrap();
        // Supporting should outweigh contradicting with similar quality
        // But confidence should be lower than with supporting alone
        let supporting_only = {
            let mut t2 = EvidenceGapTracker::new();
            t2.create_query("Test?");
            let qid = t2.queries[0].id.clone();
            t2.add_evidence(&qid, EvidenceItem::from_paper("e1", "p1", "Paper", "Content", EvidenceType::Direct)
                .with_relevance(0.9)
                .with_quality(0.9));
            t2.get_query(&qid).unwrap().confidence
        };
        assert!(query.confidence < supporting_only);
    }

    #[test]
    fn test_confidence_interval() {
        let mut tracker = EvidenceGapTracker::new();
        tracker.create_query("Test?");
        let query_id = tracker.queries[0].id.clone();

        // Add some evidence
        let evidence = EvidenceItem::from_paper("e1", "p1", "Paper", "Content", EvidenceType::Direct)
            .with_relevance(0.8)
            .with_quality(0.8);
        tracker.add_evidence(&query_id, evidence);

        let query = tracker.get_query(&query_id).unwrap();
        let (mean, lower, upper) = query.confidence_interval();

        // Mean should be between lower and upper
        assert!(lower <= mean && mean <= upper);
        // Uncertainty interval should be reasonable
        assert!(upper - lower < 1.0);
    }

    #[test]
    fn test_beta_distribution_merge() {
        let mut beta1 = BetaDistribution::new(2.0, 1.0);
        let beta2 = BetaDistribution::new(1.0, 2.0);
        beta1.merge(&beta2);
        // After merge, should still be a valid distribution
        assert!(beta1.alpha > 0.0 && beta1.beta > 0.0);
        assert!(beta1.mean() >= 0.0 && beta1.mean() <= 1.0);
    }
}