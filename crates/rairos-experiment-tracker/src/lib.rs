//! rairos-experiment-tracker — Experiment Tracker for research roadmaps.
//!
//! Ported from `llm/experiment_tracker.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExperimentStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExperimentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExperimentStatus::Running => "running",
            ExperimentStatus::Completed => "completed",
            ExperimentStatus::Failed => "failed",
            ExperimentStatus::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "running" => Some(ExperimentStatus::Running),
            "completed" => Some(ExperimentStatus::Completed),
            "failed" => Some(ExperimentStatus::Failed),
            "cancelled" => Some(ExperimentStatus::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub timestamp: String,
}

impl Metric {
    pub fn new(name: &str, value: f64, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub roadmap_milestone: String,
    #[serde(default)]
    pub hypothesis_id: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub results: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub completed_at: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_status() -> String {
    "running".to_string()
}

impl Experiment {
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        roadmap_milestone: &str,
        hypothesis_id: &str,
        config: HashMap<String, serde_json::Value>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            roadmap_milestone: roadmap_milestone.to_string(),
            hypothesis_id: hypothesis_id.to_string(),
            config,
            results: HashMap::new(),
            metrics: Vec::new(),
            status: "running".to_string(),
            created_at: Utc::now().to_rfc3339(),
            completed_at: String::new(),
            artifacts: Vec::new(),
            tags,
        }
    }
}

pub struct ExperimentTracker {
    data_file: PathBuf,
}

impl ExperimentTracker {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_dir = data_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("experiments")
        });
        let _ = fs::create_dir_all(&data_dir);
        let data_file = data_dir.join("experiments.json");
        if !data_file.exists() {
            let _ = fs::write(&data_file, "[]");
        }
        Self { data_file }
    }

    fn load_experiments(&self) -> Vec<Experiment> {
        match fs::read_to_string(&self.data_file) {
            Ok(text) => {
                if text.trim().is_empty() {
                    return Vec::new();
                }
                serde_json::from_str(&text).unwrap_or_else(|_| Vec::new())
            }
            Err(_) => Vec::new(),
        }
    }

    fn save_experiments(&self, exps: &[Experiment]) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(exps)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.data_file, json)
    }

    pub fn run(
        &self,
        name: &str,
        description: &str,
        roadmap_milestone: &str,
        hypothesis_id: &str,
        config: Option<HashMap<String, serde_json::Value>>,
        tags: Option<Vec<String>>,
    ) -> Experiment {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let exp = Experiment::new(
            &id,
            name,
            description,
            roadmap_milestone,
            hypothesis_id,
            config.unwrap_or_default(),
            tags.unwrap_or_default(),
        );

        let mut exps = self.load_experiments();
        exps.push(exp.clone());
        let _ = self.save_experiments(&exps);
        exp
    }

    pub fn get(&self, eid: &str) -> Option<Experiment> {
        self.load_experiments().into_iter().find(|e| e.id == eid)
    }

    pub fn list_experiments(
        &self,
        status: Option<&str>,
        milestone: Option<&str>,
        tag: Option<&str>,
    ) -> Vec<Experiment> {
        let mut exps = self.load_experiments();
        if let Some(s) = status {
            exps.retain(|e| e.status == s);
        }
        if let Some(m) = milestone {
            exps.retain(|e| e.roadmap_milestone == m);
        }
        if let Some(t) = tag {
            exps.retain(|e| e.tags.contains(&t.to_string()));
        }
        exps.sort_by(|a, b| {
            let a_ts = DateTime::parse_from_rfc3339(&a.created_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let b_ts = DateTime::parse_from_rfc3339(&b.created_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            b_ts.cmp(&a_ts)
        });
        exps
    }

    pub fn complete(
        &self,
        eid: &str,
        results: Option<HashMap<String, serde_json::Value>>,
    ) -> Option<Experiment> {
        let mut exps = self.load_experiments();
        let idx = exps.iter().position(|e| e.id == eid);
        if let Some(idx) = idx {
            exps[idx].status = "completed".to_string();
            exps[idx].completed_at = Utc::now().to_rfc3339();
            if let Some(r) = results {
                exps[idx].results = r;
            }
            let result = exps[idx].clone();
            let _ = self.save_experiments(&exps);
            return Some(result);
        }
        None
    }

    pub fn fail(&self, eid: &str, error: &str) -> Option<Experiment> {
        let mut exps = self.load_experiments();
        let idx = exps.iter().position(|e| e.id == eid);
        if let Some(idx) = idx {
            exps[idx].status = "failed".to_string();
            exps[idx].completed_at = Utc::now().to_rfc3339();
            if !error.is_empty() {
                exps[idx]
                    .results
                    .insert("error".to_string(), serde_json::json!(error));
            }
            let result = exps[idx].clone();
            let _ = self.save_experiments(&exps);
            return Some(result);
        }
        None
    }

    pub fn add_metric(&self, eid: &str, name: &str, value: f64, unit: &str) -> Option<Experiment> {
        let mut exps = self.load_experiments();
        let idx = exps.iter().position(|e| e.id == eid);
        if let Some(idx) = idx {
            exps[idx].metrics.push(Metric::new(name, value, unit));
            let result = exps[idx].clone();
            let _ = self.save_experiments(&exps);
            return Some(result);
        }
        None
    }

    pub fn compare(
        &self,
        exp_ids: &[String],
        metric_names: Option<Vec<String>>,
    ) -> HashMap<String, serde_json::Value> {
        let exps: Vec<Option<Experiment>> = exp_ids.iter().map(|id| self.get(id)).collect();
        let exps: Vec<&Experiment> = exps.iter().filter_map(|e| e.as_ref()).collect();

        if exps.is_empty() {
            let mut map = HashMap::new();
            map.insert(
                "error".to_string(),
                serde_json::json!("No experiments found"),
            );
            return map;
        }

        let metric_names = metric_names.unwrap_or_else(|| {
            exps.iter()
                .flat_map(|e| e.metrics.iter().map(|m| m.name.clone()))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        });

        let rows: Vec<serde_json::Map<String, serde_json::Value>> = exps
            .iter()
            .map(|e| {
                let mut row = serde_json::json!({
                    "id": e.id,
                    "name": e.name,
                    "status": e.status
                })
                .as_object()
                .unwrap()
                .clone();

                for mn in &metric_names {
                    if let Some(m) = e.metrics.iter().find(|m| &m.name == mn) {
                        row.insert(mn.clone(), serde_json::json!(m.value));
                    }
                }
                row
            })
            .collect();

        let mut result = HashMap::new();
        result.insert("metrics".to_string(), serde_json::json!(metric_names));
        result.insert("experiments".to_string(), serde_json::json!(rows));
        result
    }

    pub fn delete(&self, eid: &str) -> bool {
        let mut exps = self.load_experiments();
        let n = exps.len();
        exps.retain(|e| e.id != eid);
        if exps.len() < n {
            let _ = self.save_experiments(&exps);
            true
        } else {
            false
        }
    }

    pub fn render_list(&self, exps: &[Experiment], verbose: bool) -> String {
        if exps.is_empty() {
            return "No experiments found.".to_string();
        }

        let mut by_status: HashMap<&str, usize> = HashMap::new();
        for e in exps {
            *by_status.entry(&e.status).or_insert(0) += 1;
        }
        let total = exps.len();
        let summary = format!(
            "Total: {}  |  {}",
            total,
            by_status
                .iter()
                .map(|(s, c)| format!("{}: {}", s, c))
                .collect::<Vec<_>>()
                .join("  |  ")
        );

        let icons: HashMap<&str, &str> = [("running", "⚡"), ("completed", "✓"), ("failed", "✗")]
            .into_iter()
            .collect();

        let mut lines = vec![summary, String::new()];
        for e in exps {
            lines.push(format!(
                "{} [{}] {} ({})",
                icons.get(e.status.as_str()).unwrap_or(&"?"),
                e.id,
                e.name,
                e.status
            ));
            if !e.roadmap_milestone.is_empty() {
                lines.push(format!("  Milestone: {}", e.roadmap_milestone));
            }
            if verbose && !e.metrics.is_empty() {
                lines.push(format!(
                    "  Metrics: {}",
                    e.metrics
                        .iter()
                        .map(|m| format!("{}={}", m.name, m.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        lines.join("\n")
    }

    pub fn render_compare(&self, comp: &HashMap<String, serde_json::Value>) -> String {
        if comp.get("error").is_some() {
            return format!(
                "Error: {}",
                comp.get("error").and_then(|v| v.as_str()).unwrap_or("")
            );
        }

        let metrics = comp
            .get("metrics")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        let experiments = comp
            .get("experiments")
            .and_then(|v| v.as_array())
            .map(|a| a.to_vec())
            .unwrap_or_default();

        let mut lines = vec![
            "## Experiment Comparison".to_string(),
            String::new(),
            format!("| Exp | {} |", metrics.join(" | ")),
            format!(
                "|---| {} |",
                metrics.iter().map(|_| "---").collect::<Vec<_>>().join("|")
            ),
        ];

        for exp in experiments {
            if let Some(obj) = exp.as_object() {
                let name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .chars()
                    .take(15)
                    .collect::<String>();
                let vals: Vec<String> = metrics
                    .iter()
                    .map(|m| {
                        obj.get(*m)
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "-".to_string())
                    })
                    .collect();
                lines.push(format!("| {} | {} |", name, vals.join(" | ")));
            }
        }

        lines.join("\n")
    }
}

#[allow(dead_code)]
fn _to_slice<T: Copy>(val: &T) -> &[T] {
    std::slice::from_ref(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tracker() -> (ExperimentTracker, tempfile::TempDir) {
        let temp = tempfile::TempDir::new().unwrap();
        let tracker = ExperimentTracker::new(Some(temp.path().to_path_buf()));
        (tracker, temp)
    }

    #[test]
    fn test_experiment_status_as_str() {
        assert_eq!(ExperimentStatus::Running.as_str(), "running");
        assert_eq!(ExperimentStatus::Completed.as_str(), "completed");
        assert_eq!(ExperimentStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_experiment_status_parse() {
        assert_eq!(
            ExperimentStatus::parse("running"),
            Some(ExperimentStatus::Running)
        );
        assert_eq!(
            ExperimentStatus::parse("completed"),
            Some(ExperimentStatus::Completed)
        );
        assert_eq!(ExperimentStatus::parse("invalid"), None);
    }

    #[test]
    fn test_metric_new() {
        let metric = Metric::new("accuracy", 0.95, "%");
        assert_eq!(metric.name, "accuracy");
        assert_eq!(metric.value, 0.95);
        assert_eq!(metric.unit, "%");
        assert!(!metric.timestamp.is_empty());
    }

    #[test]
    fn test_experiment_run() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "desc", "milestone1", "hyp1", None, None);
        assert_eq!(exp.name, "Test Exp");
        assert_eq!(exp.status, "running");
        assert!(!exp.id.is_empty());
    }

    #[test]
    fn test_experiment_get() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "", "", "", None, None);
        let found = tracker.get(&exp.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Exp");
    }

    #[test]
    fn test_experiment_get_not_found() {
        let (tracker, _temp) = temp_tracker();
        let found = tracker.get("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_experiment_complete() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "", "", "", None, None);
        let completed = tracker.complete(&exp.id, None);
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, "completed");
    }

    #[test]
    fn test_experiment_fail() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "", "", "", None, None);
        let failed = tracker.fail(&exp.id, "Test error");
        assert!(failed.is_some());
        assert_eq!(failed.unwrap().status, "failed");
    }

    #[test]
    fn test_experiment_add_metric() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "", "", "", None, None);
        let updated = tracker.add_metric(&exp.id, "accuracy", 0.95, "%");
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.metrics.len(), 1);
        assert_eq!(updated.metrics[0].value, 0.95);
    }

    #[test]
    fn test_experiment_delete() {
        let (tracker, _temp) = temp_tracker();
        let exp = tracker.run("Test Exp", "", "", "", None, None);
        assert!(tracker.delete(&exp.id));
        assert!(tracker.get(&exp.id).is_none());
    }

    #[test]
    fn test_experiment_list_filter_status() {
        let (tracker, _temp) = temp_tracker();
        tracker.run("Exp1", "", "", "", None, None);
        let exp2 = tracker.run("Exp2", "", "", "", None, None);
        tracker.complete(&exp2.id, None);

        let running = tracker.list_experiments(Some("running"), None, None);
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].name, "Exp1");
    }

    #[test]
    fn test_experiment_render_list() {
        let (tracker, _temp) = temp_tracker();
        tracker.run("Exp1", "", "", "", None, None);
        let result = tracker.render_list(&tracker.load_experiments(), false);
        assert!(result.contains("Exp1"));
    }

    #[test]
    fn test_experiment_compare() {
        let (tracker, _temp) = temp_tracker();
        let exp1 = tracker.run("Exp1", "", "", "", None, None);
        tracker.add_metric(&exp1.id, "accuracy", 0.9, "%");

        let comp = tracker.compare(&[exp1.id.clone()], None);
        assert!(comp.contains_key("metrics"));
        assert!(comp.contains_key("experiments"));
    }
}
