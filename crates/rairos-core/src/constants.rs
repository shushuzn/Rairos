use std::collections::HashSet;
use std::sync::LazyLock;

pub const LLM_BASE_URL: &str = "https://api.openai.com/v1";

pub const CAPSULE_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
pub const ARXIV_API: &str = "https://export.arxiv.org/api/query";
pub const GP_DIR_NAME: &str = ".ai_research_os/evolution";
pub const CLIMATE_CATS: &[&str] = &["cs.AI", "cs.LG", "cs.ET", "physics.ao-ph", "atm.ph"];
pub const LLM_MODEL: &str = "gpt-4o-mini";

pub const OLLAMA_BASE_URL: &str = "http://localhost:11434";
pub const OLLAMA_EMBEDDING_MODEL: &str = "nomic-embed-text";
pub const OLLAMA_API_EMBEDDINGS_ENDPOINT: &str = "/api/embeddings";
pub const ENV_AIROS_USE_EMBEDDING: &str = "AIROS_USE_EMBEDDING";

pub const SEMANTIC_API: &str = "https://api.semanticscholar.org/graph/v1";
pub const CROSSREF_WORKS: &str = "https://api.crossref.org/works/{doi}";
pub const DOI_RESOLVER: &str = "https://doi.org/";
pub const RADAR_FILE: &str = "Radar.md";
pub const TIMELINE_FILE: &str = "Timeline.md";

pub static AI_RESEARCH_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("transformer");
    s.insert("attention");
    s.insert("bert");
    s.insert("gpt");
    s.insert("llm");
    s.insert("language model");
    s.insert("neural");
    s.insert("network");
    s.insert("embedding");
    s.insert("fine-tuning");
    s.insert("rlhf");
    s.insert("rag");
    s.insert("retrieval");
    s.insert("generative");
    s.insert("diffusion");
    s.insert("gan");
    s.insert("clip");
    s.insert("vit");
    s.insert("reinforcement");
    s.insert("policy");
    s.insert("reward");
    s.insert("rl");
    s.insert("dpo");
    s.insert("ppo");
    s.insert("reward model");
    s.insert("training");
    s.insert("optimization");
    s.insert("pre-training");
    s.insert("instruction");
    s.insert("alignment");
    s.insert("multimodal");
    s.insert("vision");
    s.insert("language");
    s.insert("speech");
    s.insert("audio");
    s.insert("constitutional");
    s.insert("reasoning");
    s.insert("chain-of-thought");
    s.insert("cot");
    s.insert("synthetic data");
    s.insert("model");
    s.insert("learning");
    s
});

pub static SMART_FOLLOWUP_BASE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("attention");
    s.insert("transformer");
    s.insert("bert");
    s.insert("gpt");
    s.insert("llm");
    s.insert("language model");
    s.insert("neural");
    s.insert("network");
    s.insert("embedding");
    s.insert("fine-tuning");
    s.insert("rlhf");
    s.insert("rag");
    s.insert("retrieval");
    s.insert("generative");
    s.insert("diffusion");
    s.insert("gan");
    s.insert("clip");
    s.insert("vit");
    s.insert("weight");
    s.insert("layer");
    s.insert("parameter");
    s.insert("gradient");
    s.insert("loss");
    s.insert("optimize");
    s.insert("softmax");
    s.insert("matrix");
    s.insert("dot");
    s.insert("product");
    s.insert("mechanism");
    s.insert("reinforcement");
    s.insert("policy");
    s.insert("reward");
    s.insert("rl");
    s.insert("dpo");
    s.insert("ppo");
    s.insert("training");
    s.insert("pre-training");
    s.insert("instruction");
    s.insert("alignment");
    s.insert("multimodal");
    s.insert("vision");
    s.insert("language");
    s.insert("speech");
    s.insert("audio");
    s.insert("constitutional");
    s.insert("reasoning");
    s.insert("chain-of-thought");
    s.insert("cot");
    s.insert("implement");
    s.insert("code");
    s.insert("function");
    s.insert("class");
    s.insert("api");
    s.insert("library");
    s.insert("pytorch");
    s.insert("tensorflow");
    s.insert("module");
    s.insert("algorithm");
    s.insert("vs");
    s.insert("versus");
    s.insert("better");
    s.insert("worse");
    s.insert("compare");
    s.insert("advantage");
    s.insert("disadvantage");
    s.insert("based on");
    s.insert("follow");
    s.insert("extend");
    s.insert("improve");
    s.insert("build upon");
    s.insert("later");
    s.insert("previous");
    s.insert("next");
    s.insert("evolution");
    s.insert("derived");
    s.insert("succeed");
    s.insert("apply");
    s.insert("use");
    s.insert("application");
    s.insert("industry");
    s.insert("practical");
    s.insert("deploy");
    s.insert("production");
    s.insert("real-world");
    s.insert("benchmark");
    s
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_exist() {
        assert_eq!(LLM_BASE_URL, "https://api.openai.com/v1");
        assert_eq!(LLM_MODEL, "gpt-4o-mini");
        assert_eq!(OLLAMA_BASE_URL, "http://localhost:11434");
        assert_eq!(OLLAMA_EMBEDDING_MODEL, "nomic-embed-text");
    }

    #[test]
    fn test_ai_research_keywords_not_empty() {
        assert!(!AI_RESEARCH_KEYWORDS.is_empty());
        assert!(AI_RESEARCH_KEYWORDS.contains("transformer"));
        assert!(AI_RESEARCH_KEYWORDS.contains("llm"));
        assert!(AI_RESEARCH_KEYWORDS.contains("rag"));
    }

    #[test]
    fn test_smart_followup_base_not_empty() {
        assert!(!SMART_FOLLOWUP_BASE.is_empty());
        assert!(SMART_FOLLOWUP_BASE.contains("transformer"));
        assert!(SMART_FOLLOWUP_BASE.contains("attention"));
    }

    #[test]
    fn test_keywords_disjoint_from_smart_followup() {
        let common: Vec<_> = AI_RESEARCH_KEYWORDS
            .iter()
            .filter(|k| SMART_FOLLOWUP_BASE.contains(*k))
            .collect();
        assert!(
            !common.is_empty(),
            "Keywords should have some overlap with SmartFollowup base"
        );
    }
}
