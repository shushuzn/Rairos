//! BeliefNetwork - Structured belief tracking for long-horizon research agents.
//!
//! Based on Hindsight architecture (arXiv:2512.12818) which achieves 91.4% on LongMemEval
//! by organizing memory into 4 logical networks: facts, experiences, entity summaries, and beliefs.
//!
//! ## Enhanced with Evidence Decay and Source Reliability
//!
//! Based on research from:
//! - Veritas: Confidence vectors with fragility, staleness, source diversity
//! - Epica: Dual-process uncertainty, Bayesian surprise monitoring
//! - ABBEL: Belief bottlenecks with RL training
//!
//! ## Key Concepts
//!
//! - [`Belief`] - A research belief with evolving confidence and evidence chain
//! - [`BeliefState`] - Current state of a belief (Confirmed, Questioned, Revised, Retracted)
//! - [`EntitySummary`] - Summary of an entity encountered in research
//! - [`Reflection`] - Reflection record when beliefs are updated
//! - [`Evidence`] - Evidence with source tracking, decay, and reliability
//! - [`EvidenceSource`] - Source types with reliability weights
//!
//! ## Architecture
//!
//! ```text
//! Retain → Store new evidence, update beliefs
//! Recall → Query beliefs by topic, entity, or evidence
//! Reflect → Update belief states, generate reflections
//! Decay → Evidence ages, confidence adjusts automatically
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// State of a belief in the belief network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefState {
    /// Belief is confirmed by strong evidence
    Confirmed,
    /// New evidence has questioned this belief
    Questioned,
    /// Belief has been revised based on new evidence
    Revised,
    /// Belief has been retracted (no longer held)
    Retracted,
}

impl std::fmt::Display for BeliefState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeliefState::Confirmed => write!(f, "confirmed"),
            BeliefState::Questioned => write!(f, "questioned"),
            BeliefState::Revised => write!(f, "revised"),
            BeliefState::Retracted => write!(f, "retracted"),
        }
    }
}

/// Source type for evidence with associated reliability weight.
///
/// Based on Veritas research: different sources have different credibility.
/// Theorems don't decay, but empirical evidence does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    /// Peer-reviewed paper (highest reliability)
    Paper,
    /// Preprint or technical report
    Preprint,
    /// Web source (lower reliability)
    Web,
    /// User-provided information
    User,
    /// Experimental result
    Experiment,
    /// Reasoning or deduction (theorems don't decay)
    Reasoning,
    /// Other source
    Other,
}

impl EvidenceSource {
    /// Get the base reliability weight for this source type (0.0 - 1.0).
    pub fn reliability_weight(&self) -> f64 {
        match self {
            EvidenceSource::Paper => 1.0,
            EvidenceSource::Preprint => 0.85,
            EvidenceSource::Experiment => 0.9,
            EvidenceSource::Reasoning => 1.0, // Theorems don't decay
            EvidenceSource::User => 0.5,
            EvidenceSource::Web => 0.4,
            EvidenceSource::Other => 0.5,
        }
    }

    /// Get the half-life in days for evidence from this source.
    /// None means the evidence doesn't decay (e.g., theorems).
    pub fn half_life_days(&self) -> Option<f64> {
        match self {
            EvidenceSource::Paper => Some(365.0 * 2.0),    // 2 years
            EvidenceSource::Preprint => Some(365.0),         // 1 year
            EvidenceSource::Experiment => Some(180.0),       // 6 months
            EvidenceSource::User => Some(90.0),              // 3 months
            EvidenceSource::Web => Some(30.0),              // 1 month
            EvidenceSource::Reasoning => None,              // No decay
            EvidenceSource::Other => Some(180.0),
        }
    }

    /// Whether this source type decays over time.
    pub fn decays_over_time(&self) -> bool {
        self.half_life_days().is_some()
    }
}

/// Evidence item with source tracking, timestamp, and reliability.
///
/// Based on Veritas ConfidenceVector concept:
/// - value: current best estimate
/// - source_diversity: independent confirmation compounds
/// - staleness: how much aging has cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique evidence identifier
    pub evidence_id: String,
    /// Source type
    pub source: EvidenceSource,
    /// When this evidence was recorded
    pub recorded_at: f64,
    /// Base reliability (can be adjusted based on track record)
    pub reliability: f64,
    /// Whether this evidence has been validated
    pub validated: bool,
    /// Citation count (for papers)
    pub citation_count: u32,
    /// Notes about this evidence
    pub notes: String,
}

impl Evidence {
    /// Create new evidence from a source type.
    pub fn new(evidence_id: &str, source: EvidenceSource) -> Self {
        Self {
            evidence_id: evidence_id.to_string(),
            source,
            recorded_at: current_timestamp(),
            reliability: source.reliability_weight(),
            validated: false,
            citation_count: 0,
            notes: String::new(),
        }
    }

    /// Create paper evidence with optional citation count.
    pub fn paper(evidence_id: &str, citation_count: u32) -> Self {
        Self {
            evidence_id: evidence_id.to_string(),
            source: EvidenceSource::Paper,
            recorded_at: current_timestamp(),
            reliability: EvidenceSource::Paper.reliability_weight(),
            validated: true,
            citation_count,
            notes: String::new(),
        }
    }

    /// Create reasoning evidence (theorems don't decay).
    pub fn reasoning(evidence_id: &str) -> Self {
        Self {
            evidence_id: evidence_id.to_string(),
            source: EvidenceSource::Reasoning,
            recorded_at: current_timestamp(),
            reliability: 1.0,
            validated: true,
            citation_count: 0,
            notes: String::new(),
        }
    }

    /// Get the effective reliability after time-based decay.
    pub fn effective_reliability(&self) -> f64 {
        if !self.source.decays_over_time() {
            return self.reliability;
        }

        let half_life = self.source.half_life_days().unwrap_or(365.0);
        let age_days = self.age_days();

        // Exponential decay: reliability = base * (0.5)^(age/half_life)
        let decay_factor = 0.5_f64.powf(age_days / half_life);
        self.reliability * decay_factor
    }

    /// Get age of evidence in days.
    pub fn age_days(&self) -> f64 {
        let now = current_timestamp();
        (now - self.recorded_at) / 86400.0
    }
}

/// ConfidenceComponents - breakdown of what affects confidence.
///
/// Based on Veritas ConfidenceVector:
/// - value: current best estimate
/// - fragility: how much confidence drops if best source removed
/// - staleness_penalty: how much evidence aging has cost
/// - source_diversity: how independent sources are
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceComponents {
    /// Base confidence value (0.0 - 1.0)
    pub value: f64,
    /// Fragility: how much confidence drops without best source (0.0 - 1.0)
    pub fragility: f64,
    /// Staleness penalty: how much aging has cost (0.0 - 1.0, 1.0 = fresh)
    pub staleness: f64,
    /// Source diversity score (0.0 - 1.0, 1.0 = many independent sources)
    pub source_diversity: f64,
}

/// A research belief with evolving confidence and evidence chain.
///
/// Beliefs differ from Stances in that they track the evolution of understanding
/// over time, with explicit state transitions and reflection records.
///
/// Enhanced with evidence decay and source reliability tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    /// Unique belief identifier
    pub belief_id: String,
    /// Topic/area this belief belongs to
    pub topic: String,
    /// The belief statement (what we believe)
    pub statement: String,
    /// Current state of the belief
    pub state: BeliefState,
    /// Base confidence level (0.0 to 1.0) before decay
    pub confidence: f64,
    /// Evidence items supporting this belief (with source tracking)
    pub supporting_evidence: Vec<Evidence>,
    /// Evidence IDs contradicting this belief
    pub contradicting_evidence: Vec<String>,
    /// Reasoning behind the belief
    pub reasoning: String,
    /// When the belief was first formed
    pub created_at: f64,
    /// When the belief was last updated
    pub updated_at: f64,
    /// When the belief state last changed
    pub state_changed_at: f64,
    /// Number of times this belief has been revised
    pub revision_count: u32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// IDs of beliefs this belief depends on
    pub depends_on: Vec<String>,
    /// Fragility score (0.0 - 1.0): how much confidence drops if best source removed
    pub fragility: f64,
    /// Last time confidence was recalculated with decay
    pub last_decay_at: f64,
}

impl Belief {
    /// Create a new belief with current timestamp.
    pub fn new(
        topic: &str,
        statement: &str,
        reasoning: &str,
        confidence: f64,
        supporting_evidence: Vec<Evidence>,
        tags: Vec<String>,
    ) -> Self {
        let now = current_timestamp();
        let mut belief = Self {
            belief_id: Uuid::new_v4().to_string()[..8].to_string(),
            topic: topic.to_string(),
            statement: statement.to_string(),
            state: BeliefState::Confirmed,
            confidence: confidence.clamp(0.0, 1.0),
            supporting_evidence,
            contradicting_evidence: Vec::new(),
            reasoning: reasoning.to_string(),
            created_at: now,
            updated_at: now,
            state_changed_at: now,
            revision_count: 0,
            tags,
            depends_on: Vec::new(),
            fragility: 0.0,
            last_decay_at: now,
        };
        belief.update_fragility();
        belief
    }

    /// Create a new belief with evidence IDs (convenience method).
    pub fn with_evidence_ids(
        topic: &str,
        statement: &str,
        reasoning: &str,
        confidence: f64,
        evidence_ids: Vec<String>,
        source: EvidenceSource,
        tags: Vec<String>,
    ) -> Self {
        let evidence: Vec<Evidence> = evidence_ids
            .into_iter()
            .map(|id| Evidence::new(&id, source))
            .collect();
        Self::new(topic, statement, reasoning, confidence, evidence, tags)
    }

    /// Update the belief with new evidence or reasoning.
    pub fn revise(
        &mut self,
        new_statement: Option<&str>,
        new_confidence: Option<f64>,
        additional_evidence: Option<Vec<Evidence>>,
        new_reasoning: Option<&str>,
    ) {
        if let Some(s) = new_statement {
            self.statement = s.to_string();
        }
        if let Some(c) = new_confidence {
            self.confidence = c.clamp(0.0, 1.0);
        }
        if let Some(evidence) = additional_evidence {
            self.supporting_evidence.extend(evidence);
            self.update_fragility();
        }
        if let Some(r) = new_reasoning {
            self.reasoning = r.to_string();
        }
        self.revision_count += 1;
        self.state = BeliefState::Revised;
        self.updated_at = current_timestamp();
        self.state_changed_at = current_timestamp();
    }

    /// Mark belief as questioned due to new evidence.
    pub fn question(&mut self, contradicting_evidence_id: &str, reasoning: &str) {
        self.contradicting_evidence.push(contradicting_evidence_id.to_string());
        self.state = BeliefState::Questioned;
        self.reasoning = format!("{} [Questioned: {}]", self.reasoning, reasoning);
        self.updated_at = current_timestamp();
        self.state_changed_at = current_timestamp();
    }

    /// Retract the belief entirely.
    pub fn retract(&mut self, reason: &str) {
        self.state = BeliefState::Retracted;
        self.confidence = 0.0;
        self.reasoning = format!("[RETACTED: {}] {}", reason, self.reasoning);
        self.updated_at = current_timestamp();
        self.state_changed_at = current_timestamp();
    }

    /// Check if belief has sufficient supporting evidence.
    pub fn has_sufficient_evidence(&self, min_count: usize) -> bool {
        self.supporting_evidence.len() >= min_count
    }

    /// Check if belief is contradicted.
    pub fn is_contradicted(&self) -> bool {
        !self.contradicting_evidence.is_empty()
    }

    /// Calculate evidence balance (supporting - contradicting).
    pub fn evidence_balance(&self) -> i32 {
        self.supporting_evidence.len() as i32 - self.contradicting_evidence.len() as i32
    }

    /// Add evidence to this belief.
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.supporting_evidence.push(evidence);
        self.update_fragility();
        self.updated_at = current_timestamp();
    }

    /// Add a belief dependency (this belief depends on another belief).
    pub fn add_dependency(&mut self, dependency_id: &str) {
        if !self.depends_on.contains(&dependency_id.to_string()) {
            self.depends_on.push(dependency_id.to_string());
        }
    }

    /// Update the fragility score based on current evidence.
    /// Fragility = how much confidence would drop if best source was removed.
    fn update_fragility(&mut self) {
        if self.supporting_evidence.is_empty() {
            self.fragility = 1.0;
            return;
        }

        if self.supporting_evidence.len() == 1 {
            self.fragility = 1.0;
            return;
        }

        // Sort by reliability descending
        let mut reliabilities: Vec<f64> = self.supporting_evidence
            .iter()
            .map(|e| e.effective_reliability())
            .collect();
        reliabilities.sort_by(|a, b| b.partial_cmp(a).unwrap());

        let best = reliabilities[0];
        let total: f64 = reliabilities.iter().sum();

        if total == 0.0 {
            self.fragility = 1.0;
            return;
        }

        // Fragility = how much of total weight is in best source
        self.fragility = best / total;
    }

    /// Get effective confidence considering evidence decay and source diversity.
    ///
    /// Based on Veritas research:
    /// - Evidence ages and becomes less reliable
    /// - Multiple independent sources compound confidence
    /// - Fragility reduces confidence if best source dominates
    pub fn effective_confidence(&self) -> f64 {
        if self.supporting_evidence.is_empty() {
            return self.confidence * 0.5; // No evidence = reduced confidence
        }

        // Calculate effective evidence weight using noisy-OR pooling
        let effective_weights: Vec<f64> = self.supporting_evidence
            .iter()
            .map(|e| e.effective_reliability())
            .collect();

        // Noisy-OR: combined = 1 - prod(1 - weights)
        let combined: f64 = 1.0 - effective_weights
            .iter()
            .map(|w| 1.0 - w)
            .product::<f64>();

        // Factor in source diversity bonus
        let diversity = self.source_diversity_score();
        let diversity_bonus = diversity * 0.1; // Up to 10% bonus

        // Factor in fragility penalty
        let fragility_penalty = self.fragility * 0.2; // Up to 20% penalty

        // Calculate final effective confidence
        let base = self.confidence * combined;
        let adjusted = base + diversity_bonus - fragility_penalty;

        adjusted.clamp(0.0, 1.0)
    }

    /// Calculate source diversity score (0.0 - 1.0).
    ///
    /// Higher diversity = more independent sources confirming the belief.
    /// Based on Veritas research: independent confirmation compounds.
    fn source_diversity_score(&self) -> f64 {
        if self.supporting_evidence.is_empty() {
            return 0.0;
        }

        // Count unique source types
        let source_types: std::collections::HashSet<_> = self.supporting_evidence
            .iter()
            .map(|e| e.source)
            .collect();

        // Calculate entropy-based diversity
        let source_counts: std::collections::HashMap<_, usize> = source_types
            .iter()
            .map(|st| {
                let count = self.supporting_evidence
                    .iter()
                    .filter(|e| e.source == *st)
                    .count();
                (*st, count)
            })
            .collect();

        let total = self.supporting_evidence.len() as f64;
        let entropy: f64 = source_counts
            .values()
            .map(|&count| {
                let p = count as f64 / total;
                if p > 0.0 { -p * p.log2() } else { 0.0 }
            })
            .sum();

        // Normalize by max entropy (uniform distribution)
        let max_entropy = (source_types.len() as f64).log2();
        if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        }
    }

    /// Get staleness penalty (0.0 - 1.0, where 1.0 = fresh).
    ///
    /// Based on Veritas research: evidence ages and becomes less certain.
    pub fn staleness_penalty(&self) -> f64 {
        if self.supporting_evidence.is_empty() {
            return 0.5; // No evidence = medium staleness
        }

        let total_age_days: f64 = self.supporting_evidence
            .iter()
            .map(|e| e.age_days())
            .sum();

        let avg_age_days = total_age_days / self.supporting_evidence.len() as f64;

        // If sources don't decay (theorems), no staleness
        let has_decay = self.supporting_evidence
            .iter()
            .any(|e| e.source.decays_over_time());

        if !has_decay {
            return 1.0;
        }

        // Exponential decay with 180-day characteristic scale
        let decay = (-avg_age_days / 180.0).exp();
        decay.clamp(0.1, 1.0)
    }

    /// Get detailed confidence components for debugging/explanation.
    ///
    /// Based on Veritas ConfidenceVector concept.
    pub fn confidence_breakdown(&self) -> ConfidenceComponents {
        let staleness = self.staleness_penalty();
        let diversity = self.source_diversity_score();
        let effective = self.effective_confidence();

        ConfidenceComponents {
            value: effective,
            fragility: self.fragility,
            staleness,
            source_diversity: diversity,
        }
    }

    /// Recalculate confidence with current decay state.
    /// Call this periodically or when evidence ages.
    pub fn apply_decay(&mut self) {
        let old_confidence = self.confidence;
        self.confidence = self.effective_confidence();
        self.last_decay_at = current_timestamp();

        // If confidence dropped significantly, mark as questioned
        if old_confidence - self.confidence > 0.2 {
            self.state = BeliefState::Questioned;
            self.state_changed_at = current_timestamp();
        }
    }
}

/// Summary of an entity encountered during research.
///
/// Based on Hindsight's entity summary concept - tracking what we know
/// about specific entities (papers, authors, methods, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    /// Unique entity identifier (e.g., arxiv ID, DOI)
    pub entity_id: String,
    /// Type of entity (paper, author, method, concept)
    pub entity_type: EntityType,
    /// Entity name/title
    pub name: String,
    /// Key facts about this entity
    pub facts: Vec<String>,
    /// Papers/experiences where this entity was encountered
    pub source_refs: Vec<String>,
    /// When first encountered
    pub first_seen: f64,
    /// When last updated
    pub last_updated: f64,
    /// Number of times referenced
    pub reference_count: u32,
    /// Tags
    pub tags: Vec<String>,
}

/// Type of entity being tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Paper,
    Author,
    Method,
    Concept,
    Dataset,
    Tool,
    Other,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Paper => write!(f, "paper"),
            EntityType::Author => write!(f, "author"),
            EntityType::Method => write!(f, "method"),
            EntityType::Concept => write!(f, "concept"),
            EntityType::Dataset => write!(f, "dataset"),
            EntityType::Tool => write!(f, "tool"),
            EntityType::Other => write!(f, "other"),
        }
    }
}

impl EntitySummary {
    /// Create a new entity summary.
    pub fn new(entity_id: &str, entity_type: EntityType, name: &str) -> Self {
        let now = current_timestamp();
        Self {
            entity_id: entity_id.to_string(),
            entity_type,
            name: name.to_string(),
            facts: Vec::new(),
            source_refs: Vec::new(),
            first_seen: now,
            last_updated: now,
            reference_count: 0,
            tags: Vec::new(),
        }
    }

    /// Add a fact about this entity.
    pub fn add_fact(&mut self, fact: &str, source_ref: &str) {
        self.facts.push(fact.to_string());
        self.source_refs.push(source_ref.to_string());
        self.last_updated = current_timestamp();
    }

    /// Increment reference count.
    pub fn touch(&mut self) {
        self.reference_count += 1;
        self.last_updated = current_timestamp();
    }
}

/// A reflection record documenting belief evolution.
///
/// Reflections are created when beliefs are updated, providing
/// an audit trail of how understanding evolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    /// Unique reflection identifier
    pub reflection_id: String,
    /// ID of the belief this reflection is about
    pub belief_id: String,
    /// Type of reflection
    pub reflection_type: ReflectionType,
    /// Description of what happened
    pub description: String,
    /// Previous state (if applicable)
    pub previous_state: Option<BeliefState>,
    /// New state (if applicable)
    pub new_state: Option<BeliefState>,
    /// Evidence that triggered this reflection
    pub trigger_evidence: Vec<String>,
    /// Timestamp
    pub created_at: f64,
}

impl Reflection {
    /// Create a new reflection.
    pub fn new(
        belief_id: &str,
        reflection_type: ReflectionType,
        description: &str,
        previous_state: Option<BeliefState>,
        new_state: Option<BeliefState>,
        trigger_evidence: Vec<String>,
    ) -> Self {
        Self {
            reflection_id: Uuid::new_v4().to_string()[..8].to_string(),
            belief_id: belief_id.to_string(),
            reflection_type,
            description: description.to_string(),
            previous_state,
            new_state,
            trigger_evidence,
            created_at: current_timestamp(),
        }
    }
}

/// Type of reflection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReflectionType {
    /// New belief was formed
    Formation,
    /// Belief was revised
    Revision,
    /// Belief was questioned
    Questioning,
    /// Belief was retracted
    Retraction,
    /// Belief was confirmed
    Confirmation,
}

// ─── Helper Functions ───────────────────────────────────────────────────────────

/// Get current timestamp as f64 (seconds since epoch).
fn current_timestamp() -> f64 {
    let now = Utc::now();
    now.timestamp() as f64 + now.timestamp_subsec_nanos() as f64 * 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_creation() {
        let evidence = Evidence::paper("arxiv:2301", 100);
        let belief = Belief::new(
            "AI Safety",
            "Constitutional AI reduces harmful outputs",
            "Based on Anthropic research",
            0.8,
            vec![evidence],
            vec!["safety".to_string()],
        );
        assert_eq!(belief.topic, "AI Safety");
        assert_eq!(belief.state, BeliefState::Confirmed);
        assert_eq!(belief.confidence, 0.8);
        assert_eq!(belief.supporting_evidence.len(), 1);
    }

    #[test]
    fn test_belief_revision() {
        let evidence = Evidence::new("paper1", EvidenceSource::Paper);
        let belief = Belief::new(
            "RAG",
            "RAG improves factual accuracy",
            "Initial evidence",
            0.7,
            vec![evidence],
            vec![],
        );
        let mut belief = belief;
        belief.revise(
            Some("RAG significantly improves factual accuracy"),
            Some(0.9),
            None,
            Some("Stronger evidence found"),
        );
        assert_eq!(belief.state, BeliefState::Revised);
        assert_eq!(belief.revision_count, 1);
        assert_eq!(belief.confidence, 0.9);
    }

    #[test]
    fn test_belief_questioning() {
        let belief = Belief::new(
            "Fine-tuning",
            "Fine-tuning is always better",
            "Initial assumption",
            0.6,
            vec![],
            vec![],
        );
        let mut belief = belief;
        belief.question("paper:contradiction", "New study shows otherwise");
        assert_eq!(belief.state, BeliefState::Questioned);
        assert!(belief.is_contradicted());
    }

    #[test]
    fn test_evidence_decay() {
        // Paper evidence should have high reliability
        let paper = Evidence::paper("arxiv:2301", 100);
        assert!((paper.effective_reliability() - 1.0).abs() < 0.001);

        // Reasoning (theorem) should not decay
        let reasoning = Evidence::reasoning("theorem:1");
        assert!((reasoning.effective_reliability() - 1.0).abs() < 0.001);
        assert!(!reasoning.source.decays_over_time());
    }

    #[test]
    fn test_fragility_single_source() {
        let evidence = Evidence::paper("paper1", 10);
        let belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.8,
            vec![evidence],
            vec![],
        );
        assert_eq!(belief.fragility, 1.0); // Single source = high fragility
    }

    #[test]
    fn test_fragility_multiple_sources() {
        let paper1 = Evidence::paper("paper1", 100);
        let paper2 = Evidence::paper("paper2", 50);
        let web = Evidence::new("web1", EvidenceSource::Web);

        let belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.8,
            vec![paper1, paper2, web],
            vec![],
        );

        // Multiple sources should reduce fragility
        assert!(belief.fragility < 1.0);
        // But still significant since paper dominates
        assert!(belief.fragility > 0.3);
    }

    #[test]
    fn test_source_diversity() {
        let paper = Evidence::paper("paper1", 100);
        let experiment = Evidence::new("exp1", EvidenceSource::Experiment);
        let reasoning = Evidence::reasoning("theorem1");

        let belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.8,
            vec![paper, experiment, reasoning],
            vec![],
        );

        let diversity = belief.source_diversity_score();
        assert!(diversity > 0.0);
        assert!(diversity <= 1.0);
    }

    #[test]
    fn test_effective_confidence() {
        let paper1 = Evidence::paper("paper1", 100);
        let paper2 = Evidence::paper("paper2", 50);

        let belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.9,
            vec![paper1, paper2],
            vec![],
        );

        let effective = belief.effective_confidence();
        assert!(effective > 0.0);
        assert!(effective <= 1.0);
    }

    #[test]
    fn test_confidence_breakdown() {
        let paper = Evidence::paper("paper1", 100);
        let belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.8,
            vec![paper],
            vec![],
        );

        let breakdown = belief.confidence_breakdown();
        // Use approximate comparison for floating point
        assert!((breakdown.value - belief.effective_confidence()).abs() < 1e-10);
        assert!((breakdown.fragility - belief.fragility).abs() < 1e-10);
        assert!((breakdown.staleness - belief.staleness_penalty()).abs() < 1e-10);
    }

    #[test]
    fn test_entity_summary() {
        let mut entity = EntitySummary::new(
            "2301.00001",
            EntityType::Paper,
            "Attention Is All You Need",
        );
        entity.add_fact("Introduced transformer architecture", "arxiv:2301.00001");
        entity.add_fact("Transformer uses self-attention", "arxiv:2301.00001");
        assert_eq!(entity.facts.len(), 2);
        entity.touch();
        assert_eq!(entity.reference_count, 1);
    }

    #[test]
    fn test_reflection() {
        let reflection = Reflection::new(
            "belief123",
            ReflectionType::Revision,
            "Confidence increased after new evidence",
            Some(BeliefState::Confirmed),
            Some(BeliefState::Confirmed),
            vec!["new_paper".to_string()],
        );
        assert_eq!(reflection.reflection_type, ReflectionType::Revision);
    }

    #[test]
    fn test_confidence_clamping() {
        let belief = Belief::new(
            "test",
            "test statement",
            "reasoning",
            1.5, // > 1.0
            vec![],
            vec![],
        );
        assert_eq!(belief.confidence, 1.0);

        let belief2 = Belief::new(
            "test",
            "test statement",
            "reasoning",
            -0.5, // < 0.0
            vec![],
            vec![],
        );
        assert_eq!(belief2.confidence, 0.0);
    }

    #[test]
    fn test_add_evidence_updates_fragility() {
        let mut belief = Belief::new(
            "test",
            "test belief",
            "reasoning",
            0.8,
            vec![],
            vec![],
        );
        assert_eq!(belief.fragility, 1.0); // No evidence = high fragility

        belief.add_evidence(Evidence::paper("paper1", 100));
        // Single source = max fragility = 1.0
        assert_eq!(belief.fragility, 1.0);

        belief.add_evidence(Evidence::paper("paper2", 50));
        // Two sources = reduced fragility (best is less dominant)
        assert!(belief.fragility < 1.0);
        assert!(belief.fragility > 0.5); // But still significant
    }

    #[test]
    fn test_belief_dependency() {
        let mut belief = Belief::new(
            "derived",
            "derived belief",
            "reasoning",
            0.7,
            vec![],
            vec![],
        );
        belief.add_dependency("belief1");
        belief.add_dependency("belief2");
        belief.add_dependency("belief1"); // Duplicate - should be ignored

        assert_eq!(belief.depends_on.len(), 2);
        assert!(belief.depends_on.contains(&"belief1".to_string()));
        assert!(belief.depends_on.contains(&"belief2".to_string()));
    }
}
