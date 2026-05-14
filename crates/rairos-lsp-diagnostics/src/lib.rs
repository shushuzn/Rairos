use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "  [{}] {}:{}:{} {}: {}",
            self.severity.to_uppercase(),
            self.file,
            self.line,
            self.column,
            self.code,
            self.message
        )
    }
}

#[derive(Debug, Deserialize)]
struct RuffEntry {
    location: Option<RuffLocation>,
    #[serde(alias = "rule")]
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuffLocation {
    line: Option<u32>,
    column: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PyrightOutput {
    #[serde(rename = "generalDiagnostics")]
    general_diagnostics: Option<Vec<PyrightDiagnostic>>,
}

#[derive(Debug, Deserialize)]
struct PyrightDiagnostic {
    severity: Option<String>,
    range: Option<PyrightRange>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PyrightRange {
    start: Option<PyrightPosition>,
}

#[derive(Debug, Deserialize)]
struct PyrightPosition {
    line: Option<u32>,
    character: Option<u32>,
}

fn ruff_severity(rule: &str) -> &str {
    let prefix = rule.chars().next().unwrap_or(' ');
    match prefix {
        'E' | 'F' => "error",
        'W' => "warning",
        _ => "information",
    }
}

pub fn check_ruff(code_path: &str) -> Vec<Diagnostic> {
    let ruff = which("ruff");
    if ruff.is_none() {
        return Vec::new();
    }
    let result = std::process::Command::new(ruff.unwrap())
        .args(["check", code_path, "--output-format=json"])
        .output();
    let output = match result {
        Ok(o) if o.status.success() || o.status.code() == Some(1) => o,
        _ => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ruff_json(code_path, &stdout)
}

fn parse_ruff_json(code_path: &str, stdout: &str) -> Vec<Diagnostic> {
    if stdout.trim().is_empty() {
        return Vec::new();
    }
    let entries: Vec<RuffEntry> = match serde_json::from_str(stdout) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .into_iter()
        .map(|e| {
            let loc = e.location.unwrap_or(RuffLocation {
                line: Some(1),
                column: Some(1),
            });
            let code = e.code.unwrap_or_default();
            Diagnostic {
                file: code_path.to_string(),
                line: loc.line.unwrap_or(1),
                column: loc.column.unwrap_or(1),
                severity: ruff_severity(&code).to_string(),
                code,
                message: e.message.unwrap_or_default(),
            }
        })
        .collect()
}

pub fn check_pyright(code_path: &str) -> Vec<Diagnostic> {
    let pyright = which("pyright").or_else(|| which("pyright.exe"));
    let pyright = match pyright {
        Some(p) => p,
        None => return Vec::new(),
    };
    let result = std::process::Command::new(pyright)
        .args([code_path, "--outputjson"])
        .output();
    let output = match result {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pyright_json(code_path, &stdout)
}

fn parse_pyright_json(code_path: &str, stdout: &str) -> Vec<Diagnostic> {
    if stdout.trim().is_empty() {
        return Vec::new();
    }
    let report: PyrightOutput = match serde_json::from_str(stdout) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let Some(diags) = report.general_diagnostics else {
        return Vec::new();
    };
    diags
        .into_iter()
        .map(|d| {
            let start = d
                .range
                .as_ref()
                .and_then(|r| r.start.as_ref())
                .unwrap_or(&PyrightPosition {
                    line: Some(1),
                    character: Some(1),
                });
            Diagnostic {
                file: code_path.to_string(),
                line: start.line.unwrap_or(1),
                column: start.character.unwrap_or(1),
                severity: d.severity.unwrap_or_else(|| "information".to_string()),
                code: "pyright".to_string(),
                message: d.message.unwrap_or_default(),
            }
        })
        .collect()
}

fn which(name: &str) -> Option<String> {
    std::env::var("PATH").ok().and_then(|path| {
        for dir in path.split(':') {
            let candidate = format!("{dir}/{name}");
            if std::path::Path::new(&candidate).is_file() {
                return Some(candidate);
            }
            let candidate_exe = format!("{candidate}.exe");
            if std::path::Path::new(&candidate_exe).is_file() {
                return Some(candidate_exe);
            }
        }
        None
    })
}

pub fn format_diagnostics(diagnostics: &[Diagnostic], header: &str) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }
    let mut lines = vec![format!("\n{} ({} issues):", header.to_uppercase(), diagnostics.len())];
    for d in diagnostics {
        lines.push(d.to_string());
    }
    lines.join("\n")
}

// ─── PyO3 Bindings ─────────────────────────────────────────────────────────────

fn diag_to_dict(d: &Diagnostic, py: Python<'_>) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("file", &d.file)?;
    dict.set_item("line", d.line)?;
    dict.set_item("column", d.column)?;
    dict.set_item("severity", &d.severity)?;
    dict.set_item("code", &d.code)?;
    dict.set_item("message", &d.message)?;
    Ok(dict.into())
}

fn dict_to_diag(dict: &Bound<'_, PyDict>) -> PyResult<Diagnostic> {
    Ok(Diagnostic {
        file: dict.get_item("file")?.and_then(|v| v.extract().ok()).unwrap_or_default(),
        line: dict.get_item("line")?.and_then(|v| v.extract().ok()).unwrap_or(1),
        column: dict.get_item("column")?.and_then(|v| v.extract().ok()).unwrap_or(1),
        severity: dict.get_item("severity")?.and_then(|v| v.extract().ok()).unwrap_or_default(),
        code: dict.get_item("code")?.and_then(|v| v.extract().ok()).unwrap_or_default(),
        message: dict.get_item("message")?.and_then(|v| v.extract().ok()).unwrap_or_default(),
    })
}

#[pyfunction]
pub fn check_ruff_py(code_path: &str) -> PyResult<Vec<Py<PyDict>>> {
    Ok(check_ruff(code_path).iter().map(|d| {
        Python::with_gil(|py| diag_to_dict(d, py).unwrap())
    }).collect())
}

#[pyfunction]
pub fn check_pyright_py(code_path: &str) -> PyResult<Vec<Py<PyDict>>> {
    Ok(check_pyright(code_path).iter().map(|d| {
        Python::with_gil(|py| diag_to_dict(d, py).unwrap())
    }).collect())
}

#[pyfunction]
pub fn format_diagnostics_py(diagnostics: &Bound<'_, PyAny>, header: &str) -> PyResult<String> {
    let diags: Vec<Diagnostic> = Python::with_gil(|_py| {
        let iter = PyIterator::from_object(diagnostics)?;
        let mut result = Vec::new();
        for item in iter {
            if let Ok(dict) = item?.downcast::<PyDict>() {
                if let Ok(d) = dict_to_diag(dict) {
                    result.push(d);
                }
            }
        }
        Ok::<_, PyErr>(result)
    })?;
    Ok(format_diagnostics(&diags, header))
}

#[pymodule]
pub fn rairos_lsp_diagnostics_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_ruff_py, m)?)?;
    m.add_function(wrap_pyfunction!(check_pyright_py, m)?)?;
    m.add_function(wrap_pyfunction!(format_diagnostics_py, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ruff_json_empty() {
        let result = parse_ruff_json("test.py", "");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_ruff_json_valid() {
        let json = r#"[{"location":{"line":5,"column":10},"code":"E501","message":"Line too long"}]"#;
        let result = parse_ruff_json("test.py", json);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line, 5);
        assert_eq!(result[0].code, "E501");
    }

    #[test]
    fn test_ruff_severity() {
        assert_eq!(ruff_severity("E501"), "error");
        assert_eq!(ruff_severity("W291"), "warning");
        assert_eq!(ruff_severity("I001"), "information");
    }

    #[test]
    fn test_display() {
        let d = Diagnostic {
            file: "test.py".to_string(),
            line: 1,
            column: 1,
            severity: "error".to_string(),
            code: "E999".to_string(),
            message: "Syntax error".to_string(),
        };
        let s = d.to_string();
        assert!(s.contains("ERROR"));
        assert!(s.contains("E999"));
    }

    #[test]
    fn test_format_diagnostics_empty() {
        assert!(format_diagnostics(&[], "lint").is_empty());
    }
}
