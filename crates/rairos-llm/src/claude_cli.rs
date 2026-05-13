use std::io::Write;
use std::process::{Command, Stdio};

#[cfg(target_family = "windows")]
fn get_claude_executable() -> &'static str {
    "claude.cmd"
}

#[cfg(not(target_family = "windows"))]
fn get_claude_executable() -> &'static str {
    "claude"
}

pub struct ClaudeCLIClient {
    cli_path: String,
}

impl ClaudeCLIClient {
    pub fn new(cli_path: Option<String>) -> Self {
        Self {
            cli_path: cli_path.unwrap_or_else(|| get_claude_executable().to_string()),
        }
    }

    pub fn chat(
        &self,
        prompt: &str,
        model: &str,
        system_prompt: Option<&str>,
        _temperature: f32,
        _max_tokens: usize,
    ) -> Result<String, String> {
        let full_prompt = match system_prompt {
            Some(sys) => format!("[System: {}]\n\n{}", sys, prompt),
            None => prompt.to_string(),
        };

        let mut child = Command::new(&self.cli_path)
            .args([
                "--print",
                "--model",
                model,
                "--output-format",
                "json",
                "--input-format",
                "text",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn Claude CLI: {}", e))?;

        if let Some(ref mut stdin) = child.stdin {
            stdin
                .write_all(full_prompt.as_bytes())
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Claude CLI timed out: {}", e))?;

        if output.status.code() != Some(0) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout_preview = String::from_utf8_lossy(&output.stdout)
                .trim()
                .chars()
                .take(200)
                .collect::<String>();
            if !stdout_preview.is_empty() {
                eprintln!(
                    "Warning: Claude CLI exit code {:?} (hook error?), using stdout: {}",
                    output.status.code(),
                    &stderr.trim().chars().take(120).collect::<String>()
                );
            } else {
                return Err(format!("Claude CLI error: {}", stderr.trim()));
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(result) = data.get("result").and_then(|v| v.as_str()) {
                return Ok(result.to_string());
            }
        }

        if let Some(brace_pos) = stdout.find('{') {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&stdout[brace_pos..]) {
                if let Some(result) = data.get("result").and_then(|v| v.as_str()) {
                    return Ok(result.to_string());
                }
            }
        }

        Ok(stdout)
    }

    pub fn is_available(&self) -> bool {
        Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .map(|o| o.status.code() == Some(0))
            .unwrap_or(false)
    }

    pub fn get_version(&self) -> Option<String> {
        Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.code() == Some(0))
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }
}

pub fn get_claude_cli_client() -> ClaudeCLIClient {
    ClaudeCLIClient::new(None)
}

pub fn is_claude_cli_available() -> bool {
    get_claude_cli_client().is_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ClaudeCLIClient::new(None);
        assert!(!client.cli_path.is_empty());
    }

    #[test]
    fn test_get_claude_executable() {
        let exe = get_claude_executable();
        assert!(!exe.is_empty());
    }

    #[test]
    fn test_chat_creation_with_parameters() {
        let client = ClaudeCLIClient::new(Some("fake_path".to_string()));
        assert_eq!(client.cli_path, "fake_path");
    }

    #[test]
    fn test_is_available_returns_bool() {
        let client = ClaudeCLIClient::new(Some("nonexistent".to_string()));
        let result = client.is_available();
        assert!(!result);
    }

    #[test]
    fn test_version_returns_option() {
        let client = ClaudeCLIClient::new(Some("nonexistent".to_string()));
        let result = client.get_version();
        assert!(result.is_none());
    }
}
