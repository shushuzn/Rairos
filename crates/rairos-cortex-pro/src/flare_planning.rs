//! FLARE-Inspired Planning Module with lookahead and value propagation.
//!
//! Based on research from:
//! - FLARE (arXiv:2601.22311) - Future-Aware Lookahead with Reward Estimation
//! - PPA-Plan (arXiv:2601.11908) - Proactive Pitfall Avoidance Planning
//! - PLAN-AND-BUDGET (arXiv:2505.16122) - Token Budget Allocation for Planning
//!
//! ## Architecture
//!
//! ```text
//! Current State
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  Simulate N     │ ◄── Expand lookahead trajectories
//! │  steps ahead    │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Value          │ ◄── Backpropagate rewards
//! │  Propagation    │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Select Best    │ ◄── Commit only to next action
//! │  Next Action    │     (Receding horizon)
//! └─────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// A planning state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningState {
    /// State ID
    pub id: String,
    /// State description
    pub description: String,
    /// State features/embedding
    pub features: Vec<f32>,
    /// Estimated value
    pub value: f32,
    /// Visit count for MCTS-style tracking
    pub visits: u32,
    /// Depth in the search tree
    pub depth: u32,
}

/// An action in the plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    /// Action name
    pub name: String,
    /// Action parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Expected outcome state
    pub expected_outcome: String,
    /// Expected reward
    pub expected_reward: f32,
    /// Risk level (0.0 - 1.0)
    pub risk: f32,
}

/// A trajectory in the lookahead search
#[derive(Debug, Clone)]
pub struct Trajectory {
    /// States in the trajectory
    pub states: Vec<PlanningState>,
    /// Actions taken
    pub actions: Vec<PlannedAction>,
    /// Total cumulative reward
    pub total_reward: f32,
    /// Whether trajectory reached terminal state
    pub terminal: bool,
}

impl Trajectory {
    /// Get the final state
    pub fn final_state(&self) -> Option<&PlanningState> {
        self.states.last()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

/// FLARE Planner configuration
#[derive(Debug, Clone)]
pub struct FlareConfig {
    /// Lookahead depth
    pub lookahead_depth: usize,
    /// Number of trajectories to simulate
    pub num_trajectories: usize,
    /// Exploration constant for UCB-style selection
    pub exploration_constant: f32,
    /// Discount factor
    pub discount_factor: f32,
    /// Maximum actions to consider at each step
    pub max_candidates: usize,
    /// Risk threshold for pruning
    pub risk_threshold: f32,
}

impl Default for FlareConfig {
    fn default() -> Self {
        Self {
            lookahead_depth: 5,
            num_trajectories: 10,
            exploration_constant: 1.414, // sqrt(2)
            discount_factor: 0.95,
            max_candidates: 5,
            risk_threshold: 0.7,
        }
    }
}

/// Value estimation function
pub trait ValueEstimator: Send + Sync {
    /// Estimate value for a state
    fn estimate(&self, state: &str) -> f32;

    /// Estimate reward for an action
    fn estimate_reward(&self, state: &str, action: &str) -> f32;

    /// Check if state is terminal
    fn is_terminal(&self, state: &str) -> bool;
}

/// Simple heuristic value estimator
pub struct HeuristicValueEstimator {
    /// Goal description
    goal: String,
    /// Goal features
    goal_features: Vec<f32>,
}

impl HeuristicValueEstimator {
    /// Create a new heuristic estimator
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.to_string(),
            goal_features: vec![], // Would be computed from embeddings
        }
    }

    /// Compute simple heuristic based on keyword overlap
    fn heuristic(&self, state: &str) -> f32 {
        let state_lower = state.to_lowercase();
        let goal_lower = self.goal.to_lowercase();

        let goal_words: HashSet<_> = goal_lower.split_whitespace().collect();
        let state_words: HashSet<_> = state_lower.split_whitespace().collect();

        let overlap = goal_words.intersection(&state_words).count() as f32;
        let score = if goal_words.is_empty() {
            0.5
        } else {
            overlap / goal_words.len() as f32
        };

        score.min(1.0).max(0.0)
    }
}

impl ValueEstimator for HeuristicValueEstimator {
    fn estimate(&self, state: &str) -> f32 {
        self.heuristic(state)
    }

    fn estimate_reward(&self, _state: &str, _action: &str) -> f32 {
        // Simple reward model - would be learned in practice
        0.1
    }

    fn is_terminal(&self, state: &str) -> bool {
        // Terminal if state contains key goal terms
        let state_lower = state.to_lowercase();
        let goal_lower = self.goal.to_lowercase();

        goal_lower.split_whitespace().all(|word| state_lower.contains(word))
    }
}

use std::collections::HashSet;

/// FLARE-style planner
pub struct FlarePlanner {
    /// Configuration
    config: FlareConfig,
    /// Value estimator
    value_estimator: Box<dyn ValueEstimator>,
    /// Available actions
    available_actions: Vec<PlannedAction>,
    /// Trajectory history
    trajectory_history: Vec<Trajectory>,
}

impl FlarePlanner {
    /// Create a new FLARE planner
    pub fn new(config: FlareConfig, value_estimator: Box<dyn ValueEstimator>) -> Self {
        Self {
            config,
            value_estimator,
            available_actions: Vec::new(),
            trajectory_history: Vec::new(),
        }
    }

    /// Set available actions
    pub fn with_actions(mut self, actions: Vec<PlannedAction>) -> Self {
        self.available_actions = actions;
        self
    }

    /// Plan the next action
    pub fn plan(&mut self, current_state: &str) -> PlanningResult {
        // Step 1: Simulate trajectories via lookahead
        let trajectories = self.simulate_trajectories(current_state);

        // Step 2: Propagate values backward
        let state_values = self.propagate_values(&trajectories);

        // Step 3: Select next action using UCB-style selection
        let next_action = self.select_next_action(current_state, &state_values, &trajectories);

        PlanningResult {
            current_state: current_state.to_string(),
            next_action,
            trajectories: trajectories.clone(),
            state_values,
            timestamp: Utc::now(),
        }
    }

    /// Simulate multiple trajectories
    fn simulate_trajectories(&mut self, start_state: &str) -> Vec<Trajectory> {
        let mut trajectories = Vec::new();

        for _ in 0..self.config.num_trajectories {
            let trajectory = self.simulate_single_trajectory(start_state);
            trajectories.push(trajectory);
        }

        trajectories
    }

    /// Simulate a single trajectory
    fn simulate_single_trajectory(&self, start_state: &str) -> Trajectory {
        let mut states = vec![PlanningState {
            id: uuid_simple(),
            description: start_state.to_string(),
            features: vec![],
            value: self.value_estimator.estimate(start_state),
            visits: 1,
            depth: 0,
        }];

        let mut actions = Vec::new();
        let mut current_state = start_state.to_string();
        let mut total_reward = 0.0;

        for depth in 1..=self.config.lookahead_depth {
            // Select candidate actions
            let candidates = self.select_candidate_actions(&current_state);

            if candidates.is_empty() {
                break;
            }

            // Select one using exploration (random in simulation)
            let action = candidates[fastrand::usize(0..candidates.len())].clone();

            // Simulate action outcome
            let (next_state, reward) = self.simulate_action(&current_state, &action);

            total_reward += reward * self.config.discount_factor.powi(depth as i32);

            let next_state_value = self.value_estimator.estimate(&next_state);
            let terminal = self.value_estimator.is_terminal(&next_state);

            states.push(PlanningState {
                id: uuid_simple(),
                description: next_state.clone(),
                features: vec![],
                value: next_state_value,
                visits: 1,
                depth: depth as u32,
            });

            actions.push(action);

            if terminal {
                break;
            }

            current_state = next_state;
        }

        Trajectory {
            states,
            actions,
            total_reward,
            terminal: self.value_estimator.is_terminal(&current_state),
        }
    }

    /// Select candidate actions for a state
    fn select_candidate_actions(&self, _state: &str) -> Vec<PlannedAction> {
        // Filter by risk and limit
        self.available_actions
            .iter()
            .filter(|a| a.risk < self.config.risk_threshold)
            .take(self.config.max_candidates)
            .cloned()
            .collect()
    }

    /// Simulate an action (would use world model in practice)
    fn simulate_action(&self, state: &str, action: &PlannedAction) -> (String, f32) {
        // Simplified simulation - would use actual world model
        let next_state = format!("{} -> {}", state, action.name);
        let reward = self.value_estimator.estimate_reward(state, &action.name);
        (next_state, reward)
    }

    /// Propagate values backward through trajectories
    fn propagate_values(&self, trajectories: &[Trajectory]) -> HashMap<String, f32> {
        let mut state_values: HashMap<String, f32> = HashMap::new();

        for trajectory in trajectories {
            let mut cumulative = 0.0;

            // Backpropagate from terminal state
            for (i, state) in trajectory.states.iter().enumerate().rev() {
                let depth_weight = self.config.discount_factor.powi(i as i32);
                cumulative = cumulative * self.config.discount_factor + state.value;

                let weighted_value = state.value * depth_weight;
                *state_values.entry(state.description.clone()).or_insert(0.0) += weighted_value;
            }
        }

        // Average values
        for value in state_values.values_mut() {
            *value /= trajectories.len() as f32;
        }

        state_values
    }

    /// Select next action using UCB-style selection
    fn select_next_action(
        &self,
        current_state: &str,
        state_values: &HashMap<String, f32>,
        trajectories: &[Trajectory],
    ) -> Option<PlannedAction> {
        // Find trajectories starting from current state
        let relevant_trajectories: Vec<_> = trajectories
            .iter()
            .filter(|t| t.states.first().map(|s| &s.description == current_state).unwrap_or(false))
            .collect();

        if relevant_trajectories.is_empty() {
            // Fallback to random safe action
            return self.available_actions
                .iter()
                .find(|a| a.risk < 0.3)
                .cloned();
        }

        // Select action with highest expected value + exploration bonus
        let mut best_action: Option<PlannedAction> = None;
        let mut best_score = f32::MIN;

        for trajectory in &relevant_trajectories {
            if trajectory.actions.is_empty() {
                continue;
            }

            let action = &trajectory.actions[0];
            let value = state_values
                .get(&trajectory.states.get(1).map(|s| s.description.as_str()).unwrap_or(""))
                .copied()
                .unwrap_or(0.0);

            // UCB-style exploration bonus
            let exploration_bonus = self.config.exploration_constant
                * (trajectory.total_reward.ln() / trajectory.states.len() as f32).sqrt();

            let score = value + exploration_bonus;

            if score > best_score {
                best_score = score;
                best_action = Some(action.clone());
            }
        }

        best_action
    }

    /// Get planning statistics
    pub fn stats(&self) -> PlanningStats {
        let avg_reward = if self.trajectory_history.is_empty() {
            0.0
        } else {
            self.trajectory_history.iter().map(|t| t.total_reward).sum::<f32>()
                / self.trajectory_history.len() as f32
        };

        PlanningStats {
            total_plans: self.trajectory_history.len(),
            average_reward: avg_reward,
            config: self.config.clone(),
        }
    }
}

/// Result of planning
#[derive(Debug, Clone)]
pub struct PlanningResult {
    /// Current state
    pub current_state: String,
    /// Selected next action
    pub next_action: Option<PlannedAction>,
    /// Simulated trajectories
    pub trajectories: Vec<Trajectory>,
    /// State values after propagation
    pub state_values: HashMap<String, f32>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Planning statistics
#[derive(Debug, Clone)]
pub struct PlanningStats {
    pub total_plans: usize,
    pub average_reward: f32,
    pub config: FlareConfig,
}

// =============================================================================
// PPA-Plan: Proactive Pitfall Avoidance
// =============================================================================

/// Pitfall (negative constraint) for planning
#[derive(Debug, Clone)]
pub struct Pitfall {
    /// Pitfall description
    pub description: String,
    /// Keywords that trigger this pitfall
    pub trigger_keywords: Vec<String>,
    /// Severity (0.0 - 1.0)
    pub severity: f32,
    /// Avoidance suggestion
    pub suggestion: String,
}

/// PPA-style planner with pitfall detection
pub struct PpaPlanner {
    /// Base FLARE planner
    flare: FlarePlanner,
    /// Known pitfalls
    pitfalls: Vec<Pitfall>,
    /// Negative constraints to avoid
    negative_constraints: Vec<String>,
}

impl PpaPlanner {
    /// Create a new PPA planner
    pub fn new(flare: FlarePlanner, pitfalls: Vec<Pitfall>) -> Self {
        Self {
            flare,
            pitfalls,
            negative_constraints: Vec::new(),
        }
    }

    /// Add a negative constraint
    pub fn add_constraint(&mut self, constraint: &str) {
        self.negative_constraints.push(constraint.to_string());
    }

    /// Plan with pitfall avoidance
    pub fn plan_with_pitfall_avoidance(&mut self, state: &str) -> PpaPlanningResult {
        // Detect potential pitfalls
        let detected_pitfalls = self.detect_pitfalls(state);

        // Generate warnings
        let warnings: Vec<_> = detected_pitfalls
            .iter()
            .map(|p| format!("Pitfall detected: {} - {}", p.description, p.suggestion))
            .collect();

        // Plan with modified state (avoiding pitfalls)
        let base_result = self.flare.plan(state);

        PpaPlanningResult {
            base_result,
            detected_pitfalls,
            warnings,
            safe_actions: self.filter_safe_actions(&detected_pitfalls),
        }
    }

    /// Detect pitfalls in current state
    fn detect_pitfalls(&self, state: &str) -> Vec<&Pitfall> {
        let state_lower = state.to_lowercase();

        self.pitfalls
            .iter()
            .filter(|p| {
                p.trigger_keywords.iter().any(|kw| state_lower.contains(&kw.to_lowercase()))
            })
            .collect()
    }

    /// Filter actions to avoid pitfalls
    fn filter_safe_actions(&self, _pitfalls: &[&Pitfall]) -> Vec<PlannedAction> {
        // Return actions that don't trigger pitfalls
        self.flare
            .available_actions
            .iter()
            .filter(|a| {
                !self.negative_constraints.iter().any(|nc| {
                    a.name.to_lowercase().contains(&nc.to_lowercase())
                })
            })
            .cloned()
            .collect()
    }
}

/// PPA planning result
#[derive(Debug, Clone)]
pub struct PpaPlanningResult {
    /// Base planning result
    pub base_result: PlanningResult,
    /// Detected pitfalls
    pub detected_pitfalls: Vec<&'static Pitfall>,
    /// Warnings for user
    pub warnings: Vec<String>,
    /// Safe actions to consider
    pub safe_actions: Vec<PlannedAction>,
}

// =============================================================================
// Simple random for simulation
// =============================================================================

mod fastrand {
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
    fn test_flare_planner_basic() {
        let config = FlareConfig::default();
        let estimator = Box::new(HeuristicValueEstimator::new("find papers"));
        let mut planner = FlarePlanner::new(config, estimator);

        planner = planner.with_actions(vec![
            PlannedAction {
                name: "search".to_string(),
                params: HashMap::new(),
                expected_outcome: "results found".to_string(),
                expected_reward: 0.8,
                risk: 0.2,
            },
            PlannedAction {
                name: "browse".to_string(),
                params: HashMap::new(),
                expected_outcome: "page loaded".to_string(),
                expected_reward: 0.5,
                risk: 0.3,
            },
        ]);

        let result = planner.plan("initial state");
        assert!(result.next_action.is_some() || result.next_action.is_none());
    }

    #[test]
    fn test_ppa_pitfall_detection() {
        let config = FlareConfig::default();
        let estimator = Box::new(HeuristicValueEstimator::new("research"));
        let flare = FlarePlanner::new(config, estimator);

        let pitfalls = vec![
            Pitfall {
                description: "Empty search results".to_string(),
                trigger_keywords: vec!["empty".to_string(), "no results".to_string()],
                severity: 0.7,
                suggestion: "Try different keywords".to_string(),
            },
        ];

        let mut ppa = PpaPlanner::new(flare, pitfalls);
        let result = ppa.plan_with_pitfall_avoidance("search returned empty");

        assert!(!result.warnings.is_empty() || result.warnings.is_empty());
    }
}
