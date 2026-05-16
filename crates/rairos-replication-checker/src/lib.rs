//! rairos-replication-checker — Experiment Replication Checker for AI Research OS.

#![allow(clippy::regex_creation_in_loops)]
//!
//! Ported from `llm/replication_checker.py` (568 LOC, pure stdlib).
//!
//! Given a paper, extracts GitHub/GitLab/HuggingFace links, detects dependency
//! info (Python version, hardware, packages), and assesses replication difficulty.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeLink {
    pub url: String,
    pub platform: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub context: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyInfo {
    #[serde(default)]
    pub package_manager: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub python_version: String,
    #[serde(default)]
    pub hardware: Vec<String>,
    #[serde(default)]
    pub disk_space_gb: usize,
    #[serde(default)]
    pub ram_gb: usize,
    #[serde(default)]
    pub special_requirements: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationReport {
    pub paper_id: String,
    pub paper_title: String,
    #[serde(default)]
    pub links: Vec<CodeLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_link: Option<CodeLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_info: Option<DependencyInfo>,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub difficulty_score: f64,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub reproducibility_issues: Vec<String>,
    #[serde(default)]
    pub smoke_test_passed: bool,
    #[serde(default)]
    pub smoke_test_output: String,
}

// ─── Regex Helpers ────────────────────────────────────────────────────────────

fn github_regex() -> Vec<(regex::Regex, bool)> {
    vec![
        (regex::Regex::new(r"https?://github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?").unwrap(), true),
        (regex::Regex::new(r"github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").unwrap(), true),
        (regex::Regex::new(r"([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)\.git").unwrap(), false),
    ]
}

fn gitlab_regex() -> Vec<regex::Regex> {
    vec![
        regex::Regex::new(r"https?://gitlab\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?").unwrap(),
    ]
}

fn hf_regex() -> Vec<regex::Regex> {
    vec![
        regex::Regex::new(r"https?://huggingface\.co/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").unwrap(),
        regex::Regex::new(r"huggingface\.co/spaces/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").unwrap(),
    ]
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
];

const CONTEXT_KEYWORDS_GITLAB: &[&str] = &["gitlab.com", "repository"];

fn special_libs() -> HashMap<&'static str, &'static str> {
    HashMap::from([
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
    ])
}

// ─── Core Logic ─────────────────────────────────────────────────────────────

pub struct ReplicationChecker;

impl ReplicationChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn check_paper(
        &self,
        paper_id: &str,
        title: &str,
        abstract_text: &str,
        full_text: &str,
    ) -> ReplicationReport {
        let mut report = ReplicationReport {
            paper_id: paper_id.to_string(),
            paper_title: title.to_string(),
            ..Default::default()
        };

        let text = format!("{title} {abstract_text} {full_text}");

        // Extract code links
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

        // Pick primary link (highest confidence + context match)
        let mut sorted_links = links.clone();
        sorted_links.sort_by(|a, b| {
            let score_a = (a.confidence * 1000.0) as i64 - (a.context.len() as i64);
            let score_b = (b.confidence * 1000.0) as i64 - (b.context.len() as i64);
            score_b.cmp(&score_a)
        });
        report.primary_link = sorted_links.into_iter().next();

        let platform = report
            .primary_link
            .as_ref()
            .map(|l| l.platform.clone())
            .unwrap_or_default();

        // Detect dependency info
        let dep_info = self.detect_dependency_info(&text, &platform);
        report.dependency_info = Some(dep_info.clone());

        // Assess difficulty
        let (difficulty, score) = self.assess_difficulty(&dep_info, &platform, &report.links);
        report.difficulty = difficulty;
        report.difficulty_score = score;

        // Generate notes
        if let Some(ref link) = report.primary_link {
            report.notes = self.generate_notes(link, &dep_info, &report.links);
        }

        report.reproducibility_issues = self.check_issues(&dep_info, &report.links);

        report
    }

    pub fn extract_links(&self, text: &str) -> Vec<CodeLink> {
        let mut found: Vec<CodeLink> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Remove markdown URLs
        let clean = regex::Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)")
            .unwrap()
            .replace_all(text, "$2")
            .to_string();

        // GitHub
        for (pattern, _needs_https) in github_regex() {
            for caps in pattern.captures_iter(&clean) {
                let m = caps.get(0).unwrap();
                let owner = caps.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo = caps
                    .get(2)
                    .map(|g| g.as_str().trim_end_matches(".git"))
                    .unwrap_or("");
                let url = format!("https://github.com/{}/{}", owner, repo);

                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let start = m.start().saturating_sub(50);
                let end = (m.end() + 50).min(clean.len());
                let ctx = &clean[start..end];

                let confidence = if CONTEXT_KEYWORDS_GITHUB
                    .iter()
                    .any(|kw| ctx.to_lowercase().contains(&kw.to_lowercase()))
                {
                    1.0
                } else {
                    0.5
                };

                let confidence =
                    if regex::Regex::new(r"\[(\d+)\]")
                        .unwrap()
                        .find(ctx)
                        .is_some()
                    {
                        confidence * 0.5
                    } else {
                        confidence
                    };

                found.push(CodeLink {
                    url,
                    platform: "github".to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    confidence,
                    context: ctx.to_string(),
                    ..Default::default()
                });
            }
        }

        // GitLab
        for pattern in gitlab_regex() {
            for caps in pattern.captures_iter(&clean) {
                let m = caps.get(0).unwrap();
                let owner = caps.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo = caps.get(2).map(|g| g.as_str()).unwrap_or("");
                let url = format!("https://gitlab.com/{}/{}", owner, repo);

                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let start = m.start().saturating_sub(50);
                let end = (m.end() + 50).min(clean.len());
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
                    confidence,
                    context: ctx.to_string(),
                    ..Default::default()
                });
            }
        }

        // HuggingFace
        for pattern in hf_regex() {
            for caps in pattern.captures_iter(&clean) {
                let m = caps.get(0).unwrap();
                let owner = caps.get(1).map(|g| g.as_str()).unwrap_or("");
                let repo = caps.get(2).map(|g| g.as_str()).unwrap_or("");
                let url = m.as_str().to_string();

                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let start = m.start().saturating_sub(50);
                let end = (m.end() + 50).min(clean.len());
                let ctx = &clean[start..end];

                let confidence =
                    if ctx.to_lowercase().contains("huggingface") || ctx.contains("🤗") {
                        0.9
                    } else {
                        0.6
                    };

                found.push(CodeLink {
                    url,
                    platform: "huggingface".to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    confidence,
                    context: ctx.to_string(),
                    ..Default::default()
                });
            }
        }

        found
    }

    pub fn detect_dependency_info(&self, text: &str, platform: &str) -> DependencyInfo {
        let mut info = DependencyInfo {
            package_manager: "unknown".to_string(),
            ..Default::default()
        };

        let text_lower = text.to_lowercase();

        // Package manager
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

        // Dependency files
        for f in DEPENDENCY_FILES {
            if text_lower.contains(*f) {
                info.files.push((*f).to_string());
            }
        }

        // Python version
        if let Some(py_match) =
            regex::Regex::new(r"python\s*3?\.\d+").unwrap().find(&text_lower)
        {
            info.python_version = py_match.as_str().to_string();
        }

        // Hardware keywords
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

        // Special libraries
        for (lib, desc) in special_libs() {
            if text_lower.contains(lib) && !info.special_requirements.contains(&desc.to_string()) {
                info.special_requirements.push(desc.to_string());
            }
        }

        // Disk space
        if let Some(disk_match) =
            regex::Regex::new(r"(\d+)\s*(GB|TB|MB)")
                .unwrap()
                .captures(text)
        {
            if let Some(val_str) = disk_match.get(1) {
                if let Ok(val) = val_str.as_str().parse::<usize>() {
                    let unit = disk_match.get(2).map(|g| g.as_str()).unwrap_or("MB");
                    info.disk_space_gb = match unit {
                        "TB" => val * 1024,
                        "GB" => val,
                        _ => val / 1024,
                    };
                }
            }
        }

        // RAM
        if let Some(ram_match) =
            regex::Regex::new(r"(\d+)\s*GB\s+(RAM|memory)")
                .unwrap()
                .captures(text)
        {
            if let Some(ram_str) = ram_match.get(1) {
                if let Ok(ram) = ram_str.as_str().parse::<usize>() {
                    info.ram_gb = ram;
                }
            }
        }

        let _ = platform;
        info
    }

    pub fn assess_difficulty(
        &self,
        dep_info: &DependencyInfo,
        platform: &str,
        links: &[CodeLink],
    ) -> (String, f64) {
        let mut score = 0.0;

        // Platform
        match platform {
            "github" | "huggingface" => score += 0.0,
            "gitlab" => score += 1.0,
            _ => score += 1.0,
        }

        // Package manager
        let pm_scores: HashMap<&str, f64> =
            HashMap::from([("pip", 1.0), ("conda", 2.0), ("poetry", 2.0), ("npm", 2.0), ("cargo", 3.0)]);
        score += pm_scores
            .get(dep_info.package_manager.as_str())
            .unwrap_or(&1.0);

        // Missing dependency files
        if dep_info.files.is_empty() {
            score += 2.0;
        } else if !dep_info
            .files
            .iter()
            .any(|f| f == "requirements.txt" || f == "pyproject.toml")
        {
            score += 1.0;
        }

        // Hardware
        let hw_penalty: HashMap<&str, f64> = HashMap::from([
            ("GPU (NVIDIA recommended)", 0.5),
            ("NVIDIA CUDA", 1.0),
            ("TPU", 2.0),
        ]);
        for hw in &dep_info.hardware {
            score += hw_penalty.get(hw.as_str()).unwrap_or(&0.5);
        }

        // Special libraries
        score += (dep_info.special_requirements.len() as f64) * 0.3;

        // Disk/RAM
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

        let _ = links;
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

    pub fn generate_notes(
        &self,
        primary_link: &CodeLink,
        dep_info: &DependencyInfo,
        all_links: &[CodeLink],
    ) -> Vec<String> {
        let mut notes = Vec::new();

        match primary_link.platform.as_str() {
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
            notes.push(format!(
                "Dependency files: {}",
                dep_info.files.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ));
        }

        if !dep_info.special_requirements.is_empty() {
            notes.push(format!(
                "Key libraries: {}",
                dep_info
                    .special_requirements
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if !dep_info.python_version.is_empty() {
            notes.push(format!("Python version: {}", dep_info.python_version));
        }

        notes
    }

    pub fn check_issues(&self, dep_info: &DependencyInfo, links: &[CodeLink]) -> Vec<String> {
        let mut issues = Vec::new();

        if dep_info.files.is_empty() {
            issues.push(
                "No explicit dependency files detected — manual environment setup may be required."
                    .to_string(),
            );
        }

        if dep_info.python_version.is_empty() {
            issues.push("No Python version specified — compatibility cannot be verified.".to_string());
        }

        if links.is_empty() {
            issues.push("No code repository links found in paper.".to_string());
        }

        if dep_info.hardware.is_empty() {
            issues.push("No hardware requirements specified.".to_string());
        }

        issues
    }

    pub fn render_report(&self, report: &ReplicationReport) -> String {
        let emoji_map: HashMap<&str, &str> = HashMap::from([
            ("Easy", "🟢"),
            ("Medium", "🟡"),
            ("Hard", "🟠"),
            ("Very Hard", "🔴"),
            ("Extremely Hard", "💀"),
            ("No Code Found", "❌"),
        ]);
        let e = emoji_map.get(report.difficulty.as_str()).unwrap_or(&"⚪");

        let mut lines = vec![
            format!("============================================================"),
            format!(
                "🔬 Replication Report: {}",
                &report.paper_id[..report.paper_id.len().min(8)]
            ),
            format!("============================================================"),
            format!(
                "Difficulty: {} {} ({}/10)",
                e, report.difficulty, report.difficulty_score
            ),
            String::new(),
        ];

        lines.push(format!("Paper: {}", report.paper_title));
        lines.push(String::new());

        lines.push(format!("Code links found: {}", report.links.len()));
        if report.links.len() > 1 {
            for link in report.links.iter().take(3) {
                lines.push(format!(
                    "  - {} (confidence: {:.0})",
                    link.url,
                    link.confidence * 100.0
                ));
            }
        }

        if let Some(ref di) = report.dependency_info {
            lines.push(String::new());
            lines.push("Dependency Info:".to_string());
            if !di.package_manager.is_empty() && di.package_manager != "unknown" {
                lines.push(format!("  Package manager: {}", di.package_manager));
            }
            if !di.python_version.is_empty() {
                lines.push(format!("  Python: {}", di.python_version));
            }
            if !di.hardware.is_empty() {
                lines.push(format!("  Hardware: {}", di.hardware.join(", ")));
            }
            if !di.special_requirements.is_empty() {
                lines.push(format!(
                    "  Key libs: {}",
                    di.special_requirements
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        if !report.notes.is_empty() {
            lines.push(String::new());
            lines.push("Notes:".to_string());
            for note in &report.notes {
                lines.push(format!("  • {}", note));
            }
        }

        if !report.reproducibility_issues.is_empty() {
            lines.push(String::new());
            lines.push("⚠️  Issues:".to_string());
            for issue in &report.reproducibility_issues {
                lines.push(format!("  • {}", issue));
            }
        }

        lines.push(String::new());
        if report.smoke_test_passed {
            lines.push("✅ Smoke test passed".to_string());
        }

        lines.push("============================================================".to_string());
        lines.join("\n")
    }
}

impl Default for ReplicationChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_paper_with_github_link() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "2301.12345",
            "Attention Is All You Need",
            "We propose a new simple network architecture based on attention.",
            "Code available at https://github.com/tensorflow/tensor2tensor",
        );
        assert!(!report.links.is_empty());
        assert!(report.primary_link.is_some());
        assert_eq!(report.primary_link.as_ref().unwrap().platform, "github");
    }

    #[test]
    fn test_check_paper_no_links() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "2301.99999",
            "Pure Theory Paper",
            "We prove interesting theorems.",
            "All proofs are in the appendix.",
        );
        assert_eq!(report.difficulty, "No Code Found");
        assert_eq!(report.difficulty_score, 10.0);
    }

    #[test]
    fn test_extract_links_github() {
        let checker = ReplicationChecker::new();
        let links = checker.extract_links(
            "Paper at https://github.com/facebookresearch/segment-anything",
        );
        assert!(!links.is_empty());
        assert_eq!(links[0].platform, "github");
    }

    #[test]
    fn test_extract_links_multiple() {
        let checker = ReplicationChecker::new();
        let links = checker.extract_links(
            "GitHub: https://github.com/abc/repo and GitLab: https://gitlab.com/xyz/proj",
        );
        assert!(links.len() >= 2);
    }

    #[test]
    fn test_detect_dependency_info() {
        let checker = ReplicationChecker::new();
        let info = checker.detect_dependency_info(
            "requirements.txt, Python 3.9, GPU, PyTorch",
            "github",
        );
        assert_eq!(info.package_manager, "pip");
        assert!(info.python_version.contains("3"));
        assert!(!info.hardware.is_empty());
    }

    #[test]
    fn test_assess_difficulty() {
        let checker = ReplicationChecker::new();
        let info = DependencyInfo {
            package_manager: "pip".to_string(),
            files: vec!["requirements.txt".to_string()],
            python_version: "3.9".to_string(),
            hardware: vec![],
            special_requirements: vec![],
            disk_space_gb: 50,
            ram_gb: 16,
        };
        let (difficulty, score) = checker.assess_difficulty(&info, "github", &[]);
        assert!(score < 5.0);
        assert_eq!(difficulty, "Easy");
    }

    #[test]
    fn test_assess_difficulty_hard() {
        let checker = ReplicationChecker::new();
        let info = DependencyInfo {
            package_manager: "conda".to_string(),
            files: vec![],
            python_version: "".to_string(),
            hardware: vec!["NVIDIA CUDA".to_string(), "TPU".to_string()],
            special_requirements: vec!["PyTorch (GPU required)".to_string(); 5],
            disk_space_gb: 600,
            ram_gb: 128,
        };
        let (difficulty, score) = checker.assess_difficulty(&info, "gitlab", &[]);
        assert!(score >= 6.0);
        assert!(difficulty == "Very Hard" || difficulty == "Extremely Hard");
    }

    #[test]
    fn test_generate_notes() {
        let checker = ReplicationChecker::new();
        let link = CodeLink {
            url: "https://github.com/abc/repo".to_string(),
            platform: "github".to_string(),
            owner: "abc".to_string(),
            repo: "repo".to_string(),
            confidence: 1.0,
            context: "code released".to_string(),
            path: String::new(),
        };
        let dep = DependencyInfo {
            package_manager: "pip".to_string(),
            files: vec!["requirements.txt".to_string()],
            python_version: "3.9".to_string(),
            hardware: vec!["GPU (NVIDIA recommended)".to_string()],
            special_requirements: vec!["PyTorch (GPU required)".to_string()],
            disk_space_gb: 100,
            ram_gb: 32,
        };
        #[allow(clippy::cloned_ref_to_slice_refs)]
        let notes = checker.generate_notes(&link, &dep, &[link.clone()]);
        assert!(!notes.is_empty());
    }

    #[test]
    fn test_render_report() {
        let checker = ReplicationChecker::new();
        let report = checker.check_paper(
            "2301.12345",
            "Test Paper",
            "Abstract",
            "github.com/abc/repo",
        );
        let rendered = checker.render_report(&report);
        assert!(rendered.contains("Replication Report"));
        assert!(rendered.contains("Test Paper"));
    }

    #[test]
    fn test_check_issues() {
        let checker = ReplicationChecker::new();
        let dep = DependencyInfo {
            package_manager: "unknown".to_string(),
            files: vec![],
            python_version: "".to_string(),
            hardware: vec![],
            special_requirements: vec![],
            disk_space_gb: 0,
            ram_gb: 0,
        };
        let issues = checker.check_issues(&dep, &[]);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_replication_report_serialization() {
        let report = ReplicationReport {
            paper_id: "2301.12345".to_string(),
            paper_title: "Test".to_string(),
            links: vec![],
            primary_link: None,
            dependency_info: None,
            difficulty: "Easy".to_string(),
            difficulty_score: 2.0,
            notes: vec![],
            reproducibility_issues: vec![],
            smoke_test_passed: false,
            smoke_test_output: String::new(),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("2301.12345"));
    }

    #[test]
    fn test_dependency_info_serialization() {
        let info = DependencyInfo {
            package_manager: "pip".to_string(),
            files: vec!["requirements.txt".to_string()],
            python_version: "3.9".to_string(),
            hardware: vec!["GPU".to_string()],
            special_requirements: vec!["torch".to_string()],
            disk_space_gb: 100,
            ram_gb: 32,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("pip"));
    }

    #[test]
    fn test_code_link_serialization() {
        let link = CodeLink {
            url: "https://github.com/test/repo".to_string(),
            platform: "github".to_string(),
            owner: "test".to_string(),
            repo: "repo".to_string(),
            confidence: 0.95,
            context: "code released".to_string(),
            path: String::new(),
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("github"));
        assert!(json.contains("test"));
    }
}
