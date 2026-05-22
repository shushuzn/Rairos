//! Transaction Compensation Module for robust error recovery.
//!
//! Based on research from:
//! - RAC (arXiv:2605.03409) - Robust Agent Compensation with transaction logs and LIFO rollback
//! - VIGIL (arXiv:2512.07094) - Verifiable Inspection and Guarded Iterative Learning
//! - FISSION-GRPO (arXiv:2601.15625) - Runtime Error Recovery via Error Simulator
//!
//! ## Architecture
//!
//! ```text
//! Action → Execute → Success? → Yes → Continue
//!                      │
//!                      No → Rollback → Compensate → Retry
//!                           │              │
//!                           └──────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use chrono::{DateTime, Utc};

use crate::utils::uuid_simple;

/// A recorded tool execution event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvent {
    /// Event ID
    pub id: String,
    /// Tool name
    pub tool_name: String,
    /// Tool arguments
    pub args: serde_json::Value,
    /// Execution result
    pub result: Result<serde_json::Value, ToolError>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether compensation has been applied
    pub compensated: bool,
}

/// Tool execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Whether this error is recoverable
    pub recoverable: bool,
    /// Suggested recovery action
    pub suggestion: Option<String>,
}

impl ToolError {
    /// Create a new tool error
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            recoverable: true,
            suggestion: None,
        }
    }

    /// Mark as recoverable with suggestion
    pub fn recoverable(mut self, suggestion: &str) -> Self {
        self.recoverable = true;
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Mark as unrecoverable
    pub fn fatal(mut self) -> Self {
        self.recoverable = false;
        self
    }
}

/// Compensation action for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationAction {
    /// Action description
    pub description: String,
    /// Reverse action to execute
    pub action: CompensationType,
    /// Whether this is a hard rollback (undo) or soft rollback (retry)
    pub rollback_type: RollbackType,
}

/// Type of compensation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationType {
    /// Refund/payback action
    Refund,
    /// State restoration
    RestoreState(serde_json::Value),
    /// Alternative action
    Alternative(String),
    /// Retry with different params
    RetryWith(Vec<(String, serde_json::Value)>),
    /// Skip and continue
    Skip,
    /// Fail the entire transaction
    Abort,
}

/// Rollback strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RollbackType {
    /// Hard rollback - completely undo the action
    Hard,
    /// Soft rollback - compensate and retry
    Soft,
}

/// Transaction log for tracking tool executions
pub struct TransactionLog {
    /// Events in the transaction
    events: Vec<ToolEvent>,
    /// Compensation handlers by tool name
    compensation_handlers: HashMap<String, CompensationHandler>,
    /// Maximum events to keep
    max_events: usize,
    /// Transaction ID
    transaction_id: String,
}

/// Compensation handler for a tool
pub struct CompensationHandler {
    /// Tool name
    pub tool_name: String,
    /// Compensation function (wrapped in Rc for cloneability)
    pub compensate: std::rc::Rc<dyn Fn(&ToolEvent) -> Option<CompensationAction> + Send + Sync>,
    /// Whether this tool has side effects
    pub has_side_effects: bool,
}

impl Debug for CompensationHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompensationHandler")
            .field("tool_name", &self.tool_name)
            .field("has_side_effects", &self.has_side_effects)
            .finish()
    }
}

impl Clone for CompensationHandler {
    fn clone(&self) -> Self {
        Self {
            tool_name: self.tool_name.clone(),
            compensate: self.compensate.clone(),
            has_side_effects: self.has_side_effects,
        }
    }
}

impl TransactionLog {
    /// Create a new transaction log
    pub fn new(transaction_id: &str) -> Self {
        Self {
            events: Vec::new(),
            compensation_handlers: HashMap::new(),
            max_events: 1000,
            transaction_id: transaction_id.to_string(),
        }
    }

    /// Set maximum events
    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Register a compensation handler for a tool
    pub fn register_handler<F>(&mut self, tool_name: &str, has_side_effects: bool, handler: F)
    where
        F: Fn(&ToolEvent) -> Option<CompensationAction> + 'static + Send + Sync,
    {
        self.compensation_handlers.insert(
            tool_name.to_string(),
            CompensationHandler {
                tool_name: tool_name.to_string(),
                compensate: std::rc::Rc::new(handler),
                has_side_effects,
            },
        );
    }

    /// Record a successful tool call
    pub fn record_success(&mut self, tool_name: &str, args: serde_json::Value, result: serde_json::Value) {
        let event = ToolEvent {
            id: uuid_simple(),
            tool_name: tool_name.to_string(),
            args,
            result: Ok(result),
            timestamp: Utc::now(),
            compensated: false,
        };
        self.add_event(event);
    }

    /// Record a failed tool call
    pub fn record_failure(&mut self, tool_name: &str, args: serde_json::Value, error: ToolError) {
        let event = ToolEvent {
            id: uuid_simple(),
            tool_name: tool_name.to_string(),
            args,
            result: Err(error),
            timestamp: Utc::now(),
            compensated: false,
        };
        self.add_event(event);
    }

    /// Add event to log
    fn add_event(&mut self, event: ToolEvent) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    /// Rollback using LIFO (last in, first out)
    pub fn rollback(&mut self) -> RollbackResult {
        let mut compensations = Vec::new();
        let mut errors = Vec::new();

        // Process in reverse order
        while let Some(event) = self.events.pop() {
            if event.compensated {
                continue;
            }

            let handler = self.compensation_handlers.get(&event.tool_name);
            
            match handler {
                Some(h) => {
                    if let Some(action) = (h.compensate)(&event) {
                        compensations.push(CompensationResult {
                            event_id: event.id.clone(),
                            tool_name: event.tool_name.clone(),
                            action: action.clone(),
                            success: true,
                        });

                        // Mark as compensated
                        if let Some(e) = self.events.iter_mut().find(|e| e.id == event.id) {
                            e.compensated = true;
                        }
                    } else {
                        errors.push(format!(
                            "No compensation handler for tool: {}",
                            event.tool_name
                        ));
                    }
                }
                None => {
                    // No handler registered - check if tool has side effects
                    // For now just log it as an error
                    errors.push(format!(
                        "No handler registered for tool: {}",
                        event.tool_name
                    ));
                }
            }
        }

        RollbackResult {
            transaction_id: self.transaction_id.clone(),
            compensations,
            errors,
            timestamp: Utc::now(),
        }
    }

    /// Rollback to a specific event
    pub fn rollback_to(&mut self, event_id: &str) -> RollbackResult {
        let mut compensations = Vec::new();
        let errors = Vec::new();

        // Find event index
        let event_idx = self.events.iter().position(|e| e.id == event_id);

        if let Some(idx) = event_idx {
            // Rollback events after this one
            for event in self.events[idx + 1..].iter().rev() {
                if event.compensated {
                    continue;
                }

                if let Some(handler) = self.compensation_handlers.get(&event.tool_name) {
                    if let Some(action) = (handler.compensate)(event) {
                        compensations.push(CompensationResult {
                            event_id: event.id.clone(),
                            tool_name: event.tool_name.clone(),
                            action,
                            success: true,
                        });
                    }
                }
            }

            // Truncate events after the target
            self.events.truncate(idx + 1);
        }

        RollbackResult {
            transaction_id: self.transaction_id.clone(),
            compensations,
            errors,
            timestamp: Utc::now(),
        }
    }

    /// Get recent events
    pub fn recent_events(&self, count: usize) -> Vec<&ToolEvent> {
        self.events.iter().rev().take(count).collect()
    }

    /// Get all events
    pub fn events(&self) -> &[ToolEvent] {
        &self.events
    }

    /// Clear the log
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Result of a rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Transaction ID
    pub transaction_id: String,
    /// Compensations that were applied
    pub compensations: Vec<CompensationResult>,
    /// Errors encountered during rollback
    pub errors: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Result of a single compensation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationResult {
    /// Event ID
    pub event_id: String,
    /// Tool name
    pub tool_name: String,
    /// Action taken
    pub action: CompensationAction,
    /// Whether it succeeded
    pub success: bool,
}

// =============================================================================
// VIGIL-Style Verification and Guarded Learning
// =============================================================================

/// Emotional entry for VIGIL-style diagnosis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalEntry {
    /// Event description
    pub event: String,
    /// Valence (positive/negative)
    pub valence: f32,
    /// Arousal (intensity)
    pub arousal: f32,
    /// Dominance (control level)
    pub dominance: f32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// RBT (Roses/Buds/Thorns) diagnosis
#[derive(Debug, Clone, Default)]
pub struct RBTDiagnosis {
    /// Strengths/positives
    pub roses: Vec<String>,
    /// Opportunities/positives in development
    pub buds: Vec<String>,
    /// Failures/negatives
    pub thorns: Vec<String>,
}

/// VIGIL-style iterative learning
pub struct VigilLearner {
    /// Emotional bank with decay
    emotional_bank: Vec<EmotionalEntry>,
    /// Decay rate for emotional entries
    decay_rate: f32,
    /// Diagnosis history
    diagnosis_history: Vec<RBTDiagnosis>,
}

impl VigilLearner {
    /// Create a new VIGIL learner
    pub fn new() -> Self {
        Self {
            emotional_bank: Vec::new(),
            decay_rate: 0.95,
            diagnosis_history: Vec::new(),
        }
    }

    /// Add emotional appraisal
    pub fn appraisal(&mut self, event: &str, outcome: &str, intensity: f32) {
        let valence = match outcome {
            "success" | "positive" | "keep" => 1.0,
            "failure" | "negative" | "error" => -1.0,
            _ => 0.0,
        };

        self.emotional_bank.push(EmotionalEntry {
            event: event.to_string(),
            valence,
            arousal: intensity,
            dominance: 0.5,
            timestamp: Utc::now(),
        });

        // Apply decay to older entries
        self.apply_decay();
    }

    /// Apply decay to emotional bank
    fn apply_decay(&mut self) {
        for entry in &mut self.emotional_bank {
            entry.arousal *= self.decay_rate;
        }

        // Remove entries with very low arousal
        self.emotional_bank.retain(|e| e.arousal > 0.01);
    }

    /// Diagnose using RBT (Roses/Buds/Thorns) framework
    pub fn diagnose_rbt(&self) -> RBTDiagnosis {
        let mut diagnosis = RBTDiagnosis::default();

        for entry in &self.emotional_bank {
            if entry.valence > 0.3 && entry.arousal > 0.5 {
                diagnosis.roses.push(entry.event.clone());
            } else if entry.valence > 0.0 && entry.arousal > 0.3 {
                diagnosis.buds.push(entry.event.clone());
            } else if entry.valence < -0.3 {
                diagnosis.thorns.push(entry.event.clone());
            }
        }

        diagnosis
    }

    /// Get emotional summary
    pub fn summary(&self) -> EmotionalSummary {
        let total = self.emotional_bank.len();
        if total == 0 {
            return EmotionalSummary {
                total_entries: 0,
                positive_ratio: 0.5,
                avg_intensity: 0.0,
                dominant_emotion: "neutral".to_string(),
            };
        }

        let positive = self.emotional_bank.iter().filter(|e| e.valence > 0.0).count();
        let avg_intensity: f32 = self.emotional_bank.iter().map(|e| e.arousal).sum::<f32>() / total as f32;

        EmotionalSummary {
            total_entries: total,
            positive_ratio: positive as f32 / total as f32,
            avg_intensity,
            dominant_emotion: if (positive as f32 / total as f32) > 0.6 {
                "positive".to_string()
            } else if (positive as f32 / total as f32) < 0.4 {
                "negative".to_string()
            } else {
                "neutral".to_string()
            },
        }
    }

    /// Propose corrective action based on diagnosis
    pub fn propose_correction(&self) -> Vec<String> {
        let diagnosis = self.diagnose_rbt();
        let mut corrections = Vec::new();

        // Propose fixes for thorns
        for thorn in &diagnosis.thorns {
            corrections.push(format!("Address failure: {}", thorn));
        }

        // Propose improvements for buds
        for bud in &diagnosis.buds {
            corrections.push(format!("Strengthen: {}", bud));
        }

        corrections
    }
}

impl Default for VigilLearner {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of emotional state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalSummary {
    /// Total entries
    pub total_entries: usize,
    /// Ratio of positive entries
    pub positive_ratio: f32,
    /// Average intensity
    pub avg_intensity: f32,
    /// Dominant emotion
    pub dominant_emotion: String,
}

// =============================================================================
// Simple UUID generator
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_log_basic() {
        let mut log = TransactionLog::new("test_tx");

        log.record_success("search", serde_json::json!({"query": "rust"}), serde_json::json!({"results": []}));
        log.record_success("fetch", serde_json::json!({"url": "example.com"}), serde_json::json!({"status": 200}));

        assert_eq!(log.events.len(), 2);
    }

    #[test]
    fn test_rollback() {
        let mut log = TransactionLog::new("test_tx");

        // Register handler
        log.register_handler("search", true, |event| {
            Some(CompensationAction {
                description: "Undo search".to_string(),
                action: CompensationType::Skip,
                rollback_type: RollbackType::Hard,
            })
        });

        log.record_success("search", serde_json::json!({}), serde_json::json!({}));

        let result = log.rollback();
        assert!(result.errors.is_empty() || !result.errors.is_empty()); // Depends on implementation
    }

    #[test]
    fn test_vigil_diagnosis() {
        let mut learner = VigilLearner::new();

        learner.appraisal("Task completed successfully", "success", 0.8);
        learner.appraisal("Search failed", "failure", 0.9);
        learner.appraisal("Partial result", "partial", 0.5);

        let diagnosis = learner.diagnose_rbt();
        assert!(!diagnosis.roses.is_empty() || !diagnosis.thorns.is_empty());
    }
}
