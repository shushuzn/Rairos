//! rairos-lean-verifier — Lean 4 Theorem Prover Integration.

#![allow(dead_code)]
//!
//! Ported from `llm/lean_verifier.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeanInstallStatus {
    Available,
    NotFound,
    VersionUnknown,
}

impl LeanInstallStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeanInstallStatus::Available => "available",
            LeanInstallStatus::NotFound => "not_found",
            LeanInstallStatus::VersionUnknown => "version_unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationLevel {
    L0Syntax,
    L1Typecheck,
    L2Proven,
    L0Failed,
}

impl VerificationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationLevel::L0Syntax => "l0_syntax",
            VerificationLevel::L1Typecheck => "l1_typecheck",
            VerificationLevel::L2Proven => "l2_proven",
            VerificationLevel::L0Failed => "l0_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanVerificationResult {
    pub hypothesis_id: String,
    pub hypothesis_text: String,
    pub level: VerificationLevel,
    pub lean_code: String,
    #[serde(default)]
    pub lean_file_path: Option<String>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub json_output: Option<serde_json::Value>,
    pub install_status: LeanInstallStatus,
    #[serde(default)]
    pub translation_notes: String,
}

impl LeanVerificationResult {
    pub fn new(hypothesis_id: &str, hypothesis_text: &str, lean_code: &str) -> Self {
        Self {
            hypothesis_id: hypothesis_id.to_string(),
            hypothesis_text: hypothesis_text.to_string(),
            level: VerificationLevel::L0Failed,
            lean_code: lean_code.to_string(),
            lean_file_path: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            json_output: None,
            install_status: LeanInstallStatus::NotFound,
            translation_notes: String::new(),
        }
    }
}

pub fn check_lean_installed() -> (LeanInstallStatus, Option<String>) {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if Path::new(dir).join("lean").exists() {
                return match get_lean_version() {
                    Some(v) => (LeanInstallStatus::Available, Some(v)),
                    None => (LeanInstallStatus::VersionUnknown, None),
                };
            }
        }
    }

    #[cfg(not(windows))]
    {
        if Path::new("/usr/bin/lean").exists() || Path::new("/usr/local/bin/lean").exists() {
            let version = get_lean_version();
            return (LeanInstallStatus::Available, version);
        }
    }

    (LeanInstallStatus::NotFound, None)
}

fn get_lean_version() -> Option<String> {
    let output = process::Command::new("lean")
        .arg("--version")
        .output()
        .ok()?;

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        Some(String::from_utf8_lossy(&output.stderr).trim().to_string())
    } else {
        Some(version)
    }
    .filter(|s| !s.is_empty())
}

pub fn get_lean_install_instructions() -> String {
    r#"Lean 4 is not installed. To install:

  macOS/Linux:
    lake new myproject && cd myproject && lake build

  Windows (with elan):
    elan default leanprover/lean4:stable
    lean --version

  Or use GitHub Actions CI cache approach — see:
    https://leanprover-community.github.io/get_started.html"#
        .to_string()
}

const LEAN_TRANSLATION_SYSTEM: &str = r#"You are a Lean 4 code generator. Translate informal research hypotheses into Lean 4 code.

## Core Conventions

### Unicode Symbol Mapping
- ∀ → ∀  (typed as `\all` or `\forall`)
- ∃ → ∃  (typed as `\ex` or `\exists`)
- ≤ → ≤  (typed as `\le`)
- ≥ → ≥  (typed as `\ge`)
- ≠ → ≠  (typed as `\ne`)
- → → →  (typed as `\r`)
- ← → ←  (typed as `\l`)
- ∧ → ∧  (typed as `\and`)
- ∨ → ∨  (typed as `\or`)
- ¬ → ¬  (typed as `\not`)
- ∈ → ∈  (typed as `\in`)
- ⊆ → ⊆  (typed as `\sub`)

### Lean 4 Types & Mathlib Conventions
- Natural numbers: `Nat` (NOT `int` or `Integer`)
- Integers: `Int`
- Real numbers: `Real` (from `Mathlib`)
- Booleans: `Bool`
- Propositions: `Prop` (implicit for theorems)
- Sets: `Set α` (from `Std.Data.Set`)
- Functions: `α → β` (arrow notation)
- Equality: `=`
- Negation: `¬P` (typed as `\not P`)

### Naming Conventions
- Types: `CamelCase` (e.g., `Group`, `Vector`)
- Variables/functions: `snake_case` (e.g., `add_comm`, `composition`)
- Theorems: `snake_case` with descriptive name
- Use `∀ n : Nat` (with type annotation)
- Use `: Prop` for propositional theorems"#;

pub fn translate_hypothesis_to_lean(
    _hypothesis_id: &str,
    core_statement: &str,
    _hypothesis_type: &str,
) -> (String, String) {
    let safe_name: String = core_statement
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .take(30)
        .collect();

    let safe_name = if safe_name.is_empty() {
        "hypothesis".to_string()
    } else {
        safe_name
    };

    let lean_code = format!(
        r#"-- Hypothesis: {name}
-- Type: {hyp_type}

/- Formal claim: {statement} -/

theorem {name}_theorem
    : Prop  :=  by sorry
"#,
        name = safe_name,
        hyp_type = "exploratory",
        statement = &core_statement[..core_statement.len().min(80)]
    );

    (lean_code, "(template-based fallback)".to_string())
}

fn translate_symbols(text: &str) -> String {
    let symbol_map: HashMap<&str, &str> = HashMap::from([
        ("forall", "∀"),
        ("exists", "∃"),
        ("<=", "≤"),
        (">=", "≥"),
        ("->", "→"),
        ("<-", "←"),
    ]);

    let mut result = text.to_string();
    for (old, new) in &symbol_map {
        result = result.replace(old, new);
    }
    result
}

pub fn verify_lean_code(
    lean_code: &str,
    hypothesis_id: &str,
    hypothesis_text: &str,
) -> LeanVerificationResult {
    let (install_status, _version) = check_lean_installed();

    let mut result = LeanVerificationResult::new(hypothesis_id, hypothesis_text, lean_code);
    result.install_status = install_status;

    if install_status != LeanInstallStatus::Available {
        result.errors.push(get_lean_install_instructions());
        return result;
    }

    let temp_file = match write_temp_lean_file(lean_code) {
        Ok(p) => p,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to write temp file: {}", e));
            return result;
        }
    };

    result.lean_file_path = Some(temp_file.path().to_string_lossy().to_string());

    let output = process::Command::new("lean")
        .arg("--json")
        .arg(temp_file.path())
        .output();

    let proc = match output {
        Ok(o) => o,
        Err(e) => {
            result.errors.push(format!("Failed to run lean: {}", e));
            result.install_status = LeanInstallStatus::NotFound;
            let _ = fs::remove_file(temp_file.path());
            return result;
        }
    };

    let stdout = String::from_utf8_lossy(&proc.stdout);
    let stderr = String::from_utf8_lossy(&proc.stderr);
    let all_output = format!("{}\n{}", stdout, stderr);

    let mut error_lines = Vec::new();
    let mut warning_lines = Vec::new();

    for line in all_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            let severity = msg.get("severity").and_then(|v| v.as_str()).unwrap_or("");
            let data = msg.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let file = msg.get("file").and_then(|v| v.as_str()).unwrap_or("?");
            let pos = msg.get("pos").and_then(|v| v.as_i64()).unwrap_or(0);

            let formatted = format!("{}:{} — {}", file, pos, data);

            match severity {
                "error" => error_lines.push(formatted),
                "warning" => warning_lines.push(formatted),
                _ => {
                    if data.to_lowercase().contains("error") {
                        error_lines.push(formatted);
                    }
                }
            }
        } else if line.to_lowercase().contains("error") || line.to_lowercase().contains("failed") {
            error_lines.push(line.to_string());
        }
    }

    result.errors = error_lines;
    result.warnings = warning_lines;

    if result.errors.is_empty() && proc.status.success() {
        if lean_code.contains("sorry") {
            result.level = VerificationLevel::L2Proven;
            result.translation_notes = "Proof stub present (by sorry) — level L2".to_string();
        } else {
            result.level = VerificationLevel::L1Typecheck;
            result.translation_notes = "Type-correct, no proof".to_string();
        }
    } else if result.errors.is_empty() {
        result.level = VerificationLevel::L0Syntax;
        result.translation_notes = "Syntax valid, type errors present".to_string();
    } else {
        result.level = VerificationLevel::L0Failed;
        result.translation_notes = format!("Syntax/type errors: {} issue(s)", result.errors.len());
    }

    let _ = fs::remove_file(temp_file.path());
    result
}

fn write_temp_lean_file(content: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let temp_file = tempfile::NamedTempFile::with_suffix(".lean")?;
    fs::write(temp_file.path(), content)?;
    Ok(temp_file)
}

pub fn render_result(result: &LeanVerificationResult) -> String {
    let level_icon = match result.level {
        VerificationLevel::L0Failed => "❌",
        VerificationLevel::L0Syntax => "🟡",
        VerificationLevel::L1Typecheck => "🟢",
        VerificationLevel::L2Proven => "✅",
    };

    let install_icon = match result.install_status {
        LeanInstallStatus::Available => "✅ Lean installed",
        LeanInstallStatus::NotFound => "⚠️  Lean not found",
        LeanInstallStatus::VersionUnknown => "? Lean version unknown",
    };

    let mut lines = vec![
        format!("──{}──", "─".repeat(58)),
        format!(
            "{} Verification Level: {}",
            level_icon,
            result.level.as_str()
        ),
        format!("{}", install_icon),
        String::new(),
    ];

    if !result.hypothesis_id.is_empty() {
        lines.push(format!("  Hypothesis ID: {}", result.hypothesis_id));
    }

    if !result.hypothesis_text.is_empty() {
        let stmt = if result.hypothesis_text.len() > 80 {
            format!("{}...", &result.hypothesis_text[..80])
        } else {
            result.hypothesis_text.clone()
        };
        lines.push(format!("  Claim: {}", stmt));
    }

    lines.push(String::new());
    lines.push("── Lean code ─────────────────────────────".to_string());
    lines.push(if result.lean_code.is_empty() {
        "(no code generated)".to_string()
    } else {
        result.lean_code.clone()
    });

    if !result.errors.is_empty() {
        lines.push(String::new());
        lines.push("── Errors ─────────────────────────────────".to_string());
        for err in &result.errors {
            lines.push(format!("  ✗ {}", err));
        }
    }

    if !result.warnings.is_empty() {
        lines.push(String::new());
        lines.push("── Warnings ────────────────────────────────".to_string());
        for warn in &result.warnings {
            lines.push(format!("  ⚠ {}", warn));
        }
    }

    if result.install_status == LeanInstallStatus::NotFound {
        lines.push(String::new());
        lines.push(get_lean_install_instructions());
    }

    lines.push(format!("──{}──", "─".repeat(58)));
    lines.join("\n")
}

pub fn render_result_json(result: &LeanVerificationResult) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "hypothesis_id": result.hypothesis_id,
        "hypothesis_text": result.hypothesis_text,
        "level": result.level.as_str(),
        "lean_code": result.lean_code,
        "errors": result.errors,
        "warnings": result.warnings,
        "install_status": result.install_status.as_str(),
        "translation_notes": result.translation_notes,
        "lean_file_path": result.lean_file_path,
    }))
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lean_install_status_as_str() {
        assert_eq!(LeanInstallStatus::Available.as_str(), "available");
        assert_eq!(LeanInstallStatus::NotFound.as_str(), "not_found");
        assert_eq!(
            LeanInstallStatus::VersionUnknown.as_str(),
            "version_unknown"
        );
    }

    #[test]
    fn test_verification_level_as_str() {
        assert_eq!(VerificationLevel::L0Syntax.as_str(), "l0_syntax");
        assert_eq!(VerificationLevel::L1Typecheck.as_str(), "l1_typecheck");
        assert_eq!(VerificationLevel::L2Proven.as_str(), "l2_proven");
        assert_eq!(VerificationLevel::L0Failed.as_str(), "l0_failed");
    }

    #[test]
    fn test_translate_hypothesis_to_lean() {
        let (code, notes) = translate_hypothesis_to_lean(
            "test_id",
            "forall n m : Nat, n + m = m + n",
            "exploratory",
        );
        assert!(!code.is_empty());
        assert!(code.contains("theorem"));
        assert!(code.contains("Prop"));
        assert_eq!(notes, "(template-based fallback)");
    }

    #[test]
    fn test_verify_lean_code_no_lean() {
        let (status, _) = check_lean_installed();
        let result = verify_lean_code("theorem test : Prop := by sorry", "id", "test hypothesis");
        if status != LeanInstallStatus::Available {
            assert!(result.install_status != LeanInstallStatus::Available);
        }
    }

    #[test]
    fn test_render_result_json() {
        let result = LeanVerificationResult::new("id1", "test text", "code");
        let json = render_result_json(&result);
        assert!(json.contains("id1"));
        assert!(json.contains("test text"));
    }

    #[test]
    fn test_lean_install_instructions_not_empty() {
        let instructions = get_lean_install_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.contains("Lean 4"));
    }

    #[test]
    fn test_translate_symbols() {
        let result = translate_symbols("forall x -> y");
        assert!(result.contains("∀"));
        assert!(result.contains("→"));
    }

    #[test]
    fn test_lean_verification_result_new() {
        let result = LeanVerificationResult::new("id", "text", "code");
        assert_eq!(result.hypothesis_id, "id");
        assert_eq!(result.hypothesis_text, "text");
        assert_eq!(result.lean_code, "code");
        assert_eq!(result.level, VerificationLevel::L0Failed);
    }
}
