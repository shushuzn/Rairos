//! rairos-constants — Shared constants for the Rairos AI research OS.
//!
//! Centralized keyword sets and configuration constants used across
//! trend analysis, research sessions, and question validation modules.

pub const OLLAMA_BASE_URL: &str = "http://localhost:11434";
pub const OLLAMA_EMBEDDING_MODEL: &str = "nomic-embed-text";
pub const OLLAMA_API_EMBEDDINGS_ENDPOINT: &str = "/api/embeddings";
pub const ENV_AIROS_USE_EMBEDDING: &str = "AIROS_USE_EMBEDDING";

pub static AI_RESEARCH_KEYWORDS: &[&str] = &[
    "transformer",
    "attention",
    "bert",
    "gpt",
    "llm",
    "language model",
    "neural",
    "network",
    "embedding",
    "fine-tuning",
    "rlhf",
    "rag",
    "retrieval",
    "generative",
    "diffusion",
    "gan",
    "clip",
    "vit",
    "reinforcement",
    "policy",
    "reward",
    "rl",
    "dpo",
    "ppo",
    "reward model",
    "training",
    "optimization",
    "pre-training",
    "instruction",
    "alignment",
    "multimodal",
    "vision",
    "language",
    "speech",
    "audio",
    "constitutional",
    "reasoning",
    "chain-of-thought",
    "cot",
    "synthetic data",
    "model",
    "learning",
];

pub static SMART_FOLLOWUP_BASE: &[&str] = &[
    "attention",
    "transformer",
    "bert",
    "gpt",
    "llm",
    "language model",
    "neural",
    "network",
    "embedding",
    "fine-tuning",
    "rlhf",
    "rag",
    "retrieval",
    "generative",
    "diffusion",
    "gan",
    "clip",
    "vit",
    "weight",
    "layer",
    "parameter",
    "gradient",
    "loss",
    "optimize",
    "softmax",
    "matrix",
    "dot",
    "product",
    "mechanism",
    "reinforcement",
    "policy",
    "reward",
    "rl",
    "dpo",
    "ppo",
    "training",
    "pre-training",
    "instruction",
    "alignment",
    "multimodal",
    "vision",
    "language",
    "speech",
    "audio",
    "constitutional",
    "reasoning",
    "chain-of-thought",
    "cot",
    "implement",
    "code",
    "function",
    "class",
    "api",
    "library",
    "pytorch",
    "tensorflow",
    "module",
    "algorithm",
    "vs",
    "versus",
    "better",
    "worse",
    "compare",
    "advantage",
    "disadvantage",
    "based on",
    "follow",
    "extend",
    "improve",
    "build upon",
    "later",
    "previous",
    "next",
    "evolution",
    "derived",
    "succeed",
    "apply",
    "use",
    "application",
    "industry",
    "practical",
    "deploy",
    "production",
    "real-world",
    "benchmark",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_constants() {
        assert_eq!(OLLAMA_BASE_URL, "http://localhost:11434");
        assert_eq!(OLLAMA_EMBEDDING_MODEL, "nomic-embed-text");
        assert_eq!(OLLAMA_API_EMBEDDINGS_ENDPOINT, "/api/embeddings");
    }

    #[test]
    fn test_ai_research_keywords_not_empty() {
        assert!(!AI_RESEARCH_KEYWORDS.is_empty());
        assert!(AI_RESEARCH_KEYWORDS.contains(&"transformer"));
        assert!(AI_RESEARCH_KEYWORDS.contains(&"llm"));
        assert!(AI_RESEARCH_KEYWORDS.contains(&"reinforcement"));
    }

    #[test]
    fn test_smart_followup_base_not_empty() {
        assert!(!SMART_FOLLOWUP_BASE.is_empty());
        assert!(SMART_FOLLOWUP_BASE.contains(&"attention"));
        assert!(SMART_FOLLOWUP_BASE.contains(&"pytorch"));
    }
}