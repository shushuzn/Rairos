//! rairos-litreview-analyzer — Analyze papers for trends and insights.
//!
//! Ported from `llm/litreview_analyzer.py`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: Option<String>,
    #[serde(rename = "arxiv_id")]
    pub arxiv_id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub published: Option<String>,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub categories: Vec<String>,
}

impl Paper {
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }

    pub fn abstract_text(&self) -> &str {
        self.abstract_text.as_deref().unwrap_or("")
    }

    pub fn published(&self) -> &str {
        self.published.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    #[serde(default)]
    pub method_evolution: Vec<String>,
    #[serde(default)]
    pub temporal_distribution: HashMap<String, i32>,
    #[serde(default)]
    pub rising_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewAnalyzer {
    #[serde(skip)]
    pub db: Option<()>,
}

impl LitReviewAnalyzer {
    pub fn new() -> Self {
        Self { db: None }
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
        let temporal = self.analyze_temporal_distribution(papers);
        let rising = self.detect_rising_topics(papers);

        TrendAnalysis {
            method_evolution,
            temporal_distribution: temporal,
            rising_topics: rising,
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
            let abstract_lower = paper.abstract_text().to_lowercase();
            let title = paper.title();

            for (signal1, signal2) in &controversy_signals {
                if abstract_lower.contains(signal1) && abstract_lower.contains(signal2) {
                    let short_title = &title[..title.len().min(40)];
                    controversies.push(format!(
                        "_{}..._: 可能在方法选择或结论上存在争议",
                        short_title
                    ));
                    break;
                }
            }
        }

        controversies.truncate(5);
        controversies
    }

    pub fn extract_open_problems(&self, papers: &[Paper]) -> Vec<String> {
        let signal_phrases = [
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
            let abstract_text = paper.abstract_text();
            if abstract_text.is_empty() {
                continue;
            }

            let title = &paper.title()[..paper.title().len().min(40)];
            let abstract_lower = abstract_text.to_lowercase();

            for phrase in &signal_phrases {
                if let Some(idx) = abstract_lower.find(phrase) {
                    let start = idx.saturating_sub(30);
                    let end = (idx + 100).min(abstract_text.len());
                    let context = abstract_text[start..end].trim();
                    let context = context.split_whitespace().collect::<Vec<_>>().join(" ");

                    if context.len() > 20 {
                        open_problems.push(format!("_{}..._: ...{}...", title, context));
                        break;
                    }
                }
            }
        }

        open_problems.truncate(8);
        open_problems
    }

    pub fn group_by_methodology(&self, papers: &[Paper]) -> HashMap<String, Vec<Paper>> {
        let method_keywords: HashMap<&str, Vec<&str>> = HashMap::from([
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
                    "BM25",
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
        ]);

        let mut groups: HashMap<String, Vec<Paper>> = HashMap::new();
        let mut unclassified = Vec::new();

        for paper in papers {
            let text = format!(
                "{} {}",
                paper.title().to_lowercase(),
                paper.abstract_text().to_lowercase()
            );

            let mut matched = false;
            for (method, keywords) in &method_keywords {
                if keywords.iter().any(|kw| text.contains(*kw)) {
                    groups
                        .entry((*method).to_string())
                        .or_default()
                        .push(paper.clone());
                    matched = true;
                    break;
                }
            }

            if !matched {
                unclassified.push(paper.clone());
            }
        }

        if !unclassified.is_empty() {
            groups.insert("其他/未分类".to_string(), unclassified);
        }

        groups
    }

    fn analyze_method_evolution(&self, papers: &[Paper]) -> Vec<String> {
        let mut sorted_papers = papers.to_vec();
        sorted_papers.sort_by(|a, b| b.published().cmp(a.published()));

        let mut recent_methods: HashSet<String> = HashSet::new();

        for paper in sorted_papers.iter().take(10) {
            let text = format!(
                "{} {}",
                paper.title().to_lowercase(),
                paper.abstract_text().to_lowercase()
            );

            if text.contains("transformer") || text.contains("attention") {
                recent_methods.insert("Transformer架构".to_string());
            }
            if text.contains("diffusion") || text.contains("ddpm") {
                recent_methods.insert("扩散模型".to_string());
            }
            if text.contains("retrieval") || text.contains("rag") {
                recent_methods.insert("检索增强生成".to_string());
            }
            if text.contains("multimodal") || text.contains("vision-language") {
                recent_methods.insert("多模态学习".to_string());
            }
            if text.contains("graph") || text.contains("gnn") {
                recent_methods.insert("图神经网络".to_string());
            }
        }

        recent_methods
            .iter()
            .map(|m| format!("近期研究重点: {}", m))
            .collect()
    }

    fn analyze_temporal_distribution(&self, papers: &[Paper]) -> HashMap<String, i32> {
        let mut distribution: HashMap<String, i32> = HashMap::new();

        for paper in papers {
            let published = paper.published();
            if published.len() >= 4 {
                let year = &published[..4];
                if year.chars().all(|c| c.is_ascii_digit()) {
                    *distribution.entry(year.to_string()).or_insert(0) += 1;
                }
            }
        }

        let mut sorted: Vec<_> = distribution.into_iter().collect();
        sorted.sort_by(|a, b| b.0.cmp(&a.0));
        sorted.into_iter().collect()
    }

    fn detect_rising_topics(&self, papers: &[Paper]) -> Vec<String> {
        let mut sorted_papers = papers.to_vec();
        sorted_papers.sort_by(|a, b| b.published().cmp(a.published()));

        let mut recent_text = String::new();
        for paper in sorted_papers.iter().take(20) {
            recent_text.push_str(&paper.title().to_lowercase());
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

impl Default for LitReviewAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(title: &str, abstract_text: &str, published: &str, score: f64) -> Paper {
        Paper {
            id: None,
            arxiv_id: None,
            title: Some(title.to_string()),
            abstract_text: Some(abstract_text.to_string()),
            published: Some(published.to_string()),
            score,
            categories: vec![],
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
    fn test_find_controversies() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![make_paper(
            "A New Method",
            "Our method outperforms previous methods while others argue about its limitations",
            "2024-01-01",
            0.8,
        )];
        let controversies = analyzer.find_controversies(&papers);
        assert!(!controversies.is_empty());
    }

    #[test]
    fn test_extract_open_problems() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![make_paper(
            "Future Directions",
            "This remains an open problem for future work in the field of AI",
            "2024-01-01",
            0.7,
        )];
        let problems = analyzer.extract_open_problems(&papers);
        assert!(!problems.is_empty());
    }

    #[test]
    fn test_group_by_methodology_transformer() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![make_paper(
            "Transformer Paper",
            "We propose a new transformer architecture with self-attention mechanism",
            "2024-01-01",
            0.9,
        )];
        let groups = analyzer.group_by_methodology(&papers);
        assert!(groups.contains_key("Transformer/Attention"));
    }

    #[test]
    fn test_group_by_methodology_unclassified() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![make_paper(
            "Generic Paper",
            "This is a paper about something entirely different",
            "2024-01-01",
            0.5,
        )];
        let groups = analyzer.group_by_methodology(&papers);
        assert!(groups.contains_key("其他/未分类"));
    }

    #[test]
    fn test_detect_rising_topics() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![
            make_paper(
                "Diffusion Model",
                "A new diffusion approach",
                "2024-01-01",
                0.9,
            ),
            make_paper(
                "LLM Paper",
                "Large language model advances",
                "2024-01-15",
                0.85,
            ),
        ];
        let rising = analyzer.detect_rising_topics(&papers);
        assert!(!rising.is_empty());
    }

    #[test]
    fn test_temporal_distribution() {
        let analyzer = LitReviewAnalyzer::new();
        let papers = vec![
            make_paper("Paper 1", "Abstract 1", "2023-06-15", 0.8),
            make_paper("Paper 2", "Abstract 2", "2023-06-20", 0.7),
            make_paper("Paper 3", "Abstract 3", "2024-01-10", 0.9),
        ];
        let dist = analyzer.analyze_temporal_distribution(&papers);
        assert_eq!(dist.get("2023").copied(), Some(2));
        assert_eq!(dist.get("2024").copied(), Some(1));
    }
}
