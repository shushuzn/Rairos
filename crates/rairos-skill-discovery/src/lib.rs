//! rairos-skill-discovery — Scan directories for SKILL.md files and parse frontmatter.
//!
//! Ported from `research_loop/skill_discovery.py`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use parking_lot::Mutex;

/// Cached mtime of skill dirs for hot-reload detection.
fn get_mtime_cache() -> &'static Mutex<HashMap<PathBuf, f64>> {
    static CACHE: std::sync::LazyLock<Mutex<HashMap<PathBuf, f64>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
    &CACHE
}

const SKILL_FILENAME: &str = "SKILL.md";
const SKILL_MARKER: &str = "---";

// ---------------------------------------------------------------------------
// Contractual Skills — GovernSpec (arxiv 2605.22634)
// ---------------------------------------------------------------------------

/// Input specification for a skill — what it expects before execution.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InputContract {
    /// Human-readable description of required inputs.
    pub description: Option<String>,
    /// List of required input field names.
    pub required_fields: Vec<String>,
    /// List of optional input field names.
    pub optional_fields: Vec<String>,
    /// MIME types or formats accepted (e.g. "text/plain", "application/json").
    pub accepted_formats: Vec<String>,
    /// Maximum input size in bytes (0 = unlimited).
    pub max_size_bytes: Option<u64>,
}

/// Output specification for a skill — what it guarantees after execution.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OutputContract {
    /// Human-readable description of guaranteed outputs.
    pub description: Option<String>,
    /// Expected output MIME type.
    pub output_type: Option<String>,
    /// Minimum output size in bytes (for non-empty results).
    pub min_size_bytes: Option<u64>,
    /// Fields that are always present in the output.
    pub guaranteed_fields: Vec<String>,
    /// Error outputs that the skill may produce.
    pub error_variants: Vec<String>,
}

/// Quality criteria that must be met for the skill to be considered successful.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct QualityCriteria {
    /// Maximum time to produce output (seconds).
    pub max_latency_secs: Option<f64>,
    /// Minimum accuracy or precision score (0.0–1.0).
    pub min_accuracy: Option<f64>,
    /// List of quality metrics that must be checked.
    pub required_checks: Vec<String>,
    /// Allowed failure rate as a fraction (e.g. 0.05 = 5%).
    pub max_error_rate: Option<f64>,
}

/// Step-by-step verification procedure after skill execution.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VerificationSteps {
    /// Ordered list of verification step names to execute.
    pub steps: Vec<String>,
    /// Command or script for each step (step_name → command).
    pub step_commands: HashMap<String, String>,
    /// Whether verification must pass before handoff (strict gate).
    pub strict_gate: bool,
    /// Retry count for transient verification failures.
    pub max_retries: u32,
}

/// A checkpoint where a human must explicitly approve before the skill proceeds.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ApprovalGate {
    /// Human-readable description of what is being approved.
    pub description: String,
    /// Point in the skill workflow where approval is required.
    pub trigger: String,
    /// Whether this gate is mandatory (hard) or advisory (soft).
    pub mandatory: bool,
    /// Who must approve (e.g. "user", "admin", "security-team").
    pub approver: String,
}

/// Permissions the skill requires to operate.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SkillPermissions {
    /// Filesystem paths the skill needs read access to.
    pub read_paths: Vec<String>,
    /// Filesystem paths the skill needs write access to.
    pub write_paths: Vec<String>,
    /// Environment variables the skill needs.
    pub env_vars: Vec<String>,
    /// Network hosts the skill needs to reach.
    pub network_hosts: Vec<String>,
    /// Whether the skill requires sudo/elevated privileges.
    pub requires_sudo: bool,
}

/// Evidence that the skill must produce and retain.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SkillEvidence {
    /// File paths where evidence logs must be written.
    pub log_paths: Vec<String>,
    /// JSON schema for structured evidence output.
    pub output_schema: Option<String>,
    /// Minimum retention period in days.
    pub retention_days: Option<u32>,
}

/// The full contractual specification for a skill (GovernSpec).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SkillContract {
    pub version: Option<String>,
    pub input: InputContract,
    pub output: OutputContract,
    pub quality: QualityCriteria,
    pub verification: VerificationSteps,
    pub approval_gates: Vec<ApprovalGate>,
    pub permissions: SkillPermissions,
    pub evidence: SkillEvidence,
}

impl SkillContract {
    /// Check whether all mandatory approval gates have been cleared.
    pub fn pending_approval_gates(&self) -> Vec<&ApprovalGate> {
        self.approval_gates
            .iter()
            .filter(|g| g.mandatory)
            .collect()
    }

    /// Check whether the skill's required permissions are a subset of granted permissions.
    pub fn permissions_satisfied_by(&self, granted: &SkillPermissions) -> bool {
        // Every required read path must be in granted reads
        for rp in &self.permissions.read_paths {
            if !granted.read_paths.iter().any(|g| g == "*" || rp.starts_with(g)) {
                return false;
            }
        }
        // Every required write path must be in granted writes
        for wp in &self.permissions.write_paths {
            if !granted.write_paths.iter().any(|g| g == "*" || wp.starts_with(g)) {
                return false;
            }
        }
        // Sudo requirement must match
        if self.permissions.requires_sudo && !granted.requires_sudo {
            return false;
        }
        true
    }

    /// Build a human-readable contract summary.
    pub fn summary(&self) -> String {
        let gates = self
            .pending_approval_gates()
            .iter()
            .map(|g| format!("[{}] {}", g.trigger, g.description))
            .collect::<Vec<_>>()
            .join("; ");

        format!(
            "Contract: {} inputs, {} outputs, {} quality checks, {} approval gates. Pending: {}",
            self.input.required_fields.len(),
            self.output.guaranteed_fields.len(),
            self.quality.required_checks.len(),
            self.approval_gates.len(),
            if gates.is_empty() { "none".to_string() } else { gates }
        )
    }
}

// ---------------------------------------------------------------------------
// Skill
// ---------------------------------------------------------------------------

/// A discovered skill with metadata and optional contractual specification.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub dir: PathBuf,
    /// Optional GovernSpec contractual fields (arxiv 2605.22634).
    pub contract: Option<SkillContract>,
}

impl Skill {
    /// Convert to a dictionary representation.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("name".to_string(), serde_json::json!(self.name));
        m.insert("description".to_string(), serde_json::json!(self.description));
        m.insert(
            "path".to_string(),
            serde_json::json!(self.path.to_string_lossy()),
        );
        m.insert("dir".to_string(), serde_json::json!(self.dir.to_string_lossy()));
        if let Some(ref c) = self.contract {
            m.insert(
                "contract".to_string(),
                serde_json::to_value(c).unwrap_or_default(),
            );
        }
        m
    }

    /// Return true if the skill has a fully-specified GovernSpec contract.
    pub fn has_contract(&self) -> bool {
        self.contract.is_some()
    }

    /// Return true if the skill has mandatory approval gates that need clearing.
    pub fn needs_approval(&self) -> bool {
        self.contract
            .as_ref()
            .map(|c| !c.pending_approval_gates().is_empty())
            .unwrap_or(false)
    }

    /// Validate whether required input fields are present in the given map.
    pub fn validate_inputs(&self, provided: &HashMap<String, serde_json::Value>) -> Result<(), String> {
        if let Some(ref contract) = self.contract {
            for field in &contract.input.required_fields {
                if !provided.contains_key(field) {
                    return Err(format!(
                        "Skill '{}' contract requires input field '{}' which is missing",
                        self.name, field
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Parse YAML frontmatter from SKILL.md content.
/// Handles both basic fields (name, description) and GovernSpec contract fields
/// (arxiv 2605.22634 — Contractual Skills).
fn parse_frontmatter(content: &str) -> HashMap<String, String> {
    if !content.starts_with(SKILL_MARKER) {
        return HashMap::new();
    }
    let after_first = match content.get(3..) {
        Some(s) => s,
        None => return HashMap::new(),
    };
    let end_pos = match after_first.find("---") {
        Some(pos) => pos,
        None => return HashMap::new(),
    };
    let yaml_text = &after_first[..end_pos];
    let mut result = HashMap::new();
    let mut in_multiline = false;
    let mut multiline_key = String::new();
    let mut multiline_buf = String::new();

    for line in yaml_text.lines() {
        let line_raw = line;
        let line = line_raw.trim();

        // Close multiline value
        if in_multiline {
            if line.is_empty() || line.starts_with("  #") || line == "]" || line == "\"]" {
                // Multiline ends
                result.insert(multiline_key.clone(), multiline_buf.trim().to_string());
                multiline_key.clear();
                multiline_buf.clear();
                in_multiline = false;
                if line.is_empty() || (line.starts_with("  #") && !line.trim_start_matches("  #").starts_with("-")) {
                    continue;
                }
            } else {
                // Accumulate, strip leading "- " or "* " for list items
                let stripped = line.trim_start_matches("- ").trim_start_matches("* ");
                if !multiline_buf.is_empty() {
                    multiline_buf.push('\n');
                }
                multiline_buf.push_str(stripped.trim());
                continue;
            }
        }

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Detect multiline start: line starts with "  - " or "  * " under a known list key
        let basic_keys = [
            "name", "description", "version", "trigger", "mandatory", "approver",
            "required_fields", "optional_fields", "accepted_formats",
            "output_type", "guaranteed_fields", "error_variants",
            "required_checks", "step_commands", "read_paths", "write_paths",
            "env_vars", "network_hosts", "log_paths", "output_schema",
            // GovernSpec contract keys (arxiv 2605.22634)
            "approval_gates", "input_required_fields", "input_optional_fields",
            "input_accepted_formats", "input_max_size_bytes", "input_description",
            "output_description", "output_min_size_bytes", "output_guaranteed_fields",
            "output_error_variants", "quality_max_latency_secs", "quality_min_accuracy",
            "quality_required_checks", "quality_max_error_rate",
            "verification_strict_gate", "verification_max_retries",
            "verification_steps", "verification_step_commands",
            "permission_read_paths", "permission_write_paths",
            "permission_env_vars", "permission_network_hosts", "permission_requires_sudo",
            "evidence_log_paths", "evidence_output_schema", "evidence_retention_days",
            "contract_version",
        ];
        if let Some((key, rest)) = line.split_once(':') {
            let key = key.trim();
            let rest = rest.trim();
            // Check for list start (next non-empty line is "  -")
            let rest_clean = rest.trim_end_matches(':').trim();
            if basic_keys.contains(&key) && (rest_clean.is_empty() || rest_clean == "|" || rest_clean == ">") {
                // This might start a multiline block
                if rest_clean == "|" || rest_clean == ">" {
                    // Explicit block scalar indicator: multiline content follows
                    in_multiline = true;
                    multiline_key = key.to_string();
                    multiline_buf.clear();
                    continue;
                }
            }
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if key == "name" || key == "description" || key.starts_with("contract_") {
                result.insert(key.to_string(), value.to_string());
            } else {
                // All other top-level keys go in as-is
                result.insert(key.to_string(), value.to_string());
            }
        }
    }

    // Flush any remaining multiline
    if in_multiline && !multiline_key.is_empty() {
        result.insert(multiline_key, multiline_buf.trim().to_string());
    }

    result
}

/// Parse a comma-or-newline-separated list from a YAML value string.
fn parse_list(value: &str) -> Vec<String> {
    value
        .lines()
        .flat_map(|l| l.split(','))
        .map(|s| s.trim().trim_matches('-').trim().trim_matches('*').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse the GovernSpec contract section from parsed frontmatter fields.
fn parse_contract(fm: &HashMap<String, String>) -> Option<SkillContract> {
    // Only return a contract if at least one contract field is present
    let contract_keys = [
        "input_required_fields", "input_optional_fields", "input_accepted_formats",
        "output_type", "output_guaranteed_fields", "output_error_variants",
        "quality_required_checks", "quality_max_latency_secs", "quality_min_accuracy",
        "verification_steps", "verification_strict_gate",
        "approval_gates", "permission_read_paths", "permission_write_paths",
        "evidence_log_paths",
    ];
    let has_contract = contract_keys.iter().any(|k| fm.contains_key(*k))
        || fm.keys().any(|k| k.starts_with("approval_gate_"));

    if !has_contract {
        return None;
    }

    Some(SkillContract {
        version: fm.get("contract_version").cloned(),
        input: InputContract {
            description: fm.get("input_description").cloned(),
            required_fields: fm.get("input_required_fields")
                .map(|v| parse_list(v))
                .unwrap_or_default(),
            optional_fields: fm.get("input_optional_fields")
                .map(|v| parse_list(v))
                .unwrap_or_default(),
            accepted_formats: fm.get("input_accepted_formats")
                .map(|v| parse_list(v))
                .unwrap_or_default(),
            max_size_bytes: fm.get("input_max_size_bytes")
                .and_then(|v| v.parse::<u64>().ok()),
        },
        output: OutputContract {
            description: fm.get("output_description").cloned(),
            output_type: fm.get("output_type").cloned(),
            min_size_bytes: fm.get("output_min_size_bytes")
                .and_then(|v| v.parse::<u64>().ok()),
            guaranteed_fields: fm.get("output_guaranteed_fields")
                .map(|v| parse_list(v))
                .unwrap_or_default(),
            error_variants: fm.get("output_error_variants")
                .map(|v| parse_list(v))
                .unwrap_or_default(),
        },
        quality: QualityCriteria {
            max_latency_secs: fm.get("quality_max_latency_secs")
                .and_then(|v| v.parse::<f64>().ok()),
            min_accuracy: fm.get("quality_min_accuracy")
                .and_then(|v| v.parse::<f64>().ok()),
            required_checks: fm.get("quality_required_checks").map(|v| parse_list(v)).unwrap_or_default(),
            max_error_rate: fm.get("quality_max_error_rate")
                .and_then(|v| v.parse::<f64>().ok()),
        },
        verification: VerificationSteps {
            steps: fm.get("verification_steps").map(|v| parse_list(v)).unwrap_or_default(),
            step_commands: fm.get("verification_step_commands")
                .map(|v| {
                    let mut map = HashMap::new();
                    for line in v.lines() {
                        if let Some((k, cmds)) = line.split_once(':') {
                            map.insert(k.trim().to_string(), cmds.trim().to_string());
                        }
                    }
                    map
                })
                .unwrap_or_default(),
            strict_gate: fm
                .get("verification_strict_gate")
                .map(|v| v == "true" || v == "1" || v == "yes")
                .unwrap_or(false),
            max_retries: fm
                .get("verification_max_retries")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(3),
        },
        approval_gates: fm
            .iter()
            .filter(|(k, _)| k.starts_with("approval_gate_"))
            .map(|(k, v)| {
                let trigger = k.strip_prefix("approval_gate_").unwrap_or(k).to_string();
                // Format: "approval_gate_<trigger>: <desc>" or "approval_gate_<trigger>: <desc>:soft"
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                let description = parts.first().map(|s| s.trim()).unwrap_or("").to_string();
                let mandatory = !v.ends_with(":soft");
                ApprovalGate {
                    description,
                    trigger,
                    mandatory,
                    approver: "user".to_string(),
                }
            })
            .collect(),
        permissions: SkillPermissions {
            read_paths: fm.get("permission_read_paths").map(|v| parse_list(v)).unwrap_or_default(),
            write_paths: fm.get("permission_write_paths").map(|v| parse_list(v)).unwrap_or_default(),
            env_vars: fm.get("permission_env_vars").map(|v| parse_list(v)).unwrap_or_default(),
            network_hosts: fm.get("permission_network_hosts").map(|v| parse_list(v)).unwrap_or_default(),
            requires_sudo: fm
                .get("permission_requires_sudo")
                .map(|v| v == "true" || v == "1" || v == "yes")
                .unwrap_or(false),
        },
        evidence: SkillEvidence {
            log_paths: fm.get("evidence_log_paths").map(|v| parse_list(v)).unwrap_or_default(),
            output_schema: fm.get("evidence_output_schema").cloned(),
            retention_days: fm
                .get("evidence_retention_days")
                .and_then(|v| v.parse::<u32>().ok()),
        },
    })
}

/// Scan base directory for skill directories and parse their SKILL.md.
fn discover_in_dir(base: &std::path::Path) -> Vec<Skill> {
    if !base.exists() {
        return Vec::new();
    }
    let mut skills = Vec::new();
    if let Ok(entries) = fs::read_dir(base) {
        for item in entries.filter_map(Result::ok) {
            let item_path = item.path();
            if !item_path.is_dir() {
                continue;
            }
            let skill_md = item_path.join(SKILL_FILENAME);
            if !skill_md.exists() {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&skill_md) {
                let fm = parse_frontmatter(&text);
                let name = fm.get("name").map(String::as_str).unwrap_or_else(|| {
                    item_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                });
                let desc = fm.get("description").map(String::as_str).unwrap_or("");
                let contract = parse_contract(&fm);
                skills.push(Skill {
                    name: name.to_string(),
                    description: desc.to_string(),
                    path: skill_md,
                    dir: item_path,
                    contract,
                });
            }
        }
    }
    skills
}

fn default_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}

/// Discover all skills from project and user skill directories.
pub fn discover_skills(
    project_skills_dir: Option<PathBuf>,
    user_skills_dir: Option<PathBuf>,
) -> Vec<Skill> {
    let mut discovered: HashMap<String, Skill> = HashMap::new();

    // Project skills
    let proj_dir = project_skills_dir.unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join(".claude")
            .join("skills")
    });
    for skill in discover_in_dir(&proj_dir) {
        discovered.insert(skill.name.clone(), skill);
    }

    // User skills
    let user_dir = user_skills_dir
        .map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(s.replace('~', &home))
                } else {
                    p
                }
            } else {
                p
            }
        })
        .unwrap_or_else(|| default_home_dir().join(".claude").join("skills"));

    for skill in discover_in_dir(&user_dir) {
        discovered.entry(skill.name.clone()).or_insert(skill);
    }

    let mut skills: Vec<_> = discovered.into_values().collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Re-scan skill directories, returning fresh list of skills if dirs changed.
pub fn reload_skills(
    project_skills_dir: Option<PathBuf>,
    user_skills_dir: Option<PathBuf>,
) -> Vec<Skill> {
    let proj_dir_raw = project_skills_dir.clone();
    let user_dir_raw = user_skills_dir.clone();
    let proj_dir = proj_dir_raw.unwrap_or_else(|| PathBuf::from(".claude/skills"));
    let user_dir = user_dir_raw
        .map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(s.replace('~', &home))
                } else {
                    p
                }
            } else {
                p
            }
        })
        .unwrap_or_else(|| default_home_dir().join(".claude").join("skills"));

    let mut dirs: Vec<PathBuf> = Vec::new();
    if proj_dir.exists() {
        dirs.push(proj_dir);
    }
    if user_dir.exists() {
        dirs.push(user_dir);
    }

    let mut changed = false;
    for d in &dirs {
        if let Ok(m) = d.metadata() {
            if let Ok(modified) = m.modified() {
                let mtime = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                let cache = get_mtime_cache().lock();
                if cache.get(d).copied() != Some(mtime) {
                    changed = true;
                }
            }
        }
    }

    if changed {
        discover_skills(project_skills_dir.clone(), user_skills_dir.clone())
    } else {
        Vec::new()
    }
}

/// Find a skill by exact name.
pub fn get_skill_by_name(name: &str, skills: Option<&[Skill]>) -> Option<Skill> {
    match skills {
        Some(slice) => slice.iter().find(|s| s.name == name).cloned(),
        None => {
            let all = discover_skills(None, None);
            all.into_iter().find(|s| s.name == name)
        }
    }
}

/// Match skills by query string (name + description keyword match).
pub fn match_skills(query: &str, skills: Option<&[Skill]>) -> Vec<Skill> {
    let skill_list: Vec<Skill> = match skills {
        Some(slice) => slice.to_vec(),
        None => discover_skills(None, None),
    };
    let q = query.to_lowercase();
    let mut scored: Vec<(i32, usize, Skill)> = Vec::new();
    for s in &skill_list {
        let name_lower = s.name.to_lowercase();
        let desc_lower = s.description.to_lowercase();
        let score = if name_lower.contains(&q) {
            if desc_lower.contains(&q) {
                2
            } else {
                3
            }
        } else if desc_lower.contains(&q) {
            1
        } else {
            continue;
        };
        let name_pos = name_lower.find(&q).unwrap_or(usize::MAX);
        scored.push((score, name_pos, s.clone()));
    }
    scored.sort_by_key(|x| (-x.0, x.1));
    scored.into_iter().map(|(_, _, s)| s).collect()
}

/// Quick helper — return just skill names as strings.
pub fn list_skill_names() -> Vec<String> {
    discover_skills(None, None)
        .into_iter()
        .map(|s| s.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_skill(parent: &std::path::Path, name: &str, description: &str) -> PathBuf {
        let skill_dir = parent.join(name);
        std::fs::create_dir(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        let mut f = std::fs::File::create(&skill_md).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "name: {}", name).unwrap();
        writeln!(f, "description: {}", description).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# Skill content").unwrap();
        skill_md
    }

    fn make_contractual_skill(
        parent: &std::path::Path,
        name: &str,
        required_fields: &[&str],
        approval_gates: &[(&str, &str)],
    ) -> PathBuf {
        let skill_dir = parent.join(name);
        std::fs::create_dir(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        let mut f = std::fs::File::create(&skill_md).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "name: {}", name).unwrap();
        writeln!(f, "description: A contractual skill").unwrap();
        writeln!(f, "input_required_fields: {}", required_fields.join(", ")).unwrap();
        writeln!(f, "quality_max_latency_secs: 10.0").unwrap();
        writeln!(f, "quality_min_accuracy: 0.85").unwrap();
        for (trigger, desc) in approval_gates {
            // Flat format: approval_gate_<trigger>: <desc> (mandatory unless desc ends with [soft])
            writeln!(f, "approval_gate_{}: {}", trigger, desc).unwrap();
        }
        writeln!(f, "verification_strict_gate: true").unwrap();
        writeln!(f, "verification_steps: validate_output, check_quality").unwrap();
        writeln!(f, "permission_read_paths: src/, tests/").unwrap();
        writeln!(f, "permission_write_paths: target/, .claude/").unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# Skill body").unwrap();
        skill_md
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: test-skill\ndescription: A test skill\n---\n# Body";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"test-skill".to_string()));
        assert_eq!(fm.get("description"), Some(&"A test skill".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_missing() {
        assert!(parse_frontmatter("no frontmatter").is_empty());
        assert!(parse_frontmatter("---no closing").is_empty());
    }

    #[test]
    fn test_parse_frontmatter_with_contract_fields() {
        let content = r#"---
name: my-skill
description: Does things
input_required_fields: file, model
input_optional_fields: timeout
input_accepted_formats: text/plain, application/json
quality_required_checks: not_empty, valid_json
quality_max_latency_secs: 5.0
verification_strict_gate: true
permission_read_paths: src/, lib/
permission_write_paths: target/
---"#;
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"my-skill".to_string()));
        let req: Vec<&str> = fm.get("input_required_fields")
            .map(|v| v.split(',').map(|s| s.trim()).collect::<Vec<_>>())
            .unwrap();
        assert!(req.contains(&"file") && req.contains(&"model"));
    }

    #[test]
    fn test_discover_in_dir() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "my-skill", "Does things");
        let skills = discover_in_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].description, "Does things");
        assert!(skills[0].contract.is_none());
    }

    #[test]
    fn test_discover_contractual_skill() {
        let dir = TempDir::new().unwrap();
        make_contractual_skill(
            dir.path(),
            "secure-write",
            &["file_path", "content"],
            &[("pre_write", "Verify file path is within project root")],
        );
        let skills = discover_in_dir(dir.path());
        assert_eq!(skills.len(), 1);
        let skill = &skills[0];
        assert!(skill.has_contract(), "Skill should have a contract");
        let contract = skill.contract.as_ref().unwrap();
        assert_eq!(contract.input.required_fields, vec!["file_path", "content"]);
        assert!(!contract.pending_approval_gates().is_empty());
        assert_eq!(contract.quality.max_latency_secs, Some(10.0));
        assert_eq!(contract.quality.min_accuracy, Some(0.85));
        assert!(contract.verification.strict_gate);
    }

    #[test]
    fn test_skill_validate_inputs_success() {
        let dir = TempDir::new().unwrap();
        make_contractual_skill(dir.path(), "test", &["foo", "bar"], &[]);
        let skills = discover_in_dir(dir.path());
        let skill = &skills[0];

        let mut provided = HashMap::new();
        provided.insert("foo".to_string(), serde_json::json!("value1"));
        provided.insert("bar".to_string(), serde_json::json!("value2"));
        assert!(skill.validate_inputs(&provided).is_ok());
    }

    #[test]
    fn test_skill_validate_inputs_missing_field() {
        let dir = TempDir::new().unwrap();
        make_contractual_skill(dir.path(), "test2", &["foo", "bar"], &[]);
        let skills = discover_in_dir(dir.path());
        let skill = &skills[0];

        let mut provided = HashMap::new();
        provided.insert("foo".to_string(), serde_json::json!("value1"));
        // bar is missing
        let err = skill.validate_inputs(&provided).unwrap_err();
        assert!(err.contains("bar"));
    }

    #[test]
    fn test_skill_needs_approval() {
        let dir = TempDir::new().unwrap();
        make_contractual_skill(
            dir.path(),
            "needs-approval",
            &[],
            &[("before_deploy", "Admin must sign off")],
        );
        let skills = discover_in_dir(dir.path());
        let skill = &skills[0];
        assert!(skill.needs_approval());
    }

    #[test]
    fn test_skill_no_approval_when_empty_gates() {
        let dir = TempDir::new().unwrap();
        make_contractual_skill(dir.path(), "no-gates", &[], &[]);
        let skills = discover_in_dir(dir.path());
        let skill = &skills[0];
        assert!(!skill.needs_approval());
    }

    #[test]
    fn test_permissions_satisfied() {
        let contract = SkillContract {
            permissions: SkillPermissions {
                read_paths: vec!["src/".to_string(), "tests/".to_string()],
                write_paths: vec!["target/".to_string()],
                requires_sudo: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let granted = SkillPermissions {
            read_paths: vec!["*".to_string()],
            write_paths: vec!["target/".to_string(), "other/".to_string()],
            requires_sudo: false,
            ..Default::default()
        };
        assert!(contract.permissions_satisfied_by(&granted));
    }

    #[test]
    fn test_permissions_not_satisfied_write() {
        let contract = SkillContract {
            permissions: SkillPermissions {
                write_paths: vec!["/etc/".to_string()],
                requires_sudo: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let granted = SkillPermissions {
            read_paths: vec![],
            write_paths: vec!["src/".to_string()],
            requires_sudo: false,
            ..Default::default()
        };
        assert!(!contract.permissions_satisfied_by(&granted));
    }

    #[test]
    fn test_contract_summary() {
        let contract = SkillContract {
            approval_gates: vec![
                ApprovalGate {
                    description: "Admin must approve".to_string(),
                    trigger: "pre_deploy".to_string(),
                    mandatory: true,
                    approver: "admin".to_string(),
                },
            ],
            ..Default::default()
        };
        let summary = contract.summary();
        assert!(summary.contains("approval gates"));
        assert!(summary.contains("pre_deploy"));
    }

    #[test]
    fn test_discover_in_dir_empty() {
        let dir = TempDir::new().unwrap();
        assert!(discover_in_dir(dir.path()).is_empty());
    }

    #[test]
    fn test_get_skill_by_name() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "find-me", "Description");
        let skills = discover_in_dir(dir.path());
        assert!(get_skill_by_name("find-me", Some(&skills)).is_some());
        assert!(get_skill_by_name("not-found", Some(&skills)).is_none());
    }

    #[test]
    fn test_match_skills() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "rust-coding", "Write Rust fast");
        make_skill(dir.path(), "python-coding", "Write Python code");
        let skills = discover_in_dir(dir.path());
        let matched = match_skills("rust", Some(&skills));
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "rust-coding");
    }

    #[test]
    fn test_list_skill_names() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "alpha", "First");
        make_skill(dir.path(), "beta", "Second");
        let all = discover_in_dir(dir.path());
        let names: Vec<String> = all.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }
}
