//! LLM-Powered Research Route Planner — plans and tracks research routes.
//!
//! Mirrors llm/routing/route_planner.py

use serde::{Deserialize, Serialize};

// ─── Data Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    ReadPaper,
    RunExperiment,
    CompareMethods,
    WriteAnalysis,
    SurveyBaselines,
    CheckContradiction,
    ReviseHypothesis,
}

impl StepType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadPaper => "read_paper",
            Self::RunExperiment => "run_experiment",
            Self::CompareMethods => "compare_methods",
            Self::WriteAnalysis => "write_analysis",
            Self::SurveyBaselines => "survey_baselines",
            Self::CheckContradiction => "check_contradiction",
            Self::ReviseHypothesis => "revise_hypothesis",
        }
    }
    pub fn from_string(s: &str) -> Self {
        match s {
            "read_paper" => Self::ReadPaper,
            "run_experiment" => Self::RunExperiment,
            "compare_methods" => Self::CompareMethods,
            "write_analysis" => Self::WriteAnalysis,
            "survey_baselines" => Self::SurveyBaselines,
            "check_contradiction" => Self::CheckContradiction,
            "revise_hypothesis" => Self::ReviseHypothesis,
            _ => Self::ReadPaper,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    Active,
    Completed,
    Abandoned,
    Revised,
}

impl PlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
            Self::Revised => "revised",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: String,
    pub step_type: String,
    pub description: String,
    pub estimated_hours: f64,
    pub dependencies: Vec<String>,
    pub status: String,
    pub result: String,
    pub notes: String,
}

impl PlanStep {
    pub fn new(id: &str, step_type: &str, description: &str, hours: f64, deps: Vec<String>) -> Self {
        Self {
            step_id: id.to_string(),
            step_type: step_type.to_string(),
            description: description.to_string(),
            estimated_hours: hours,
            dependencies: deps,
            status: "pending".to_string(),
            result: String::new(),
            notes: String::new(),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "skipped")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub total_steps: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub progress_pct: f64,
    pub estimated_hours: f64,
    pub completed_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchPlan {
    pub plan_id: String,
    pub hypothesis: String,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub status: String,
    pub current_step_id: String,
    pub revision_count: u32,
    pub parent_plan_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoutePlanVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

impl RoutePlanVerificationResult {
    pub fn valid() -> Self {
        Self { is_valid: true, warnings: Vec::new() }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { is_valid: warnings.is_empty(), warnings }
    }
}

impl ResearchPlan {
    pub fn new(plan_id: &str, hypothesis: &str, goal: &str, steps: Vec<PlanStep>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            plan_id: plan_id.to_string(),
            hypothesis: hypothesis.to_string(),
            goal: goal.to_string(),
            steps,
            status: "active".to_string(),
            current_step_id: String::new(),
            revision_count: 0,
            parent_plan_id: String::new(),
            created_at: now.clone(),
            updated_at: now,
            verification_warnings: Vec::new(),
        }
    }

    pub fn get_step(&self, step_id: &str) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.step_id == step_id)
    }

    pub fn get_ready_steps(&self) -> Vec<&PlanStep> {
        let completed_ids: std::collections::HashSet<&str> = self.steps
            .iter()
            .filter(|s| s.is_done())
            .map(|s| s.step_id.as_str())
            .collect();

        self.steps
            .iter()
            .filter(|s| s.status == "pending")
            .filter(|s| s.dependencies.iter().all(|d| completed_ids.contains(d.as_str())))
            .collect()
    }

    pub fn get_progress(&self) -> Progress {
        let total = self.steps.len();
        let completed = self.steps.iter().filter(|s| s.status == "completed").count();
        let failed = self.steps.iter().filter(|s| s.status == "failed").count();
        let pending = total - completed - failed;
        let total_hours: f64 = self.steps.iter().map(|s| s.estimated_hours).sum();
        let completed_hours: f64 = self.steps
            .iter()
            .filter(|s| s.status == "completed")
            .map(|s| s.estimated_hours)
            .sum();

        Progress {
            total_steps: total,
            completed,
            failed,
            pending,
            progress_pct: if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 },
            estimated_hours: total_hours,
            completed_hours,
        }
    }

    /// Validate that dependencies form a DAG (no circular deps).
    pub fn validate_dag(&self) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut in_stack = std::collections::HashSet::new();

        for step in &self.steps {
            if !visited.contains(&step.step_id)
                && has_cycle(&step.step_id, &self.steps, &mut visited, &mut in_stack) {
                    return false;
                }
        }
        true
    }
}

fn has_cycle(
    node: &str,
    steps: &[PlanStep],
    visited: &mut std::collections::HashSet<String>,
    in_stack: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(node.to_string());
    in_stack.insert(node.to_string());

    if let Some(step) = steps.iter().find(|s| s.step_id == node) {
        for dep in &step.dependencies {
            if !visited.contains(dep) {
                if has_cycle(dep, steps, visited, in_stack) {
                    return true;
                }
            } else if in_stack.contains(dep) {
                return true;
            }
        }
    }

    in_stack.remove(node);
    false
}

// ─── Persistence ───────────────────────────────────────────────────────────

fn route_plans_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai_research_os")
        .join("route_plans")
}

pub fn save_plan(plan: &ResearchPlan) -> Result<(), String> {
    let dir = route_plans_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", plan.plan_id));
    let json = serde_json::to_string_pretty(plan).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_plans() -> Vec<ResearchPlan> {
    let dir = route_plans_dir();
    if !dir.exists() { return vec![]; }
    let mut plans = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(plan) = serde_json::from_str::<ResearchPlan>(&content) {
                        plans.push(plan);
                    }
                }
            }
        }
    }
    plans.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    plans
}

pub fn list_plans(limit: usize) -> Vec<ResearchPlan> {
    let plans = load_plans();
    plans.into_iter().take(limit).collect()
}

pub fn update_step(plan_id: &str, step_id: &str, status: &str, result: &str, notes: &str) -> Option<ResearchPlan> {
    let mut plans = load_plans();
    let plan = plans.iter_mut().find(|p| p.plan_id == plan_id)?;
    let step = plan.steps.iter_mut().find(|s| s.step_id == step_id)?;
    step.status = status.to_string();
    if !result.is_empty() { step.result = result.to_string(); }
    if !notes.is_empty() { step.notes = notes.to_string(); }
    plan.updated_at = chrono::Utc::now().to_rfc3339();

    // Check if all steps done → mark plan completed
    if plan.steps.iter().all(|s| s.status == "completed" || s.status == "skipped") {
        plan.status = "completed".to_string();
    }

    let result = plan.clone();
    // Re-save all plans
    for p in &plans { let _ = save_plan(p); }
    Some(result)
}

pub fn revise_plan(plan_id: &str, _reason: &str) -> Option<ResearchPlan> {
    let mut plans = load_plans();
    let old_idx = plans.iter().position(|p| p.plan_id == plan_id)?;
    let old = &plans[old_idx];

    let new_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut new_plan = ResearchPlan::new(&new_id, &old.hypothesis, &old.goal, old.steps.clone());
    new_plan.parent_plan_id = plan_id.to_string();
    new_plan.revision_count = old.revision_count + 1;

    // Mark old plan as revised
    plans[old_idx].status = "revised".to_string();
    plans[old_idx].updated_at = chrono::Utc::now().to_rfc3339();

    // Save everything
    for p in &plans { let _ = save_plan(p); }
    let _ = save_plan(&new_plan);

    Some(new_plan)
}

// ─── LLM-based plan creation ──────────────────────────────────────────────

pub async fn create_plan(
    llm: &dyn crate::LlmClient,
    model: &str,
    hypothesis: &str,
    goal: &str,
    known_papers: &[String],
) -> ResearchPlan {
    let papers_ctx = if known_papers.is_empty() {
        String::new()
    } else {
        format!("\n\nKnown papers:\n{}", known_papers.join("\n"))
    };

    let prompt = format!(
        "HYPOTHESIS: {}\nGOAL: {}{}\n\nCreate a research plan with 5-8 steps.",
        hypothesis, goal, papers_ctx
    );

    let msg = crate::Message { role: "user".to_string(), content: prompt };
    let plan_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let steps = match llm.complete(vec![msg], model, 0.3, 2000).await {
        Ok(crate::LlmResponse::NonStream(ns)) => parse_plan_steps(&ns.content),
        _ => default_steps(hypothesis),
    };

    let verification = verify_route_plan(llm, model, hypothesis, goal, &steps).await;

    let mut plan = ResearchPlan::new(&plan_id, hypothesis, goal, steps);
    plan.verification_warnings = verification.warnings;
    plan
}

const VERIFY_ROUTE_PLAN_PROMPT: &str = r#"你是一个严谨的研究计划验证助手。检查以下计划是否合理。

假说: {hypothesis}
目标: {goal}
步骤数: {step_count}

请验证：
1. 步骤是否与假说和目标相关？
2. 步骤顺序是否合理（依赖关系是否正确）？
3. 每个步骤是否可行？
4. 估计时间是否合理？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1", "问题2"]}}

如果计划合理，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_route_plan(
    llm: &dyn crate::LlmClient,
    model: &str,
    hypothesis: &str,
    goal: &str,
    steps: &[PlanStep],
) -> RoutePlanVerificationResult {
    if steps.is_empty() {
        return RoutePlanVerificationResult::valid();
    }

    let step_count = steps.len();
    let steps_str = steps.iter().enumerate().map(|(i, s)| {
        format!("{}. [{}] {} (预计{}小时)", i + 1, s.step_type, s.description, s.estimated_hours)
    }).collect::<Vec<_>>().join("\n");

    let prompt = VERIFY_ROUTE_PLAN_PROMPT
        .replace("{hypothesis}", hypothesis)
        .replace("{goal}", goal)
        .replace("{step_count}", &step_count.to_string())
        .replace("{steps}", &steps_str);

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 300).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_verification_result(&ns.content)
        }
        _ => RoutePlanVerificationResult::valid(),
    }
}

fn parse_verification_result(content: &str) -> RoutePlanVerificationResult {
    let content = content.trim();

    let _is_valid = if content.contains("\"is_valid\": true") || content.contains("\"is_valid\":true") {
        true
    } else if content.contains("\"is_valid\": false") || content.contains("\"is_valid\":false") {
        false
    } else {
        return RoutePlanVerificationResult::valid();
    };

    let mut warnings = Vec::new();
    if let Some(start) = content.find("\"warnings\":") {
        let warnings_str = &content[start..];
        if let Some(arr_start) = warnings_str.find('[') {
            if let Some(arr_end) = warnings_str.find(']') {
                let items = &warnings_str[arr_start + 1..arr_end];
                for item in items.split(',') {
                    let item = item.trim().trim_matches('"').trim_matches(|c| c == '"' || c == ' ');
                    if !item.is_empty() && item != "[]" && item != "warnings" {
                        warnings.push(item.to_string());
                    }
                }
            }
        }
    }

    RoutePlanVerificationResult::with_warnings(warnings)
}

fn parse_plan_steps(body: &str) -> Vec<PlanStep> {
    let json_start = body.find('{');
    let json_end = body.rfind('}');
    let json_str = match (json_start, json_end) {
        (Some(s), Some(e)) if s < e => &body[s..=e],
        _ => return default_steps(""),
    };

    let val: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return default_steps(""),
    };

    let steps_arr = match val.get("steps").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return default_steps(""),
    };

    let mut steps = Vec::new();
    for (i, s) in steps_arr.iter().enumerate() {
        let step_type = s.get("type").and_then(|v| v.as_str()).unwrap_or("read_paper");
        let description = s.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let hours = s.get("estimated_hours").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let deps: Vec<String> = s.get("dependencies")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|d| d.as_str().map(String::from)).collect())
            .unwrap_or_default();

        steps.push(PlanStep::new(
            &format!("step_{}", i + 1),
            step_type,
            description,
            hours,
            deps,
        ));
    }

    if steps.is_empty() { default_steps("") } else { steps }
}

fn default_steps(hypothesis: &str) -> Vec<PlanStep> {
    let h = if hypothesis.len() > 60 { &hypothesis[..60] } else { hypothesis };
    vec![
        PlanStep::new("step_1", "survey_baselines", &format!("Survey existing baseline methods for {}", h), 4.0, vec![]),
        PlanStep::new("step_2", "read_paper", &format!("Read 3 most relevant papers on {}", h), 3.0, vec![]),
        PlanStep::new("step_3", "check_contradiction", "Check if any evidence contradicts the hypothesis", 2.0,
                      vec!["step_1".into(), "step_2".into()]),
        PlanStep::new("step_4", "run_experiment", &format!("Design and run experiments to test {}", h), 8.0,
                      vec!["step_3".into()]),
        PlanStep::new("step_5", "compare_methods", "Compare results against reported baselines", 3.0,
                      vec!["step_4".into()]),
        PlanStep::new("step_6", "write_analysis", "Write up findings and conclusions", 4.0,
                      vec!["step_5".into()]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_step_found() {
        let plan = ResearchPlan::new("p1", "hypothesis", "goal", vec![
            PlanStep::new("s1", "read_paper", "Read paper", 2.0, vec![]),
        ]);
        assert!(plan.get_step("s1").is_some());
        assert!(plan.get_step("nonexistent").is_none());
    }

    #[test]
    fn test_get_ready_steps() {
        let steps = vec![
            PlanStep::new("s1", "read_paper", "Read A", 2.0, vec![]),
            PlanStep::new("s2", "run_experiment", "Run B", 4.0, vec!["s1".into()]),
            PlanStep::new("s3", "write_analysis", "Write", 2.0, vec!["s2".into()]),
        ];
        let plan = ResearchPlan::new("p1", "h", "g", steps);
        let ready: Vec<String> = plan.get_ready_steps().iter().map(|s| s.step_id.clone()).collect();
        assert_eq!(ready, vec!["s1"]);
    }

    #[test]
    fn test_get_ready_steps_with_deps_met() {
        let mut steps = vec![
            PlanStep::new("s1", "read_paper", "Read A", 2.0, vec![]),
            PlanStep::new("s2", "run_experiment", "Run B", 4.0, vec!["s1".into()]),
        ];
        steps[0].status = "completed".to_string();
        let plan = ResearchPlan::new("p1", "h", "g", steps);
        assert_eq!(plan.get_ready_steps().len(), 1);
        assert_eq!(plan.get_ready_steps()[0].step_id, "s2");
    }

    #[test]
    fn test_progress() {
        let mut steps = vec![
            PlanStep::new("s1", "read_paper", "R1", 2.0, vec![]),
            PlanStep::new("s2", "run_experiment", "R2", 4.0, vec![]),
        ];
        steps[0].status = "completed".to_string();
        let plan = ResearchPlan::new("p1", "h", "g", steps);
        let p = plan.get_progress();
        assert_eq!(p.total_steps, 2);
        assert_eq!(p.completed, 1);
        assert_eq!(p.completed_hours, 2.0);
    }

    #[test]
    fn test_dag_valid() {
        let plan = ResearchPlan::new("p1", "h", "g", vec![
            PlanStep::new("s1", "read_paper", "R1", 1.0, vec![]),
            PlanStep::new("s2", "run_experiment", "R2", 1.0, vec!["s1".into()]),
        ]);
        assert!(plan.validate_dag());
    }

    #[test]
    fn test_dag_cycle_detected() {
        let plan = ResearchPlan::new("p1", "h", "g", vec![
            PlanStep::new("s1", "read_paper", "R1", 1.0, vec!["s2".into()]),
            PlanStep::new("s2", "run_experiment", "R2", 1.0, vec!["s1".into()]),
        ]);
        assert!(!plan.validate_dag());
    }

    #[test]
    fn test_default_steps_nonempty() {
        let steps = default_steps("test hypothesis");
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[0].step_type, "survey_baselines");
        assert_eq!(steps[5].step_type, "write_analysis");
    }

    #[test]
    fn test_parse_plan_steps_valid() {
        let json = r#"{"steps": [{"type": "read_paper", "description": "Read paper A", "estimated_hours": 2.0, "dependencies": []}]}"#;
        let steps = parse_plan_steps(json);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].description, "Read paper A");
    }

    #[test]
    fn test_parse_plan_steps_invalid() {
        let steps = parse_plan_steps("not valid json");
        assert!(!steps.is_empty()); // falls back to defaults
    }

    #[test]
    fn test_step_type_roundtrip() {
        for t in &["read_paper", "run_experiment", "write_analysis", "survey_baselines"] {
            assert_eq!(StepType::from_string(t).as_str(), *t);
        }
    }

    #[test]
    fn test_parse_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_verification_result(json);
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["步骤顺序不合理", "估计时间过长"]}"#;
        let result = parse_verification_result(json);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn test_parse_verification_result_malformed() {
        let json = "not json at all";
        let result = parse_verification_result(json);
        assert!(result.is_valid);
    }

    #[test]
    fn test_route_plan_verification_result_valid() {
        let result = RoutePlanVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_route_plan_verification_result_with_warnings() {
        let warnings = vec!["依赖关系错误".to_string()];
        let result = RoutePlanVerificationResult::with_warnings(warnings);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_research_plan_has_verification_warnings() {
        let steps = vec![PlanStep::new("s1", "read_paper", "R1", 1.0, vec![])];
        let plan = ResearchPlan::new("p1", "h", "g", steps);
        assert!(plan.verification_warnings.is_empty());
    }
}
