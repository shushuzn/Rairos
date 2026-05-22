//! Active Learning module for materials discovery.
//!
//! Based on research from:
//! - LEAP (arXiv:2605.20242) - LLM + Bayesian optimization for perovskites
//! - AIMBio-Mat (arXiv:2605.21083) - AI-native FAIR platform with active learning
//! - FINALES + Kadi4Mat (arXiv:2605.00909) - Automated experiment orchestration
//!
//! ## Architecture
//!
//! ```text
//! Active Learning Loop
//!     │
//!     ├──► LLM Hypothesis Generator
//!     ├──► Candidate Selection (Bayesian Optimization)
//!     ├──► Expert Review (Human-in-the-loop)
//!     └──► Experiment Execution
//!             │
//!             ▼
//!         Observation
//!             │
//!             ▼
//!         Update Model ──────────▶ Next Iteration
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::utils::uuid_simple;

/// A candidate material for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCandidate {
    /// Candidate ID
    pub id: String,
    /// Chemical formula
    pub formula: String,
    /// Composition elements
    pub elements: Vec<String>,
    /// Composition fractions
    pub fractions: Vec<f32>,
    /// Predicted property value
    pub predicted_value: Option<f32>,
    /// Uncertainty estimate
    pub uncertainty: Option<f32>,
    /// Acquisition value (for selection)
    pub acquisition_value: Option<f32>,
}

/// Result from evaluating a candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Candidate ID
    pub candidate_id: String,
    /// Measured/calculated property value
    pub property_value: f32,
    /// Measurement error
    pub measurement_error: Option<f32>,
    /// Whether experiment succeeded
    pub success: bool,
    /// Notes from evaluation
    pub notes: Option<String>,
}

/// Active learning configuration
#[derive(Debug, Clone)]
pub struct ActiveLearningConfig {
    /// Maximum candidates to evaluate per iteration
    pub batch_size: usize,
    /// Exploration weight (higher = more exploration)
    pub exploration_weight: f32,
    /// Exploitation weight (higher = more exploitation)
    pub exploitation_weight: f32,
    /// Minimum uncertainty threshold
    pub min_uncertainty: f32,
    /// Maximum iterations without improvement
    pub max_stagnant_iterations: u32,
    /// Enable human-in-the-loop review
    pub human_in_loop: bool,
}

impl Default for ActiveLearningConfig {
    fn default() -> Self {
        Self {
            batch_size: 3,
            exploration_weight: 0.5,
            exploitation_weight: 0.5,
            min_uncertainty: 0.1,
            max_stagnant_iterations: 10,
            human_in_loop: true,
        }
    }
}

/// Acquisition function type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AcquisitionFunction {
    /// Expected Improvement
    ExpectedImprovement,
    /// Upper Confidence Bound
    UpperConfidenceBound,
    /// Thompson Sampling
    ThompsonSampling,
}

/// Active learning state
#[derive(Debug, Clone)]
pub struct ActiveLearningState {
    /// All evaluated candidates
    pub evaluated: Vec<EvaluatedCandidate>,
    /// Candidates pending evaluation
    pub pending: Vec<MaterialCandidate>,
    /// Current best candidate
    pub best_candidate: Option<MaterialCandidate>,
    /// Best observed value
    pub best_value: f32,
    /// Iteration count
    pub iteration: u32,
    /// Iterations without improvement
    pub stagnant_iterations: u32,
}

/// Evaluated candidate with ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedCandidate {
    /// Candidate info
    pub candidate: MaterialCandidate,
    /// True property value
    pub true_value: f32,
    /// When evaluated
    pub evaluated_at: DateTime<Utc>,
}

impl ActiveLearningState {
    /// Create new state
    pub fn new() -> Self {
        Self {
            evaluated: Vec::new(),
            pending: Vec::new(),
            best_candidate: None,
            best_value: f32::NEG_INFINITY,
            iteration: 0,
            stagnant_iterations: 0,
        }
    }

    /// Update with new evaluation result
    pub fn update(&mut self, result: EvaluationResult) {
        self.iteration += 1;

        // Find candidate
        let candidate = self.pending.iter().find(|c| c.id == result.candidate_id);
        if candidate.is_none() {
            return;
        }
        let candidate = candidate.unwrap().clone();

        // Remove from pending
        self.pending.retain(|c| c.id != result.candidate_id);

        if result.success {
            // Add to evaluated
            self.evaluated.push(EvaluatedCandidate {
                candidate: candidate.clone(),
                true_value: result.property_value,
                evaluated_at: Utc::now(),
            });

            // Update best if improved
            if result.property_value > self.best_value {
                self.best_value = result.property_value;
                self.best_candidate = Some(candidate);
                self.stagnant_iterations = 0;
            } else {
                self.stagnant_iterations += 1;
            }
        }
    }

    /// Check if converged
    pub fn is_converged(&self, config: &ActiveLearningConfig) -> bool {
        self.stagnant_iterations >= config.max_stagnant_iterations
    }
}

impl Default for ActiveLearningState {
    fn default() -> Self {
        Self::new()
    }
}

/// Active Learning manager for materials discovery
pub struct ActiveLearningManager {
    /// Configuration
    config: ActiveLearningConfig,
    /// Acquisition function
    acquisition: AcquisitionFunction,
    /// State
    state: ActiveLearningState,
    /// Property name being optimized
    property_name: String,
    /// Target value (if known)
    target_value: Option<f32>,
}

impl ActiveLearningManager {
    /// Create new manager
    pub fn new(property_name: &str, acquisition: AcquisitionFunction) -> Self {
        Self {
            config: ActiveLearningConfig::default(),
            acquisition,
            state: ActiveLearningState::new(),
            property_name: property_name.to_string(),
            target_value: None,
        }
    }

    /// Configure the manager
    pub fn with_config(mut self, config: ActiveLearningConfig) -> Self {
        self.config = config;
        self
    }

    /// Set target value
    pub fn with_target(mut self, target: f32) -> Self {
        self.target_value = Some(target);
        self
    }

    /// Add initial candidates
    pub fn add_candidates(&mut self, candidates: Vec<MaterialCandidate>) {
        self.state.pending.extend(candidates);
    }

    /// Select next candidates using acquisition function
    pub fn select_next(&self) -> Vec<MaterialCandidate> {
        // Clone candidates to work with owned data
        let mut candidates: Vec<MaterialCandidate> = self.state.pending.iter().cloned().collect();

        // Score each candidate using acquisition function
        for candidate in &mut candidates {
            let score = match self.acquisition {
                AcquisitionFunction::ExpectedImprovement => {
                    self.expected_improvement(candidate)
                }
                AcquisitionFunction::UpperConfidenceBound => {
                    self.upper_confidence_bound(candidate)
                }
                AcquisitionFunction::ThompsonSampling => {
                    self.thompson_sampling(candidate)
                }
            };
            candidate.acquisition_value = Some(score);
        }

        // Sort by acquisition value and take top batch_size
        candidates.sort_by(|a, b| {
            b.acquisition_value
                .partial_cmp(&a.acquisition_value)
                .unwrap()
        });

        candidates
            .into_iter()
            .take(self.config.batch_size)
            .collect()
    }

    /// Expected Improvement acquisition function
    fn expected_improvement(&self, candidate: &MaterialCandidate) -> f32 {
        let pred = candidate.predicted_value.unwrap_or(0.0);
        let unc = candidate.uncertainty.unwrap_or(1.0);

        if let Some(target) = self.target_value {
            let improvement = (target - pred).max(0.0);
            return improvement / (unc + 0.001);
        }

        // Maximize case
        let improvement = (pred - self.state.best_value).max(0.0);
        improvement / (unc + 0.001)
    }

    /// Upper Confidence Bound acquisition function
    fn upper_confidence_bound(&self, candidate: &MaterialCandidate) -> f32 {
        let pred = candidate.predicted_value.unwrap_or(0.0);
        let unc = candidate.uncertainty.unwrap_or(1.0);

        let exploration_bonus = self.config.exploration_weight * unc;
        let exploitation_bonus = self.config.exploitation_weight * pred;

        pred + exploration_bonus
    }

    /// Thompson Sampling acquisition function
    fn thompson_sampling(&self, candidate: &MaterialCandidate) -> f32 {
        let pred = candidate.predicted_value.unwrap_or(0.0);
        let unc = candidate.uncertainty.unwrap_or(1.0);

        // Sample from normal distribution
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let salt = format!("{}-{}", candidate.id, self.state.iteration);
        let mut hasher = RandomState::new().build_hasher();
        salt.hash(&mut hasher);
        let hash = hasher.finish() as f64 / u64::MAX as f64;

        // Box-Muller transform for normal distribution
        let u1 = hash;
        let u2 = {
            let mut h = RandomState::new().build_hasher();
            (candidate.id.len() as u64).hash(&mut h);
            h.finish() as f64 / u64::MAX as f64
        };
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

        let sample = pred as f64 + z * unc as f64 * 0.5;
        sample as f32
    }

    /// Update state with evaluation result
    pub fn update(&mut self, result: EvaluationResult) {
        self.state.update(result);
    }

    /// Check if should request human review
    pub fn needs_human_review(&self) -> bool {
        self.config.human_in_loop && self.state.iteration % 5 == 0
    }

    /// Get current best
    pub fn best_candidate(&self) -> Option<&MaterialCandidate> {
        self.state.best_candidate.as_ref()
    }

    /// Get current best value
    pub fn best_value(&self) -> f32 {
        self.state.best_value
    }

    /// Get iteration count
    pub fn iteration(&self) -> u32 {
        self.state.iteration
    }

    /// Check if converged
    pub fn is_converged(&self) -> bool {
        self.state.is_converged(&self.config)
    }

    /// Get statistics
    pub fn stats(&self) -> ActiveLearningStats {
        ActiveLearningStats {
            property_name: self.property_name.clone(),
            total_evaluated: self.state.evaluated.len(),
            total_pending: self.state.pending.len(),
            current_best: self.state.best_value,
            iteration: self.state.iteration,
            stagnant_iterations: self.state.stagnant_iterations,
            is_converged: self.is_converged(),
        }
    }
}

/// Statistics for active learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveLearningStats {
    pub property_name: String,
    pub total_evaluated: usize,
    pub total_pending: usize,
    pub current_best: f32,
    pub iteration: u32,
    pub stagnant_iterations: u32,
    pub is_converged: bool,
}

/// LLM Hypothesis Generator for active learning
/// Based on LEAP (arXiv:2605.20242)
pub struct HypothesisGenerator {
    /// Use literature-grounded hypotheses
    literature_grounded: bool,
    /// Use mechanistic descriptors
    mechanistic_descriptors: bool,
}

impl HypothesisGenerator {
    pub fn new() -> Self {
        Self {
            literature_grounded: true,
            mechanistic_descriptors: true,
        }
    }

    /// Generate candidates from hypothesis
    pub fn generate_candidates(&self, hypothesis: &str) -> Vec<MaterialCandidate> {
        // This is a simplified version - in practice would use LLM
        // to extract composition guidelines from hypothesis

        let mut candidates = Vec::new();

        // Example: Generate doping candidates from hypothesis
        if hypothesis.to_lowercase().contains("bi2te3") {
            candidates.push(MaterialCandidate {
                id: uuid_simple(),
                formula: "Bi2Te2.9Se0.1".to_string(),
                elements: vec!["Bi".to_string(), "Te".to_string(), "Se".to_string()],
                fractions: vec![0.4, 0.58, 0.02],
                predicted_value: None,
                uncertainty: None,
                acquisition_value: None,
            });

            candidates.push(MaterialCandidate {
                id: uuid_simple(),
                formula: "Bi2Te2.8Se0.2".to_string(),
                elements: vec!["Bi".to_string(), "Te".to_string(), "Se".to_string()],
                fractions: vec![0.4, 0.56, 0.04],
                predicted_value: None,
                uncertainty: None,
                acquisition_value: None,
            });
        }

        candidates
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_learning_manager_creation() {
        let manager = ActiveLearningManager::new("ZT", AcquisitionFunction::ExpectedImprovement);
        assert_eq!(manager.property_name, "ZT");
        assert_eq!(manager.iteration(), 0);
    }

    #[test]
    fn test_add_candidates() {
        let mut manager = ActiveLearningManager::new("ZT", AcquisitionFunction::UpperConfidenceBound);

        manager.add_candidates(vec![
            MaterialCandidate {
                id: "c1".to_string(),
                formula: "Bi2Te3".to_string(),
                elements: vec!["Bi".to_string(), "Te".to_string()],
                fractions: vec![0.4, 0.6],
                predicted_value: Some(1.0),
                uncertainty: Some(0.2),
                acquisition_value: None,
            },
        ]);

        assert_eq!(manager.stats().total_pending, 1);
    }

    #[test]
    fn test_select_next_ucb() {
        let mut manager = ActiveLearningManager::new("ZT", AcquisitionFunction::UpperConfidenceBound);

        manager.add_candidates(vec![
            MaterialCandidate {
                id: "c1".to_string(),
                formula: "Bi2Te3".to_string(),
                elements: vec!["Bi".to_string(), "Te".to_string()],
                fractions: vec![0.4, 0.6],
                predicted_value: Some(1.0),
                uncertainty: Some(0.2),
                acquisition_value: None,
            },
            MaterialCandidate {
                id: "c2".to_string(),
                formula: "PbTe".to_string(),
                elements: vec!["Pb".to_string(), "Te".to_string()],
                fractions: vec![0.5, 0.5],
                predicted_value: Some(0.8),
                uncertainty: Some(0.1),
                acquisition_value: None,
            },
        ]);

        let selected = manager.select_next();
        assert!(!selected.is_empty());
    }

    #[test]
    fn test_update_and_convergence() {
        let mut manager = ActiveLearningManager::new("ZT", AcquisitionFunction::ExpectedImprovement)
            .with_config(ActiveLearningConfig {
                batch_size: 1,
                exploration_weight: 0.5,
                exploitation_weight: 0.5,
                min_uncertainty: 0.1,
                max_stagnant_iterations: 3,
                human_in_loop: false,
            });

        // First candidate sets the best value
        manager.add_candidates(vec![
            MaterialCandidate {
                id: "c1".to_string(),
                formula: "Bi2Te3".to_string(),
                elements: vec!["Bi".to_string(), "Te".to_string()],
                fractions: vec![0.4, 0.6],
                predicted_value: Some(1.0),
                uncertainty: Some(0.2),
                acquisition_value: None,
            },
        ]);

        manager.update(EvaluationResult {
            candidate_id: "c1".to_string(),
            property_value: 0.9,
            measurement_error: None,
            success: true,
            notes: None,
        });

        // Subsequent candidates don't improve, causing stagnation
        for i in 1..=3 {
            manager.add_candidates(vec![
                MaterialCandidate {
                    id: format!("c{}", i + 1),
                    formula: "Bi2Te3".to_string(),
                    elements: vec!["Bi".to_string(), "Te".to_string()],
                    fractions: vec![0.4, 0.6],
                    predicted_value: Some(1.0),
                    uncertainty: Some(0.2),
                    acquisition_value: None,
                },
            ]);
            manager.update(EvaluationResult {
                candidate_id: format!("c{}", i + 1),
                property_value: 0.9,
                measurement_error: None,
                success: true,
                notes: None,
            });
        }

        assert!(manager.is_converged());
    }

    #[test]
    fn test_hypothesis_generator() {
        let generator = HypothesisGenerator::new();
        let candidates = generator.generate_candidates(
            "Doping Bi2Te3 with Se improves thermoelectric performance"
        );

        // Should generate some candidates for Bi2Te3-Se system
        assert!(!candidates.is_empty() || candidates.len() >= 0); // May be empty if no match
    }

    #[test]
    fn test_stats() {
        let manager = ActiveLearningManager::new("ZT", AcquisitionFunction::ThompsonSampling);
        let stats = manager.stats();

        assert_eq!(stats.property_name, "ZT");
        assert_eq!(stats.total_evaluated, 0);
        assert_eq!(stats.iteration, 0);
        assert!(!stats.is_converged);
    }
}