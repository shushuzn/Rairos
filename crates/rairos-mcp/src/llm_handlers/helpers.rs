use rairos_llm::{AnthropicClient, LlmClient, OpenAiClient};
use std::sync::OnceLock;

pub fn llm_client() -> Option<&'static dyn LlmClient> {
    static CLIENT: OnceLock<Option<Box<dyn LlmClient>>> = OnceLock::new();
    CLIENT.get_or_init(|| {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            Some(Box::new(OpenAiClient::new(key)))
        } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            Some(Box::new(AnthropicClient::new(key)))
        } else {
            None
        }
    }).as_deref()
}

pub fn llm_model() -> &'static str {
    static MODEL: OnceLock<String> = OnceLock::new();
    MODEL.get_or_init(|| {
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "claude-sonnet-4-20250514".to_string()
        } else {
            "gpt-4o".to_string()
        }
    })
}

pub fn gene_pool_data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}
