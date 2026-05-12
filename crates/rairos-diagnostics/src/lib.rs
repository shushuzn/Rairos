//! LSP-style diagnostics — progressive ruff (fast) + pyright (background).
//!
//! Integrates into the paper2code pipeline to surface code quality issues
//! before pytest runs, inspired by DeepSeek-TUI's LSP diagnostics integration.
//!
//! Python original: `research_loop/lsp_diagnostics.py` (175 lines)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

// ─── Diagnostic ────────────────────────────────────────────────────────────────

/// A single code diagnostic (lint or type error).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    /// "error" | "warning" | "information"
    pub severity: String,
    /// e.g. "E501", "PTH"
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "  [{}] {}:{}:{} {}: {}",
            self.severity.to_uppercase(),
            self.file.display(),
            self.line,
            self.column,
            self.code,
            self.message
        )
    }
}

// ─── Ruff (fast synchronous lint) ──────────────────────────────────────────────

fn ruff_executable() -> Option<PathBuf> {
    which::which("ruff").ok()
}

/// Run ruff linter synchronously. Returns immediately — typically < 1s.
pub fn check_ruff(code_path: &PathBuf) -> Vec<Diagnostic> {
    let ruff = match ruff_executable() {
        Some(p) => p,
        None => return vec![],
    };

    let output = Command::new(&ruff)
        .args([
            "check",
            code_path.to_str().unwrap_or_default(),
            "--output-format=json",
        ])
        .output();

    match output {
        Ok(out) => parse_ruff_json(code_path, String::from_utf8_lossy(&out.stdout).as_ref()),
        Err(_) => vec![],
    }
}

#[allow(dead_code)]
fn parse_ruff_json(code_path: &std::path::Path, stdout: &str) -> Vec<Diagnostic> {
    if stdout.trim().is_empty() {
        return vec![];
    }

    let entries: Vec<serde_json::Value> = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    entries
        .iter()
        .filter_map(|entry| {
            let location = entry.get("location")?;
            let rule = entry.get("rule")?.as_str()?;

            Some(Diagnostic {
                file: code_path.to_path_buf(),
                line: location.get("line")?.as_u64()? as u32,
                column: location.get("column")?.as_u64()? as u32,
                severity: ruff_to_severity(rule),
                code: entry.get("rule")?.as_str()?.to_string(),
                message: entry.get("message")?.as_str()?.to_string(),
            })
        })
        .collect()
}

fn ruff_to_severity(rule: &str) -> String {
    let prefix = rule.get(..1).unwrap_or("");
    if prefix == "E" || prefix == "F" {
        return "error".to_string();
    }
    if prefix == "W"
        || rule.starts_with("F4")
        || rule.starts_with("F5")
        || rule.starts_with("F6")
        || rule.starts_with("F7")
    {
        return "warning".to_string();
    }
    "information".to_string()
}

// ─── Pyright (slow async type check) ─────────────────────────────────────────

fn pyright_executable() -> Option<PathBuf> {
    which::which("pyright")
        .ok()
        .or_else(|| which::which("pyright.exe").ok())
}

/// Run pyright type checker. May take 10-60s depending on codebase size.
pub fn check_pyright(code_path: &PathBuf) -> Vec<Diagnostic> {
    let pyright = match pyright_executable() {
        Some(p) => p,
        None => return vec![],
    };

    let output = Command::new(&pyright)
        .args([code_path.to_str().unwrap_or_default(), "--outputjson"])
        .output();

    match output {
        Ok(out) => parse_pyright_json(code_path, String::from_utf8_lossy(&out.stdout).as_ref()),
        Err(_) => vec![],
    }
}

#[allow(dead_code)]
fn parse_pyright_json(code_path: &std::path::Path, stdout: &str) -> Vec<Diagnostic> {
    if stdout.trim().is_empty() {
        return vec![];
    }

    let report: serde_json::Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let Some(general_diagnostics) = report.get("generalDiagnostics").and_then(|v| v.as_array())
    else {
        return vec![];
    };

    general_diagnostics
        .iter()
        .filter_map(|diag| {
            let severity = diag.get("severity")?.as_str()?.to_string();
            let range = diag.get("range")?;
            let start = range.get("start")?;

            Some(Diagnostic {
                file: code_path.to_path_buf(),
                line: start.get("line")?.as_u64()? as u32,
                column: start.get("character")?.as_u64()? as u32,
                severity,
                code: "pyright".to_string(),
                message: diag.get("message")?.as_str()?.to_string(),
            })
        })
        .collect()
}

// ─── Progressive runner ────────────────────────────────────────────────────────

/// Run diagnostics progressively: ruff first (fast), then pyright (background).
pub fn run_progressive<F, C>(code_path: PathBuf, on_fast: F, on_complete: C)
where
    F: FnOnce(&[Diagnostic]) + Send + 'static,
    C: FnOnce(&[Diagnostic]) + Send + 'static,
{
    // Fast path: ruff synchronously
    let ruff_results = check_ruff(&code_path);
    on_fast(&ruff_results);

    // Slow path: pyright in background thread
    std::thread::spawn(move || {
        let pyright_results = check_pyright(&code_path);
        on_complete(&pyright_results);
    });
}

// ─── Formatting ────────────────────────────────────────────────────────────────

/// Format diagnostics for terminal display.
pub fn format_diagnostics(diagnostics: &[Diagnostic], header: &str) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!(
        "\n{} ({} issues):",
        header.to_uppercase(),
        diagnostics.len()
    )];
    for d in diagnostics {
        lines.push(d.to_string());
    }
    lines.join("\n")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruff_to_severity() {
        assert_eq!(ruff_to_severity("E501"), "error");
        assert_eq!(ruff_to_severity("W503"), "warning");
        assert_eq!(ruff_to_severity("F401"), "error");
        assert_eq!(ruff_to_severity("I001"), "information");
        assert_eq!(ruff_to_severity("PTH"), "information"); // P prefix → information
        assert_eq!(ruff_to_severity("UP006"), "information"); // UP prefix → information
    }

    #[test]
    fn test_diagnostic_display() {
        let d = Diagnostic {
            file: PathBuf::from("/tmp/test.py"),
            line: 10,
            column: 5,
            severity: "error".to_string(),
            code: "E501".to_string(),
            message: "Line too long".to_string(),
        };
        let s = d.to_string();
        assert!(s.contains("E501"));
        assert!(s.contains("10"));
        assert!(s.contains("Line too long"));
    }

    #[test]
    fn test_format_empty() {
        let out = format_diagnostics(&[], "diagnostics");
        assert!(out.is_empty());
    }

    #[test]
    fn test_format_multiple() {
        let diags = vec![
            Diagnostic {
                file: PathBuf::from("test.py"),
                line: 1,
                column: 1,
                severity: "error".to_string(),
                code: "E501".to_string(),
                message: "Line too long".to_string(),
            },
            Diagnostic {
                file: PathBuf::from("test.py"),
                line: 2,
                column: 1,
                severity: "warning".to_string(),
                code: "W503".to_string(),
                message: "Old style".to_string(),
            },
        ];
        let out = format_diagnostics(&diags, "ruff");
        assert!(out.contains("2 issues"));
        assert!(out.contains("E501"));
        assert!(out.contains("W503"));
    }
}
