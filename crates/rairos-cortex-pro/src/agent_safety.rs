//! Agent Safety and Audit Module.
//!
//! Based on research from:
//! - SafeAgent arXiv:2604.17562 - Runtime protection architecture
//! - SafeHarness arXiv:2604.13630 - 4-layer defense
//! - RAC arXiv:2605.03409 - Robust Agent Compensation
//! - ValueFlow arXiv:2602.08567 - Value drift propagation
//! - ReliabilityBench arXiv:2601.06112 - Agent reliability metrics
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              Agent Safety Guard                         │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
//! │  │Admission │→ │Execution │→ │Output    │→ │Audit   │  │
//! │  │Control   │  │Monitor  │  │Validator │  │Logger │  │
//! │  └──────────┘  └──────────┘  └──────────┘  └────────┘  │
//! └─────────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use chrono::{DateTime, Utc};

use crate::utils::uuid_simple;

/// Safety verdict
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SafetyVerdict {
    /// Action allowed
    Allowed,
    /// Action requires confirmation
    PendingConfirmation,
    /// Action blocked
    Blocked,
    /// Action failed safety check
    Failed,
}

/// Risk level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: String,
    /// Agent ID
    pub agent_id: String,
    /// Action attempted
    pub action: String,
    /// Safety verdict
    pub verdict: SafetyVerdict,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Details
    pub details: Option<String>,
    /// Whether blocked
    pub blocked: bool,
}

/// Safety policy rule
pub struct SafetyRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Risk level this rule applies to
    pub risk_level: RiskLevel,
    /// Whether to block or warn
    pub block_on_violation: bool,
    /// Keywords that trigger this rule
    pub trigger_keywords: Vec<String>,
    /// Compiled check function (simplified, wrapped in Rc for cloneability)
    pub check_fn: std::rc::Rc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl Debug for SafetyRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafetyRule")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("risk_level", &self.risk_level)
            .field("block_on_violation", &self.block_on_violation)
            .field("trigger_keywords", &self.trigger_keywords)
            .finish()
    }
}

impl Clone for SafetyRule {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            risk_level: self.risk_level,
            block_on_violation: self.block_on_violation,
            trigger_keywords: self.trigger_keywords.clone(),
            check_fn: self.check_fn.clone(),
        }
    }
}

/// Agent Safety Guard
pub struct AgentSafetyGuard {
    /// Safety rules
    rules: Vec<SafetyRule>,
    /// Audit log
    audit_log: Vec<AuditEntry>,
    /// Blocked agents
    blocked_agents: HashSet<String>,
    /// Configuration
    config: SafetyConfig,
}

/// Safety configuration
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// Enable strict mode (block on any risk)
    pub strict_mode: bool,
    /// Maximum audit entries
    pub max_audit_entries: usize,
    /// Enable automatic blocking
    pub auto_block: bool,
    /// Block threshold (number of violations)
    pub block_threshold: usize,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            max_audit_entries: 10000,
            auto_block: false,
            block_threshold: 3,
        }
    }
}

impl AgentSafetyGuard {
    /// Create new safety guard
    pub fn new() -> Self {
        let default_rules = Self::default_rules();

        Self {
            rules: default_rules,
            audit_log: Vec::new(),
            blocked_agents: HashSet::new(),
            config: SafetyConfig::default(),
        }
    }

    /// Create with config
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            rules: Self::default_rules(),
            audit_log: Vec::new(),
            blocked_agents: HashSet::new(),
            config,
        }
    }

    /// Default safety rules
    fn default_rules() -> Vec<SafetyRule> {
        vec![
            SafetyRule {
                name: "Dangerous Commands".to_string(),
                description: "Blocks dangerous system commands".to_string(),
                risk_level: RiskLevel::Critical,
                block_on_violation: true,
                trigger_keywords: vec!["rm -rf".to_string(), "format".to_string(), "delete everything".to_string()],
                check_fn: std::rc::Rc::new(|action: &str| {
                    let lower = action.to_lowercase();
                    lower.contains("rm -rf") || lower.contains("format disk") || lower.contains("delete everything")
                }),
            },
            SafetyRule {
                name: "External Network".to_string(),
                description: "Monitor external network access".to_string(),
                risk_level: RiskLevel::Medium,
                block_on_violation: false,
                trigger_keywords: vec!["http".to_string(), "tcp".to_string(), "connect to".to_string()],
                check_fn: std::rc::Rc::new(|action: &str| {
                    let lower = action.to_lowercase();
                    lower.contains("http://") || lower.contains("https://") || lower.contains("tcp://")
                }),
            },
            SafetyRule {
                name: "Data Exfiltration".to_string(),
                description: "Prevent data exfiltration".to_string(),
                risk_level: RiskLevel::High,
                block_on_violation: true,
                trigger_keywords: vec!["send to".to_string(), "upload".to_string(), "exfiltrate".to_string()],
                check_fn: std::rc::Rc::new(|action: &str| {
                    let lower = action.to_lowercase();
                    lower.contains("send to external") || lower.contains("upload to") || lower.contains("exfiltrate")
                }),
            },
        ]
    }

    /// Add a custom rule
    pub fn add_rule(&mut self, rule: SafetyRule) {
        self.rules.push(rule);
    }

    /// Check if action is safe
    pub fn check(&self, agent_id: &str, action: &str) -> SafetyCheckResult {
        // Check if agent is blocked
        if self.blocked_agents.contains(agent_id) {
            return SafetyCheckResult {
                verdict: SafetyVerdict::Blocked,
                risk_level: RiskLevel::Critical,
                triggered_rules: vec!["Agent is blocked".to_string()],
                recommendation: "Agent has been blocked due to policy violations".to_string(),
            };
        }

        let mut triggered_rules = Vec::new();
        let mut max_risk = RiskLevel::Low;

        for rule in &self.rules {
            if (rule.check_fn)(action) {
                triggered_rules.push(rule.name.clone());
                if rule.risk_level > max_risk {
                    max_risk = rule.risk_level;
                }
            }
        }

        let verdict = if triggered_rules.is_empty() {
            SafetyVerdict::Allowed
        } else if max_risk >= RiskLevel::High || self.config.strict_mode {
            SafetyVerdict::Blocked
        } else {
            SafetyVerdict::PendingConfirmation
        };

        let recommendation = if triggered_rules.is_empty() {
            "Action is safe".to_string()
        } else {
            format!("Triggered rules: {}", triggered_rules.join(", "))
        };

        SafetyCheckResult {
            verdict,
            risk_level: max_risk,
            triggered_rules,
            recommendation,
        }
    }

    /// Audit an action
    pub fn audit(&mut self, agent_id: &str, action: &str, result: &SafetyCheckResult) {
        let blocked = result.verdict == SafetyVerdict::Blocked;
        
        let entry = AuditEntry {
            id: uuid_simple(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            verdict: result.verdict,
            risk_level: result.risk_level,
            timestamp: Utc::now(),
            details: Some(result.recommendation.clone()),
            blocked,
        };

        self.audit_log.push(entry);

        // Auto-block if threshold exceeded
        if self.config.auto_block && blocked {
            let recent_blocks = self.audit_log
                .iter()
                .filter(|e| e.agent_id == agent_id && e.blocked)
                .count();

            if recent_blocks >= self.config.block_threshold {
                self.blocked_agents.insert(agent_id.to_string());
            }
        }

        // Trim log if needed
        if self.audit_log.len() > self.config.max_audit_entries {
            self.audit_log.remove(0);
        }
    }

    /// Block an agent
    pub fn block_agent(&mut self, agent_id: &str, reason: &str) {
        self.blocked_agents.insert(agent_id.to_string());

        self.audit_log.push(AuditEntry {
            id: uuid_simple(),
            agent_id: agent_id.to_string(),
            action: format!("Block agent: {}", reason),
            verdict: SafetyVerdict::Blocked,
            risk_level: RiskLevel::Critical,
            timestamp: Utc::now(),
            details: Some(reason.to_string()),
            blocked: true,
        });
    }

    /// Unblock an agent
    pub fn unblock_agent(&mut self, agent_id: &str) {
        self.blocked_agents.remove(agent_id);
    }

    /// Check if agent is blocked
    pub fn is_blocked(&self, agent_id: &str) -> bool {
        self.blocked_agents.contains(agent_id)
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Get statistics
    pub fn stats(&self) -> SafetyStats {
        let total = self.audit_log.len();
        let blocked = self.audit_log.iter().filter(|e| e.blocked).count();
        let by_risk: HashMap<_, _> = self.audit_log
            .iter()
            .fold(HashMap::new(), |mut acc, e| {
                *acc.entry(e.risk_level).or_insert(0) += 1;
                acc
            });

        SafetyStats {
            total_audits: total,
            blocked_count: blocked,
            blocked_agents: self.blocked_agents.len(),
            by_risk_level: by_risk,
        }
    }
}

/// Result of safety check
#[derive(Debug, Clone)]
pub struct SafetyCheckResult {
    pub verdict: SafetyVerdict,
    pub risk_level: RiskLevel,
    pub triggered_rules: Vec<String>,
    pub recommendation: String,
}

/// Safety statistics
#[derive(Debug, Clone)]
pub struct SafetyStats {
    pub total_audits: usize,
    pub blocked_count: usize,
    pub blocked_agents: usize,
    pub by_risk_level: HashMap<RiskLevel, usize>,
}

// =============================================================================
// Value Drift Detector (ValueFlow-style)
// =============================================================================

/// Value principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuePrinciple {
    /// Principle ID
    pub id: String,
    /// Principle text
    pub text: String,
    /// Category
    pub category: ValueCategory,
    /// Weight (importance)
    pub weight: f32,
}

/// Value category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueCategory {
    Safety,
    Honesty,
    Helpfulness,
    Fairness,
    Privacy,
    Other,
}

/// Value drift detector
pub struct ValueDriftDetector {
    /// Core principles
    principles: Vec<ValuePrinciple>,
    /// Drift history
    drift_history: Vec<DriftRecord>,
    /// Drift threshold
    drift_threshold: f32,
}

/// Drift record
#[derive(Debug, Clone)]
pub struct DriftRecord {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub principle_id: String,
    pub drift_score: f32,
    pub action: String,
}

impl ValueDriftDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            principles: Self::default_principles(),
            drift_history: Vec::new(),
            drift_threshold: 0.3,
        }
    }

    /// Default principles
    fn default_principles() -> Vec<ValuePrinciple> {
        vec![
            ValuePrinciple {
                id: "safety".to_string(),
                text: "Do not cause harm".to_string(),
                category: ValueCategory::Safety,
                weight: 1.0,
            },
            ValuePrinciple {
                id: "honesty".to_string(),
                text: "Be truthful and accurate".to_string(),
                category: ValueCategory::Honesty,
                weight: 0.9,
            },
            ValuePrinciple {
                id: "privacy".to_string(),
                text: "Respect user privacy".to_string(),
                category: ValueCategory::Privacy,
                weight: 0.8,
            },
        ]
    }

    /// Measure drift for an action
    pub fn measure_drift(&self, action: &str) -> Vec<(String, f32)> {
        let mut drift_scores = Vec::new();

        for principle in &self.principles {
            let score = self.calculate_drift(action, &principle.text);
            drift_scores.push((principle.id.clone(), score));
        }

        drift_scores
    }

    /// Calculate drift score
    fn calculate_drift(&self, action: &str, principle: &str) -> f32 {
        // Simplified: check for keywords that might indicate drift
        let action_lower = action.to_lowercase();
        let principle_lower = principle.to_lowercase();

        // Keywords that might indicate violation
        let violation_keywords = match principle_lower.as_str() {
            s if s.contains("harm") => vec!["harm", "damage", "destroy", "hurt"],
            s if s.contains("truth") => vec!["lie", "deceive", "fake", "mislead"],
            s if s.contains("privacy") => vec!["share", "expose", "leak", "public"],
            _ => vec![],
        };

        let violations: usize = violation_keywords
            .iter()
            .filter(|kw| action_lower.contains(*kw))
            .count();

        // Normalize to 0-1
        (violations as f32 * 0.3).min(1.0)
    }

    /// Record drift
    pub fn record(&mut self, agent_id: &str, drift_scores: &[(String, f32)], action: &str) {
        for (principle_id, score) in drift_scores {
            if *score > self.drift_threshold {
                self.drift_history.push(DriftRecord {
                    timestamp: Utc::now(),
                    agent_id: agent_id.to_string(),
                    principle_id: principle_id.clone(),
                    drift_score: *score,
                    action: action.to_string(),
                });
            }
        }
    }

    /// Check if drift is concerning
    pub fn is_drift_concerning(&self, agent_id: &str) -> bool {
        let recent_drifts: Vec<_> = self.drift_history
            .iter()
            .filter(|d| d.agent_id == agent_id)
            .filter(|d| (Utc::now() - d.timestamp).num_hours() < 24)
            .collect();

        let avg_drift: f32 = if recent_drifts.is_empty() {
            0.0
        } else {
            recent_drifts.iter().map(|d| d.drift_score).sum::<f32>() / recent_drifts.len() as f32
        };

        avg_drift > self.drift_threshold
    }

    /// Get drift summary
    pub fn summary(&self, agent_id: &str) -> DriftSummary {
        let agent_drifts: Vec<_> = self.drift_history
            .iter()
            .filter(|d| d.agent_id == agent_id)
            .collect();

        let total_violations = agent_drifts.len();
        let avg_drift = if agent_drifts.is_empty() {
            0.0
        } else {
            agent_drifts.iter().map(|d| d.drift_score).sum::<f32>() / agent_drifts.len() as f32
        };

        DriftSummary {
            agent_id: agent_id.to_string(),
            total_violations,
            average_drift_score: avg_drift,
            concerning: self.is_drift_concerning(agent_id),
        }
    }
}

impl Default for ValueDriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Drift summary
#[derive(Debug, Clone)]
pub struct DriftSummary {
    pub agent_id: String,
    pub total_violations: usize,
    pub average_drift_score: f32,
    pub concerning: bool,
}

// =============================================================================
// Agent Reliability Metrics (ReliabilityBench-style)
// =============================================================================

/// Reliability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityMetrics {
    /// Pass rate
    pub pass_rate: f32,
    /// Mean time between failures (in seconds)
    pub mtbf: f64,
    /// Recovery time (in seconds)
    pub recovery_time: f64,
    /// Consistency score
    pub consistency: f32,
    /// Graceful degradation score
    pub graceful_degradation: f32,
}

/// Reliability tracker
pub struct ReliabilityTracker {
    /// Success count
    successes: u64,
    /// Failure count
    failures: u64,
    /// Total execution time
    total_time_ms: u64,
    /// Recovery times
    recovery_times: Vec<u64>,
    /// Recent results
    recent_results: Vec<bool>,
    /// Max recent results to track
    max_recent: usize,
}

impl ReliabilityTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            successes: 0,
            failures: 0,
            total_time_ms: 0,
            recovery_times: Vec::new(),
            recent_results: Vec::new(),
            max_recent: 100,
        }
    }

    /// Record success
    pub fn record_success(&mut self, execution_time_ms: u64) {
        self.successes += 1;
        self.total_time_ms += execution_time_ms;
        self.recent_results.push(true);

        if self.recent_results.len() > self.max_recent {
            self.recent_results.remove(0);
        }
    }

    /// Record failure
    pub fn record_failure(&mut self, execution_time_ms: u64, recovery_time_ms: Option<u64>) {
        self.failures += 1;
        self.total_time_ms += execution_time_ms;

        if let Some(rt) = recovery_time_ms {
            self.recovery_times.push(rt);
        }

        self.recent_results.push(false);

        if self.recent_results.len() > self.max_recent {
            self.recent_results.remove(0);
        }
    }

    /// Calculate metrics
    pub fn metrics(&self) -> ReliabilityMetrics {
        let total = self.successes + self.failures;
        let pass_rate = if total == 0 {
            0.0
        } else {
            self.successes as f32 / total as f32
        };

        let mtbf = if self.failures == 0 {
            f64::MAX
        } else {
            self.total_time_ms as f64 / self.failures as f64
        };

        let avg_recovery: f64 = if self.recovery_times.is_empty() {
            0.0
        } else {
            self.recovery_times.iter().sum::<u64>() as f64 / self.recovery_times.len() as f64
        };

        let consistency = self.calculate_consistency();

        let graceful_degradation = self.calculate_graceful_degradation();

        ReliabilityMetrics {
            pass_rate,
            mtbf,
            recovery_time: avg_recovery,
            consistency,
            graceful_degradation,
        }
    }

    /// Calculate consistency
    fn calculate_consistency(&self) -> f32 {
        if self.recent_results.len() < 2 {
            return 1.0;
        }

        let changes: usize = self.recent_results
            .windows(2)
            .filter(|w| w[0] != w[1])
            .count();

        1.0 - (changes as f32 / (self.recent_results.len() - 1) as f32)
    }

    /// Calculate graceful degradation
    fn calculate_graceful_degradation(&self) -> f32 {
        // Simplified: based on recent trend
        if self.recent_results.len() < 10 {
            return 0.5;
        }

        let recent: Vec<_> = self.recent_results.iter().rev().take(10).collect();
        let old: Vec<_> = self.recent_results.iter().rev().skip(10).take(10).collect();

        if old.is_empty() {
            return 0.5;
        }

        let recent_success_rate = recent.iter().filter(|&&r| *r).count() as f32 / recent.len() as f32;
        let old_success_rate = old.iter().filter(|&&r| *r).count() as f32 / old.len() as f32;

        // Graceful if recent is not much worse than old
        if recent_success_rate >= old_success_rate * 0.8 {
            0.8
        } else if recent_success_rate >= old_success_rate * 0.5 {
            0.5
        } else {
            0.2
        }
    }
}

impl Default for ReliabilityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_check() {
        let guard = AgentSafetyGuard::new();

        let result = guard.check("agent1", "read file.txt");
        assert_eq!(result.verdict, SafetyVerdict::Allowed);
    }

    #[test]
    fn test_safety_check_blocked() {
        let guard = AgentSafetyGuard::new();

        let result = guard.check("agent1", "rm -rf /");
        assert_eq!(result.verdict, SafetyVerdict::Blocked);
    }

    #[test]
    fn test_value_drift() {
        let detector = ValueDriftDetector::new();

        let drift = detector.measure_drift("harm the user");
        assert!(!drift.is_empty());
    }

    #[test]
    fn test_reliability_tracker() {
        let mut tracker = ReliabilityTracker::new();

        tracker.record_success(100);
        tracker.record_success(100);
        tracker.record_failure(100, Some(50));

        let metrics = tracker.metrics();
        assert!(metrics.pass_rate > 0.0);
    }
}
