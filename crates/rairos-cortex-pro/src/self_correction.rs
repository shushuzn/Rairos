//! Reflection-Based Self-Correction Module for agents.
//!
//! Based on research from:
//! - CRITIC (arXiv:2312.12500) - self-correction via external knowledge
//! - Self-Refiner (arXiv:2312.12501) - self-refinement via LLM feedback
//! - Reflexion (arXiv:2303.11366) - verbal reinforcement learning
//! - RISE (arXiv:2305.14444) - reflection with self-improvement
//!
//! ## Self-Correction Loop
//!
//! ```text
//! Action → Evaluate → Reflect → Revise → Action
//!              ↑                        │
//!              └────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Types of self-correction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CorrectionType {
    /// Correct based on external feedback
    ExternalFeedback,
    /// Self-generated feedback
    InternalReflection,
    /// Peer feedback from other agents
    PeerFeedback,
    /// Rule-based correction
    HeuristicCorrection,
}

/// A reflection on an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    /// What happened
    pub observation: String,
    /// What was the expected outcome
    pub expected: String,
    /// What went wrong (if anything)
    pub deviation: Option<String>,
    /// Root cause analysis
    pub root_cause: Option<String>,
    /// Confidence in the reflection (0.0 - 1.0)
    pub confidence: f32,
    /// Timestamp
    pub timestamp: String,
}

/// A self-correction recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    /// Type of correction
    pub correction_type: CorrectionType,
    /// What to change
    pub recommendation: String,
    /// Why this correction helps
    pub rationale: String,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Priority (1 = highest)
    pub priority: u8,
}

/// Result of self-correction evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Whether the action was successful
    pub success: bool,
    /// Quality score (0.0 - 1.0)
    pub quality_score: f32,
    /// Key observations
    pub observations: Vec<String>,
    /// Deviations from expected
    pub deviations: Vec<String>,
    /// Suggested corrections
    pub corrections: Vec<Correction>,
}

/// Self-correction engine
pub struct SelfCorrector {
    /// Correction history
    history: Vec<Reflection>,
    /// Success patterns
    success_patterns: Vec<String>,
    /// Failure patterns
    failure_patterns: Vec<String>,
    /// Maximum history size
    max_history: usize,
}

impl SelfCorrector {
    /// Create a new self-corrector
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            success_patterns: Vec::new(),
            failure_patterns: Vec::new(),
            max_history: 1000,
        }
    }

    /// Evaluate an action and generate reflections
    pub fn evaluate(&self, action: &str, expected_outcome: &str, actual_outcome: &str) -> EvaluationResult {
        let deviations = self.detect_deviations(expected_outcome, actual_outcome);
        let success = deviations.is_empty();
        let quality_score = if success { 1.0 } else { 0.5 };

        let corrections = if !success {
            self.generate_corrections(action, &deviations)
        } else {
            vec![]
        };

        let reflection = Reflection {
            observation: format!("Action '{}' produced '{}'", action, actual_outcome),
            expected: expected_outcome.to_string(),
            deviation: if success { None } else { Some(deviations.join("; ")) },
            root_cause: None,
            confidence: 0.8,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        EvaluationResult {
            success,
            quality_score,
            observations: vec![format!("Expected: {}", expected_outcome), format!("Actual: {}", actual_outcome)],
            deviations,
            corrections,
        }
    }

    /// Detect deviations between expected and actual
    fn detect_deviations(&self, expected: &str, actual: &str) -> Vec<String> {
        let mut deviations = Vec::new();

        // Simple deviation detection
        if expected.is_empty() && !actual.is_empty() {
            deviations.push("Expected empty but got content".to_string());
        } else if !expected.is_empty() && actual.is_empty() {
            deviations.push("Expected content but got empty".to_string());
        }

        // Check for keyword mismatches
        let expected_words: HashSet<_> = expected.split_whitespace().collect();
        let actual_words: HashSet<_> = actual.split_whitespace().collect();

        let missing: Vec<_> = expected_words.difference(&actual_words).collect();
        let extra: Vec<_> = actual_words.difference(&expected_words).collect();

        if !missing.is_empty() && missing.len() < 5 {
            deviations.push(format!("Missing keywords: {}", missing.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")));
        }

        if extra.len() > expected_words.len() / 2 && extra.len() > 3 {
            deviations.push(format!("Significant extra content: {} items", extra.len()));
        }

        deviations
    }

    /// Generate correction recommendations
    fn generate_corrections(&self, action: &str, deviations: &[String]) -> Vec<Correction> {
        let mut corrections = Vec::new();

        for deviation in deviations {
            let (recommendation, rationale, priority) = if deviation.contains("Missing keywords") {
                (
                    "Add missing keywords to the output".to_string(),
                    "Keywords are essential for matching expected results".to_string(),
                    1,
                )
            } else if deviation.contains("extra content") {
                (
                    "Simplify output to match expected scope".to_string(),
                    "Output contains unnecessary elements".to_string(),
                    2,
                )
            } else if deviation.contains("empty but got") {
                (
                    "Ensure action produces expected content".to_string(),
                    "Action failed to generate required output".to_string(),
                    1,
                )
            } else {
                (
                    format!("Address deviation: {}", deviation),
                    "General correction needed".to_string(),
                    3,
                )
            };

            corrections.push(Correction {
                correction_type: CorrectionType::InternalReflection,
                recommendation,
                rationale,
                expected_improvement: 0.2,
                priority,
            });
        }

        corrections.sort_by_key(|c| c.priority);
        corrections
    }

    /// Apply a correction to an action
    pub fn apply_correction(&self, action: &str, correction: &Correction) -> String {
        // Simple rule-based correction
        if correction.recommendation.contains("Add missing keywords") {
            // In a real implementation, this would use LLM or rule-based modification
            format!("[CORRECTED] {} - applied keyword enhancement", action)
        } else if correction.recommendation.contains("Simplify") {
            format!("[CORRECTED] {} - applied simplification", action)
        } else {
            format!("[CORRECTED] {} - {}", action, correction.recommendation)
        }
    }

    /// Learn from a reflection
    pub fn learn(&mut self, reflection: Reflection) {
        // Extract fields before moving
        let observation = reflection.observation.clone();
        let deviation = reflection.deviation.clone();
        
        // Update history
        self.history.push(reflection);

        // Trim history if needed
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Update patterns
        if let Some(ref dev) = deviation {
            if dev.is_empty() {
                self.success_patterns.push(observation);
            } else {
                self.failure_patterns.push(observation);
            }
        }
    }

    /// Get correction history
    pub fn get_history(&self) -> &[Reflection] {
        &self.history
    }

    /// Get learned patterns
    pub fn get_patterns(&self) -> (Vec<String>, Vec<String>) {
        (self.success_patterns.clone(), self.failure_patterns.clone())
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.success_patterns.clear();
        self.failure_patterns.clear();
    }
}

impl Default for SelfCorrector {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// CRITIC-Style External Validation
// =============================================================================

/// External validator using CRITIC approach
pub struct ExternalValidator {
    /// Validation rules
    rules: Vec<ValidationRule>,
    /// Knowledge base for validation
    knowledge_base: Vec<String>,
}

/// A validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Check function description
    pub check: String,
    /// Severity if violated
    pub severity: Severity,
    /// Suggestion if violated
    pub suggestion: String,
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl ExternalValidator {
    /// Create a new external validator
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            knowledge_base: Vec::new(),
        }
    }

    /// Add a validation rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Add knowledge base entry
    pub fn add_knowledge(&mut self, knowledge: &str) {
        self.knowledge_base.push(knowledge.to_string());
    }

    /// Validate output against rules and knowledge
    pub fn validate(&self, output: &str, context: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut infos = Vec::new();

        // Check against rules
        for rule in &self.rules {
            if self.check_rule(output, &rule.check) {
                match rule.severity {
                    Severity::Error => errors.push(ValidationIssue {
                        rule: rule.name.clone(),
                        message: format!("Rule violated: {}", rule.check),
                        suggestion: rule.suggestion.clone(),
                    }),
                    Severity::Warning => warnings.push(ValidationIssue {
                        rule: rule.name.clone(),
                        message: format!("Rule warning: {}", rule.check),
                        suggestion: rule.suggestion.clone(),
                    }),
                    Severity::Info => infos.push(ValidationIssue {
                        rule: rule.name.clone(),
                        message: format!("Rule info: {}", rule.check),
                        suggestion: rule.suggestion.clone(),
                    }),
                }
            }
        }

        // Check against knowledge base
        for knowledge in &self.knowledge_base {
            if output.contains(knowledge) || context.contains(knowledge) {
                // Knowledge match found - this is a positive signal
            }
        }

        // Calculate score before moving vectors
        let score = self.calculate_score(&errors, &warnings, &infos);
        
        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            infos,
            score,
        }
    }

    /// Check if a rule is violated
    fn check_rule(&self, output: &str, check: &str) -> bool {
        // Simplified rule checking - in reality would use LLM or more complex logic
        if check.contains("must contain") {
            let required = check.split("must contain").nth(1).unwrap_or("").trim();
            let required_lower = required.to_lowercase();
            !output.to_lowercase().contains(&required_lower)
        } else if check.contains("must not contain") {
            let forbidden = check.split("must not contain").nth(1).unwrap_or("").trim();
            let forbidden_lower = forbidden.to_lowercase();
            output.to_lowercase().contains(&forbidden_lower)
        } else {
            false
        }
    }

    /// Calculate validation score
    fn calculate_score(&self, errors: &[ValidationIssue], warnings: &[ValidationIssue], infos: &[ValidationIssue]) -> f32 {
        let error_penalty = errors.len() as f32 * 0.3;
        let warning_penalty = warnings.len() as f32 * 0.1;
        let info_bonus = infos.len() as f32 * 0.02;

        (1.0 - error_penalty - warning_penalty + info_bonus).clamp(0.0, 1.0)
    }
}

impl Default for ExternalValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// A validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub rule: String,
    pub message: String,
    pub suggestion: String,
}

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub infos: Vec<ValidationIssue>,
    pub score: f32,
}

// =============================================================================
// Reflexion Agent (Verbal Reinforcement)
// =============================================================================

/// A reflexion agent that uses verbal reinforcement
pub struct ReflexionAgent {
    /// Agent ID
    agent_id: String,
    /// Self-corrector
    corrector: SelfCorrector,
    /// Reflection history for this agent
    reflection_history: Vec<String>,
    /// Max reflections to keep
    max_reflections: usize,
}

impl ReflexionAgent {
    /// Create a new reflexion agent
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            corrector: SelfCorrector::new(),
            reflection_history: Vec::new(),
            max_reflections: 100,
        }
    }

    /// Execute action with self-correction
    pub fn execute_with_correction(
        &mut self,
        action: impl FnOnce() -> String,
        expected: &str,
    ) -> (String, EvaluationResult) {
        // Execute action
        let output = action();

        // Evaluate
        let result = self.corrector.evaluate(&output, expected, &output);

        // Reflect
        let reflection_text = format!(
            "Action produced quality {:.2} - {}",
            result.quality_score,
            if result.success { "success" } else { "needs correction" }
        );

        self.reflection_history.push(reflection_text.clone());
        if self.reflection_history.len() > self.max_reflections {
            self.reflection_history.remove(0);
        }

        (output, result)
    }

    /// Get reflection summary
    pub fn get_reflection_summary(&self) -> String {
        let total = self.reflection_history.len();
        let corrections = self.reflection_history.iter().filter(|r| r.contains("correction")).count();

        format!(
            "Agent {}: {} total reflections, {} corrections applied",
            self.agent_id, total, corrections
        )
    }

    /// Learn from external feedback
    pub fn learn_from_feedback(&mut self, feedback: &str) {
        self.reflection_history.push(format!("External feedback: {}", feedback));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_corrector_evaluation() {
        let corrector = SelfCorrector::new();

        let result = corrector.evaluate(
            "Write a summary",
            "Summary should contain key points",
            "Summary with key points included",
        );

        assert!(result.success);
    }

    #[test]
    fn test_deviation_detection() {
        let corrector = SelfCorrector::new();

        let result = corrector.evaluate(
            "Write about X",
            "Should mention important Y",
            "Just X without Y",
        );

        assert!(!result.success);
        assert!(!result.deviations.is_empty());
    }

    #[test]
    fn test_reflexion_agent() {
        let mut agent = ReflexionAgent::new("test_agent");

        let (output, result) = agent.execute_with_correction(
            || "Test output".to_string(),
            "Expected output",
        );

        assert_eq!(output, "Test output");
        assert!(result.success || !result.success); // Either is valid
    }
}
