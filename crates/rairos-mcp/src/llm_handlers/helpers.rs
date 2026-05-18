use rairos_llm::{AnthropicClient, LlmClient, OpenAiClient};

pub fn llm_client() -> Option<Box<dyn LlmClient>> {
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        Some(Box::new(OpenAiClient::new(key)))
    } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        Some(Box::new(AnthropicClient::new(key)))
    } else {
        None
    }
}

pub fn llm_model() -> &'static str {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        "claude-sonnet-4-20250514"
    } else {
        "gpt-4o"
    }
}

pub fn gene_pool_data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}
