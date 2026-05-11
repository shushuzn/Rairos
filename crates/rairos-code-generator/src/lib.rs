//! Code Generator — generate Python implementation skeletons from parsed paper content.
//!
//! Translates structured paper content (algorithms, equations, claims) into
//! runnable Python code using an LLM.
//!
//! Python original: `research_loop/code_generator.py` (372 lines)

use rairos_llm::{LlmClient, LlmResponse, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenConfig {
    /// Framework: "pytorch" | "jax" | "numpy"
    pub framework: String,
    /// Override model name (None = auto-detect via LLM credentials)
    pub model_name: Option<String>,
    /// Timeout for LLM call in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300
}

impl Default for CodeGenConfig {
    fn default() -> Self {
        Self {
            framework: "pytorch".to_string(),
            model_name: None,
            timeout_secs: 300,
        }
    }
}

/// Paper content fields used for code generation.
/// Minimal subset of the full PaperContent struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperContent {
    pub title: String,
    pub arxiv_id: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub algorithm_descriptions: Vec<String>,
    pub equations: Vec<String>,
    pub claims: Vec<String>,
    pub hyperparameters: std::collections::HashMap<String, String>,
    pub datasets: Vec<String>,
    pub methods: Vec<String>,
}

impl Default for PaperContent {
    fn default() -> Self {
        Self {
            title: String::new(),
            arxiv_id: String::new(),
            abstract_text: String::new(),
            authors: Vec::new(),
            algorithm_descriptions: Vec::new(),
            equations: Vec::new(),
            claims: Vec::new(),
            hyperparameters: std::collections::HashMap::new(),
            datasets: Vec::new(),
            methods: Vec::new(),
        }
    }
}

/// System prompt injected into the LLM to enforce code quality rules.
pub const CODE_GEN_SYSTEM: &str = r#"You are an expert ML/AI researcher and Python programmer.

Given a research paper's structured content, generate a clean, runnable Python implementation.

CRITICAL RULES:
1. EVERY class body must contain at least one statement (pass or a real implementation).
2. EVERY function body must contain at least one statement (pass or a real implementation).
3. NEVER leave a class or function with only a comment and no body.
4. Use pass only when truly no implementation is possible; never use it as a placeholder.
5. Output a SINGLE valid Python file content (no markdown code blocks).
6. Every function must have a docstring citing the paper.
7. Include a main() function with a usage example.
8. Use type hints on all function signatures.
9. Include assertions for key preconditions from the paper.
10. Generate realistic placeholder implementations for LLM-related parts.
"#;

/// Generate code skeleton from parsed paper content using an LLM client.
pub fn generate_code<'a>(
    client: Arc<dyn LlmClient>,
    paper_content: &'a PaperContent,
    config: &'a CodeGenConfig,
) -> impl std::future::Future<Output = Result<String, String>> + 'a {
    generate_code_inner(client, paper_content, config)
}

async fn generate_code_inner(
    client: Arc<dyn LlmClient>,
    paper_content: &PaperContent,
    config: &CodeGenConfig,
) -> Result<String, String> {
    let model = config
        .model_name
        .clone()
        .unwrap_or_else(|| "minimax-m2.7-highspeed".to_string());

    let prompt = build_prompt(paper_content, &config.framework);

    let messages = vec![Message {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = client
        .complete(messages, &model, 0.3, 4096)
        .await
        .map_err(|e| e.to_string())?;

    let content = response.content().trim().to_string();

    // Strip thinking/reasoning blocks that some models emit before code
    let stripped = strip_thinking_blocks(&content);

    // Strip markdown code-block wrappers
    let code = strip_markdown_wrappers(&stripped);

    Ok(code)
}

/// Strip MiniMax/DeepSeek-style thinking blocks: 去想...```python and 「reasoning」...```
pub fn strip_thinking_blocks(input: &str) -> String {
    let re_think = Regex::new(r"(?s)^去想.*?```python\n").unwrap();
    let re_reason = Regex::new(r"(?s)^<think>.*?```python\n").unwrap();

    let mut result = input.to_string();
    result = re_think.replace_all(&result, "").to_string();
    result = re_reason.replace_all(&result, "").to_string();
    result.trim().to_string()
}

/// Strip triple-backtick markdown code-block wrappers from LLM output.
pub fn strip_markdown_wrappers(input: &str) -> String {
    let mut result = input.to_string();

    // Strip leading ```python / ``` markers
    let re_lead = Regex::new(r"(?m)^\s*```(?:python)?\s*\n").unwrap();
    result = re_lead.replace_all(&result, "").to_string();

    // Strip trailing ``` markers
    let re_trail = Regex::new(r"(?m)\n\s*```\s*$").unwrap();
    result = re_trail.replace_all(&result, "").to_string();

    // Strip plain leading/trailing triple backticks
    if result.starts_with("```") {
        result = result[3..].to_string();
    }
    if result.ends_with("```") {
        result = result[..result.len() - 3].to_string();
    }

    result.trim().to_string()
}

/// Build the user prompt from paper content.
pub fn build_prompt(paper_content: &PaperContent, framework: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("# Paper: {}", paper_content.title));
    parts.push(format!("arXiv ID: {}", paper_content.arxiv_id));

    let authors: Vec<&str> = paper_content.authors.iter().take(3).map(|s| s.as_str()).collect();
    let author_str = if paper_content.authors.len() > 3 {
        format!("{} et al.", authors.join(", "))
    } else {
        authors.join(", ")
    };
    parts.push(format!("Authors: {}", author_str));

    let abstract_preview = paper_content.abstract_text.chars().take(500).collect::<String>();
    parts.push(format!("\n## Abstract\n{}", abstract_preview));

    // Algorithm descriptions
    if !paper_content.algorithm_descriptions.is_empty() {
        parts.push("\n## Algorithms\n".to_string());
        for (i, algo) in paper_content.algorithm_descriptions.iter().take(3).enumerate() {
            let preview = algo.chars().take(500).collect::<String>();
            parts.push(format!("### Algorithm {}\n{}", i + 1, preview));
        }
    }

    // Key equations
    if !paper_content.equations.is_empty() {
        parts.push("\n## Key Equations\n".to_string());
        for eq in paper_content.equations.iter().take(5) {
            parts.push(format!("$${}$$", eq));
        }
    }

    // Key claims
    if !paper_content.claims.is_empty() {
        parts.push("\n## Key Claims\n".to_string());
        for claim in paper_content.claims.iter().take(5) {
            let preview = claim.chars().take(200).collect::<String>();
            parts.push(format!("- {}", preview));
        }
    }

    // Hyperparameters
    if !paper_content.hyperparameters.is_empty() {
        parts.push("\n## Hyperparameters\n".to_string());
        for (k, v) in &paper_content.hyperparameters {
            parts.push(format!("- {}: {}", k, v));
        }
    }

    // Datasets
    if !paper_content.datasets.is_empty() {
        parts.push(format!("\n## Datasets\n{}", paper_content.datasets.join(", ")));
    }

    // Methods
    if !paper_content.methods.is_empty() {
        parts.push("\n## Methods\n".to_string());
        for m in paper_content.methods.iter().take(5) {
            let preview = m.chars().take(150).collect::<String>();
            parts.push(format!("- {}", preview));
        }
    }

    parts.push(format!("\n## Framework: {}", framework.to_uppercase()));

    // Source tracing instruction
    parts.push(
        r#"
## Source Tracing Instructions
For each section of code that implements a specific equation, algorithm description,
or claim, add an inline comment on the first line with its source tag.
Format: # source: @<type>[<index>] — <human-readable description>
Examples:
  # source: @eq[0] — Attention equation from §3.2 p4
  # source: @algo[1] — Transformer encoder from §2.1
  # source: @eq[0], @eq[2] — Combined attention and feed-forward
If a code section implements multiple sources, list all tags separated by commas.
Tag each distinct functional block (class, main function, key sub-routine) that maps to a paper source.
"#
        .to_string(),
    );

    parts.push(format!(
        "\nGenerate a clean Python implementation skeleton in {}. \
         Match the algorithm exactly as described. \
         Output ONLY the Python code (no markdown).",
        framework.to_uppercase()
    ));

    parts.join("\n\n")
}

/// Strip prose lines using alpha-ratio + code-marker heuristic.
/// Applied after primary marker-based stripping for models (e.g. MiniMax)
/// that output plain-text descriptions without markdown fences.
pub fn strip_prose_secondary(code: &str) -> String {
    let re_py_kw = Regex::new(
        r"^(class |def |async |@|if |elif |for |while |with |try:|except:|finally:|raise |return |yield |pass |break |continue |assert |import |from |#|$)",
    )
    .unwrap();

    let markers: std::collections::HashSet<char> =
        "(){}=[]<>:@#\"".chars().collect();

    let mut result: Vec<&str> = Vec::new();
    for line in code.split('\n') {
        let stripped = line.trim_start();
        let total = stripped.trim_end_matches('\r').len();
        if total == 0 {
            result.push(line);
            continue;
        }
        let alpha = stripped.chars().filter(|c| c.is_alphabetic()).count();
        let ratio = alpha as f64 / total as f64;
        let has_marker = stripped.chars().any(|c| markers.contains(&c));
        let is_import =
            stripped.starts_with("import ") || stripped.starts_with("from ");
        let is_py_kw = re_py_kw.is_match(stripped);
        if total > 10 && ratio > 0.75 && !has_marker && !is_import && !is_py_kw {
            continue; // drop prose line
        }
        result.push(line);
    }
    result.join("\n")
}

/// Save generated code to a file.
pub fn save_code(
    code: &str,
    output_dir: &std::path::Path,
    module_name: &str,
) -> std::path::PathBuf {
    let re_fence = Regex::new(r"(?m)^\s*```(?:python)?\s*\n").unwrap();
    let code = re_fence.replace_all(code, "").to_string();

    let re_end_fence = Regex::new(r"(?m)\n\s*```\s*$").unwrap();
    let code = re_end_fence.replace_all(&code, "\n").to_string();

    // Strip thinking/reasoning blocks
    let code = strip_thinking_blocks(&code);

    // Strip text appended after valid Python entry point
    let re_main = Regex::new(
        r#"\nif __name__ == "__main__":\s*main\(\)\s*[\w\W]*$"#,
    )
    .unwrap();
    let code = re_main.replace_all(&code, "").to_string();

    // Secondary prose stripping
    let code = strip_prose_secondary(&code);

    let output_dir = std::path::PathBuf::from(output_dir);
    std::fs::create_dir_all(&output_dir).ok();
    let out_path = output_dir.join(format!("{}.py", module_name));
    // Write file — only create parent dirs if module_name has sub-paths
    std::fs::write(&out_path, code.trim().as_bytes()).expect("write code file");
    out_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_wrappers() {
        let input = "```python\nprint('hello')\n```";
        let out = strip_markdown_wrappers(input);
        assert_eq!(out.trim(), "print('hello')");
    }

    #[test]
    fn test_strip_markdown_wrappers_no_fence() {
        let input = "def foo():\n    pass";
        let out = strip_markdown_wrappers(input);
        assert!(out.contains("def foo"));
    }

    #[test]
    fn test_strip_thinking_blocks() {
        let input = "去想这是一个思考过程\n```python\nprint('hello')\n```";
        let out = strip_thinking_blocks(input);
        assert!(out.starts_with("print('hello')"));
    }

    #[test]
    fn test_build_prompt() {
        let paper = PaperContent {
            title: "Attention Is All You Need".to_string(),
            arxiv_id: "1706.03762".to_string(),
            abstract_text: "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.".to_string(),
            authors: vec!["Vaswani".to_string(), "Shazeer".to_string(), "Parmar".to_string(), "Gomes".to_string()],
            algorithm_descriptions: vec!["Transformer uses multi-head self-attention".to_string()],
            equations: vec!["Attention(Q,K,V) = softmax(QK^T / sqrt(d_k))V".to_string()],
            claims: vec!["Transformers outperform prior models".to_string()],
            hyperparameters: std::collections::HashMap::new(),
            datasets: vec!["WMT 2014".to_string()],
            methods: vec!["Multi-head attention".to_string()],
        };

        let prompt = build_prompt(&paper, "pytorch");
        assert!(prompt.contains("Attention Is All You Need"));
        assert!(prompt.contains("1706.03762"));
        assert!(prompt.contains("Transformer uses multi-head self-attention"));
        assert!(prompt.contains("## Framework: PYTORCH"));
    }

    #[test]
    fn test_strip_prose_secondary() {
        let code = "This is a description of the algorithm.\ndef foo():\n    pass\nMore explanation text here.\nclass Bar:\n    pass";
        let out = strip_prose_secondary(code);
        assert!(out.contains("def foo"));
        assert!(out.contains("class Bar"));
        assert!(!out.contains("This is a description"));
    }

    #[test]
    fn test_save_code() {
        let tmp_dir = std::env::temp_dir();
        let code = "def hello():\n    print('world')";
        let out_path = save_code(code, &tmp_dir, "test_model");
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("def hello"));
        assert_eq!(out_path.file_name().unwrap(), "test_model.py");
    }
}
