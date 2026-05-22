//! PICCO Prompt Framework - Structured Prompt Engineering.
//!
//! Based on research from:
//! - PICCO Framework arXiv:2604.14197 - Prompt structure taxonomy
//! - PREFPO arXiv:2603.19311 - Preference-based prompt optimization
//! - aPSF arXiv:2604.06699 - Adaptive Prompt Structure Factorization
//!
//! ## PICCO Elements
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  P - Persona      │  Who is the AI?               │
//! ├─────────────────────────────────────────────────────┤
//! │  I - Instructions │  What should it do?           │
//! ├─────────────────────────────────────────────────────┤
//! │  C - Context      │  Background information         │
//! ├─────────────────────────────────────────────────────┤
//! │  C - Constraints  │  Rules and limitations        │
//! ├─────────────────────────────────────────────────────┤
//! │  O - Output       │  Expected format              │
//! └─────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PICCO Prompt structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiccoPrompt {
    /// Persona - Who is the AI
    pub persona: Option<Persona>,
    /// Instructions - What to do
    pub instructions: Instructions,
    /// Context - Background information
    pub context: Vec<ContextItem>,
    /// Constraints - Rules and limitations
    pub constraints: Vec<Constraint>,
    /// Output - Expected format
    pub output: Option<OutputSpec>,
    /// Metadata
    pub metadata: PromptMetadata,
}

/// Persona definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Role name
    pub role: String,
    /// Detailed description
    pub description: String,
    /// Tone/style
    pub tone: Option<String>,
    /// Expertise level
    pub expertise: ExpertiseLevel,
}

/// Expertise level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpertiseLevel {
    /// Beginner level
    Beginner,
    /// Intermediate level
    Intermediate,
    /// Expert level
    Expert,
    /// Specialist level
    Specialist,
}

impl ExpertiseLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpertiseLevel::Beginner => "beginner",
            ExpertiseLevel::Intermediate => "intermediate",
            ExpertiseLevel::Expert => "expert",
            ExpertiseLevel::Specialist => "specialist",
        }
    }
}

/// Instructions definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instructions {
    /// Primary task
    pub primary: String,
    /// Step-by-step steps
    pub steps: Vec<String>,
    /// Quality requirements
    pub requirements: Vec<String>,
    /// Priority level
    pub priority: Priority,
}

/// Priority level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
            Priority::Critical => "critical",
        }
    }
}

/// Context item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Context type
    pub context_type: ContextType,
    /// Content
    pub content: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

/// Context type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextType {
    /// Background information
    Background,
    /// Examples
    Example,
    /// Previous conversation
    History,
    /// External knowledge
    Knowledge,
    /// Task-specific data
    Data,
    /// Constraints reference
    Constraints,
}

impl ContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::Background => "background",
            ContextType::Example => "example",
            ContextType::History => "history",
            ContextType::Knowledge => "knowledge",
            ContextType::Data => "data",
            ContextType::Constraints => "constraints",
        }
    }
}

/// Constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Description
    pub description: String,
    /// Whether it's strict or soft
    pub strict: bool,
}

/// Constraint type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintType {
    /// Content constraint
    Content,
    /// Format constraint
    Format,
    /// Length constraint
    Length,
    /// Safety constraint
    Safety,
    /// Style constraint
    Style,
    /// Domain constraint
    Domain,
}

impl ConstraintType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConstraintType::Content => "content",
            ConstraintType::Format => "format",
            ConstraintType::Length => "length",
            ConstraintType::Safety => "safety",
            ConstraintType::Style => "style",
            ConstraintType::Domain => "domain",
        }
    }
}

/// Output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    /// Format type
    pub format_type: FormatType,
    /// Structure definition
    pub structure: OutputStructure,
    /// Example output
    pub example: Option<String>,
}

/// Format type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormatType {
    /// Plain text
    Text,
    /// JSON
    Json,
    /// Markdown
    Markdown,
    /// XML
    Xml,
    /// Custom
    Custom,
}

impl FormatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FormatType::Text => "text",
            FormatType::Json => "json",
            FormatType::Markdown => "markdown",
            FormatType::Xml => "xml",
            FormatType::Custom => "custom",
        }
    }
}

/// Output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputStructure {
    /// Required fields
    pub required_fields: Vec<String>,
    /// Optional fields
    pub optional_fields: Vec<String>,
    /// Field descriptions
    pub field_descriptions: HashMap<String, String>,
    /// Max length
    pub max_length: Option<usize>,
}

/// Prompt metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// Prompt version
    pub version: String,
    /// Author
    pub author: Option<String>,
    /// Created at
    pub created_at: String,
    /// Tags
    pub tags: Vec<String>,
}

impl PiccoPrompt {
    /// Create a new empty prompt
    pub fn new(primary_task: impl Into<String>) -> Self {
        Self {
            persona: None,
            instructions: Instructions {
                primary: primary_task.into(),
                steps: Vec::new(),
                requirements: Vec::new(),
                priority: Priority::Medium,
            },
            context: Vec::new(),
            constraints: Vec::new(),
            output: None,
            metadata: PromptMetadata {
                version: "1.0".to_string(),
                author: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                tags: Vec::new(),
            },
        }
    }

    /// Set persona
    pub fn with_persona(mut self, persona: Persona) -> Self {
        self.persona = Some(persona);
        self
    }

    /// Add step
    pub fn add_step(mut self, step: impl Into<String>) -> Self {
        self.instructions.steps.push(step.into());
        self
    }

    /// Add context
    pub fn add_context(mut self, context_type: ContextType, content: impl Into<String>) -> Self {
        self.context.push(ContextItem {
            context_type,
            content: content.into(),
            relevance: 1.0,
        });
        self
    }

    /// Add constraint
    pub fn add_constraint(mut self, constraint_type: ConstraintType, description: impl Into<String>, strict: bool) -> Self {
        self.constraints.push(Constraint {
            constraint_type,
            description: description.into(),
            strict,
        });
        self
    }

    /// Set output format
    pub fn with_output(mut self, output: OutputSpec) -> Self {
        self.output = Some(output);
        self
    }

    /// Build the prompt string
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        // Persona section
        if let Some(ref persona) = self.persona {
            parts.push(format!(
                "You are a {} {}{}.",
                persona.expertise.as_str(),
                persona.role,
                persona.tone.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default()
            ));
            if !persona.description.is_empty() {
                parts.push(format!("Description: {}", persona.description));
            }
            parts.push(String::new());
        }

        // Instructions section
        parts.push("## Task".to_string());
        parts.push(self.instructions.primary.clone());
        parts.push(String::new());

        if !self.instructions.steps.is_empty() {
            parts.push("## Steps".to_string());
            for (i, step) in self.instructions.steps.iter().enumerate() {
                parts.push(format!("{}. {}", i + 1, step));
            }
            parts.push(String::new());
        }

        if !self.instructions.requirements.is_empty() {
            parts.push("## Requirements".to_string());
            for req in &self.instructions.requirements {
                parts.push(format!("- {}", req));
            }
            parts.push(String::new());
        }

        // Context section
        if !self.context.is_empty() {
            parts.push("## Context".to_string());
            // Sort by relevance
            let mut sorted_ctx = self.context.clone();
            sorted_ctx.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

            for ctx in sorted_ctx {
                parts.push(format!("[{}] {}", ctx.context_type.as_str(), ctx.content));
            }
            parts.push(String::new());
        }

        // Constraints section
        if !self.constraints.is_empty() {
            parts.push("## Constraints".to_string());
            for constraint in &self.constraints {
                let prefix = if constraint.strict { "MUST" } else { "SHOULD" };
                parts.push(format!("{}: {}", prefix, constraint.description));
            }
            parts.push(String::new());
        }

        // Output section
        if let Some(ref output) = self.output {
            parts.push("## Output Format".to_string());
            parts.push(format!("Format: {}", output.format_type.as_str()));

            if !output.structure.required_fields.is_empty() {
                parts.push("Required fields:".to_string());
                for field in &output.structure.required_fields {
                    let desc = output.structure.field_descriptions.get(field).map(|d| format!(" - {}", d)).unwrap_or_default();
                    parts.push(format!("- {}{}", field, desc));
                }
            }

            if let Some(ref example) = output.example {
                parts.push(String::new());
                parts.push("## Example".to_string());
                parts.push(example.clone());
            }
        }

        parts.join("\n")
    }

    /// Build JSON representation
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// =============================================================================
// Preference-Based Prompt Optimizer (PREFPO-style)
// =============================================================================

/// A prompt candidate
#[derive(Debug, Clone)]
pub struct PromptCandidate {
    /// Candidate ID
    pub id: String,
    /// Prompt content
    pub content: String,
    /// Score (from evaluation)
    pub score: f32,
    /// Parent ID (for evolutionary tracking)
    pub parent_id: Option<String>,
    /// Generation number
    pub generation: u32,
}

/// Preference feedback
#[derive(Debug, Clone)]
pub struct PreferenceFeedback {
    /// Preferred candidate
    pub preferred: String,
    /// Rejected candidate
    pub rejected: String,
    /// Feedback text
    pub feedback: Option<String>,
}

/// Preference-based prompt optimizer
pub struct PreferencePromptOptimizer {
    /// Population of candidates
    population: Vec<PromptCandidate>,
    /// Configuration
    config: OptimizerConfig,
    /// Current generation
    generation: u32,
}

/// Optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Population size
    pub population_size: usize,
    /// Mutation rate
    pub mutation_rate: f32,
    /// Crossover rate
    pub crossover_rate: f32,
    /// Maximum generations
    pub max_generations: usize,
    /// Elite ratio (preserve top performers)
    pub elite_ratio: f32,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            mutation_rate: 0.1,
            crossover_rate: 0.3,
            max_generations: 50,
            elite_ratio: 0.1,
        }
    }
}

impl PreferencePromptOptimizer {
    /// Create a new optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            population: Vec::new(),
            config,
            generation: 0,
        }
    }

    /// Initialize population from base prompt
    pub fn initialize(&mut self, base_prompt: &str, num_variations: usize) {
        self.population.clear();
        self.generation = 0;

        // Add base prompt
        self.population.push(PromptCandidate {
            id: uuid_simple(),
            content: base_prompt.to_string(),
            score: 0.5,
            parent_id: None,
            generation: 0,
        });

        // Generate variations
        let variations = generate_variations(base_prompt, num_variations);
        for (i, var) in variations.into_iter().enumerate() {
            self.population.push(PromptCandidate {
                id: format!("gen{}_v{}", 0, i),
                content: var,
                score: 0.5,
                parent_id: Some(uuid_simple()),
                generation: 0,
            });
        }
    }

    /// Update with preference feedback
    pub fn update_with_preference(&mut self, feedback: PreferenceFeedback) {
        // Find and update scores
        for candidate in &mut self.population {
            if candidate.id == feedback.preferred {
                candidate.score += 0.1;
            } else if candidate.id == feedback.rejected {
                candidate.score -= 0.1;
            }
            candidate.score = candidate.score.clamp(0.0, 1.0);
        }

        // Generate new generation
        self.evolve();
    }

    /// Evolve the population
    fn evolve(&mut self) {
        self.generation += 1;

        // Sort by score
        self.population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Preserve elite
        let elite_count = (self.population.len() as f32 * self.config.elite_ratio).ceil() as usize;
        let elites: Vec<_> = self.population.drain(..elite_count).collect();

        // Generate new candidates through mutation/crossover
        let mut new_candidates = elites.clone();

        while new_candidates.len() < self.config.population_size {
            let new_candidate = if fastrand::f32() < self.config.crossover_rate {
                // Crossover
                self.crossover(&elites)
            } else {
                // Mutation
                let base_content = elites.first().map(|e| e.content.as_str()).unwrap_or("");
                self.mutate(base_content)
            };

            new_candidates.push(PromptCandidate {
                id: format!("gen{}_{}", self.generation, new_candidates.len()),
                content: new_candidate,
                score: 0.5,
                parent_id: elites.first().map(|e| e.id.clone()),
                generation: self.generation,
            });
        }

        self.population = new_candidates;
    }

    /// Crossover two prompts
    fn crossover(&self, parents: &[PromptCandidate]) -> String {
        if parents.len() < 2 {
            return parents.first().map(|p| p.content.clone()).unwrap_or_default();
        }

        let p1 = &parents[0].content;
        let p2 = &parents[1].content;

        // Simple split-point crossover
        let lines1: Vec<_> = p1.lines().collect();
        let lines2: Vec<_> = p2.lines().collect();

        if lines1.is_empty() || lines2.is_empty() {
            return p1.to_string();
        }

        let split1 = fastrand::usize(1..lines1.len());
        let split2 = fastrand::usize(1..lines2.len());

        let mut result = lines1[..split1].to_vec();
        result.extend_from_slice(&lines2[split2..]);

        result.join("\n")
    }

    /// Mutate a prompt
    fn mutate(&self, base: &str) -> String {
        // Simple mutation: add/remove/rephrase a section
        let mutation_type = fastrand::usize(0..3);

        match mutation_type {
            0 => format!("{}\n\n[NEXT STEP]", base), // Add step
            1 => base.lines().filter(|l| !l.contains("IMPORTANT")).collect::<Vec<_>>().join("\n"), // Remove warning
            2 => format!("[REFINED] {}", base), // Add refinement marker
            _ => base.to_string(),
        }
    }

    /// Get best candidate
    pub fn best(&self) -> Option<&PromptCandidate> {
        self.population.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
    }

    /// Get top N candidates
    pub fn top_n(&self, n: usize) -> Vec<&PromptCandidate> {
        let mut sorted = self.population.clone();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        sorted.into_iter().take(n).collect()
    }

    /// Get statistics
    pub fn stats(&self) -> OptimizerStats {
        let scores: Vec<_> = self.population.iter().map(|p| p.score).collect();
        let avg = scores.iter().sum::<f32>() / scores.len() as f32;
        let max = scores.iter().cloned().fold(0.0f32, f32::max);
        let min = scores.iter().cloned().fold(1.0f32, f32::min);

        OptimizerStats {
            generation: self.generation,
            population_size: self.population.len(),
            best_score: max,
            average_score: avg,
            worst_score: min,
        }
    }
}

/// Optimizer statistics
#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub generation: u32,
    pub population_size: usize,
    pub best_score: f32,
    pub average_score: f32,
    pub worst_score: f32,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate prompt variations
fn generate_variations(base: &str, num: usize) -> Vec<String> {
    let mut variations = Vec::new();

    let variations_templates = vec![
        format!("[IMPORTANT] {}", base),
        format!("{}\n\nPlease be thorough.", base),
        base.replace("## Task", "## Primary Objective"),
        format!("As an expert, {}", base.to_lowercase()),
        format!("{}\n\nConsider all angles.", base),
    ];

    for i in 0..num.min(variations_templates.len()) {
        variations.push(variations_templates[i].clone());
    }

    variations
}

mod fastrand {
    pub fn f32() -> f32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        (nanos % 1000) as f32 / 1000.0
    }

    pub fn usize(range: std::ops::Range<usize>) -> usize {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        ((nanos as usize) % range.len()) + range.start
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picco_basic() {
        let prompt = PiccoPrompt::new("Analyze this data")
            .with_persona(Persona {
                role: "Data Analyst".to_string(),
                description: "Experienced in statistical analysis".to_string(),
                tone: Some("professional".to_string()),
                expertise: ExpertiseLevel::Expert,
            })
            .add_step("Load the data")
            .add_step("Clean and preprocess")
            .add_context(ContextType::Data, "Sales data from Q4 2025")
            .add_constraint(ConstraintType::Format, "Output as JSON", true)
            .build();

        assert!(prompt.contains("Data Analyst"));
        assert!(prompt.contains("Analyze this data"));
    }

    #[test]
    fn test_picco_to_json() {
        let prompt = PiccoPrompt::new("Test task");
        let json = prompt.to_json();
        assert!(json.contains("instructions"));
    }

    #[test]
    fn test_optimizer_initialization() {
        let mut optimizer = PreferencePromptOptimizer::new(OptimizerConfig::default());
        optimizer.initialize("Base prompt", 5);
        assert!(optimizer.population.len() == 6);
    }

    #[test]
    fn test_optimizer_best() {
        let mut optimizer = PreferencePromptOptimizer::new(OptimizerConfig::default());
        optimizer.initialize("Base prompt", 3);
        let best = optimizer.best();
        assert!(best.is_some());
    }
}
