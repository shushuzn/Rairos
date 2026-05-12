use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const PNOTE_SYSTEM_PROMPT: &str =
    "You are a rigorous AI research assistant, skilled in adversarial review.";

const PNOTE_USER_PROMPT_TEMPLATE: &str = r#"Paper title: {paper_title}
Authors: {paper_authors}
Source: {paper_source}:{paper_uid}
Published: {paper_published}
Tags: {paper_tags}

【Abstract】
{paper_abstract}

【Extracted Body】(prioritized by importance)

{paper_body}

Please generate draft according to sections:

## 1. Background
One sentence: What problem does this paper solve? (cite abstract)

## 2. Core Problem
What is the core technical solution? (cite body with > quotes; add [推测] if uncertain)

## 3. Method Structure
### 3.1 Architecture
### 3.2 Algorithm Logic
### 3.3 Key Components

## 4. Key Innovation
One sentence summarizing the biggest innovation.

## 5. Experimental Analysis
### 5.1 Datasets
### 5.2 Baseline Comparison
### 5.3 Ablation Study
### 5.4 Cost Analysis

## 6. Adversarial Review
List 3 strongest critiques: (1) Logic/assumption flaws; (2) Insufficient experiment coverage; (3) Generalization/reproducibility risks. (add [推测] markers)

## 7. Advantages
Main advantages of this paper. (cite experimental/analysis results from reference papers)

## 8. Limitations
Main limitations. (cite discussions from reference papers, add [推测])

## 9. Essential Abstraction
One sentence abstracting the essence of this paper.

## 10. Comparison with Other Methods
Core differences from similar methods.

## 11. Decision
Is it worth deeper attention? Use cases?

## 12. Knowledge Distillation
### Facts
### Principles
### Insights

## 13. Cognitive Upgrade
Long-term value, scale effects, technical moat, paradigm shift potential.

## 14. Scoring Rubric
Must include: Novelty / Leverage / Evidence / Cost / Moat / Adoption Signal
Format per item: `* Novelty (1-5): N`
Overall Judgment: One sentence summary

(No fabricating experimental data; cite with "> original text")

Output JSON after Markdown:
{{"novelty": 3, "leverage": 4, "evidence": 3, "cost": 2, "moat": 2, "adoption": 3, "overall": "one sentence evaluation"}}
"#;

const CNOTE_USER_PROMPT_TEMPLATE: &str = r#"Concept: {concept}

Reference papers ({num_papers} total):
{pnotes_text}

Please generate C-Note draft with ## sections:

## Core Definition
One sentence definition. (cite from reference papers, or synthesize with [推测] if no exact quote)

## Background
What research context did this concept emerge from? What problem does it solve? (cite, or [推测] if none)

## Technical Essence
What is the core technical mechanism? (cite, add [推测] for inference)

## Common Implementation Paths
List typical implementations. (cite from reference papers, add [推测])

## Advantages
Main advantages. (cite experimental/analysis results from reference papers)

## Limitations
Main limitations. (cite discussions, add [推测])

## Relationship with Other Concepts
Relations and differences with related concepts. (synthesize multiple papers, add [推测])

## Representative Papers
Select papers that best represent this concept from references, with reasoning.

## Evolution Timeline
Infer the evolution path of this concept based on reference papers. (add [推测])

## Future Trends
Predict future development directions based on discussions. (add [推测])

(No fabricating paper data; cite with "> original text")
"#;

const CNOTE_SYSTEM_PROMPT: &str = "You are a rigorous AI research assistant, skilled in concept analysis and knowledge graph construction.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub total_cost_usd: f64,
}

pub fn get_model_price(model: &str) -> (f64, f64) {
    let prices: HashMap<&str, (f64, f64)> = HashMap::from([
        ("gpt-4", (30.0, 60.0)),
        ("gpt-3.5", (0.5, 1.5)),
        ("claude", (3.0, 15.0)),
        ("default", (1.0, 2.0)),
    ]);

    let model_lower = model.to_lowercase();
    for (prefix, price) in prices.iter() {
        if model_lower.contains(prefix) {
            return *price;
        }
    }
    *prices.get("default").unwrap()
}

pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    (text.len() / 4).max(1)
}

pub fn estimate_cost(model: &str, input_text: &str, output_text: &str) -> CostEstimate {
    let (in_per_1m, out_per_1m) = get_model_price(model);
    let in_toks = estimate_tokens(input_text);
    let out_toks = estimate_tokens(output_text);

    CostEstimate {
        input_tokens: in_toks,
        output_tokens: out_toks,
        total_tokens: in_toks + out_toks,
        input_cost_usd: round((in_toks as f64 / 1_000_000.0) * in_per_1m, 6),
        output_cost_usd: round((out_toks as f64 / 1_000_000.0) * out_per_1m, 6),
        total_cost_usd: round(
            (in_toks as f64 / 1_000_000.0) * in_per_1m
                + (out_toks as f64 / 1_000_000.0) * out_per_1m,
            6,
        ),
    }
}

pub fn format_pnote_prompt(
    paper_title: &str,
    paper_authors: &str,
    paper_source: &str,
    paper_uid: &str,
    paper_published: &str,
    paper_tags: &str,
    paper_abstract: &str,
    paper_body: &str,
) -> String {
    PNOTE_USER_PROMPT_TEMPLATE
        .replace("{paper_title}", paper_title)
        .replace("{paper_authors}", paper_authors)
        .replace("{paper_source}", paper_source)
        .replace("{paper_uid}", paper_uid)
        .replace("{paper_published}", paper_published)
        .replace("{paper_tags}", paper_tags)
        .replace("{paper_abstract}", paper_abstract)
        .replace("{paper_body}", paper_body)
}

pub fn format_cnote_prompt(concept: &str, pnotes: &[HashMap<String, String>]) -> String {
    let pnotes_chunks: Vec<String> = pnotes.iter().enumerate().map(|(i, p)| {
        let title = p.get("title").map(|s| s.as_str()).unwrap_or("N/A");
        let authors = p.get("authors").map(|s| s.as_str()).unwrap_or("Unknown");
        let year = p.get("year").map(|s| s.as_str()).unwrap_or("N/A");
        let source = p.get("source").map(|s| s.as_str()).unwrap_or("N/A");
        let uid = p.get("uid").map(|s| s.as_str()).unwrap_or("N/A");
        let tags = p.get("tags").map(|s| s.as_str()).unwrap_or("");
        let abstract_text = p.get("abstract").map(|s| s.as_str()).unwrap_or("(none)");

        format!(
            "---\nPaper {}:\nTitle: {}\nAuthors: {}\nYear: {}\nSource: {}:{}\nTags: {}\nAbstract: {}",
            i + 1, title, authors, year, source, uid, tags, abstract_text
        )
    }).collect();

    let pnotes_text = pnotes_chunks.join("\n");

    CNOTE_USER_PROMPT_TEMPLATE
        .replace("{concept}", concept)
        .replace("{pnotes_text}", &pnotes_text)
        .replace("{num_papers}", &pnotes.len().to_string())
}

pub fn format_reading_recommendation_prompt(
    paper_title: &str,
    paper_authors: &str,
    paper_year: &str,
    paper_category: &str,
    score: f64,
    semantic_score: f64,
    citation_score: f64,
    tag_score: f64,
    recency_score: f64,
    read_papers_context: &[HashMap<String, String>],
) -> String {
    let scores = [
        ("语义相似度", semantic_score),
        ("引用关系", citation_score),
        ("标签重叠", tag_score),
        ("时效性", recency_score),
    ];
    let (top_signal, top_value) = scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .copied()
        .unwrap_or(("语义相似度", 0.0));

    let read_papers_str = if read_papers_context.is_empty() {
        "（暂无已读论文记录）".to_string()
    } else {
        read_papers_context
            .iter()
            .map(|p| {
                let title = p.get("title").map(|s| s.as_str()).unwrap_or("Unknown");
                let authors = p.get("authors").map(|s| s.as_str()).unwrap_or("未知");
                let year = p.get("year").map(|s| s.as_str()).unwrap_or("");
                let category = p.get("category").map(|s| s.as_str()).unwrap_or("N/A");
                format!(
                    r#"- "{}" by {} ({}), 领域: {}"#,
                    title, authors, year, category
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"## Task
Generate a Chinese recommendation explanation for the following paper.

## Paper to Recommend
- Title: {paper_title}
- Authors: {paper_authors}
- Year: {paper_year}
- Category: {paper_category}

## Recommendation Score Details
- Combined score: {score:.2} (max 1.0)
- Semantic similarity: {semantic_score:.2} (weight 40%)
- Citation relationship: {citation_score:.2} (weight 30%)
- Tag overlap: {tag_score:.2} (weight 20%)
- Recency: {recency_score:.2} (weight 10%)
- Top signal: {top_signal} ({top_value:.2})

## User's Read Paper Background
Papers you have already read:
{read_papers_str}

## Output Requirements
Generate recommendation explanation in three sections:

### Recommendation Reason
(Based on score data, explain why this paper is recommended, focus on highest signal: {top_signal})

### Relation to Read Papers
(Analyze this paper's connection to papers you've read, its position in your research trajectory)

### Best Reading Scenarios
(Explain when this paper should be prioritized)

Each section 2-4 sentences, concise and professional."#,
        paper_title = paper_title,
        paper_authors = paper_authors,
        paper_year = paper_year,
        paper_category = paper_category,
        score = score,
        semantic_score = semantic_score,
        citation_score = citation_score,
        tag_score = tag_score,
        recency_score = recency_score,
        top_signal = top_signal,
        top_value = top_value,
        read_papers_str = &read_papers_str
    )
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_price() {
        let (in_p, out_p) = get_model_price("gpt-4");
        assert_eq!(in_p, 30.0);
        assert_eq!(out_p, 60.0);

        let (in_p, out_p) = get_model_price("gpt-3.5-turbo");
        assert_eq!(in_p, 0.5);
        assert_eq!(out_p, 1.5);

        let (in_p, out_p) = get_model_price("unknown");
        assert_eq!(in_p, 1.0);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens("hello world"), 2);
    }

    #[test]
    fn test_estimate_cost() {
        let cost = estimate_cost("gpt-4", "Hello world", "Goodbye world");
        assert!(cost.input_tokens > 0);
        assert!(cost.output_tokens > 0);
        assert!(cost.total_cost_usd > 0.0);
    }

    #[test]
    fn test_format_pnote_prompt() {
        let prompt = format_pnote_prompt(
            "Test Title",
            "Author 1, Author 2",
            "arxiv",
            "1234.5678",
            "2024-01-01",
            "AI, ML",
            "This is an abstract.",
            "Body text here.",
        );
        assert!(prompt.contains("Test Title"));
        assert!(prompt.contains("Author 1, Author 2"));
        assert!(prompt.contains("This is an abstract"));
    }

    #[test]
    fn test_format_cnote_prompt() {
        let pnotes = vec![{
            let mut m = HashMap::new();
            m.insert("title".to_string(), "Paper 1".to_string());
            m.insert("authors".to_string(), "Author A".to_string());
            m.insert("year".to_string(), "2023".to_string());
            m.insert("source".to_string(), "arxiv".to_string());
            m.insert("uid".to_string(), "1111".to_string());
            m.insert("tags".to_string(), "AI".to_string());
            m.insert("abstract".to_string(), "Abstract 1".to_string());
            m
        }];
        let prompt = format_cnote_prompt("Attention Mechanism", &pnotes);
        assert!(prompt.contains("Attention Mechanism"));
        assert!(prompt.contains("Paper 1"));
    }

    #[test]
    fn test_format_reading_recommendation_prompt() {
        let read_papers = vec![{
            let mut m = HashMap::new();
            m.insert("title".to_string(), "Read Paper 1".to_string());
            m.insert("authors".to_string(), "Author X".to_string());
            m.insert("year".to_string(), "2022".to_string());
            m.insert("category".to_string(), "NLP".to_string());
            m
        }];
        let prompt = format_reading_recommendation_prompt(
            "New Paper",
            "Author Y",
            "2024",
            "ML",
            0.85,
            0.9,
            0.8,
            0.7,
            0.6,
            &read_papers,
        );
        assert!(prompt.contains("New Paper"));
        assert!(prompt.contains("Author Y"));
        assert!(prompt.contains("语义相似度"));
    }
}
