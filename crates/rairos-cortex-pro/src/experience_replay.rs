//! Experience Replay Module for Self-Improving Agents.
//!
//! Based on research from:
//! - R³ (arXiv:2601.19620) - Replay, Reflection, and Ranking Rewards
//! - ERL (arXiv:2603.24639) - Experiential Reflective Learning
//! - LEAFE (arXiv:2603.16843) - Learning from Feedback-Grounded Experience
//!
//! ## Architecture
//!
//! ```text
//! Agent Experience
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  Experience      │ ← Store trajectory with outcome
//! │  Buffer         │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Self-Reflection │ ← Analyze failure patterns
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Replay +       │ ← Retrieve and reuse experiences
//! │  Consolidation   │
//! └────────┬────────┘
//!          │
//!          ▼
//!     Improved Agent
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};

/// Maximum experiences to store per category
const MAX_EXPERIENCES: usize = 1000;

/// Experience entry storing a complete trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// Experience ID
    pub id: String,
    /// Task description
    pub task: String,
    /// Full trajectory (reasoning steps, actions, outcomes)
    pub trajectory: Vec<TrajectoryStep>,
    /// Final outcome
    pub outcome: ExperienceOutcome,
    /// When this was recorded
    pub timestamp: DateTime<Utc>,
    /// Extracted lessons/learnings
    pub lessons: Vec<String>,
    /// Effectiveness score (0.0 - 1.0)
    pub effectiveness: f32,
}

/// Single step in a trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    /// Step index
    pub step: usize,
    /// What the agent did
    pub action: String,
    /// Reasoning at this step
    pub reasoning: String,
    /// Observation/result
    pub observation: String,
    /// Whether this step succeeded
    pub success: bool,
}

/// Outcome of an experience
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExperienceOutcome {
    /// Task completed successfully
    Success,
    /// Task failed
    Failure,
    /// Task partially completed
    PartialSuccess,
    /// Task timed out
    Timeout,
}

/// Experience buffer with replay capabilities
pub struct ExperienceReplay {
    /// All experiences
    experiences: VecDeque<Experience>,
    /// Experiences by outcome type
    by_outcome: HashMap<ExperienceOutcome, Vec<String>>,
    /// Experiences by task category (simplified)
    by_category: HashMap<String, Vec<String>>,
    /// Lesson statistics
    lesson_counts: HashMap<String, usize>,
}

impl ExperienceReplay {
    /// Create a new experience replay buffer
    pub fn new() -> Self {
        Self {
            experiences: VecDeque::new(),
            by_outcome: HashMap::new(),
            by_category: HashMap::new(),
            lesson_counts: HashMap::new(),
        }
    }

    /// Add a new experience
    pub fn add(&mut self, experience: Experience) {
        let id = experience.id.clone();

        // Update outcome index
        self.by_outcome
            .entry(experience.outcome.clone())
            .or_default()
            .push(id.clone());

        // Update lesson counts
        for lesson in &experience.lessons {
            *self.lesson_counts.entry(lesson.clone()).or_insert(0) += 1;
        }

        // Add to main buffer
        self.experiences.push_back(experience);

        // Evict oldest if over capacity
        if self.experiences.len() > MAX_EXPERIENCES {
            if let Some(oldest) = self.experiences.pop_front() {
                // Remove from indices
                if let Some(ids) = self.by_outcome.get_mut(&oldest.outcome) {
                    ids.retain(|i| i != &oldest.id);
                }
            }
        }
    }

    /// Get successful experiences for replay
    pub fn get_successful(&self, limit: usize) -> Vec<&Experience> {
        self.by_outcome
            .get(&ExperienceOutcome::Success)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.experiences.iter().find(|e| &e.id == id))
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get failed experiences for analysis
    pub fn get_failed(&self, limit: usize) -> Vec<&Experience> {
        self.by_outcome
            .get(&ExperienceOutcome::Failure)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.experiences.iter().find(|e| &e.id == id))
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get most common lessons
    pub fn get_common_lessons(&self, limit: usize) -> Vec<(String, usize)> {
        let mut lessons: Vec<_> = self.lesson_counts.iter().collect();
        lessons.sort_by(|a, b| b.1.cmp(a.1));
        lessons
            .into_iter()
            .take(limit)
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Get experiences similar to a task (simplified keyword matching)
    pub fn get_similar(&self, task: &str, limit: usize) -> Vec<&Experience> {
        let task_lower = task.to_lowercase();
        let keywords: Vec<&str> = task_lower.split_whitespace().collect();

        self.experiences
            .iter()
            .filter(|e| {
                keywords.iter().any(|kw| e.task.to_lowercase().contains(kw))
            })
            .take(limit)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> ExperienceReplayStats {
        ExperienceReplayStats {
            total_experiences: self.experiences.len(),
            success_count: self.by_outcome.get(&ExperienceOutcome::Success).map(|v| v.len()).unwrap_or(0),
            failure_count: self.by_outcome.get(&ExperienceOutcome::Failure).map(|v| v.len()).unwrap_or(0),
            unique_lessons: self.lesson_counts.len(),
        }
    }
}

impl Default for ExperienceReplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the experience replay buffer
#[derive(Debug, Clone)]
pub struct ExperienceReplayStats {
    pub total_experiences: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub unique_lessons: usize,
}

// =============================================================================
// Self-Reflection Module (Based on R³ and ERL papers)
// =============================================================================

/// Self-reflection analyzer for extracting lessons from experiences
pub struct SelfReflector {
    /// Minimum effectiveness to consider an experience successful
    success_threshold: f32,
}

impl SelfReflector {
    /// Create a new self-reflector
    pub fn new() -> Self {
        Self {
            success_threshold: 0.7,
        }
    }

    /// Analyze an experience and extract lessons
    pub fn reflect(&self, experience: &Experience) -> Vec<String> {
        let mut lessons = Vec::new();

        // Analyze trajectory for patterns
        let failed_steps: Vec<_> = experience
            .trajectory
            .iter()
            .filter(|s| !s.success)
            .collect();

        let successful_steps: Vec<_> = experience
            .trajectory
            .iter()
            .filter(|s| s.success)
            .collect();

        // Extract failure patterns
        if !failed_steps.is_empty() {
            lessons.push(format!(
                "Avoid: {} failed step(s) in trajectory",
                failed_steps.len()
            ));

            // Analyze first failure
            if let Some(first_failure) = failed_steps.first() {
                if first_failure.reasoning.is_empty() {
                    lessons.push("Ensure reasoning is explicit before taking actions".to_string());
                }
                if first_failure.observation.is_empty() {
                    lessons.push("Always capture observations after actions".to_string());
                }
            }
        }

        // Extract success patterns
        if successful_steps.len() > 2 {
            lessons.push(format!(
                "Good: {} successful steps maintained trajectory",
                successful_steps.len()
            ));
        }

        // Analyze outcome-based lessons
        match experience.outcome {
            ExperienceOutcome::Success => {
                lessons.push("Outcome: Task completed successfully".to_string());
            }
            ExperienceOutcome::Failure => {
                lessons.push("Outcome: Task failed - review strategy".to_string());
            }
            ExperienceOutcome::PartialSuccess => {
                lessons.push("Outcome: Partial success - consider alternative approaches".to_string());
            }
            ExperienceOutcome::Timeout => {
                lessons.push("Outcome: Timeout - optimize for efficiency".to_string());
            }
        }

        // General lessons based on effectiveness
        if experience.effectiveness >= self.success_threshold {
            lessons.push(format!(
                "High effectiveness ({:.1}) - consider this approach for similar tasks",
                experience.effectiveness
            ));
        } else {
            lessons.push(format!(
                "Low effectiveness ({:.1}) - refine approach",
                experience.effectiveness
            ));
        }

        lessons
    }

    /// Generate a revised strategy based on reflection
    pub fn generate_revision(&self, failed_experience: &Experience, similar_successes: &[&Experience]) -> String {
        let mut revision = String::new();

        revision.push_str("## Reflection Analysis\n\n");

        // What went wrong
        revision.push_str("### What Went Wrong\n");
        for step in &failed_experience.trajectory {
            if !step.success {
                revision.push_str(&format!("- Step {}: {} (reasoning: {})\n",
                    step.step, step.action, step.reasoning));
            }
        }

        // What worked in similar successes
        if !similar_successes.is_empty() {
            revision.push_str("\n### What Worked in Similar Tasks\n");
            for success in similar_successes.iter().take(3) {
                revision.push_str(&format!("- Task: {}\n", success.task));
                for step in &success.trajectory {
                    if step.success && !step.reasoning.is_empty() {
                        revision.push_str(&format!("  * {}\n", step.reasoning));
                    }
                }
            }
        }

        // Suggested revision
        revision.push_str("\n### Recommended Approach\n");
        if let Some(first_failure) = failed_experience.trajectory.iter().find(|s| !s.success) {
            revision.push_str(&format!("Instead of '{}', try a more systematic approach.\n",
                first_failure.action));
        }

        revision
    }
}

impl Default for SelfReflector {
    fn default() -> Self {
        Self::new()
    }
}

/// Consolidation result after processing experiences
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    /// New lessons learned
    pub new_lessons: Vec<String>,
    /// Revised strategies
    pub revised_strategies: Vec<String>,
    /// Experiences to replay
    pub replay_experiences: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_experience() -> Experience {
        Experience {
            id: "exp_001".to_string(),
            task: "Search materials database for thermoelectric properties".to_string(),
            trajectory: vec![
                TrajectoryStep {
                    step: 1,
                    action: "Query materials project API".to_string(),
                    reasoning: "Need to search for Bi2Te3 compounds".to_string(),
                    observation: "Found 15 matching materials".to_string(),
                    success: true,
                },
                TrajectoryStep {
                    step: 2,
                    action: "Filter by thermoelectric figure of merit".to_string(),
                    reasoning: "ZT > 1 is desirable".to_string(),
                    observation: "Found 3 candidates".to_string(),
                    success: true,
                },
                TrajectoryStep {
                    step: 3,
                    action: "Run DFT calculation".to_string(),
                    reasoning: "Need accurate band structure".to_string(),
                    observation: "Calculation failed - convergence issue".to_string(),
                    success: false,
                },
            ],
            outcome: ExperienceOutcome::Failure,
            timestamp: Utc::now(),
            lessons: vec![],
            effectiveness: 0.6,
        }
    }

    #[test]
    fn test_experience_replay_add() {
        let mut replay = ExperienceReplay::new();
        let exp = sample_experience();
        replay.add(exp);

        assert_eq!(replay.stats().total_experiences, 1);
        assert_eq!(replay.stats().failure_count, 1);
    }

    #[test]
    fn test_get_failed() {
        let mut replay = ExperienceReplay::new();
        replay.add(sample_experience());

        let failed = replay.get_failed(10);
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_self_reflector() {
        let reflector = SelfReflector::new();
        let exp = sample_experience();
        let lessons = reflector.reflect(&exp);

        assert!(!lessons.is_empty());
        assert!(lessons.iter().any(|l| l.contains("failed")));
    }

    #[test]
    fn test_generate_revision() {
        let reflector = SelfReflector::new();
        let exp = sample_experience();

        let revision = reflector.generate_revision(&exp, &[]);
        assert!(revision.contains("What Went Wrong"));
    }
}
