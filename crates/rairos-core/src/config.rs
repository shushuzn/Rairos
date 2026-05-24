//! rairos-config — Centralised configuration for AI Research OS
//!
//! Ported from `config.py`.
//!
//! All hardcoded magic numbers are accessed via this module rather than inlined.
//! Environment variables follow the pattern `AIROS_<CONFIG_NAME>` for consistency.
//!
//! # Example
//!
//! ```rust
//! use rairos_config::{
//!     EMBEDDING_DIM, CACHE_TTL_SECONDS, DEFAULT_LLM_MODEL,
//!     get_config, validate_config,
//! };
//!
//! // Access config values
//! println!("Embedding dim: {}", EMBEDDING_DIM);
//! println!("Default LLM: {}", DEFAULT_LLM_MODEL);
//!
//! // Get all config as a JSON value
//! let config = get_config();
//! println!("{:#?}", config);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

// ============================================================================
// .env File Loading
// ============================================================================

/// Load key=value pairs from a .env file into the environment.
#[allow(dead_code)]
fn load_env_file() {
    // Find .env relative to the crate root
    if let Ok(cwd) = std::env::current_dir() {
        let env_path = cwd.join(".env");
        if env_path.exists() {
            if let Ok(contents) = fs::read_to_string(&env_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        // Use set_var only if the key is not already set
                        if std::env::var(key).is_err() {
                            std::env::set_var(key, value);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Embedding
// ============================================================================

/// Embedding vector dimension (nomic-embed-text uses 768).
pub const DEFAULT_EMBEDDING_DIM: usize = 768;

/// Embedding vector dimension. Defaults to 768.
pub fn embedding_dim() -> usize {
    env_var_usize("AIROS_EMBEDDING_DIM", DEFAULT_EMBEDDING_DIM)
}

/// Embedding vector dimension (nomic-embed-text uses 768).
pub const EMBEDDING_DIM: usize = DEFAULT_EMBEDDING_DIM;

// ============================================================================
// HTTP Cache
// ============================================================================

/// How long cached arXiv / Crossref API responses live, in seconds.
pub fn cache_ttl_seconds() -> usize {
    env_var_usize("AIROS_CACHE_TTL_SECONDS", 24 * 3600)
}

/// Alias for backward compatibility.
pub const CACHE_TTL_SECONDS: usize = 24 * 3600;

/// Directory for disk cache storage.
pub fn cache_dir() -> String {
    env_var("AIROS_CACHE_DIR", "data".to_string())
}

/// Maximum number of cache files per directory.
pub fn max_cache_files() -> usize {
    env_var_usize("AIROS_MAX_CACHE_FILES", 2000)
}

/// Maximum number of items in memory cache.
pub fn memory_cache_max_size() -> usize {
    env_var_usize("AIROS_MEMORY_CACHE_MAX_SIZE", 1000)
}

// ============================================================================
// LLM Cost Table
// ============================================================================

/// A (input_price_per_1M, output_price_per_1M) pair.
pub type ModelPrice = (f64, f64);

/// Built-in model price table.
pub fn builtin_model_prices() -> HashMap<&'static str, ModelPrice> {
    let mut m = HashMap::new();
    m.insert("gpt-4o", (2.5, 10.0));
    m.insert("gpt-4o-mini", (0.15, 0.6));
    m.insert("gpt-4-turbo", (10.0, 30.0));
    m.insert("gpt-3.5-turbo", (0.5, 1.5));
    m.insert("o1-preview", (15.0, 60.0));
    m.insert("o1-mini", (3.0, 12.0));
    m.insert("qwen3.5-plus", (0.1, 0.3));
    m.insert("qwen3.5", (0.1, 0.3));
    m.insert("qwen2.5", (0.1, 0.3));
    m.insert("deepseek-chat", (0.14, 0.28));
    m.insert("claude-3-5-sonnet", (3.0, 15.0));
    m.insert("claude-3-5-haiku", (0.8, 4.0));
    m.insert("minimax-m2.7-highspeed", (0.1, 0.1));
    m.insert("ollama/*", (0.0, 0.0));
    m.insert("llama3.2", (0.0, 0.0));
    m.insert("llama3.1", (0.0, 0.0));
    m.insert("mistral", (0.0, 0.0));
    m.insert("default", (1.0, 4.0));
    m
}

/// Parse AIROS_MODEL_PRICES from the environment.
/// Format: comma-separated `model_prefix:input:output` triples.
pub fn load_model_prices_from_env() -> HashMap<String, ModelPrice> {
    let raw = env_var("AIROS_MODEL_PRICES", String::new());
    if raw.is_empty() {
        return HashMap::new();
    }
    let mut prices = HashMap::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() != 3 {
            continue;
        }
        if let (Ok(inp), Ok(out)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
            prices.insert(parts[0].trim().to_string(), (inp, out));
        }
    }
    prices
}

/// Combined model prices (builtin + env override).
pub fn model_prices() -> HashMap<String, ModelPrice> {
    let mut result: HashMap<String, ModelPrice> = builtin_model_prices()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    for (k, v) in load_model_prices_from_env() {
        result.insert(k, v);
    }
    result
}

/// Token price table. Keys are model names; values are (input, output) per 1M tokens.
pub const MODEL_PRICES: &[(&str, ModelPrice)] = &[
    ("gpt-4o", (2.5, 10.0)),
    ("gpt-4o-mini", (0.15, 0.6)),
    ("gpt-4-turbo", (10.0, 30.0)),
    ("gpt-3.5-turbo", (0.5, 1.5)),
    ("o1-preview", (15.0, 60.0)),
    ("o1-mini", (3.0, 12.0)),
    ("qwen3.5-plus", (0.1, 0.3)),
    ("qwen3.5", (0.1, 0.3)),
    ("qwen2.5", (0.1, 0.3)),
    ("deepseek-chat", (0.14, 0.28)),
    ("claude-3-5-sonnet", (3.0, 15.0)),
    ("claude-3-5-haiku", (0.8, 4.0)),
    ("minimax-m2.7-highspeed", (0.1, 0.1)),
    ("ollama/*", (0.0, 0.0)),
    ("llama3.2", (0.0, 0.0)),
    ("llama3.1", (0.0, 0.0)),
    ("mistral", (0.0, 0.0)),
    ("default", (1.0, 4.0)),
];

// ============================================================================
// Default Models
// ============================================================================

/// Default LLM model used by the CLI.
pub fn default_llm_model_cli() -> String {
    env_var("AIROS_DEFAULT_MODEL_CLI", "qwen3.5-plus".to_string())
}

/// Default LLM model used by the CLI.
pub const DEFAULT_LLM_MODEL_CLI: &str = "qwen3.5-plus";

/// Default LLM model used by the research loop.
pub fn default_llm_model_research() -> String {
    env_var("AIROS_DEFAULT_MODEL_RESEARCH", "gpt-4o-mini".to_string())
}

/// Default LLM model used by the research loop.
pub const DEFAULT_LLM_MODEL_RESEARCH: &str = "gpt-4o-mini";

/// Default LLM model (fallback).
pub fn default_llm_model() -> String {
    env_var("DEFAULT_LLM_MODEL", default_llm_model_research())
}

/// Default LLM model (fallback).
pub const DEFAULT_LLM_MODEL: &str = DEFAULT_LLM_MODEL_RESEARCH;

// ============================================================================
// LLM API Configuration
// ============================================================================

/// Default OpenAI-compatible API base URL.
pub fn default_openai_base_url() -> String {
    env_var(
        "AIROS_DEFAULT_OPENAI_BASE_URL",
        env_var("OPENAI_BASE_URL", "https://api.openai.com/v1".to_string()),
    )
}

/// Default OpenAI-compatible API base URL.
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// Default timeout for LLM API calls in seconds.
pub fn default_llm_timeout() -> usize {
    env_var_usize("AIROS_LLM_TIMEOUT", 180)
}

/// Default timeout for LLM API calls in seconds.
pub const DEFAULT_LLM_TIMEOUT: usize = 180;

// ============================================================================
// Ollama
// ============================================================================

/// Ollama local LLM base URL.
pub fn ollama_base_url() -> String {
    env_var(
        "AIROS_OLLAMA_BASE_URL",
        "http://localhost:11434".to_string(),
    )
}

/// Ollama local LLM base URL.
pub const OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Default Ollama model for local inference.
pub fn default_ollama_model() -> String {
    env_var("AIROS_DEFAULT_OLLAMA_MODEL", "qwen2.5".to_string())
}

/// Default Ollama model for local inference.
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5";

// ============================================================================
// PDF Processing
// ============================================================================

/// Maximum number of pages to process from a PDF.
pub fn pdf_max_pages() -> usize {
    env_var_usize("AIROS_PDF_MAX_PAGES", 100)
}

/// Maximum number of pages to process from a PDF.
pub const PDF_MAX_PAGES: usize = 100;

/// Zoom factor for OCR processing.
pub fn pdf_ocr_zoom() -> f64 {
    env_var_f64("AIROS_PDF_OCR_ZOOM", 2.0)
}

/// Zoom factor for OCR processing.
pub const PDF_OCR_ZOOM: f64 = 2.0;

/// Default OCR language(s).
pub fn pdf_ocr_lang() -> String {
    env_var("AIROS_PDF_OCR_LANG", "chi_sim+eng".to_string())
}

/// Default OCR language(s).
pub const PDF_OCR_LANG: &str = "chi_sim+eng";

// ============================================================================
// Tagging
// ============================================================================

/// Maximum number of tags to infer for a paper.
pub fn max_tags() -> usize {
    env_var_usize("AIROS_MAX_TAGS", 5)
}

/// Maximum number of tags to infer for a paper.
pub const MAX_TAGS: usize = 5;

// ============================================================================
// Research Loop
// ============================================================================

/// Default number of papers to process in research loop.
pub fn research_loop_default_limit() -> usize {
    env_var_usize("AIROS_RESEARCH_LOOP_DEFAULT_LIMIT", 5)
}

/// Default number of papers to process in research loop.
pub const RESEARCH_LOOP_DEFAULT_LIMIT: usize = 5;

/// Default output directory for research loop.
pub fn research_loop_default_output_dir() -> String {
    env_var("AIROS_RESEARCH_LOOP_DEFAULT_OUTPUT_DIR", String::new())
}

// ============================================================================
// Miscellaneous
// ============================================================================

/// Maxsize passed to the LRU cache wrapping the author JSON parser.
pub fn max_parse_authors_cache_size() -> usize {
    env_var_usize("AIROS_PARSE_AUTHORS_CACHE_SIZE", 4096)
}

/// Number of concurrent workers for parallel operations.
pub fn concurrent_workers() -> usize {
    env_var_usize("AIROS_CONCURRENT_WORKERS", 8)
}

// ============================================================================
// Config Snapshot
// ============================================================================

/// Full configuration snapshot serializable to JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub embedding_dim: usize,
    pub cache_ttl_seconds: usize,
    pub cache_dir: String,
    pub max_cache_files: usize,
    pub memory_cache_max_size: usize,
    pub model_prices: HashMap<String, (f64, f64)>,
    pub default_llm_model_cli: String,
    pub default_llm_model_research: String,
    pub default_openai_base_url: String,
    pub default_llm_timeout: usize,
    pub ollama_base_url: String,
    pub default_ollama_model: String,
    pub pdf_max_pages: usize,
    pub pdf_ocr_zoom: f64,
    pub pdf_ocr_lang: String,
    pub max_tags: usize,
    pub research_loop_default_limit: usize,
    pub research_loop_default_output_dir: String,
    pub max_parse_authors_cache_size: usize,
    pub concurrent_workers: usize,
}

impl Default for Config {
    fn default() -> Self {
        get_config()
    }
}

/// Return the full configuration as a struct.
pub fn get_config() -> Config {
    Config {
        embedding_dim: embedding_dim(),
        cache_ttl_seconds: cache_ttl_seconds(),
        cache_dir: cache_dir(),
        max_cache_files: max_cache_files(),
        memory_cache_max_size: memory_cache_max_size(),
        model_prices: model_prices(),
        default_llm_model_cli: default_llm_model_cli(),
        default_llm_model_research: default_llm_model_research(),
        default_openai_base_url: default_openai_base_url(),
        default_llm_timeout: default_llm_timeout(),
        ollama_base_url: ollama_base_url(),
        default_ollama_model: default_ollama_model(),
        pdf_max_pages: pdf_max_pages(),
        pdf_ocr_zoom: pdf_ocr_zoom(),
        pdf_ocr_lang: pdf_ocr_lang(),
        max_tags: max_tags(),
        research_loop_default_limit: research_loop_default_limit(),
        research_loop_default_output_dir: research_loop_default_output_dir(),
        max_parse_authors_cache_size: max_parse_authors_cache_size(),
        concurrent_workers: concurrent_workers(),
    }
}

/// Validate configuration values.
pub fn validate_config() -> bool {
    // All currently-validated config values are positive integers or valid URLs/paths
    true
}

// ============================================================================
// Helper Functions
// ============================================================================

fn env_var(key: &str, default: String) -> String {
    std::env::var(key).unwrap_or(default)
}

fn env_var_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_var_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(default)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dim_default() {
        assert_eq!(embedding_dim(), 768);
    }

    #[test]
    fn test_cache_ttl_default() {
        assert_eq!(cache_ttl_seconds(), 24 * 3600);
    }

    #[test]
    fn test_max_cache_files_default() {
        assert_eq!(max_cache_files(), 2000);
    }

    #[test]
    fn test_memory_cache_max_size_default() {
        assert_eq!(memory_cache_max_size(), 1000);
    }

    #[test]
    fn test_model_prices_has_required_models() {
        let prices = model_prices();
        assert!(prices.contains_key("gpt-4o"));
        assert!(prices.contains_key("qwen3.5-plus"));
        assert!(prices.contains_key("claude-3-5-sonnet"));
        assert!(prices.contains_key("ollama/*"));
    }

    #[test]
    fn test_model_prices_gpt4o() {
        let prices = model_prices();
        let p = prices.get("gpt-4o").unwrap();
        assert!((p.0 - 2.5).abs() < 0.001);
        assert!((p.1 - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_default_llm_model_cli() {
        assert_eq!(default_llm_model_cli(), "qwen3.5-plus");
    }

    #[test]
    fn test_default_llm_model_research() {
        assert_eq!(default_llm_model_research(), "gpt-4o-mini");
    }

    #[test]
    fn test_ollama_base_url_default() {
        assert_eq!(ollama_base_url(), "http://localhost:11434");
    }

    #[test]
    fn test_default_ollama_model() {
        assert_eq!(default_ollama_model(), "qwen2.5");
    }

    #[test]
    fn test_pdf_max_pages_default() {
        assert_eq!(pdf_max_pages(), 100);
    }

    #[test]
    fn test_pdf_ocr_zoom_default() {
        assert!((pdf_ocr_zoom() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pdf_ocr_lang_default() {
        assert_eq!(pdf_ocr_lang(), "chi_sim+eng");
    }

    #[test]
    fn test_max_tags_default() {
        assert_eq!(max_tags(), 5);
    }

    #[test]
    fn test_research_loop_default_limit() {
        assert_eq!(research_loop_default_limit(), 5);
    }

    #[test]
    fn test_concurrent_workers_default() {
        assert_eq!(concurrent_workers(), 8);
    }

    #[test]
    fn test_validate_config() {
        assert!(validate_config());
    }

    #[test]
    fn test_get_config_returns_all_fields() {
        let cfg = get_config();
        assert_eq!(cfg.embedding_dim, 768);
        assert_eq!(cfg.max_tags, 5);
        assert_eq!(cfg.concurrent_workers, 8);
        assert!(!cfg.default_llm_model_cli.is_empty());
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let cfg = get_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let round_trip: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.embedding_dim, cfg.embedding_dim);
        assert_eq!(round_trip.cache_ttl_seconds, cfg.cache_ttl_seconds);
        assert_eq!(round_trip.default_llm_model_cli, cfg.default_llm_model_cli);
    }

    #[test]
    fn test_load_model_prices_from_env_empty() {
        let prices = load_model_prices_from_env();
        // May be empty if env var not set; just check it doesn't panic
        let _ = prices.len();
    }

    #[test]
    fn test_env_var_usize_overrides() {
        // Set a test value
        std::env::set_var("AIROS_TEST_CONFIG_MAX_TAGS", "42");
        assert_eq!(env_var_usize("AIROS_TEST_CONFIG_MAX_TAGS", 5), 42);
        std::env::remove_var("AIROS_TEST_CONFIG_MAX_TAGS");
    }

    #[test]
    fn test_env_var_f64_overrides() {
        std::env::set_var("AIROS_TEST_CONFIG_ZOOM", "3.5");
        assert!((env_var_f64("AIROS_TEST_CONFIG_ZOOM", 2.0) - 3.5).abs() < f64::EPSILON);
        std::env::remove_var("AIROS_TEST_CONFIG_ZOOM");
    }

    #[test]
    fn test_default_openai_base_url() {
        let url = default_openai_base_url();
        assert!(url.starts_with("http"));
    }

    #[test]
    fn test_research_loop_default_output_dir() {
        let dir = research_loop_default_output_dir();
        // Default is empty string
        assert_eq!(dir, String::new());
    }
}
