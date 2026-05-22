//! BeliefNetwork - Structured belief tracking for long-horizon research agents.
//!
//! Based on Hindsight architecture (arXiv:2512.12818) which achieves 91.4% on LongMemEval
//! by organizing memory into 4 logical networks: facts, experiences, entity summaries, and beliefs.
//!
//! ## Key Concepts
//!
//! - [`Belief`] - A research belief with evolving confidence and evidence chain
//! - [`BeliefState`] - Current state of a belief (Confirmed, Questioned, Revised, Retracted)
//! - [`EntitySummary`] - Summary of an entity encountered in research
//! - [`Reflection`] - Reflection record when beliefs are updated
//!
//! ## Architecture
//!
//! ```text
//! Retain → Store new evidence, update beliefs
//! Recall → Query beliefs by topic, entity, or evidence
//! Reflect → Update belief states, generate reflections
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

/// A research belief with evolving confidence and evidence chain.
///
/// Beliefs differ from Stances in that they track the evolution of understanding
/// over time, with explicit state transitions and reflection records.
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
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Evidence IDs supporting this belief
    pub supporting_evidence: Vec<String>,
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
}

impl Belief {
    /// Create a new belief with current timestamp.
    pub fn new(
        topic: &str,
        statement: &str,
        reasoning: &str,
        confidence: f64,
        supporting_evidence: Vec<String>,
        tags: Vec<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
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
        }
    }

    /// Update the belief with new evidence or reasoning.
    pub fn revise(
        &mut self,
        new_statement: Option<&str>,
        new_confidence: Option<f64>,
        additional_evidence: Option<Vec<String>>,
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
        let belief = Belief::new(
            "AI Safety",
            "Constitutional AI reduces harmful outputs",
            "Based on Anthropic research",
            0.8,
            vec!["arxiv:2301".to_string()],
            vec!["safety".to_string()],
        );
        assert_eq!(belief.topic, "AI Safety");
        assert_eq!(belief.state, BeliefState::Confirmed);
        assert_eq!(belief.confidence, 0.8);
    }

    #[test]
    fn test_belief_revision() {
        let mut belief = Belief::new(
            "RAG",
            "RAG improves factual accuracy",
            "Initial evidence",
            0.7,
            vec!["paper1".to_string()],
            vec![],
        );
        belief.revise(Some("RAG significantly improves factual accuracy"), Some(0.9), None, Some("Stronger evidence found"));
        assert_eq!(belief.state, BeliefState::Revised);
        assert_eq!(belief.revision_count, 1);
        assert_eq!(belief.confidence, 0.9);
    }

    #[test]
    fn test_belief_questioning() {
        let mut belief = Belief::new(
            "Fine-tuning",
            "Fine-tuning is always better",
            "Initial assumption",
            0.6,
            vec![],
            vec![],
        );
        belief.question("paper:contradiction", "New study shows otherwise");
        assert_eq!(belief.state, BeliefState::Questioned);
        assert!(belief.is_contradicted());
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
}
