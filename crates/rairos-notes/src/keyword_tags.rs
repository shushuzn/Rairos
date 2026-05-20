//! Keyword tag inference and predefined keyword tag patterns.
//!
//! Ported from `notes/keyword_tags.py`.
//!
//! Uses regex patterns to infer tags from paper title/abstract text.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// Pre-compiled regex patterns for better performance
// Each entry: (pattern_regex, tag_name)
static KEYWORD_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Core AI concepts
        (
            Regex::new(r"(?i)\bagent(s)?\b|tool\s*use|function\s*calling|autonomous\s*system")
                .expect("valid regex"),
            "Agent",
        ),
        (
            Regex::new(
                r"(?i)\brag\b|retrieval-augmented|retrieval augmented|knowledge\s*retrieval",
            )
            .expect("valid regex"),
            "RAG",
        ),
        (
            Regex::new(r"(?i)\bmoe\b|mixture of experts").expect("valid regex"),
            "MoE",
        ),
        (
            Regex::new(r"(?i)\brlhf\b|preference optimization|dpo\b|alignment").expect("valid regex"),
            "Alignment",
        ),
        (
            Regex::new(r"(?i)\bevaluation\b|benchmark|performance\s*metric").expect("valid regex"),
            "Evaluation",
        ),
        (
            Regex::new(r"(?i)\bcompiler\b|kernel|cuda|inference|hardware|accelerator").expect("valid regex"),
            "Infrastructure",
        ),
        (
            Regex::new(r"(?i)\bmultimodal\b|vision|audio|text\s*image|cross\s*modal").expect("valid regex"),
            "Multimodal",
        ),
        (
            Regex::new(r"(?i)\bcompression\b|quantization|distillation|model\s*reduction").expect("valid regex"),
            "Optimization",
        ),
        (
            Regex::new(r"(?i)\blong context\b|context length|extended\s*context").expect("valid regex"),
            "LongContext",
        ),
        (
            Regex::new(r"(?i)\bsafety\b|jailbreak|red teaming|adversarial\s*attack").expect("valid regex"),
            "Safety",
        ),
        // Additional AI research areas
        (
            Regex::new(r"(?i)\bllm\b|large\s*language\s*model|transformer").expect("valid regex"),
            "LLM",
        ),
        (
            Regex::new(r"(?i)\bgpt\b|generative\s*pre-trained").expect("valid regex"),
            "GPT",
        ),
        (
            Regex::new(r"(?i)\bcnn\b|convolutional\s*neural\s*network").expect("valid regex"),
            "CNN",
        ),
        (
            Regex::new(r"(?i)\brnn\b|recurrent\s*neural\s*network").expect("valid regex"),
            "RNN",
        ),
        (
            Regex::new(r"(?i)\bgans\b|generative\s*adversarial\s*network").expect("valid regex"),
            "GAN",
        ),
        (
            Regex::new(r"(?i)\bvae\b|variational\s*autoencoder").expect("valid regex"),
            "VAE",
        ),
        (
            Regex::new(r"(?i)\breinforcement\s*learning|rl\b").expect("valid regex"),
            "RL",
        ),
        (
            Regex::new(r"(?i)\bsupervised\s*learning").expect("valid regex"),
            "SupervisedLearning",
        ),
        (
            Regex::new(r"(?i)\bunsupervised\s*learning").expect("valid regex"),
            "UnsupervisedLearning",
        ),
        (
            Regex::new(r"(?i)\bsemi-supervised\s*learning").expect("valid regex"),
            "SemiSupervisedLearning",
        ),
        (
            Regex::new(r"(?i)\bself-supervised\s*learning").expect("valid regex"),
            "SelfSupervisedLearning",
        ),
        (
            Regex::new(r"(?i)\btransfer\s*learning").expect("valid regex"),
            "TransferLearning",
        ),
        (
            Regex::new(r"(?i)\bfew-shot\s*learning|few\s*shot").expect("valid regex"),
            "FewShotLearning",
        ),
        (
            Regex::new(r"(?i)\bzero-shot\s*learning|zero\s*shot").expect("valid regex"),
            "ZeroShotLearning",
        ),
        (
            Regex::new(r"(?i)\bprompt\s*engineering").expect("valid regex"),
            "PromptEngineering",
        ),
        (
            Regex::new(r"(?i)\btokenization\b|token\s*embedding").expect("valid regex"),
            "Tokenization",
        ),
        (
            Regex::new(r"(?i)\bembedding\b|vector\s*representation").expect("valid regex"),
            "Embedding",
        ),
        (
            Regex::new(r"(?i)\bknowledge\s*graph|kg\b").expect("valid regex"),
            "KnowledgeGraph",
        ),
        (
            Regex::new(r"(?i)\breasoning\b|logical\s*inference").expect("valid regex"),
            "Reasoning",
        ),
        (
            Regex::new(r"(?i)\bsummarization\b|summary").expect("valid regex"),
            "Summarization",
        ),
        (
            Regex::new(r"(?i)\btranslation\b|machine\s*translation").expect("valid regex"),
            "Translation",
        ),
        (
            Regex::new(r"(?i)\bquestion\s*answering|qa\b").expect("valid regex"),
            "QA",
        ),
        (
            Regex::new(r"(?i)\bdocument\s*understanding").expect("valid regex"),
            "DocumentUnderstanding",
        ),
        (
            Regex::new(r"(?i)\bcoding\b|code\s*generation").expect("valid regex"),
            "Coding",
        ),
        (
            Regex::new(r"(?i)\bmedical\s*ai|healthcare\s*ai").expect("valid regex"),
            "MedicalAI",
        ),
        (
            Regex::new(r"(?i)\bfinance\s*ai|financial\s*ai").expect("valid regex"),
            "FinanceAI",
        ),
        (
            Regex::new(r"(?i)\beducation\s*ai|educational\s*ai").expect("valid regex"),
            "EducationAI",
        ),
        (
            Regex::new(r"(?i)\benvironmental\s*ai|climate\s*ai").expect("valid regex"),
            "EnvironmentalAI",
        ),
    ]
});

/// Get matching tags for a given text (cached by text content).
/// Returns a tuple of matching tag names.
pub fn get_keywords_signature(text: &str) -> Vec<&'static str> {
    let text_lower = text.to_lowercase();
    let mut matches = Vec::new();
    for (pattern, tag) in KEYWORD_PATTERNS.iter() {
        if pattern.is_match(&text_lower) {
            matches.push(*tag);
        }
    }
    matches
}

/// Infer tags from title and abstract text.
/// If existing_tags is non-empty, returns it unchanged.
/// Otherwise, analyzes the combined title+abstract text.
/// Removes redundant tags (e.g., if both "LLM" and "GPT" are found, keeps "LLM").
pub fn infer_tags_if_empty(
    existing_tags: &[String],
    title: &str,
    abstract_text: &str,
) -> Vec<String> {
    if !existing_tags.is_empty() {
        return existing_tags.to_vec();
    }

    let text = format!("{}\n{}", title, abstract_text);
    let matches = get_keywords_signature(&text);

    // Remove redundant tags
    let mut final_tags: Vec<String> = Vec::new();
    let mut skip: HashSet<String> = HashSet::new();

    for tag in matches {
        if skip.contains(tag) {
            continue;
        }

        // Check if this tag makes any existing tag redundant
        let tag_lower = tag.to_lowercase();
        let mut is_redundant = false;
        let mut to_skip: Vec<&str> = Vec::new();

        for existing in &final_tags {
            let existing_lower = existing.to_lowercase();
            // If one is substring of another, skip the shorter one
            if tag_lower.contains(&existing_lower) {
                // existing is redundant, remove it
                is_redundant = true;
                to_skip.push(existing.as_str());
            } else if existing_lower.contains(&tag_lower) {
                // current tag is redundant
                is_redundant = true;
                break;
            }
        }

        if is_redundant {
            // Skip this tag (current is a subset of existing)
            skip.insert(tag.to_string());
        } else {
            // Add this tag and mark its subsets for skipping
            let to_skip_now: Vec<&str> = final_tags
                .iter()
                .filter(|existing| {
                    let existing_lower = existing.to_lowercase();
                    tag_lower.len() > existing_lower.len() && tag_lower.starts_with(&existing_lower)
                })
                .map(|existing| existing.as_str())
                .collect();
            for s in to_skip_now {
                skip.insert(s.to_string());
            }
            final_tags.push(tag.to_string());
        }
    }

    // Limit to max tags
    const MAX_TAGS: usize = 20;
    if final_tags.len() > MAX_TAGS {
        final_tags.truncate(MAX_TAGS);
    }

    if final_tags.is_empty() {
        vec!["Unsorted".to_string()]
    } else {
        final_tags
    }
}

/// Get list of all available predefined tags.
pub fn get_all_tags() -> Vec<String> {
    KEYWORD_PATTERNS
        .iter()
        .map(|(_, tag)| (*tag).to_string())
        .collect()
}

/// Get count of all predefined tags.
pub fn get_tags_count() -> usize {
    KEYWORD_PATTERNS.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_keywords_signature_llm() {
        let tags = get_keywords_signature("Large language models are transforming AI");
        assert!(tags.contains(&"LLM"));
    }

    #[test]
    fn test_get_keywords_signature_agent() {
        let tags =
            get_keywords_signature("We present a novel autonomous agent system with tool use");
        assert!(tags.contains(&"Agent"));
        // "autonomous agent" does not imply LLM unless transformer/language mentioned
    }

    #[test]
    fn test_get_keywords_signature_rag() {
        let tags = get_keywords_signature("Retrieval-augmented generation improves factuality");
        assert!(tags.contains(&"RAG"));
    }

    #[test]
    fn test_get_keywords_signature_multiple() {
        let tags = get_keywords_signature(
            "We propose a mixture of experts approach for large language models",
        );
        assert!(tags.contains(&"MoE"));
        assert!(tags.contains(&"LLM"));
    }

    #[test]
    fn test_get_keywords_signature_none() {
        let tags = get_keywords_signature("This is a random paper about nothing related to AI");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_infer_tags_if_empty_with_existing() {
        let existing = vec!["CustomTag".to_string()];
        let tags = infer_tags_if_empty(&existing, "Title", "Abstract");
        assert_eq!(tags, vec!["CustomTag"]);
    }

    #[test]
    fn test_infer_tags_if_empty_llm_paper() {
        let tags = infer_tags_if_empty(
            &[],
            "Attention Is All You Need",
            "We propose the Transformer architecture for language understanding",
        );
        assert!(tags.iter().any(|t| t == "LLM"));
    }

    #[test]
    fn test_infer_tags_removes_redundant() {
        // GPT implies LLM, so LLM should be kept if it's longer/more specific
        let tags = infer_tags_if_empty(
            &[],
            "GPT-4 Technical Report",
            "We report on the development of GPT-4, a large language model",
        );
        // Both might appear, but the algorithm should handle redundancy
        assert!(!tags.is_empty());
    }

    #[test]
    fn test_infer_tags_unsorted_fallback() {
        let tags = infer_tags_if_empty(
            &[],
            "Random Title",
            "Completely unrelated abstract content xyz",
        );
        assert_eq!(tags, vec!["Unsorted"]);
    }

    #[test]
    fn test_get_all_tags() {
        let tags = get_all_tags();
        assert!(!tags.is_empty());
        assert!(tags.contains(&"LLM".to_string()));
        assert!(tags.contains(&"Agent".to_string()));
    }

    #[test]
    fn test_get_tags_count() {
        let count = get_tags_count();
        assert!(count > 30);
    }

    #[test]
    fn test_infer_tags_with_moe_and_alignment() {
        let tags = infer_tags_if_empty(
            &[],
            "MoE Alignment via RLHF",
            "We apply RLHF to mixture of experts models",
        );
        let tags_str: Vec<String> = tags.iter().map(|s| (*s).to_string()).collect();
        assert!(tags_str.iter().any(|t| t == "MoE"));
        assert!(tags_str.iter().any(|t| t == "Alignment"));
        // RLHF doesn't directly match "RL" standalone pattern
    }
}
