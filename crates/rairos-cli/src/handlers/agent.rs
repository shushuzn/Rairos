#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::Result;
use rairos_core::constants::{LLM_BASE_URL, LLM_MODEL};
use rairos_core::{Database, Paper};

pub fn handle_agent(
    db: &Database,
    topic: &str,
    max_papers: usize,
    _max_time_minutes: u64,
    format: &str,
) -> Result<()> {
    println!("=== Rairos Research Agent ===");
    println!("Topic: {}", topic);
    println!("Max papers: {}", max_papers);
    println!();

    let mut papers = db.search_papers_smart(topic, max_papers)?;

    if papers.len() < 5 {
        println!("Not enough local papers ({}), fetching from arXiv...", papers.len());
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(ref rt) = rt {
            if let Ok(arxiv_papers) = rt.block_on(rairos_parser::search_arxiv_recent(topic, 20)) {
                for arxiv_paper in arxiv_papers {
                    let paper = Paper::with_metadata(
                        arxiv_paper.arxiv_id,
                        arxiv_paper.title,
                        arxiv_paper.abstract_text,
                        arxiv_paper.authors,
                        arxiv_paper.categories,
                        rairos_core::PaperMetadata::default(),
                    );
                    if db.insert_paper(&paper).is_ok() {
                        papers.push(paper);
                    }
                }
            }
        }
    }

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Analyzing {} papers...\n", papers.len());

    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Please set it to enable chat."))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| LLM_MODEL.to_string());

    let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
        let abstract_text = if p.abstract_text.len() > 600 {
            format!("{}...", &p.abstract_text[..600])
        } else {
            p.abstract_text.clone()
        };
        format!(
            "[Paper {}] Title: {}\nAuthors: {}\nAbstract: {}",
            i + 1,
            p.title,
            p.authors.join(", "),
            abstract_text
        )
    }).collect();
    let context_str = context_parts.join("\n\n");

    let rt = tokio::runtime::Runtime::new()?;

    let system_prompt = "You are a research analysis AI. Analyze papers and provide structured insights.

Format your response exactly as:

## Key Themes
- [theme 1]
- [theme 2]

## Methodologies
- [method 1]
- [method 2]

## Research Gaps
1. [gap 1 with specific recommendation]
2. [gap 2 with specific recommendation]

## Potential Research Directions
- [direction 1]
- [direction 2]";

    let user_prompt = format!(
        "Topic: {}\n\nPapers:\n{}\n\nAnalyze these papers and provide insights.",
        topic, context_str
    );

    println!("Running LLM analysis...\n");

    let result = rt.block_on(async {
        let client = rairos_llm::client_async::AsyncClient::new(
            api_key,
            base_url,
            chat_model,
        );
        let messages = vec![
            std::collections::HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), user_prompt.clone()),
            ]),
        ];
        client.chat_completions(messages, None, Some(system_prompt), false).await
    });

    match result {
        Ok(analysis) => {
            println!("{}", "═".repeat(60));
            println!("📊 Research Analysis for: {}", topic);
            println!("{}", "═".repeat(60));
            println!("{}", analysis);
            println!("{}", "═".repeat(60));
            println!("\n📚 Papers analyzed: {}", papers.len());
            if format == "json" {
                let out = serde_json::json!({
                    "topic": topic,
                    "papers_analyzed": papers.len(),
                    "analysis": analysis,
                    "paper_titles": papers.iter().map(|p| p.title.clone()).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
        Err(e) => {
            eprintln!("Analysis failed: {}", e);
        }
    }

    Ok(())
}
