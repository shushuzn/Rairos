use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use sysinfo::System;

// ============================================================================
// BackupManager (from core/backup.py)
// ============================================================================

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl Default for BackupManager {
    fn default() -> Self {
        let dir = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache")
            .join("ai_research_os")
            .join("backups");
        std::fs::create_dir_all(&dir).ok();
        Self { backup_dir: dir }
    }
}

impl BackupManager {
    pub fn new(backup_dir: Option<PathBuf>) -> Self {
        let dir = backup_dir.unwrap_or_else(|| {
            dirs_next()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache")
                .join("ai_research_os")
                .join("backups")
        });
        std::fs::create_dir_all(&dir).ok();
        Self { backup_dir: dir }
    }

    pub fn create_backup(&self, source_dir: &str, description: &str) -> Result<String, String> {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("backup_{ts}");
        let dest = self.backup_dir.join(&backup_name);
        let src = PathBuf::from(source_dir);
        if !src.exists() {
            return Err(format!("Source directory not found: {source_dir}"));
        }
        let result = std::process::Command::new("cp")
            .arg("-r")
            .arg(&src)
            .arg(&dest)
            .status();
        match result {
            Ok(s) if s.success() => {
                let manifest = format!(
                    "backup_name={backup_name}\nsource={source_dir}\ndescription={description}\ndate={ts}\n"
                );
                std::fs::write(dest.join("_backup_manifest.txt"), manifest).ok();
                Ok(dest.to_string_lossy().to_string())
            }
            _ => Err(format!("Failed to backup {source_dir}")),
        }
    }

    pub fn list_backups(&self) -> Vec<HashMap<String, String>> {
        let mut result = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.backup_dir) else {
            return result;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let manifest = entry.path().join("_backup_manifest.txt");
            let desc = std::fs::read_to_string(&manifest).unwrap_or_default();
            result.push(HashMap::from([
                ("name".to_string(), name),
                ("description".to_string(), desc),
            ]));
        }
        result.sort_by(|a, b| b["name"].cmp(&a["name"]));
        result
    }
}

// ============================================================================
// WaterMarker (from core/watermarker.py)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterMark {
    pub mark_type: String,
    pub content: String,
    pub time: String,
}

pub struct WaterMarker {
    marks: Mutex<Vec<WaterMark>>,
}

impl Default for WaterMarker {
    fn default() -> Self {
        Self::new()
    }
}

impl WaterMarker {
    pub fn new() -> Self {
        Self {
            marks: Mutex::new(Vec::new()),
        }
    }

    pub fn add_mark(&self, mark_type: &str, content: &str) {
        let mut marks = self.marks.lock().unwrap();
        marks.push(WaterMark {
            mark_type: mark_type.to_string(),
            content: content.to_string(),
            time: Local::now().to_rfc3339(),
        });
    }

    pub fn get_marks(&self) -> Vec<WaterMark> {
        self.marks.lock().unwrap().clone()
    }

    pub fn get_marks_by_type(&self, mark_type: &str) -> Vec<WaterMark> {
        self.marks
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.mark_type == mark_type)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.marks.lock().unwrap().len()
    }
}

// ============================================================================
// Workflow (from core/workflow.py)
// ============================================================================

type StepFn = Box<dyn Fn() -> Result<String, String> + Send>;

pub struct Workflow {
    name: String,
    steps: Vec<(String, StepFn)>,
}

impl Workflow {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            steps: Vec::new(),
        }
    }

    pub fn add_step<F>(&mut self, description: &str, func: F)
    where
        F: Fn() -> Result<String, String> + Send + 'static,
    {
        self.steps.push((description.to_string(), Box::new(func)));
    }

    pub fn run(&self) -> Vec<HashMap<String, String>> {
        let mut results = Vec::new();
        for (desc, func) in &self.steps {
            println!("Running: {desc}");
            match func() {
                Ok(msg) => {
                    results.push(HashMap::from([
                        ("step".to_string(), desc.clone()),
                        ("status".to_string(), "ok".to_string()),
                        ("message".to_string(), msg),
                    ]));
                }
                Err(e) => {
                    results.push(HashMap::from([
                        ("step".to_string(), desc.clone()),
                        ("status".to_string(), "error".to_string()),
                        ("message".to_string(), e),
                    ]));
                    break;
                }
            }
        }
        results
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

// ============================================================================
// SearchOptimizer (from core/search_optimizer.py)
// ============================================================================

pub struct SearchOptimizer {
    cache: Mutex<HashMap<String, Vec<String>>>,
}

impl Default for SearchOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchOptimizer {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expansions = vec![query.to_string()];
        let lower = query.to_lowercase();
        if lower.contains("llm") || lower.contains("language model") {
            expansions.push(format!("{query} transformer"));
            expansions.push(format!("{query} attention mechanism"));
        }
        if lower.contains("rl") || lower.contains("reinforcement") {
            expansions.push(format!("{query} policy gradient"));
            expansions.push(format!("{query} reward model"));
        }
        if lower.contains("rag") || lower.contains("retrieval") {
            expansions.push(format!("{query} retrieval augmented"));
        }
        expansions
    }

    pub fn score_relevance(&self, query: &str, text: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let text_lower = text.to_lowercase();
        if query_words.is_empty() {
            return 0.0;
        }
        let matches = query_words
            .iter()
            .filter(|w| text_lower.contains(*w))
            .count();
        matches as f64 / query_words.len() as f64
    }

    pub fn rank_results(&self, query: &str, results: &[HashMap<String, String>]) -> Vec<HashMap<String, String>> {
        let mut scored: Vec<(f64, HashMap<String, String>)> = results
            .iter()
            .map(|r| {
                let text = r.get("text").map(|s| s.as_str()).unwrap_or("");
                let score = self.score_relevance(query, text);
                (score, r.clone())
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(_, r)| r).collect()
    }

    pub fn cache_result(&self, query: &str, results: Vec<String>) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(query.to_string(), results);
    }

    pub fn get_cached(&self, query: &str) -> Option<Vec<String>> {
        self.cache.lock().unwrap().get(query).cloned()
    }
}

// ============================================================================
// PerformanceGuarantee (from core/performance_guarantee.py)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage: f32,
    pub memory_mb: f64,
    pub memory_percent: f32,
    pub disk_used_gb: f64,
    pub disk_percent: f32,
    pub is_idle: bool,
}

pub struct PerformanceGuard {
    max_cpu_threshold: f32,
    max_memory_percent: f32,
    sys: Mutex<System>,
}

impl Default for PerformanceGuard {
    fn default() -> Self {
        Self::new(50.0, 80.0)
    }
}

impl PerformanceGuard {
    pub fn new(max_cpu_threshold: f32, max_memory_percent: f32) -> Self {
        Self {
            max_cpu_threshold,
            max_memory_percent,
            sys: Mutex::new(System::new()),
        }
    }

    pub fn check_performance(&self) -> PerformanceMetrics {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_memory();
        sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());

        let cpu = sys.global_cpu_usage();
        let mem_total = sys.total_memory() as f64;
        let mem_used = sys.used_memory() as f64;
        let mem_pct = if mem_total > 0.0 {
            (mem_used / mem_total * 100.0) as f32
        } else {
            0.0
        };

        PerformanceMetrics {
            cpu_usage: cpu,
            memory_mb: mem_used / 1_048_576.0,
            memory_percent: mem_pct,
            disk_used_gb: 0.0,
            disk_percent: 0.0,
            is_idle: cpu < self.max_cpu_threshold && mem_pct < self.max_memory_percent,
        }
    }

    pub fn can_run_background_task(&self) -> bool {
        let metrics = self.check_performance();
        metrics.is_idle
    }

    pub fn wait_until_idle(&self, max_wait_secs: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < max_wait_secs {
            if self.can_run_background_task() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        false
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watermarker() {
        let wm = WaterMarker::new();
        wm.add_mark("test", "hello");
        assert_eq!(wm.count(), 1);
        let marks = wm.get_marks_by_type("test");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].content, "hello");
    }

    #[test]
    fn test_workflow() {
        let mut wf = Workflow::new("test");
        wf.add_step("step1", || Ok("done".to_string()));
        wf.add_step("step2", || Ok("done".to_string()));
        assert_eq!(wf.step_count(), 2);
        let results = wf.run();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_optimizer_expand() {
        let opt = SearchOptimizer::new();
        let expansions = opt.expand_query("llm");
        assert!(expansions.len() >= 1);
    }

    #[test]
    fn test_search_optimizer_relevance() {
        let opt = SearchOptimizer::new();
        let score = opt.score_relevance("llm transformer", "This paper uses a transformer based LLM");
        assert!(score > 0.0);
    }

    #[test]
    fn test_search_optimizer_cache() {
        let opt = SearchOptimizer::new();
        opt.cache_result("test", vec!["a".to_string(), "b".to_string()]);
        assert_eq!(opt.get_cached("test").unwrap().len(), 2);
    }

    #[test]
    fn test_performance_guard() {
        let guard = PerformanceGuard::new(100.0, 100.0);
        let metrics = guard.check_performance();
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_percent >= 0.0);
    }

    #[test]
    fn test_backup_manager() {
        let dir = tempfile::tempdir().unwrap();
        let bm = BackupManager::new(Some(dir.path().join("backups")));
        let source = dir.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("test.txt"), b"data").unwrap();
        let result = bm.create_backup(source.to_str().unwrap(), "test backup");
        assert!(result.is_ok());
        let list = bm.list_backups();
        assert_eq!(list.len(), 1);
    }
}
