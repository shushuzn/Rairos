//! rairos-replication-checker — Experiment Replication Checker for AI Research OS.

//!
//! Ported from `llm/replication_checker.py` (568 LOC, pure stdlib).
//!
//! Given a paper, extracts GitHub/GitLab/HuggingFace links, detects dependency
//! info (Python version, hardware, packages), and assesses replication difficulty.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

// ─── Static Regex Patterns ─────────────────────────────────────────────────

static GITHUB_REGEX: LazyLock<Vec<(Regex, bool)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"https?://github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?").expect("valid regex"), true),
        (Regex::new(r"github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").expect("valid regex"), true),
        (Regex::new(r"([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)\.git").expect("valid regex"), false),
    ]
});

static GITLAB_REGEX: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"https?://gitlab\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?").expect("valid regex"),
    ]
});

static HF_REGEX: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"https?://huggingface\.co/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").expect("valid regex"),
        Regex::new(r"huggingface\.co/spaces/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)").expect("valid regex"),
    ]
});

static MARKDOWN_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)").expect("valid regex")
});

static CITATION_BRACKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(\d+)\]").expect("valid regex")
});

static PYTHON_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"python\s*3?\.\d+").expect("valid regex")
});

static MEMORY_SIZE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+)\s*(GB|TB|MB)").expect("valid regex")
});

static RAM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+)\s*GB\s+(RAM|memory)").expect("valid regex")
});

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
        let clean = MARKDOWN_LINK_REGEX.replace_all(text, "$2").to_string();

        // GitHub
        for (pattern, _needs_https) in GITHUB_REGEX.iter() {
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

                let ctx_lower = ctx.to_lowercase();
                let kw_github_lowers: Vec<String> = CONTEXT_KEYWORDS_GITHUB.iter().map(|kw| kw.to_lowercase()).collect();
                let confidence = if kw_github_lowers.iter().any(|kw_lower| ctx_lower.contains(kw_lower))
                {
                    1.0
                } else {
                    0.5
                };

                let confidence =
                    if CITATION_BRACKET_REGEX.find(ctx).is_some()
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
        for pattern in GITLAB_REGEX.iter() {
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

                let ctx_lower = ctx.to_lowercase();
                let kw_gitlab_lowers: Vec<String> = CONTEXT_KEYWORDS_GITLAB.iter().map(|kw| kw.to_lowercase()).collect();
                let confidence = if kw_gitlab_lowers.iter().any(|kw_lower| ctx_lower.contains(kw_lower))
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
        for pattern in HF_REGEX.iter() {
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

                let ctx_lower = ctx.to_lowercase();
                let confidence =
                    if ctx_lower.contains("huggingface") || ctx.contains("🤗") {
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
        if let Some(py_match) = PYTHON_VERSION_REGEX.find(&text_lower) {
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
        if let Some(disk_match) = MEMORY_SIZE_REGEX.captures(text) {
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
        if let Some(ram_match) = RAM_REGEX.captures(text) {
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
            "=".repeat(60),
            format!(
                "🔬 Replication Report: {}",
                &report.paper_id[..report.paper_id.len().min(8)]
            ),
            "=".repeat(60),
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

// ─── GitHub API Client ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMetadata {
    pub full_name: String,
    pub description: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub language: Option<String>,
    pub license: Option<String>,
    pub topics: Vec<String>,
    pub created_at: String,
    pub pushed_at: String,
    pub open_issues: u32,
    pub subscribers_count: u32,
}

impl GitHubClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token: Some(token),
        }
    }

    pub async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, String> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

        let mut request = self.client.get(&url)
            .header("User-Agent", "Rairos-Research-OS")
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        if response.status() == 404 {
            return Err(format!("Repository not found: {}/{}", owner, repo));
        }

        if response.status() == 403 {
            return Err("GitHub API rate limit exceeded".to_string());
        }

        if !response.status().is_success() {
            return Err(format!("GitHub API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let full_name = json["full_name"].as_str().unwrap_or("").to_string();
        let description = json["description"].as_str().map(|s| s.to_string());
        let stars = json["stargazers_count"].as_u64().unwrap_or(0) as u32;
        let forks = json["forks_count"].as_u64().unwrap_or(0) as u32;
        let language = json["language"].as_str().map(|s| s.to_string());
        let license = json["license"]["name"].as_str().map(|s| s.to_string());
        let topics = json["topics"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let created_at = json["created_at"].as_str().unwrap_or("").to_string();
        let pushed_at = json["pushed_at"].as_str().unwrap_or("").to_string();
        let open_issues = json["open_issues_count"].as_u64().unwrap_or(0) as u32;
        let subscribers_count = json["subscribers_count"].as_u64().unwrap_or(0) as u32;

        Ok(RepoMetadata {
            full_name,
            description,
            stars,
            forks,
            language,
            license,
            topics,
            created_at,
            pushed_at,
            open_issues,
            subscribers_count,
        })
    }

    pub async fn get_readme_preview(&self, owner: &str, repo: &str, max_len: usize) -> Result<String, String> {
        let url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);

        let mut request = self.client.get(&url)
            .header("User-Agent", "Rairos-Research-OS")
            .header("Accept", "application/vnd.github.v3.raw");

        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch README: {}", response.status()));
        }

        let text = response.text().await
            .map_err(|e| format!("Failed to read README: {}", e))?;

        Ok(if text.len() > max_len {
            format!("{}...[truncated]", &text[..max_len])
        } else {
            text
        })
    }
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── HuggingFace Dataset API Client ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HuggingFaceClient {
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub papers_with_code: Option<u32>,
    pub trending: bool,
}

impl HuggingFaceClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_dataset_metadata(&self, dataset_name: &str) -> Result<DatasetMetadata, String> {
        let url = format!("https://huggingface.co/api/datasets/{}", dataset_name);

        let response = self.client.get(&url)
            .header("User-Agent", "Rairos-Research-OS")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("HuggingFace API request failed: {}", e))?;

        if response.status() == 404 {
            return Err(format!("Dataset not found: {}", dataset_name));
        }

        if !response.status().is_success() {
            return Err(format!("HuggingFace API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let id = json["id"].as_str().unwrap_or(dataset_name).to_string();
        let name = json["name"].as_str().unwrap_or(&id).to_string();
        let tags = json["tags"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let downloads = json["downloads"].as_u64().unwrap_or(0);
        let papers_with_code = json["paperswithcode"]
            .as_object()
            .and_then(|obj| obj.get("count"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
        let trending = json["trending"].as_bool().unwrap_or(false);

        Ok(DatasetMetadata {
            id,
            name,
            tags,
            downloads,
            papers_with_code,
            trending,
        })
    }

    pub async fn search_datasets(&self, query: &str, limit: usize) -> Result<Vec<DatasetMetadata>, String> {
        let url = format!("https://huggingface.co/api/datasets?search={}&limit={}", query, limit);

        let response = self.client.get(&url)
            .header("User-Agent", "Rairos-Research-OS")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("HuggingFace API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HuggingFace API error: {}", response.status()));
        }

        let json: Vec<serde_json::Value> = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let datasets: Vec<DatasetMetadata> = json.iter()
            .map(|item| {
                let id = item["id"].as_str().unwrap_or("").to_string();
                let name = item["name"].as_str().unwrap_or(&id).to_string();
                let tags = item["tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let downloads = item["downloads"].as_u64().unwrap_or(0);
                let papers_with_code = item["paperswithcode"]
                    .as_object()
                    .and_then(|obj| obj.get("count"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                let trending = item["trending"].as_bool().unwrap_or(false);

                DatasetMetadata {
                    id,
                    name,
                    tags,
                    downloads,
                    papers_with_code,
                    trending,
                }
            })
            .collect();

        Ok(datasets)
    }
}

impl Default for HuggingFaceClient {
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

    #[test]
    fn test_repo_metadata_serialization() {
        let meta = RepoMetadata {
            full_name: "test/repo".to_string(),
            description: Some("A test repo".to_string()),
            stars: 100,
            forks: 50,
            language: Some("Rust".to_string()),
            license: Some("MIT".to_string()),
            topics: vec!["ai".to_string(), "ml".to_string()],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            pushed_at: "2024-06-01T00:00:00Z".to_string(),
            open_issues: 10,
            subscribers_count: 5,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("test/repo"));
        assert!(json.contains("100"));
        assert!(json.contains("Rust"));
        assert!(json.contains("MIT"));
    }

    #[test]
    fn test_dataset_metadata_serialization() {
        let meta = DatasetMetadata {
            id: "imagenet-1k".to_string(),
            name: "imagenet-1k".to_string(),
            tags: vec!["image-classification".to_string(), "computer-vision".to_string()],
            downloads: 50000,
            papers_with_code: Some(100),
            trending: false,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("imagenet-1k"));
        assert!(json.contains("50000"));
        assert!(json.contains("100"));
    }
}

// ─── Critical Thinking Checker ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StudyDesign {
    Unknown,
    RandomizedControlledTrial,
    Cohort,
    CaseControl,
    CrossSectional,
    CaseSeries,
    Review,
    MetaAnalysis,
    Experiment,
    Simulation,
    Theoretical,
}

impl StudyDesign {
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();
        if lower.contains("randomized") || lower.contains("rct") || lower.contains("randomised") {
            StudyDesign::RandomizedControlledTrial
        } else if lower.contains("cohort") || lower.contains("prospective") || lower.contains("retrospective") {
            StudyDesign::Cohort
        } else if lower.contains("case-control") || lower.contains("case control") {
            StudyDesign::CaseControl
        } else if lower.contains("cross-sectional") || lower.contains("cross sectional") {
            StudyDesign::CrossSectional
        } else if lower.contains("case series") || lower.contains("case report") {
            StudyDesign::CaseSeries
        } else if lower.contains("systematic review") || lower.contains("review") {
            StudyDesign::Review
        } else if lower.contains("meta-analysis") || lower.contains("meta analysis") {
            StudyDesign::MetaAnalysis
        } else if lower.contains("experiment") || lower.contains("experimental") {
            StudyDesign::Experiment
        } else if lower.contains("simulation") || lower.contains("in silico") {
            StudyDesign::Simulation
        } else if lower.contains("theoretical") || lower.contains("theorem") {
            StudyDesign::Theoretical
        } else {
            StudyDesign::Unknown
        }
    }

    pub fn quality_level(&self) -> u8 {
        match self {
            StudyDesign::MetaAnalysis => 5,
            StudyDesign::RandomizedControlledTrial => 4,
            StudyDesign::Cohort => 3,
            StudyDesign::CaseControl => 3,
            StudyDesign::CrossSectional => 2,
            StudyDesign::CaseSeries => 1,
            StudyDesign::Review => 2,
            StudyDesign::Experiment => 3,
            StudyDesign::Simulation => 2,
            StudyDesign::Theoretical => 1,
            StudyDesign::Unknown => 0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StudyDesign::Unknown => "Unknown",
            StudyDesign::RandomizedControlledTrial => "Randomized Controlled Trial",
            StudyDesign::Cohort => "Cohort Study",
            StudyDesign::CaseControl => "Case-Control Study",
            StudyDesign::CrossSectional => "Cross-Sectional Study",
            StudyDesign::CaseSeries => "Case Series",
            StudyDesign::Review => "Literature Review",
            StudyDesign::MetaAnalysis => "Meta-Analysis",
            StudyDesign::Experiment => "Experiment",
            StudyDesign::Simulation => "Simulation",
            StudyDesign::Theoretical => "Theoretical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiasType {
    SelectionBias,
    MeasurementBias,
    Confounding,
    PublicationBias,
    RecallBias,
    ObserverBias,
    AttritionBias,
    FundingBias,
    ConflictOfInterest,
}

impl BiasType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BiasType::SelectionBias => "Selection Bias",
            BiasType::MeasurementBias => "Measurement Bias",
            BiasType::Confounding => "Confounding",
            BiasType::PublicationBias => "Publication Bias",
            BiasType::RecallBias => "Recall Bias",
            BiasType::ObserverBias => "Observer Bias",
            BiasType::AttritionBias => "Attrition Bias",
            BiasType::FundingBias => "Funding Bias",
            BiasType::ConflictOfInterest => "Conflict of Interest",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasFlag {
    pub bias_type: String,
    pub severity: String,
    pub description: String,
    pub indicator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalConcern {
    pub concern_type: String,
    pub severity: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceQuality {
    High,
    Moderate,
    Low,
    VeryLow,
}

impl EvidenceQuality {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            EvidenceQuality::High
        } else if score >= 0.6 {
            EvidenceQuality::Moderate
        } else if score >= 0.4 {
            EvidenceQuality::Low
        } else {
            EvidenceQuality::VeryLow
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EvidenceQuality::High => "High",
            EvidenceQuality::Moderate => "Moderate",
            EvidenceQuality::Low => "Low",
            EvidenceQuality::VeryLow => "Very Low",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalThinkingReport {
    pub paper_id: String,
    pub study_design: String,
    pub design_quality_score: f64,
    pub evidence_quality: String,
    pub biases: Vec<BiasFlag>,
    pub statistical_concerns: Vec<StatisticalConcern>,
    pub logical_fallacies: Vec<String>,
    pub strengths: Vec<String>,
    pub recommendations: Vec<String>,
    pub overall_score: f64,
}

pub struct CriticalThinkingChecker;

impl CriticalThinkingChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, paper_id: &str, title: &str, abstract_text: &str) -> CriticalThinkingReport {
        let text = format!("{} {}", title, abstract_text);

        let study_design = StudyDesign::from_text(&text);
        let design_score = study_design.quality_level() as f64 / 5.0;

        let mut biases = Vec::new();
        let mut concerns = Vec::new();
        let mut fallacies = Vec::new();
        let mut strengths = Vec::new();
        let mut recommendations = Vec::new();

        self.check_selection_bias(&text, &mut biases);
        self.check_measurement_bias(&text, &mut biases);
        self.check_confounding(&text, &mut biases);
        self.check_statistical_concerns(&text, &mut concerns);
        self.check_logical_fallacies(&text, &mut fallacies);
        self.assess_strengths(&text, &mut strengths);
        self.generate_recommendations(&text, &mut recommendations);

        let bias_penalty = biases.iter()
            .filter(|b| b.severity == "High")
            .count() as f64 * 0.1;

        let concern_penalty = concerns.iter()
            .filter(|c| c.severity == "High")
            .count() as f64 * 0.05;

        let overall_score = (design_score - bias_penalty - concern_penalty).clamp(0.0, 1.0);

        let evidence_quality = EvidenceQuality::from_score(overall_score);

        CriticalThinkingReport {
            paper_id: paper_id.to_string(),
            study_design: study_design.as_str().to_string(),
            design_quality_score: design_score,
            evidence_quality: evidence_quality.as_str().to_string(),
            biases,
            statistical_concerns: concerns,
            logical_fallacies: fallacies,
            strengths,
            recommendations,
            overall_score,
        }
    }

    fn check_selection_bias(&self, text: &str, biases: &mut Vec<BiasFlag>) {
        let lower = text.to_lowercase();

        if lower.contains("convenience sample") || lower.contains("volunteer") {
            biases.push(BiasFlag {
                bias_type: BiasType::SelectionBias.as_str().to_string(),
                severity: "Medium".to_string(),
                description: "Use of convenience sampling may limit generalizability".to_string(),
                indicator: "Convenience sample or volunteer participants detected".to_string(),
            });
        }

        if lower.contains("self-select") || lower.contains("self refer") {
            biases.push(BiasFlag {
                bias_type: BiasType::SelectionBias.as_str().to_string(),
                severity: "Medium".to_string(),
                description: "Self-referral may introduce selection bias".to_string(),
                indicator: "Self-selection bias detected".to_string(),
            });
        }

        if lower.contains("online survey") && !lower.contains("random") {
            biases.push(BiasFlag {
                bias_type: BiasType::SelectionBias.as_str().to_string(),
                severity: "Low".to_string(),
                description: "Online surveys may not represent target population".to_string(),
                indicator: "Non-random online sample".to_string(),
            });
        }
    }

    fn check_measurement_bias(&self, text: &str, biases: &mut Vec<BiasFlag>) {
        let lower = text.to_lowercase();

        if lower.contains("self-report") || lower.contains("self report") {
            biases.push(BiasFlag {
                bias_type: BiasType::MeasurementBias.as_str().to_string(),
                severity: "Medium".to_string(),
                description: "Self-reported measures may be affected by recall or social desirability bias".to_string(),
                indicator: "Self-report methodology detected".to_string(),
            });
        }

        if lower.contains("unblinded") || lower.contains("non-blinded") {
            biases.push(BiasFlag {
                bias_type: BiasType::ObserverBias.as_str().to_string(),
                severity: "Medium".to_string(),
                description: "Lack of blinding may introduce observer bias".to_string(),
                indicator: "Non-blinded design".to_string(),
            });
        }
    }

    fn check_confounding(&self, text: &str, biases: &mut Vec<BiasFlag>) {
        let lower = text.to_lowercase();

        if lower.contains("observational") && !lower.contains("adjust") && !lower.contains("control") {
            biases.push(BiasFlag {
                bias_type: BiasType::Confounding.as_str().to_string(),
                severity: "High".to_string(),
                description: "Observational study without adjustment for confounders may have confounding bias".to_string(),
                indicator: "Unadjusted observational study".to_string(),
            });
        }

        if lower.contains("correlation") && lower.contains("caus") {
            biases.push(BiasFlag {
                bias_type: BiasType::Confounding.as_str().to_string(),
                severity: "High".to_string(),
                description: "Correlation does not imply causation".to_string(),
                indicator: "Causal language used with correlational data".to_string(),
            });
        }
    }

    fn check_statistical_concerns(&self, text: &str, concerns: &mut Vec<StatisticalConcern>) {
        let lower = text.to_lowercase();

        if (lower.contains("p <") || lower.contains("p-value") || lower.contains("p value"))
            && !lower.contains("correction") && !lower.contains("bonferroni") && !lower.contains("fdr") {
                concerns.push(StatisticalConcern {
                    concern_type: "Multiple Comparisons".to_string(),
                    severity: "Medium".to_string(),
                    description: "Multiple statistical tests without correction may increase false positive rate".to_string(),
                    suggestion: "Apply multiple comparison correction (Bonferroni, FDR)".to_string(),
                });
            }

        if (lower.contains("sample size") || lower.contains("n ="))
            && (lower.contains("small") || lower.contains("limited")) {
                concerns.push(StatisticalConcern {
                    concern_type: "Small Sample Size".to_string(),
                    severity: "Medium".to_string(),
                    description: "Small sample size may limit statistical power".to_string(),
                    suggestion: "Conduct power analysis and increase sample size if possible".to_string(),
                });
            }

        if lower.contains("post-hoc") || lower.contains("posthoc") || lower.contains("post hoc") {
            concerns.push(StatisticalConcern {
                concern_type: "Post-hoc Analysis".to_string(),
                severity: "Medium".to_string(),
                description: "Post-hoc analyses are exploratory and should be interpreted cautiously".to_string(),
                suggestion: "Distinguish confirmatory from exploratory analyses".to_string(),
            });
        }

        if lower.contains("effect size") || lower.contains("cohen") {
            // Effect size mentioned is a good sign
        } else if lower.contains("significant") && !lower.contains("effect size") {
            concerns.push(StatisticalConcern {
                concern_type: "Missing Effect Size".to_string(),
                severity: "Low".to_string(),
                description: "Effect sizes not reported alongside significance tests".to_string(),
                suggestion: "Report effect sizes with confidence intervals".to_string(),
            });
        }
    }

    fn check_logical_fallacies(&self, text: &str, fallacies: &mut Vec<String>) {
        let lower = text.to_lowercase();

        if (lower.contains("this proves") || lower.contains("clearly demonstrates"))
            && (lower.contains("correlation") || lower.contains("association")) {
                fallacies.push("Causation fallacy: Using causal language (proves, demonstrates) with correlational evidence".to_string());
            }

        if lower.contains("while this") && lower.contains("may") {
            // Hedging detected - not a fallacy
        } else if lower.contains("all") && lower.contains("never") {
            fallacies.push("Hasty generalization: Absolute quantifiers (all, never) in empirical claims".to_string());
        }

        if lower.contains("experts believe") || lower.contains("scientists think") {
            fallacies.push("Appeal to authority: Citing consensus without empirical evidence".to_string());
        }
    }

    fn assess_strengths(&self, text: &str, strengths: &mut Vec<String>) {
        let lower = text.to_lowercase();

        if lower.contains("randomized") || lower.contains("rct") {
            strengths.push("Randomized design helps control for confounding".to_string());
        }

        if lower.contains("blind") || lower.contains("blinded") {
            strengths.push("Blinding reduces measurement bias".to_string());
        }

        if lower.contains("control group") || lower.contains("control condition") {
            strengths.push("Presence of control group enables comparison".to_string());
        }

        if lower.contains("replication") || lower.contains("reproduced") {
            strengths.push("Replication attempt strengthens credibility".to_string());
        }

        if lower.contains("open source") || lower.contains("code available") || lower.contains("github") {
            strengths.push("Code availability enhances reproducibility".to_string());
        }

        if lower.contains("pre-regist") || lower.contains("preregist") {
            strengths.push("Preregistration reduces publication bias".to_string());
        }

        if lower.contains("effect size") || lower.contains("confidence interval") {
            strengths.push("Effect sizes and CIs reported".to_string());
        }

        if lower.contains("power analysis") || lower.contains("sample size calculation") {
            strengths.push("Sample size determined by power analysis".to_string());
        }
    }

    fn generate_recommendations(&self, text: &str, recommendations: &mut Vec<String>) {
        let lower = text.to_lowercase();

        if !lower.contains("replication") && !lower.contains("reproduced") {
            recommendations.push("Consider independent replication to validate findings".to_string());
        }

        if !lower.contains("code") && !lower.contains("github") && !lower.contains("open source") {
            recommendations.push("Make code and data available for reproducibility".to_string());
        }

        if lower.contains("observational") && !lower.contains("causal") {
            recommendations.push("Use causal inference methods if making causal claims".to_string());
        }

        if !lower.contains("limitation") {
            recommendations.push("Discuss limitations explicitly in the paper".to_string());
        }
    }
}

impl Default for CriticalThinkingChecker {
    fn default() -> Self {
        Self::new()
    }
}
