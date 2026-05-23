//! rairos-failure-router — Failure-Aware Mixture-of-Experts Router
//!
//! Based on:
//! - STAR (arxiv 2605.10057): Typed failure-aware Markovian routing with distinct
//!   recovery transitions per failure type (malformed output vs. missing dependency
//!   vs. tool-query mismatch), not a single generic retry signal.
//! - Geometric Metrics for MoE (arxiv 2604.14500): Fisher information specialization
//!   indices and heterogeneity scores for early failure prediction.
//!
//! ## Key Concepts
//!
//! 1. **Typed Failure States** — each failure type gets its own routing transition.
//! 2. **Failure Traces as Training Data** — unsuccessful traces are more informative
//!    than success-only logs for routing policy learning.
//! 3. **Fisher Information Metrics** — principled specialization measurement using
//!    Fisher Information Metric on probability simplex, replacing ad hoc entropy.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Typed Failure States (STAR 2605.10057)
// ---------------------------------------------------------------------------

/// Failure type taxonomy — routes differently per failure mode, not a generic retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureType {
    /// Output is malformed (invalid JSON, wrong schema, truncated response)
    MalformedOutput,
    /// Required dependency or resource is missing
    MissingDependency,
    /// Tool name or parameters don't match any registered tool
    ToolQueryMismatch,
    /// Tool executed but returned an error
    ToolExecutionError,
    /// Context window exceeded
    ContextOverflow,
    /// Timeout during tool execution
    Timeout,
    /// Permission or capability denied
    PermissionDenied,
    /// Rate limit hit
    RateLimited,
    /// Unknown/unclassified failure
    Unknown,
}

impl FailureType {
    /// Classify a failure from an execution result string.
    pub fn classify(exec_result: &str, error_hint: Option<&str>) -> Self {
        let hint = error_hint.unwrap_or("").to_lowercase();
        let result_lower = exec_result.to_lowercase();

        if result_lower.contains("timeout") || hint.contains("timeout") {
            return FailureType::Timeout;
        }
        if result_lower.contains("permission") || result_lower.contains("denied")
            || hint.contains("permission") || hint.contains("access denied") {
            return FailureType::PermissionDenied;
        }
        if result_lower.contains("rate limit") || hint.contains("rate limit")
            || result_lower.contains("429") {
            return FailureType::RateLimited;
        }
        if result_lower.contains("context") && (result_lower.contains("exceed")
            || result_lower.contains("overflow") || result_lower.contains("too long")) {
            return FailureType::ContextOverflow;
        }
        if result_lower.contains("tool") && (result_lower.contains("not found")
            || result_lower.contains("unknown tool") || result_lower.contains("not registered")) {
            return FailureType::ToolQueryMismatch;
        }
        if result_lower.contains("dependency") && result_lower.contains("not found")
            || hint.contains("missing dependency") || hint.contains("not found:") {
            return FailureType::MissingDependency;
        }
        if result_lower.contains("execution error") || result_lower.contains("tool error")
            || hint.contains("execution error") {
            return FailureType::ToolExecutionError;
        }
        if result_lower.contains("parse") || result_lower.contains("json")
            || result_lower.contains("schema") || result_lower.contains("truncated")
            || hint.contains("malformed") {
            return FailureType::MalformedOutput;
        }
        FailureType::Unknown
    }

    /// Severity weight — higher = more severe = router should more aggressively route to recovery.
    pub fn severity(&self) -> f64 {
        match self {
            FailureType::Timeout => 0.9,
            FailureType::PermissionDenied => 0.8,
            FailureType::RateLimited => 0.7,
            FailureType::ContextOverflow => 0.85,
            FailureType::MissingDependency => 0.6,
            FailureType::ToolQueryMismatch => 0.5,
            FailureType::ToolExecutionError => 0.75,
            FailureType::MalformedOutput => 0.4,
            FailureType::Unknown => 0.3,
        }
    }
}

// ---------------------------------------------------------------------------
// Expert Registry — the "experts" in the MoE
// ---------------------------------------------------------------------------

/// An expert agent/skill that can handle specific task types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    pub expert_id: String,
    pub name: String,
    pub capabilities: Vec<String>,  // capability tags
    pub specialty: String,           // e.g. "reasoning", "code_gen", "web_search"
    pub success_rate: f64,          // rolling success rate
    pub usage_count: u64,
    pub failure_counts: HashMap<String, u64>,  // failure_type → count
    pub avg_latency_ms: f64,
    pub is_available: bool,
}

impl Expert {
    /// Compute per-failure-type specialization: fraction of failures of this type
    /// relative to total failures for this expert.
    pub fn failure_profile(&self) -> HashMap<FailureType, f64> {
        let total: u64 = self.failure_counts.values().sum();
        if total == 0 {
            return HashMap::new();
        }
        self.failure_counts
            .iter()
            .filter_map(|(k, v)| {
                let ft = parse_failure_type(k)?;
                Some((ft, *v as f64 / total as f64))
            })
            .collect()
    }

    /// True if this expert handles the given failure type reasonably well.
    pub fn handles_failure(&self, ft: FailureType) -> bool {
        let profile = self.failure_profile();
        let frac = profile.get(&ft).copied().unwrap_or(0.0);
        // Expert is considered "recovery-capable" for this failure if < 30% of its
        // failures are of this type (i.e. it doesn't consistently fail on this type)
        frac < 0.3 && self.is_available
    }
}

fn failure_type_to_key(ft: FailureType) -> String {
    match ft {
        FailureType::MalformedOutput => "malformed_output".to_string(),
        FailureType::MissingDependency => "missing_dependency".to_string(),
        FailureType::ToolQueryMismatch => "tool_query_mismatch".to_string(),
        FailureType::ToolExecutionError => "tool_execution_error".to_string(),
        FailureType::ContextOverflow => "context_overflow".to_string(),
        FailureType::Timeout => "timeout".to_string(),
        FailureType::PermissionDenied => "permission_denied".to_string(),
        FailureType::RateLimited => "rate_limited".to_string(),
        FailureType::Unknown => "unknown".to_string(),
    }
}

fn parse_failure_type(s: &str) -> Option<FailureType> {
    match s {
        "malformed_output" => Some(FailureType::MalformedOutput),
        "missing_dependency" => Some(FailureType::MissingDependency),
        "tool_query_mismatch" => Some(FailureType::ToolQueryMismatch),
        "tool_execution_error" => Some(FailureType::ToolExecutionError),
        "context_overflow" => Some(FailureType::ContextOverflow),
        "timeout" => Some(FailureType::Timeout),
        "permission_denied" => Some(FailureType::PermissionDenied),
        "rate_limited" => Some(FailureType::RateLimited),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Routing Decision
// ---------------------------------------------------------------------------

/// The routing decision returned by the router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub selected_expert: Option<String>,  // None = fallback/default
    pub fallback_expert: Option<String>,
    pub recovery_mode: bool,
    pub failure_type: Option<FailureType>,
    pub confidence: f64,
    pub reasoning: String,
    pub alternative_experts: Vec<String>,
}

/// Nominal (default) route specification from an expert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominalRoute {
    pub from_expert: String,
    pub to_expert: String,
    pub task_type: String,
    pub transition: TransitionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    /// Normal forwarding
    Forward,
    /// Retry same expert
    Retry,
    /// Route to recovery specialist
    Recovery,
    /// Escalate to human
    Escalate,
    /// Split and run in parallel
    Parallel,
}

// ---------------------------------------------------------------------------
// Fisher Information Metrics (arxiv 2604.14500)
// ---------------------------------------------------------------------------

/// Compute the Fisher Specialization Index (FSI) for an expert's capability vector.
/// FSI = sum_i p_i * I_i  where I_i = observed_information(p_i)
/// Higher FSI = more specialized (good for targeted routing).
pub fn fisher_specialization_index(probabilities: &[f64]) -> f64 {
    if probabilities.is_empty() {
        return 0.0;
    }
    probabilities
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| {
            // Fisher information for Bernoulli: I(p) = 1/(p(1-p))
            // on probability simplex, we use the simplified form
            let i = if p > 0.0 && p < 1.0 {
                1.0 / (p * (1.0 - p))
            } else {
                0.0
            };
            p * i
        })
        .sum::<f64>()
}

/// Fisher Heterogeneity Score (FHS) — predicts training/inference failure at low
/// completion. FHS > 1.0 is the theoretically grounded threshold for intervention.
/// FHS = concentration * FSI, where concentration = 1 - normalized_entropy.
/// Higher FHS → routing is highly concentrated in few experts → failure risk.
pub fn fisher_heterogeneity_score(probabilities: &[f64]) -> f64 {
    if probabilities.is_empty() || probabilities.iter().all(|&p| p == 0.0) {
        return 0.0;
    }
    let n = probabilities.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }
    let sum_p = probabilities.iter().filter(|&&p| p > 0.0).sum::<f64>();
    if sum_p == 0.0 {
        return 0.0;
    }
    // Entropy H = -sum(p * log2(p)) for normalized probabilities
    let h: f64 = probabilities
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| {
            let p_norm = p / sum_p;
            -p_norm * p_norm.log2()
        })
        .sum::<f64>();
    // Max entropy for n categories: log2(n)
    let max_entropy = n.log2().max(1e-9);
    let normalized_entropy = h / max_entropy;
    // Concentration: 1 means concentrated (low entropy), 0 means uniform
    let concentration = 1.0 - normalized_entropy;
    let fsi = fisher_specialization_index(probabilities);
    (concentration * fsi).max(0.0)
}

/// Predict if a system is approaching failure based on expert routing distribution.
/// Returns (is_failing, confidence) using the FHS > 1.0 threshold.
pub fn predict_failure_from_distribution(expert_loads: &[f64]) -> (bool, f64) {
    let fhs = fisher_heterogeneity_score(expert_loads);
    let is_failing = fhs > 1.0;
    // Confidence increases as FHS moves away from threshold
    let confidence = (fhs - 1.0).abs() / (fhs.abs() + 0.1);
    (is_failing, confidence.min(1.0))
}

// ---------------------------------------------------------------------------
// Routing Matrix (STAR-style)
// ---------------------------------------------------------------------------

/// Routing matrix: current_state → (expert, task_type) → next_state → next_expert
/// Maps failure states to recovery transitions, not just success paths.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingMatrix {
    /// Nominal routes (expert-specified defaults)
    nominal_routes: Vec<NominalRoute>,
    /// Recovery routes keyed by (from_expert, failure_type)
    recovery_routes: HashMap<(String, FailureType), String>,
    /// Experts in the system
    experts: Vec<Expert>,
}

impl RoutingMatrix {
    /// Register an expert in the routing matrix.
    pub fn register_expert(&mut self, expert: Expert) {
        if !self.experts.iter().any(|e| e.expert_id == expert.expert_id) {
            self.experts.push(expert);
        }
    }

    /// Add a nominal route from expert A to expert B for a task type.
    pub fn add_nominal_route(&mut self, route: NominalRoute) {
        self.nominal_routes.push(route);
    }

    /// Set a recovery route: when `from_expert` fails with `failure_type`,
    /// route to `to_expert`.
    pub fn set_recovery_route(&mut self, from_expert: &str, failure_type: FailureType, to_expert: &str) {
        self.recovery_routes.insert(
            (from_expert.to_string(), failure_type),
            to_expert.to_string(),
        );
    }

    /// Route from current expert given the task type and optional failure context.
    pub fn route(
        &self,
        current_expert: Option<&str>,
        task_type: &str,
        failure_context: Option<(&str, FailureType)>,
    ) -> RouteDecision {
        // Recovery routing (STAR): when there's a failure, use typed recovery
        if let Some((failed_expert, failure_type)) = failure_context {
            if let Some(recovery_target) = self.recovery_routes.get(&(failed_expert.to_string(), failure_type)) {
                return RouteDecision {
                    selected_expert: Some(recovery_target.clone()),
                    fallback_expert: self.get_default_expert(),
                    recovery_mode: true,
                    failure_type: Some(failure_type),
                    confidence: failure_type.severity(),
                    reasoning: format!(
                        "Recovery routing: {} failed with {:?} (severity={:.2}) → {}",
                        failed_expert, failure_type, failure_type.severity(), recovery_target
                    ),
                    alternative_experts: self.get_alternatives(recovery_target),
                };
            }
        }

        // Nominal routing
        if let Some(ce) = current_expert {
            if let Some(route) = self.nominal_routes.iter().find(|r| r.from_expert == ce && r.task_type == task_type) {
                return RouteDecision {
                    selected_expert: Some(route.to_expert.clone()),
                    fallback_expert: self.get_default_expert(),
                    recovery_mode: false,
                    failure_type: None,
                    confidence: 0.9,
                    reasoning: format!("Nominal: {} → {} for task '{}'", ce, route.to_expert, task_type),
                    alternative_experts: self.get_alternatives(&route.to_expert),
                };
            }
        }

        // Fallback: route to best available expert for this task type
        let fallback = self.get_best_expert(task_type);
        RouteDecision {
            selected_expert: fallback.clone(),
            fallback_expert: self.get_default_expert(),
            recovery_mode: false,
            failure_type: None,
            confidence: 0.5,
            reasoning: format!("Fallback routing to '{}' for task '{}'", fallback.as_deref().unwrap_or("none"), task_type),
            alternative_experts: self.get_alternatives(fallback.as_deref().unwrap_or("")),
        }
    }

    fn get_default_expert(&self) -> Option<String> {
        self.experts
            .iter()
            .filter(|e| e.is_available)
            .max_by(|a, b| {
                a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.expert_id.clone())
    }

    fn get_best_expert(&self, task_type: &str) -> Option<String> {
        self.experts
            .iter()
            .filter(|e| e.is_available && e.capabilities.iter().any(|c| c == task_type || task_type.is_empty()))
            .max_by(|a, b| {
                let a_score = a.success_rate * (1.0 / (1.0 + a.avg_latency_ms / 1000.0));
                let b_score = b.success_rate * (1.0 / (1.0 + b.avg_latency_ms / 1000.0));
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.expert_id.clone())
    }

    fn get_alternatives(&self, selected: &str) -> Vec<String> {
        self.experts
            .iter()
            .filter(|e| e.expert_id != selected && e.is_available)
            .take(3)
            .map(|e| e.expert_id.clone())
            .collect()
    }

    /// Record a failure event to update expert failure profiles.
    pub fn record_failure(&mut self, expert_id: &str, failure_type: FailureType) {
        if let Some(expert) = self.experts.iter_mut().find(|e| e.expert_id == expert_id) {
            let key = failure_type_to_key(failure_type);
            *expert.failure_counts.entry(key).or_insert(0) += 1;
        }
    }

    /// Record a success event (increments usage, updates success rate).
    pub fn record_success(&mut self, expert_id: &str, latency_ms: f64) {
        if let Some(expert) = self.experts.iter_mut().find(|e| e.expert_id == expert_id) {
            expert.usage_count += 1;
            let n = expert.usage_count as f64;
            let prev = (n - 1.0) / n;
            let curr = 1.0 / n;
            expert.success_rate = prev * expert.success_rate + curr;
            expert.avg_latency_ms = prev * expert.avg_latency_ms + curr * latency_ms;
        }
    }

    /// Compute Fisher metrics for current expert load distribution.
    pub fn expert_loads(&self) -> Vec<f64> {
        let total: u64 = self.experts.iter().map(|e| e.usage_count).sum();
        if total == 0 {
            return vec![1.0_f64 / self.experts.len().max(1) as f64; self.experts.len().max(1)];
        }
        self.experts.iter().map(|e| e.usage_count as f64 / total as f64).collect()
    }
}

// ---------------------------------------------------------------------------
// Failure Router — high-level API
// ---------------------------------------------------------------------------

pub struct FailureRouter {
    matrix: RoutingMatrix,
}

impl Default for FailureRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl FailureRouter {
    pub fn new() -> Self {
        Self {
            matrix: RoutingMatrix::default(),
        }
    }

    pub fn with_experts(experts: Vec<Expert>) -> Self {
        let mut matrix = RoutingMatrix::default();
        for expert in experts {
            matrix.register_expert(expert);
        }
        Self { matrix }
    }

    /// Add a recovery route: when `from` fails with `ft`, route to `to`.
    pub fn add_recovery_route(&mut self, from: &str, ft: FailureType, to: &str) {
        self.matrix.set_recovery_route(from, ft, to);
    }

    /// Route a request, optionally with failure context from a previous attempt.
    pub fn route(
        &self,
        current_expert: Option<&str>,
        task_type: &str,
        failure_context: Option<(&str, FailureType)>,
    ) -> RouteDecision {
        self.matrix.route(current_expert, task_type, failure_context)
    }

    /// Record an execution outcome to update expert statistics.
    pub fn record(&mut self, expert_id: &str, success: bool, failure_type: Option<FailureType>, latency_ms: f64) {
        if success {
            self.matrix.record_success(expert_id, latency_ms);
        } else if let Some(ft) = failure_type {
            self.matrix.record_failure(expert_id, ft);
        }
    }

    /// Predict system failure from current expert load distribution.
    pub fn predict_failure(&self) -> (bool, f64) {
        predict_failure_from_distribution(&self.matrix.expert_loads())
    }

    /// Get Fisher specialization index for all experts.
    pub fn fsi(&self) -> f64 {
        fisher_specialization_index(&self.matrix.expert_loads())
    }

    /// Get Fisher heterogeneity score.
    pub fn fhs(&self) -> f64 {
        fisher_heterogeneity_score(&self.matrix.expert_loads())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_expert(id: &str, success_rate: f64, capabilities: &[&str]) -> Expert {
        Expert {
            expert_id: id.to_string(),
            name: id.to_string(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            specialty: "general".to_string(),
            success_rate,
            usage_count: 10,
            failure_counts: HashMap::new(),
            avg_latency_ms: 100.0,
            is_available: true,
        }
    }

    #[test]
    fn test_failure_type_classify_timeout() {
        let ft = FailureType::classify("execution timeout after 30s", None);
        assert_eq!(ft, FailureType::Timeout);
    }

    #[test]
    fn test_failure_type_classify_tool_not_found() {
        let ft = FailureType::classify("tool not found in registry", None);
        assert_eq!(ft, FailureType::ToolQueryMismatch);
    }

    #[test]
    fn test_failure_type_classify_malformed_json() {
        let ft = FailureType::classify("invalid json response", Some("parse error"));
        assert_eq!(ft, FailureType::MalformedOutput);
    }

    #[test]
    fn test_failure_type_severity() {
        assert!(FailureType::Timeout.severity() > FailureType::MalformedOutput.severity());
        assert!(FailureType::ContextOverflow.severity() > FailureType::Unknown.severity());
    }

    #[test]
    fn test_fisher_specialization_index() {
        // Uniform distribution → low FSI (low specialization)
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        let fsi = fisher_specialization_index(&uniform);
        assert!(fsi >= 0.0);

        // Skewed distribution → higher FSI
        let skewed = vec![0.9, 0.05, 0.03, 0.02];
        let fsi_skewed = fisher_specialization_index(&skewed);
        assert!(fsi_skewed > fsi, "skewed should have higher FSI");
    }

    #[test]
    fn test_fisher_heterogeneity_score() {
        let probs = vec![0.5, 0.3, 0.2];
        let fhs = fisher_heterogeneity_score(&probs);
        assert!(fhs >= 0.0);
    }

    #[test]
    fn test_fhs_threshold_prediction() {
        // Concentrated load → FHS > 1.0 → failure predicted
        let concentrated = vec![0.95, 0.03, 0.02];
        let (is_failing, conf) = predict_failure_from_distribution(&concentrated);
        assert!(is_failing, "concentrated load should predict failure");

        // Balanced load → FHS < 1.0 → no failure
        let balanced = vec![0.33, 0.33, 0.34];
        let (is_failing_balanced, _) = predict_failure_from_distribution(&balanced);
        assert!(!is_failing_balanced, "balanced load should not predict failure");
    }

    #[test]
    fn test_router_nominal_routing() {
        let mut router = FailureRouter::new();
        router.matrix.register_expert(make_expert("reasoner", 0.9, &["reasoning"]));
        router.matrix.register_expert(make_expert("coder", 0.8, &["code_gen"]));
        router.matrix.add_nominal_route(NominalRoute {
            from_expert: "reasoner".to_string(),
            to_expert: "coder".to_string(),
            task_type: "code_gen".to_string(),
            transition: TransitionType::Forward,
        });

        let decision = router.route(Some("reasoner"), "code_gen", None);
        assert_eq!(decision.selected_expert.as_deref(), Some("coder"));
        assert!(!decision.recovery_mode);
        assert_eq!(decision.confidence, 0.9);
    }

    #[test]
    fn test_router_recovery_routing() {
        let mut router = FailureRouter::new();
        router.matrix.register_expert(make_expert("fast", 0.7, &["quick"]));
        router.matrix.register_expert(make_expert("slow", 0.95, &["thorough"]));

        router.add_recovery_route("fast", FailureType::Timeout, "slow");

        let decision = router.route(Some("fast"), "thorough", Some(("fast", FailureType::Timeout)));
        assert_eq!(decision.selected_expert.as_deref(), Some("slow"));
        assert!(decision.recovery_mode);
        assert_eq!(decision.failure_type, Some(FailureType::Timeout));
        assert!(decision.confidence > 0.0);
    }

    #[test]
    fn test_record_failure_updates_profile() {
        let mut router = FailureRouter::new();
        router.matrix.register_expert(make_expert("test", 0.5, &["test"]));
        router.matrix.record_failure("test", FailureType::Timeout);
        router.matrix.record_failure("test", FailureType::Timeout);
        router.matrix.record_failure("test", FailureType::MalformedOutput);

        let expert = router.matrix.experts.iter().find(|e| e.expert_id == "test").unwrap();
        assert_eq!(*expert.failure_counts.get("timeout").unwrap_or(&0), 2);
        assert_eq!(*expert.failure_counts.get("malformed_output").unwrap_or(&0), 1);
    }

    #[test]
    fn test_record_success_updates_rates() {
        let mut router = FailureRouter::new();
        router.matrix.register_expert(make_expert("test", 0.5, &["test"]));
        router.matrix.record_success("test", 150.0);

        let expert = router.matrix.experts.iter().find(|e| e.expert_id == "test").unwrap();
        assert_eq!(expert.usage_count, 11); // started at 10
        assert!(expert.success_rate > 0.5);
    }

    #[test]
    fn test_fsi_empty() {
        assert_eq!(fisher_specialization_index(&[]), 0.0);
    }

    #[test]
    fn test_expert_handles_failure() {
        let mut expert = make_expert("test", 0.5, &["test"]);
        expert.failure_counts.insert("timeout".to_string(), 5);
        expert.failure_counts.insert("malformed_output".to_string(), 1);
        // timeout is 5/6 ≈ 83%, malformed_output is 1/6 ≈ 17%
        assert!(!expert.handles_failure(FailureType::Timeout), "high failure rate → not recovery-capable");
        assert!(expert.handles_failure(FailureType::MalformedOutput), "low failure rate → recovery-capable");
    }

    #[test]
    fn test_route_decision_has_alternatives() {
        let mut router = FailureRouter::new();
        router.matrix.register_expert(make_expert("a", 0.9, &["task"]));
        router.matrix.register_expert(make_expert("b", 0.8, &["task"]));
        router.matrix.register_expert(make_expert("c", 0.7, &["task"]));

        let decision = router.route(Some("a"), "task", None);
        assert!(!decision.alternative_experts.is_empty());
        assert!(!decision.alternative_experts.contains(&"a".to_string()));
    }
}
