#![allow(dead_code)]
#![allow(
    clippy::manual_clamp,
)]
pub use rairos_core::constants::{LLM_BASE_URL, LLM_MODEL};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub use rairos_core::constants::OLLAMA_BASE_URL;
pub use rairos_core::constants::OLLAMA_EMBEDDING_MODEL;
pub use rairos_core::constants::OLLAMA_API_EMBEDDINGS_ENDPOINT;

const EMBEDDING_CACHE_MAX: usize = 1000;
const RETRIEVAL_CACHE_MAX: usize = 500;
const RETRIEVAL_CACHE_TTL: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryType {
    Factual,
    Conceptual,
    Comparative,
    Temporal,
    General,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryType::Factual => "factual",
            QueryType::Conceptual => "conceptual",
            QueryType::Comparative => "comparative",
            QueryType::Temporal => "temporal",
            QueryType::General => "general",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatContext {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone)]
pub struct Citation {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f32,
    pub section: String,
    pub char_start: i32,
    pub char_end: i32,
    pub quote: String,
}

#[derive(Debug, Clone)]
pub struct ConfidenceScore {
    pub score: i32,
    pub papers_count: i32,
    pub coverage: String,
    pub warnings: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CrossPaperInsight {
    pub insight_type: String,
    pub summary: String,
    pub papers: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub papers_used: Vec<String>,
    pub session_id: Option<String>,
    pub resolved_context: Option<HashMap<String, String>>,
    pub probing_questions: Vec<String>,
    pub confidence: Option<ConfidenceScore>,
    pub cross_paper_insights: Vec<CrossPaperInsight>,
}

pub struct RagChat {
    model: String,
    base_url: String,
    api_key: String,
}

impl RagChat {
    pub fn new(model: Option<String>, base_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| LLM_MODEL.to_string()),
            base_url: base_url.unwrap_or_else(|| LLM_BASE_URL.to_string()),
            api_key: api_key.unwrap_or_default(),
        }
    }

    pub fn classify_query(&self, query: &str) -> QueryType {
        let factual_patterns = [
            r"(?i)\b(who|whom|whose|who\'s)\b",
            r"(?i)\b(when|what year|what date)\b",
            r"(?i)\b(which (paper|author|model))\b",
            r"(?i)\b(who proposed|who introduced|who published|who wrote|who created)\b",
            r"(?i)\b(where (published|presented|introduced|released))\b",
            r"(?i)\b(what organization|what institution|what company)\b",
            r"(是|谁|哪篇|哪个作者|何时)",
        ];
        let conceptual_patterns = [
            r"(?i)\b(what is|what are|explain|describe|how does|how do|why does|why do|understand|definition|meaning)\b",
            r"(原理|机制|概念|解释|是什么|如何|为什么|理解|定义)",
        ];
        let comparative_patterns = [
            r"(?i)\b(vs|versus|compared to|compared with)\b",
            r"(?i)\b(difference between|differences between)\b",
            r"(?i)\b(compare|comparison)\b",
            r"(?i)\b(which is better|which is worse|which is stronger)\b",
            r"(?i)\b(pros and cons|pros/cons|strengths? and weaknesses?)\b",
            r"(和.*比较|比较.*和|对比|区别|差异)",
        ];
        let temporal_patterns = [
            r"(?i)\b(recent|latest|newest|recently)\b",
            r"(?i)\b(202[0-9]|20[2-9]\d)\b",
            r"(?i)\b(published in|released in|presented in|from 20)\b",
            r"(?i)\b(evolution|development|history|progress)\b",
            r"(?i)\b(before|after|since|until|past|future)\b",
        ];

        let mut scores: HashMap<String, i32> = HashMap::new();
        scores.insert("factual".to_string(), 0);
        scores.insert("conceptual".to_string(), 0);
        scores.insert("comparative".to_string(), 0);
        scores.insert("temporal".to_string(), 0);

        for pattern in factual_patterns {
            if Regex::new(pattern).is_ok_and(|r| r.is_match(query)) {
                *scores.get_mut("factual").unwrap() += 1;
            }
        }
        for pattern in conceptual_patterns {
            if Regex::new(pattern).is_ok_and(|r| r.is_match(query)) {
                *scores.get_mut("conceptual").unwrap() += 1;
            }
        }
        for pattern in comparative_patterns {
            if Regex::new(pattern).is_ok_and(|r| r.is_match(query)) {
                *scores.get_mut("comparative").unwrap() += 1;
            }
        }
        for pattern in temporal_patterns {
            if Regex::new(pattern).is_ok_and(|r| r.is_match(query)) {
                *scores.get_mut("temporal").unwrap() += 1;
            }
        }

        let max_score = *scores.values().max().unwrap_or(&0);
        if max_score == 0 {
            return QueryType::General;
        }

        scores
            .into_iter()
            .find(|(_, s)| *s == max_score)
            .map(|(k, _)| match k.as_str() {
                "factual" => QueryType::Factual,
                "conceptual" => QueryType::Conceptual,
                "comparative" => QueryType::Comparative,
                "temporal" => QueryType::Temporal,
                _ => QueryType::General,
            })
            .unwrap_or(QueryType::General)
    }

    pub fn extract_topic(&self, text: &str) -> Option<String> {
        let clean_patterns = [
            r"(?i)(是什么|什么是|请问|帮我|找找|解释|说明|介绍)",
            r"(?i)(what is|what are|explain|describe|introduce)",
        ];
        let mut cleaned = text.to_string();
        for p in clean_patterns {
            cleaned =
                Regex::new(p).map_or(cleaned.clone(), |r| r.replace_all(&cleaned, "").to_string());
        }

        let stop_words = ["的", "了", "是", "在", "和", "the", "a", "an", "is", "are"];

        for word in cleaned.split_whitespace() {
            let word_trimmed = word.trim();
            if word_trimmed.len() >= 2
                && word_trimmed.len() <= 15
                && !stop_words.contains(&word_trimmed)
            {
                return Some(word_trimmed.chars().take(20).collect());
            }
        }
        None
    }

    pub fn detect_section(&self, text: &str, pos: usize) -> String {
        if text.is_empty() || pos > text.len() {
            return String::new();
        }

        let before_pos = &text[..pos.min(text.len())];
        let section_patterns = [
            (r"(?i)\babstract\b", "Abstract"),
            (r"(?i)\bintroduction\b", "Introduction"),
            (r"(?i)\brelated work\b", "Related Work"),
            (r"(?i)\bbackground\b", "Background"),
            (r"(?i)\bpreliminaries\b", "Preliminaries"),
            (r"(?i)\bmethod\b", "Method"),
            (r"(?i)\bmethodology\b", "Methodology"),
            (r"(?i)\bmodel\b", "Model"),
            (r"(?i)\bexperiments?\b", "Experiments"),
            (r"(?i)\bresults?\b", "Results"),
            (r"(?i)\bevaluation\b", "Evaluation"),
            (r"(?i)\bdiscussion\b", "Discussion"),
            (r"(?i)\bconclusion\b", "Conclusion"),
            (r"(?i)\breferences?\b", "References"),
        ];

        for (pattern, name) in section_patterns {
            if Regex::new(pattern).is_ok_and(|r| r.is_match(before_pos)) {
                return name.to_string();
            }
        }
        String::new()
    }

    pub fn extract_snippet(
        &self,
        text: &str,
        query: &str,
        context_chars: usize,
    ) -> (String, String, i32, i32) {
        if text.is_empty() || query.is_empty() {
            let end = context_chars.min(text.len());
            return (
                text[..end].to_string(),
                self.detect_section(text, 0),
                0,
                end as i32,
            );
        }

        let query_terms: Vec<&str> = query.split_whitespace().filter(|t| t.len() >= 3).collect();
        let text_lower = text.to_lowercase();

        let mut best_pos = None;
        for term in &query_terms {
            if let Some(pos) = text_lower.find(term) {
                best_pos = Some(pos);
                break;
            }
        }

        let best_pos = match best_pos {
            Some(p) => p,
            None => {
                let end = context_chars.min(text.len());
                return (
                    text[..end].to_string(),
                    self.detect_section(text, 0),
                    0,
                    end as i32,
                );
            }
        };

        let start = best_pos.saturating_sub(context_chars / 2);
        let end = (best_pos + context_chars / 2).min(text.len());

        let snippet = text[start..end].trim().to_string();
        let section = self.detect_section(text, start);
        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text.len() { "..." } else { "" };

        (
            format!("{}{}{}", prefix, snippet, suffix),
            section,
            start as i32,
            end as i32,
        )
    }

    pub fn compress_snippet(&self, text: &str, max_chars: usize) -> String {
        if text.is_empty() {
            return String::new();
        }

        let text =
            Regex::new(r"\s+").map_or(text.to_string(), |r| r.replace_all(text, " ").to_string());

        if text.len() <= max_chars {
            return text;
        }

        let sentence_ends = Regex::new(r"[。！？.!?]").unwrap();
        let mut result = String::new();

        for cap in sentence_ends.split(&text) {
            let sentence = cap.trim();
            if result.len() + sentence.len() < max_chars {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(sentence);
            } else {
                break;
            }
        }

        if result.is_empty() {
            result = format!("{}...", &text[..max_chars.saturating_sub(3)]);
        }

        result
    }

    pub fn calculate_confidence(&self, contexts: &[ChatContext]) -> ConfidenceScore {
        if contexts.is_empty() {
            return ConfidenceScore {
                score: 0,
                papers_count: 0,
                coverage: "无相关论文".to_string(),
                warnings: vec!["未找到相关论文，无法验证回答准确性".to_string()],
                sources: vec![],
            };
        }

        let papers_count = contexts
            .iter()
            .map(|c| &c.paper_id)
            .collect::<HashSet<_>>()
            .len();
        let avg_relevance = contexts
            .iter()
            .map(|c| c.relevance_score as f64)
            .sum::<f64>()
            / contexts.len() as f64;

        let mut sections = HashSet::new();
        for ctx in contexts {
            let snippet_lower = ctx.snippet.to_lowercase();
            if snippet_lower.len() >= 100 {
                let first_100 = &snippet_lower[..100.min(snippet_lower.len())];
                if first_100.contains("abstract") {
                    sections.insert("Abstract");
                }
                if first_100.contains("introduction") {
                    sections.insert("Introduction");
                }
            }
            if snippet_lower.len() >= 100 {
                let first_100 = &snippet_lower[..100.min(snippet_lower.len())];
                for kw in ["method", "approach", "model", "architecture"] {
                    if first_100.contains(kw) {
                        sections.insert("Method");
                        break;
                    }
                }
            }
        }

        if sections.is_empty() {
            sections.insert("General");
        }

        let mut score = 50.0;

        if papers_count >= 3 {
            score += 20.0;
        } else if papers_count >= 2 {
            score += 15.0;
        } else if papers_count == 1 {
            score += 10.0;
        }

        score += avg_relevance * 20.0;

        if sections.len() >= 3 {
            score += 10.0;
        } else if sections.len() >= 2 {
            score += 7.0;
        } else {
            score += 3.0;
        }

        let score = score.min(100.0).max(0.0) as i32;

        let mut warnings = Vec::new();
        if papers_count == 1 {
            warnings.push("仅基于单篇论文，建议补充更多证据".to_string());
        }
        if avg_relevance < 0.6 {
            warnings.push("部分检索结果相关性较低".to_string());
        }

        let coverage = format!(
            "{}篇论文，覆盖{}",
            papers_count,
            sections
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );

        ConfidenceScore {
            score,
            papers_count: papers_count as i32,
            coverage,
            warnings,
            sources: sections.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn extract_citations(&self, contexts: &[ChatContext]) -> Vec<Citation> {
        let mut citations = Vec::new();
        let mut seen = HashSet::new();

        for ctx in contexts {
            if seen.contains(&ctx.paper_id) {
                continue;
            }
            seen.insert(ctx.paper_id.clone());

            let quote = self.extract_quote(&ctx.snippet);
            let snippet = if ctx.snippet.len() > 300 {
                format!("{}...", &ctx.snippet[..300])
            } else {
                ctx.snippet.clone()
            };

            citations.push(Citation {
                paper_id: ctx.paper_id.clone(),
                paper_title: ctx.paper_title.clone(),
                authors: ctx.authors.clone(),
                published: ctx.published.clone(),
                snippet,
                relevance_score: ctx.relevance_score,
                section: String::new(),
                char_start: 0,
                char_end: 0,
                quote,
            });
        }

        citations
    }

    fn extract_quote(&self, snippet: &str) -> String {
        if snippet.is_empty() {
            return String::new();
        }

        let clean = snippet.replace('\n', " ").replace("  ", " ");

        let sentence_end = Regex::new(r"[.!?]\s").unwrap();
        if let Some(m) = sentence_end.find(&clean) {
            let quote = clean[..m.end()].trim().to_string();
            return self.clean_quote(&quote);
        }

        let quote = clean[..150.min(clean.len())].trim().to_string();
        self.clean_quote(&quote)
    }

    fn clean_quote(&self, quote: &str) -> String {
        let quote = quote
            .trim_matches('"')
            .trim_matches('…')
            .trim_matches('»')
            .to_string();
        if quote.len() > 150 {
            format!("{}...", &quote[..147])
        } else {
            quote
        }
    }

    pub fn build_prompt(&self, question: &str, contexts: &[ChatContext]) -> String {
        let mut context_parts = Vec::new();

        let mut paper_contexts: HashMap<String, Vec<&ChatContext>> = HashMap::new();
        for ctx in contexts {
            paper_contexts
                .entry(ctx.paper_id.clone())
                .or_default()
                .push(ctx);
        }

        for (_, ctxs) in paper_contexts {
            let ctx = ctxs[0];
            let compressed_snippets: Vec<String> = ctxs
                .iter()
                .take(2)
                .map(|c| format!("> {}", self.compress_snippet(&c.snippet, 300)))
                .collect();
            let snippets_text = compressed_snippets.join("\n\n");
            let authors = if ctx.authors.is_empty() {
                "Unknown".to_string()
            } else {
                ctx.authors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let year = if ctx.published.len() >= 4 {
                ctx.published[..4].to_string()
            } else {
                "N/A".to_string()
            };

            context_parts.push(format!(
                "【论文】{}\n作者：{}\n年份：{}\n\n{}",
                ctx.paper_title, authors, year, snippets_text
            ));
        }

        let context_text = context_parts.join("\n\n---\n\n");

        format!(
            "基于以下论文内容回答问题。如果信息不足以回答，请明确说明。\n\n【论文内容】\n{}\n\n【问题】\n{}\n\n请按以下格式回答：\n1. 首先用1-2句话总结回答要点\n2. 然后详细解释，引用原文片段（用 > 块引用格式）\n3. 对于不确定的信息，加 [推测] 标注\n4. 最后列出参考论文列表",
            context_text,
            question
        )
    }

    pub fn format_result(
        &self,
        result: &ChatResult,
        show_citations: bool,
        _show_probing: bool,
        show_confidence: bool,
        show_insights: bool,
    ) -> String {
        let mut output = Vec::new();
        output.push("-".repeat(60).to_string());
        output.push(result.answer.clone());
        output.push("-".repeat(60).to_string());

        if show_insights && !result.cross_paper_insights.is_empty() {
            output.push(String::new());
            output.push("Cross-Paper Insights:".to_string());
            for insight in &result.cross_paper_insights {
                output.push(format!("  [{}] {}", insight.insight_type, insight.summary));
            }
        }

        if show_confidence {
            if let Some(conf) = &result.confidence {
                output.push(String::new());
                output.push(format!("Confidence: {}%", conf.score));
                output.push(format!("  {}", conf.coverage));
                for w in conf.warnings.iter().take(2) {
                    output.push(format!("  Warning: {}", w));
                }
            }
        }

        if show_citations && !result.citations.is_empty() {
            output.push(String::new());
            output.push("Citations:".to_string());
            for (i, cite) in result.citations.iter().enumerate() {
                let authors = cite
                    .authors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let year = if cite.published.len() >= 4 {
                    &cite.published[..4]
                } else {
                    "N/A"
                };
                output.push(format!(
                    "[{}] {} by {} ({})",
                    i + 1,
                    cite.paper_title,
                    authors,
                    year
                ));
                output.push(format!("  Relevance: {:.2}", cite.relevance_score));
                if !cite.quote.is_empty() {
                    output.push(format!("  \"{}\"", cite.quote));
                }
            }
        }

        output.join("\n")
    }
}

pub fn get_embedding_cache_key(model: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", model, text.trim()));
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_chat_creation() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(chat.model, LLM_MODEL);
        assert_eq!(chat.base_url, LLM_BASE_URL);
    }

    #[test]
    fn test_classify_query_factual() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(
            chat.classify_query("Who proposed the transformer?"),
            QueryType::Factual
        );
        assert_eq!(
            chat.classify_query("Which paper introduced BERT?"),
            QueryType::Factual
        );
    }

    #[test]
    fn test_classify_query_conceptual() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(
            chat.classify_query("What is attention mechanism?"),
            QueryType::Conceptual
        );
        assert_eq!(
            chat.classify_query("Explain how transformers work"),
            QueryType::Conceptual
        );
    }

    #[test]
    fn test_classify_query_comparative() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(chat.classify_query("BERT vs GPT"), QueryType::Comparative);
        assert_eq!(
            chat.classify_query("Compare BERT and GPT"),
            QueryType::Comparative
        );
    }

    #[test]
    fn test_classify_query_temporal() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(
            chat.classify_query("Recent advances in NLP"),
            QueryType::Temporal
        );
        assert_eq!(
            chat.classify_query("2024 papers on AI"),
            QueryType::Temporal
        );
    }

    #[test]
    fn test_classify_query_general() {
        let chat = RagChat::new(None, None, None);
        assert_eq!(chat.classify_query("Hello world"), QueryType::General);
    }

    #[test]
    fn test_extract_topic() {
        let chat = RagChat::new(None, None, None);
        let topic = chat.extract_topic("What is the meaning of life");
        assert!(topic.is_some());
        assert!(topic.unwrap().len() <= 20);
    }

    #[test]
    fn test_detect_section() {
        let chat = RagChat::new(None, None, None);
        let text = "This is the abstract section. Introduction starts here.";
        let section = chat.detect_section(text, 25);
        assert_eq!(section, "Abstract");
    }

    #[test]
    fn test_extract_snippet() {
        let chat = RagChat::new(None, None, None);
        let (snippet, _, _, _) = chat.extract_snippet("Hello world test query", "query", 100);
        assert!(!snippet.is_empty());
    }

    #[test]
    fn test_compress_snippet() {
        let chat = RagChat::new(None, None, None);
        let text = "This is a very long text. That should be compressed. With multiple sentences.";
        let compressed = chat.compress_snippet(text, 50);
        assert!(compressed.len() <= 53);
    }

    #[test]
    fn test_calculate_confidence_empty() {
        let chat = RagChat::new(None, None, None);
        let conf = chat.calculate_confidence(&[]);
        assert_eq!(conf.score, 0);
        assert_eq!(conf.papers_count, 0);
    }

    #[test]
    fn test_calculate_confidence_with_contexts() {
        let chat = RagChat::new(None, None, None);
        let contexts = vec![ChatContext {
            paper_id: "p1".to_string(),
            paper_title: "Paper 1".to_string(),
            authors: vec![],
            published: "2024".to_string(),
            snippet: "abstract test".to_string(),
            relevance_score: 0.9,
        }];
        let conf = chat.calculate_confidence(&contexts);
        assert!(conf.score > 0);
    }

    #[test]
    fn test_extract_citations() {
        let chat = RagChat::new(None, None, None);
        let contexts = vec![ChatContext {
            paper_id: "p1".to_string(),
            paper_title: "Paper 1".to_string(),
            authors: vec!["Author".to_string()],
            published: "2024".to_string(),
            snippet: "Test snippet content".to_string(),
            relevance_score: 0.9,
        }];
        let citations = chat.extract_citations(&contexts);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].paper_id, "p1");
    }

    #[test]
    fn test_build_prompt() {
        let chat = RagChat::new(None, None, None);
        let contexts = vec![ChatContext {
            paper_id: "p1".to_string(),
            paper_title: "Paper 1".to_string(),
            authors: vec![],
            published: "2024".to_string(),
            snippet: "Test snippet".to_string(),
            relevance_score: 0.9,
        }];
        let prompt = chat.build_prompt("What is this about?", &contexts);
        assert!(prompt.contains("Paper 1"));
        assert!(prompt.contains("What is this about?"));
    }

    #[test]
    fn test_get_embedding_cache_key() {
        let key1 = get_embedding_cache_key("model", "test text");
        let key2 = get_embedding_cache_key("model", "test text");
        let key3 = get_embedding_cache_key("model", "different");
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_query_type_as_str() {
        assert_eq!(QueryType::Factual.as_str(), "factual");
        assert_eq!(QueryType::Conceptual.as_str(), "conceptual");
        assert_eq!(QueryType::Comparative.as_str(), "comparative");
        assert_eq!(QueryType::Temporal.as_str(), "temporal");
        assert_eq!(QueryType::General.as_str(), "general");
    }
}
