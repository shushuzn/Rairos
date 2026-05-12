//! rairos-litreview — Literature Review Analyzer
//!
//! Analyzes paper collections for trends, controversies, open problems,
//! and methodological groupings.
//!
//! Ported from `llm/litreview_analyzer.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: Option<String>,
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub published: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    #[serde(rename = "method_evolution")]
    pub method_evolution: Vec<String>,
    #[serde(rename = "temporal_distribution")]
    pub temporal_distribution: HashMap<String, usize>,
    #[serde(rename = "rising_topics")]
    pub rising_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewAnalyzer {
    _private: (),
}

impl Default for LitReviewAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LitReviewAnalyzer {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn analyze_trends(&self, papers: &[Paper]) -> TrendAnalysis {
        if papers.is_empty() {
            return TrendAnalysis {
                method_evolution: Vec::new(),
                temporal_distribution: HashMap::new(),
                rising_topics: Vec::new(),
            };
        }

        let method_evolution = self.analyze_method_evolution(papers);
        let temporal_distribution = self.analyze_temporal_distribution(papers);
        let rising_topics = self.detect_rising_topics(papers);

        TrendAnalysis {
            method_evolution,
            temporal_distribution,
            rising_topics,
        }
    }

    pub fn find_controversies(&self, papers: &[Paper]) -> Vec<String> {
        let controversy_signals: [(&str, &str); 5] = [
            ("outperforms", "while others argue"),
            ("contrary to", "however"),
            ("different from", "unlike"),
            ("challenge", "question"),
            ("limitation of", "we show that"),
        ];

        let mut controversies = Vec::new();

        for paper in papers.iter().take(20) {
            let abstract_lower = paper.abstract_text.as_deref().unwrap_or("").to_lowercase();
            let title = &paper.title;

            for (signal1, signal2) in controversy_signals {
                if abstract_lower.contains(signal1) && abstract_lower.contains(signal2) {
                    let truncated = if title.len() > 40 {
                        format!("_{}..._: 可能在方法选择或结论上存在争议", &title[..40])
                    } else {
                        format!("_{}..._: 可能在方法选择或结论上存在争议", title)
                    };
                    controversies.push(truncated);
                    break;
                }
            }
        }

        controversies.truncate(5);
        controversies
    }

    pub fn extract_open_problems(&self, papers: &[Paper]) -> Vec<String> {
        let signal_phrases: [&str; 10] = [
            "remain an open problem",
            "future work",
            "future research",
            "left for future",
            "beyond the scope",
            "opportunity for future",
            "potential future direction",
            "remain challenging",
            "still needs",
            "requires further",
        ];

        let mut open_problems = Vec::new();

        for paper in papers.iter().take(30) {
            let abstract_text = match paper.abstract_text.as_deref() {
                Some(t) if !t.is_empty() => t,
                _ => continue,
            };

            let title = if paper.title.len() > 40 {
                format!("{}...", &paper.title[..40])
            } else {
                paper.title.clone()
            };

            let abstract_lower = abstract_text.to_lowercase();

            for phrase in signal_phrases {
                if let Some(idx) = abstract_lower.find(phrase) {
                    let start = idx.saturating_sub(30);
                    let end = (idx + 100).min(abstract_text.len());
                    let context = abstract_text[start..end].trim();
                    let cleaned: String = context.split_whitespace().collect::<Vec<_>>().join(" ");

                    if cleaned.len() > 20 {
                        open_problems.push(format!("_{}: ...{}...", title, cleaned));
                        break;
                    }
                }
            }
        }

        open_problems.truncate(8);
        open_problems
    }

    pub fn group_by_methodology<'a>(&self, papers: &'a [Paper]) -> HashMap<String, Vec<&'a Paper>> {
        let method_keywords: HashMap<&str, Vec<&str>> = [
            (
                "Transformer/Attention",
                vec![
                    "transformer",
                    "attention",
                    "self-attention",
                    "bert",
                    "gpt",
                    "vit",
                    "vision transformer",
                    "llama",
                    "decoder-only",
                ],
            ),
            (
                "CNN/卷积网络",
                vec![
                    "convolutional",
                    "cnn",
                    "convolution",
                    "resnet",
                    "vgg",
                    "efficientnet",
                    "mobilenet",
                    "inception",
                ],
            ),
            (
                "图神经网络",
                vec![
                    "graph",
                    "gnn",
                    "gcn",
                    "gat",
                    "graph neural",
                    "message passing",
                    "graph attention",
                ],
            ),
            (
                "强化学习",
                vec![
                    "reinforcement learning",
                    "rl ",
                    "policy gradient",
                    "q-learning",
                    "ddpg",
                    "ppo",
                    "actor-critic",
                    "reward",
                    "environment interaction",
                ],
            ),
            (
                "扩散模型",
                vec![
                    "diffusion",
                    "ddpm",
                    "score-based",
                    "stable diffusion",
                    "ddim",
                    "latent diffusion",
                    "generative model",
                ],
            ),
            (
                "检索增强",
                vec![
                    "retrieval-augmented",
                    "rag",
                    "knowledge retrieval",
                    "retrieval",
                    "dense retrieval",
                    "bm25",
                ],
            ),
            (
                "多模态",
                vec![
                    "multimodal",
                    "vision-language",
                    "image-text",
                    "vqa",
                    "visual question",
                    "cross-modal",
                    "clip",
                    "flamingo",
                ],
            ),
            (
                "对比学习",
                vec![
                    "contrastive learning",
                    "contrastive loss",
                    "simclr",
                    "triplet loss",
                    "infoNCE",
                    "momentum contrast",
                ],
            ),
            (
                "知识蒸馏",
                vec![
                    "knowledge distillation",
                    "distillation",
                    "teacher-student",
                    "model compression",
                    "pruning",
                    "quantization",
                ],
            ),
            (
                "自监督学习",
                vec![
                    "self-supervised",
                    " pretext task",
                    "masked",
                    "BYOL",
                    "SwAV",
                    "momentum encoder",
                ],
            ),
        ]
        .into_iter()
        .collect();

        let mut groups: HashMap<String, Vec<&Paper>> = HashMap::new();
        let mut unclassified: Vec<&Paper> = Vec::new();

        for paper in papers {
            let text = format!(
                "{} {}",
                paper.title.to_lowercase(),
                paper.abstract_text.as_deref().unwrap_or("").to_lowercase()
            );

            let mut matched = false;
            for (method, keywords) in &method_keywords {
                if keywords.iter().any(|kw| text.contains(*kw)) {
                    groups.entry(method.to_string()).or_default().push(paper);
                    matched = true;
                    break;
                }
            }

            if !matched {
                unclassified.push(paper);
            }
        }

        if !unclassified.is_empty() {
            groups.insert("其他/未分类".to_string(), unclassified);
        }

        groups
    }

    fn analyze_method_evolution(&self, papers: &[Paper]) -> Vec<String> {
        let mut sorted_papers: Vec<&Paper> = papers.iter().collect();
        sorted_papers.sort_by(|a, b| b.published.cmp(&a.published));

        let mut recent_methods: Vec<String> = Vec::new();

        for paper in sorted_papers.iter().take(10) {
            let text = format!(
                "{} {}",
                paper.title.to_lowercase(),
                paper.abstract_text.as_deref().unwrap_or("").to_lowercase()
            );

            if (text.contains("transformer") || text.contains("attention"))
                && !recent_methods.contains(&"Transformer架构".to_string())
            {
                recent_methods.push("Transformer架构".to_string());
            }
            if (text.contains("diffusion") || text.contains("ddpm"))
                && !recent_methods.contains(&"扩散模型".to_string())
            {
                recent_methods.push("扩散模型".to_string());
            }
            if (text.contains("retrieval") || text.contains("rag"))
                && !recent_methods.contains(&"检索增强生成".to_string())
            {
                recent_methods.push("检索增强生成".to_string());
            }
            if (text.contains("multimodal") || text.contains("vision-language"))
                && !recent_methods.contains(&"多模态学习".to_string())
            {
                recent_methods.push("多模态学习".to_string());
            }
            if (text.contains("graph") || text.contains("gnn"))
                && !recent_methods.contains(&"图神经网络".to_string())
            {
                recent_methods.push("图神经网络".to_string());
            }
        }

        recent_methods
            .into_iter()
            .map(|m| format!("近期研究重点: {}", m))
            .collect()
    }

    fn analyze_temporal_distribution(&self, papers: &[Paper]) -> HashMap<String, usize> {
        let mut distribution: HashMap<String, usize> = HashMap::new();

        for paper in papers {
            if let Some(published) = &paper.published {
                if published.len() >= 4 {
                    let year = &published[..4];
                    if year.chars().all(|c| c.is_ascii_digit()) {
                        *distribution.entry(year.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut pairs: Vec<(String, usize)> = distribution.into_iter().collect();
        pairs.sort_by(|a, b| b.0.cmp(&a.0));
        pairs.into_iter().collect()
    }

    fn detect_rising_topics(&self, papers: &[Paper]) -> Vec<String> {
        let mut sorted_papers: Vec<&Paper> = papers.iter().collect();
        sorted_papers.sort_by(|a, b| b.published.cmp(&a.published));

        let mut recent_text = String::new();
        for paper in sorted_papers.iter().take(20) {
            recent_text.push_str(&paper.title.to_lowercase());
            recent_text.push(' ');
        }

        let mut rising = Vec::new();

        if recent_text.contains("diffusion") || recent_text.contains("ddpm") {
            rising.push("扩散模型 (Diffusion Models)".to_string());
        }
        if recent_text.contains("llm") || recent_text.contains("large language") {
            rising.push("大语言模型 (LLMs)".to_string());
        }
        if recent_text.contains("multimodal") || recent_text.contains("vision-language") {
            rising.push("多模态学习".to_string());
        }
        if recent_text.contains("retrieval") || recent_text.contains("rag") {
            rising.push("检索增强生成 (RAG)".to_string());
        }
        if recent_text.contains("instruction") && recent_text.contains("tuning") {
            rising.push("指令微调 (Instruction Tuning)".to_string());
        }
        if recent_text.contains("chain-of-thought") || recent_text.contains("cot") {
            rising.push("思维链推理 (Chain-of-Thought)".to_string());
        }
        if recent_text.contains("scaling") && recent_text.contains("law") {
            rising.push("Scaling Laws".to_string());
        }

        rising.truncate(5);
        rising
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(title: &str, abstract_text: &str, published: &str) -> Paper {
        Paper {
            id: None,
            title: title.to_string(),
            abstract_text: Some(abstract_text.to_string()),
            published: Some(published.to_string()),
            score: None,
        }
    }

    #[test]
    fn test_analyze_trends_empty() {
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.analyze_trends(&[]);
        assert!(result.method_evolution.is_empty());
        assert!(result.temporal_distribution.is_empty());
        assert!(result.rising_topics.is_empty());
    }

    #[test]
    fn test_find_controversies_empty() {
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.find_controversies(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_open_problems_empty() {
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.extract_open_problems(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_group_by_methodology_empty() {
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.group_by_methodology(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_analyze_trends_with_papers() {
        let papers = vec![
            make_paper(
                "Transformer Paper",
                "We propose a new transformer architecture with attention",
                "2024-01-01",
            ),
            make_paper(
                "Diffusion Paper",
                "A diffusion model for image generation",
                "2024-06-01",
            ),
        ];
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.analyze_trends(&papers);
        assert!(!result.method_evolution.is_empty());
        assert_eq!(*result.temporal_distribution.get("2024").unwrap(), 2);
    }

    #[test]
    fn test_find_controversies_detected() {
        let papers = vec![make_paper(
            "Paper A",
            "Our method outperforms existing approaches while others argue about baseline selection",
            "2024-01-01",
        )];
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.find_controversies(&papers);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_group_by_methodology_transformer() {
        let papers = vec![make_paper(
            "Attention Is All You Need",
            "We propose the transformer architecture with multi-head attention",
            "2024-01-01",
        )];
        let analyzer = LitReviewAnalyzer::new();
        let groups = analyzer.group_by_methodology(&papers);
        assert!(groups.contains_key("Transformer/Attention"));
    }

    #[test]
    fn test_group_by_methodology_unclassified() {
        let papers = vec![make_paper(
            "Unknown Method Paper",
            "We propose a novel approach",
            "2024-01-01",
        )];
        let analyzer = LitReviewAnalyzer::new();
        let groups = analyzer.group_by_methodology(&papers);
        assert!(groups.contains_key("其他/未分类"));
    }

    #[test]
    fn test_extract_open_problems_detected() {
        let papers = vec![make_paper(
            "Future Work Paper",
            "This remains an open problem for future research",
            "2024-01-01",
        )];
        let analyzer = LitReviewAnalyzer::new();
        let result = analyzer.extract_open_problems(&papers);
        assert!(!result.is_empty());
    }
}
