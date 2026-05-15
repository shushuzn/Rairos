//! Rairos Replication — Experiment Replication Checker
//!
//! Detects code links (GitHub/GitLab/HuggingFace) in papers and assesses reproducibility.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLink {
    pub url: String,
    pub platform: String,
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub confidence: f64,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub package_manager: String,
    pub files: Vec<String>,
    pub python_version: String,
    pub hardware: Vec<String>,
    pub disk_space_gb: usize,
    pub ram_gb: usize,
    pub special_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationReport {
    pub paper_id: String,
    pub paper_title: String,
    pub links: Vec<CodeLink>,
    pub primary_link: Option<CodeLink>,
    pub dependency_info: Option<DependencyInfo>,
    pub difficulty: String,
    pub difficulty_score: f64,
    pub notes: Vec<String>,
    pub reproducibility_issues: Vec<String>,
    pub smoke_test_passed: bool,
    pub smoke_test_output: String,
}

const DEPENDENCY_FILES: &[&str] = &[
    "requirements.txt",
    "setup.py",
    "setup.cfg",
    "pyproject.toml",
    "environment.yml",
    "conda.yml",
    "Dockerfile",
    "docker-compose.yml",
    "Makefile",
    "package.json",
    "Cargo.toml",
    "go.mod",
];

const SPECIAL_LIBS: &[(&str, &str)] = &[
    ("torch", "PyTorch (GPU required)"),
    ("tensorflow", "TensorFlow (GPU recommended)"),
    ("jax", "JAX (TPU/JAX compatible)"),
    ("cuda", "NVIDIA CUDA required"),
    ("cudnn", "cuDNN required"),
    ("apex", "NVIDIA Apex (mixed precision)"),
    ("transformers", "HuggingFace Transformers"),
    ("detectron2", "Detectron2"),
    ("tensorboard", "TensorBoard"),
    ("wandb", "Weights & Biases"),
    ("hydra", "Hydra config"),
    ("accelerate", "HuggingFace Accelerate"),
];

const CONTEXT_KEYWORDS_GITHUB: &[&str] = &[
    "code",
    "implementation",
    "repository",
    "repo",
    "released",
    "open source",
    "github.com",
    "our code",
    "available at",
    "https://",
];

const CONTEXT_KEYWORDS_GITLAB: &[&str] = &["gitlab.com", "repository"];

pub struct ReplicationChecker {
    re_github: Vec<Regex>,
    re_gitlab: Vec<Regex>,
    re_hf: Vec<Regex>,
    re_clean_markdown: Regex,
    re_citation_ref: Regex,
    re_py_version: Regex,
    re_disk_space: Regex,
    re_ram: Regex,
}

impl Default for ReplicationChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationChecker {
    pub fn new() -> Self {
        Self {
            re_github: vec![
                Regex::new(r"https?://github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?")
                    .unwrap(),
                Regex::new(r"github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").unwrap(),
                Regex::new(r"([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)\.git").unwrap(),
            ],
            re_gitlab: vec![Regex::new(
                r"https?://gitlab\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?",
            )
            .unwrap()],
            re_hf: vec![
                Regex::new(r"https?://huggingface\.co/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)")
                    .unwrap(),
                Regex::new(r"huggingface\.co/spaces/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").unwrap(),
            ],
            re_clean_markdown: Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)").unwrap(),
            re_citation_ref: Regex::new(r"\[(\d+)\]").unwrap(),
            re_py_version: Regex::new(r"python\s*3?\.\d+").unwrap(),
            re_disk_space: Regex::new(r"(\d+)\s*(GB|TB|MB)").unwrap(),
            re_ram: Regex::new(r"(\d+)\s*GB\s+(RAM|memory)").unwrap(),
        }
    }

    pub fn check_paper(
        &self,
        paper_id: &str,
        title: &str,
        abstract_text: &str,
        full_text: &str,
    ) -> ReplicationReport {
        let text = format!("{} {} {}", title, abstract_text, full_text);
        let mut report = ReplicationReport {
            paper_id: paper_id.to_string(),
            paper_title: title.to_string(),
            links: Vec::new(),
            primary_link: None,
            dependency_info: None,
            difficulty: String::new(),
            difficulty_score: 0.0,
            notes: Vec::new(),
            reproducibility_issues: Vec::new(),
            smoke_test_passed: false,
            smoke_test_output: String::new(),
        };

        let links = self.extract_links(&text);
        report.links = links.clone();

        if links.is_empty() {
            report.difficulty = "No Code Found".to_string();
            report.difficulty_score = 10.0;
            report
                .notes
                .push("No GitHub/GitLab/HuggingFace links detected in paper text.".to_string());
            return report;
        }

        let mut sorted_links = links;
        sorted_links.sort_by(|a, b| {
            let score_a = (a.confidence, a.context.len());
            let score_b = (b.confidence, b.context.len());
            match score_a.partial_cmp(&score_b) {
                Some(std::cmp::Ordering::Equal) => std::cmp::Ordering::Equal,
                Some(o) => o.reverse(),
                None => std::cmp::Ordering::Equal,
            }
        });
        report.primary_link = sorted_links.into_iter().next();

        let platform = report
            .primary_link
            .as_ref()
            .map(|l| l.platform.as_str())
            .unwrap_or("");

        let dep_info = self.detect_dependency_info(&text, platform);
        report.dependency_info = Some(dep_info.clone());

        let (difficulty, score) = self.assess_difficulty(&dep_info, platform, &report.links);
        report.difficulty = difficulty;
        report.difficulty_score = score;

        report.notes = self.generate_notes(
            report.primary_link.as_ref().unwrap(),
            &dep_info,
            &report.links,
        );
        report.reproducibility_issues = self.check_issues(&dep_info, &report.links);

        report
    }

    fn extract_links(&self, text: &str) -> Vec<CodeLink> {
        let mut found: Vec<CodeLink> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        let clean = self.re_clean_markdown.replace_all(text, "$2");

        for pattern in &self.re_github {
            for m in pattern.captures_iter(&clean) {
                let owner = m.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo_full = m.get(2).map(|g| g.as_str()).unwrap_or("");
                let repo = repo_full.replace(".git", "");

                let url = if m
                    .get(0)
                    .map(|g| g.as_str().starts_with("http"))
                    .unwrap_or(false)
                {
                    m.get(0).map(|g| g.as_str()).unwrap_or("").to_string()
                } else {
                    format!("https://github.com/{}/{}", owner, repo)
                };

                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let full_match = m.get(0);
                let start = full_match
                    .map(|x| x.start())
                    .unwrap_or(0)
                    .saturating_sub(50);
                let end = (full_match.map(|x| x.end()).unwrap_or(0) + 50).min(clean.len());
                let ctx = &clean[start..end];

                let mut confidence = 0.5;
                for kw in CONTEXT_KEYWORDS_GITHUB {
                    if ctx.to_lowercase().contains(&kw.to_lowercase()) {
                        confidence = 1.0;
                        break;
                    }
                }
                if self.re_citation_ref.is_match(ctx) {
                    confidence *= 0.5;
                }

                found.push(CodeLink {
                    url,
                    platform: "github".to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    path: String::new(),
                    confidence,
                    context: ctx.to_string(),
                });
            }
        }

        for pattern in &self.re_gitlab {
            for m in pattern.captures_iter(&clean) {
                let owner = m.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo = m.get(2).map(|g| g.as_str()).unwrap_or("");

                let url = if m
                    .get(0)
                    .map(|g| g.as_str().starts_with("http"))
                    .unwrap_or(false)
                {
                    m.get(0).map(|g| g.as_str()).unwrap_or("").to_string()
                } else {
                    format!("https://gitlab.com/{}/{}", owner, repo)
                };

                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let full_match = m.get(0);
                let start = full_match
                    .map(|x| x.start())
                    .unwrap_or(0)
                    .saturating_sub(50);
                let end = (full_match.map(|x| x.end()).unwrap_or(0) + 50).min(clean.len());
                let ctx = &clean[start..end];

                let confidence = if CONTEXT_KEYWORDS_GITLAB
                    .iter()
                    .any(|kw| ctx.to_lowercase().contains(&kw.to_lowercase()))
                {
                    0.8
                } else {
                    0.5
                };

                found.push(CodeLink {
                    url,
                    platform: "gitlab".to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    path: String::new(),
                    confidence,
                    context: ctx.to_string(),
                });
            }
        }

        for pattern in &self.re_hf {
            for m in pattern.captures_iter(&clean) {
                let url = m.get(0).map(|g| g.as_str()).unwrap_or("").to_string();
                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let owner = m.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo = m.get(2).map(|g| g.as_str()).unwrap_or("");

                let full_match = m.get(0);
                let start = full_match
                    .map(|x| x.start())
                    .unwrap_or(0)
                    .saturating_sub(50);
                let end = (full_match.map(|x| x.end()).unwrap_or(0) + 50).min(clean.len());
                let ctx = &clean[start..end];

                let confidence = if ctx.to_lowercase().contains("huggingface") || ctx.contains('🤗')
                {
                    0.9
                } else {
                    0.6
                };

                found.push(CodeLink {
                    url,
                    platform: "huggingface".to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    path: String::new(),
                    confidence,
                    context: ctx.to_string(),
                });
            }
        }

        found
    }

    fn detect_dependency_info(&self, text: &str, _platform: &str) -> DependencyInfo {
        let mut info = DependencyInfo {
            package_manager: "unknown".to_string(),
            files: Vec::new(),
            python_version: String::new(),
            hardware: Vec::new(),
            disk_space_gb: 0,
            ram_gb: 0,
            special_requirements: Vec::new(),
        };

        let text_lower = text.to_lowercase();

        if text_lower.contains("requirements.txt") {
            info.package_manager = "pip".to_string();
        }
        if text_lower.contains("pyproject.toml") || text_lower.contains("poetry") {
            info.package_manager = "poetry".to_string();
        }
        if text_lower.contains("conda") || text_lower.contains("environment.yml") {
            info.package_manager = "conda".to_string();
        }
        if text_lower.contains("package.json") {
            info.package_manager = "npm".to_string();
        }
        if text_lower.contains("cargo") || text_lower.contains("cargo.toml") {
            info.package_manager = "cargo".to_string();
        }

        for f in DEPENDENCY_FILES {
            if text_lower.contains(&f.to_lowercase()) {
                info.files.push((*f).to_string());
            }
        }

        if let Some(m) = self.re_py_version.find(&text_lower) {
            info.python_version = m.as_str().to_string();
        }

        let hw_keywords: [(&str, &str); 8] = [
            ("gpu", "GPU (NVIDIA recommended)"),
            ("cuda", "NVIDIA CUDA"),
            ("tpu", "TPU"),
            ("v100", "NVIDIA V100 GPU"),
            ("a100", "NVIDIA A100 GPU"),
            ("3090", "NVIDIA RTX 3090"),
            ("ram", "Large RAM"),
            ("memory", "High memory"),
        ];
        for (kw, desc) in hw_keywords {
            if text_lower.contains(kw) && !info.hardware.contains(&desc.to_string()) {
                info.hardware.push(desc.to_string());
            }
        }

        for (lib, desc) in SPECIAL_LIBS {
            if text_lower.contains(lib) && !info.special_requirements.contains(&desc.to_string()) {
                info.special_requirements.push(desc.to_string());
            }
        }

        if let Some(m) = self.re_disk_space.captures(text) {
            if let (Some(val_str), Some(unit)) = (m.get(1), m.get(2)) {
                if let Ok(val) = val_str.as_str().parse::<usize>() {
                    info.disk_space_gb = match unit.as_str().to_uppercase().as_str() {
                        "TB" => val * 1024,
                        "GB" => val,
                        "MB" => val / 1024,
                        _ => val,
                    };
                }
            }
        }

        if let Some(m) = self.re_ram.captures(text) {
            if let Some(val_str) = m.get(1) {
                if let Ok(val) = val_str.as_str().parse::<usize>() {
                    info.ram_gb = val;
                }
            }
        }

        info
    }

    fn assess_difficulty(
        &self,
        dep_info: &DependencyInfo,
        platform: &str,
        _links: &[CodeLink],
    ) -> (String, f64) {
        let mut score = 0.0;

        match platform {
            "github" => score += 0.0,
            "huggingface" => score += 0.0,
            "gitlab" => score += 1.0,
            _ => score += 0.0,
        }

        let pm_scores: std::collections::HashMap<&str, f64> = [
            ("pip", 1.0),
            ("conda", 2.0),
            ("poetry", 2.0),
            ("npm", 2.0),
            ("cargo", 3.0),
        ]
        .iter()
        .cloned()
        .collect();
        score += pm_scores
            .get(dep_info.package_manager.as_str())
            .unwrap_or(&1.0);

        if dep_info.files.is_empty() {
            score += 2.0;
        } else if !dep_info
            .files
            .iter()
            .any(|f| f == "requirements.txt" || f == "pyproject.toml")
        {
            score += 1.0;
        }

        let hw_penalty: std::collections::HashMap<&str, f64> = [
            ("GPU (NVIDIA recommended)", 0.5),
            ("NVIDIA CUDA", 1.0),
            ("TPU", 2.0),
        ]
        .iter()
        .cloned()
        .collect();
        for hw in &dep_info.hardware {
            score += hw_penalty.get(hw.as_str()).unwrap_or(&0.5);
        }

        score += dep_info.special_requirements.len() as f64 * 0.3;

        if dep_info.disk_space_gb > 500 {
            score += 1.5;
        } else if dep_info.disk_space_gb > 100 {
            score += 0.5;
        }
        if dep_info.ram_gb > 64 {
            score += 1.5;
        } else if dep_info.ram_gb > 32 {
            score += 0.5;
        }

        score = score.min(10.0);

        let difficulty = if score <= 2.0 {
            "Easy"
        } else if score <= 4.0 {
            "Medium"
        } else if score <= 6.0 {
            "Hard"
        } else if score <= 8.0 {
            "Very Hard"
        } else {
            "Extremely Hard"
        };

        (difficulty.to_string(), (score * 10.0).round() / 10.0)
    }

    fn generate_notes(
        &self,
        primary_link: &CodeLink,
        dep_info: &DependencyInfo,
        all_links: &[CodeLink],
    ) -> Vec<String> {
        let mut notes = Vec::new();
        let platform = &primary_link.platform;

        match platform.as_str() {
            "github" => notes.push(format!(
                "GitHub repo: {}/{}",
                primary_link.owner, primary_link.repo
            )),
            "huggingface" => notes.push(format!(
                "HuggingFace space/model: {}/{}",
                primary_link.owner, primary_link.repo
            )),
            "gitlab" => notes.push(format!(
                "GitLab repo: {}/{}",
                primary_link.owner, primary_link.repo
            )),
            _ => {}
        }

        if all_links.len() > 1 {
            notes.push(format!(
                "Found {} code links total — verify the correct one is used.",
                all_links.len()
            ));
        }

        if dep_info.package_manager != "unknown" {
            notes.push(format!(
                "Package manager: {}",
                dep_info.package_manager.to_uppercase()
            ));
        }

        if !dep_info.files.is_empty() {
            notes.push(format!("Dependency files: {}", dep_info.files.join(", ")));
        }

        if !dep_info.hardware.is_empty() {
            notes.push(format!("Hardware needs: {}", dep_info.hardware.join(", ")));
        }

        if !dep_info.special_requirements.is_empty() {
            let top_libs: Vec<&str> = dep_info
                .special_requirements
                .iter()
                .take(3)
                .map(|s| s.as_str())
                .collect();
            notes.push(format!("Key libraries: {}", top_libs.join(", ")));
        }

        if !dep_info.python_version.is_empty() {
            notes.push(format!("Python version hint: {}", dep_info.python_version));
        }

        notes
    }

    fn check_issues(&self, dep_info: &DependencyInfo, _links: &[CodeLink]) -> Vec<String> {
        let mut issues = Vec::new();

        if dep_info.files.is_empty() {
            issues.push(
                "No explicit dependency files detected — manual environment setup may be required."
                    .to_string(),
            );
        }

        if dep_info.python_version.is_empty() {
            issues.push("No Python version specified — possible version conflicts.".to_string());
        }

        if dep_info.files.iter().any(|f| f == "requirements.txt") {
            issues.push("requirements.txt may have unpinned versions — recommend pip-compile or poetry lock.".to_string());
        }

        for lib in &dep_info.special_requirements {
            if lib.contains("CUDA") || lib.contains("TPU") {
                issues.push(format!("{} required — hardware access needed.", lib));
            }
        }

        if dep_info.hardware.is_empty() {
            issues.push("No hardware requirements mentioned — unclear if GPU needed.".to_string());
        }

        issues
    }

    pub fn render_report(&self, report: &ReplicationReport) -> String {
        let emoji = std::collections::HashMap::from([
            ("Easy", "🟢"),
            ("Medium", "🟡"),
            ("Hard", "🟠"),
            ("Very Hard", "🔴"),
            ("Extremely Hard", "💀"),
            ("No Code Found", "❌"),
        ]);

        let e = emoji.get(report.difficulty.as_str()).unwrap_or(&"⚪");

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push(format!(
            "🔬 Replication Report: {}",
            &report.paper_id[..report.paper_id.len().min(8)]
        ));
        lines.push("=".repeat(60));
        lines.push(format!(
            "Difficulty: {} {} ({}/10)",
            e, report.difficulty, report.difficulty_score
        ));
        lines.push(String::new());

        if let Some(link) = &report.primary_link {
            lines.push(format!("Primary Link: {}", link.url));
        }

        lines.push(format!("Code links found: {}", report.links.len()));
        if report.links.len() > 1 {
            for link in report.links.iter().take(3) {
                lines.push(format!(
                    "  - {} (confidence: {}%)",
                    link.url,
                    (link.confidence * 100.0).round() as i32
                ));
            }
        }

        if let Some(di) = &report.dependency_info {
            lines.push(String::new());
            lines.push("Dependencies:".to_string());
            lines.push(format!("  Package manager: {}", di.package_manager));
            if !di.files.is_empty() {
                lines.push(format!("  Files: {}", di.files.join(", ")));
            }
            if !di.python_version.is_empty() {
                lines.push(format!("  Python: {}", di.python_version));
            }
            if !di.hardware.is_empty() {
                lines.push(format!("  Hardware: {}", di.hardware.join(", ")));
            }
            if !di.special_requirements.is_empty() {
                let libs: Vec<&str> = di
                    .special_requirements
                    .iter()
                    .take(3)
                    .map(|s| s.as_str())
                    .collect();
                lines.push(format!("  Key libs: {}", libs.join(", ")));
            }
        }

        if !report.reproducibility_issues.is_empty() {
            lines.push(String::new());
            lines.push("⚠️  Issues:".to_string());
            for issue in &report.reproducibility_issues {
                lines.push(format!("  - {}", issue));
            }
        }

        if !report.notes.is_empty() {
            lines.push(String::new());
            lines.push("Notes:".to_string());
            for note in &report.notes {
                lines.push(format!("  • {}", note));
            }
        }

        if report.smoke_test_passed {
            lines.push(String::new());
            lines.push("✅ Smoke test passed".to_string());
        }

        lines.push("=".repeat(60));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_github_link() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "test123",
            "Attention Is All You Need",
            "We propose a new architecture based on transformer.",
            "Code available at https://github.com/tensorflow/tensor2tensor",
        );
        assert!(!report.links.is_empty());
        assert_eq!(report.links[0].platform, "github");
    }

    #[test]
    fn test_no_code_found() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "test456",
            "A Paper Without Code",
            "We prove something theoretically.",
            "This paper has no code links.",
        );
        assert_eq!(report.difficulty, "No Code Found");
        assert_eq!(report.difficulty_score, 10.0);
    }

    #[test]
    fn test_huggingface_detection() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "hf123",
            "HuggingFace Model Card",
            "We release a model on HuggingFace.",
            "Model available at https://huggingface.co/openai/whisper-large",
        );
        assert!(!report.links.is_empty());
        assert_eq!(report.links[0].platform, "huggingface");
    }

    #[test]
    fn test_difficulty_assessment() {
        let checker = ReplicationChecker::new();
        let text = "Our method uses PyTorch with CUDA on NVIDIA A100 GPUs. ".repeat(10);
        let report = checker.check_paper("diff_test", "Test", "", &text);
        assert!(report.difficulty_score > 3.0);
    }

    #[test]
    fn test_render_report() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "paper1",
            "Test Paper",
            "Abstract text.",
            "Code at https://github.com/test/repo",
        );
        let rendered = checker.render_report(&report);
        assert!(rendered.contains("Replication Report"));
    }
}
