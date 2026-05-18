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
use rairos_core::Database;
use std::path::PathBuf;
use crate::handlers::*;

pub fn handle_chat(
    question: Option<&str>,
    paper: Option<&str>,
    _concept: Option<&str>,
    limit: usize,
    interactive: bool,
    no_cite: bool,
    model: Option<&str>,
    verbose: bool,
    stream: bool,
    export_path: Option<&str>,
    export_fmt: Option<&str>,
) -> Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Please set it to enable chat."))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = model.unwrap_or(LLM_MODEL).to_string();

    let db_path = PathBuf::from("rairos.db");
    let db = Database::open(&db_path)?;

    let rt = tokio::runtime::Runtime::new()?;

    let rag_system_prompt = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分"原文明确说"和"可推断"
5. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点（1-2句话）
- 详细解释部分引用原文片段
- 结尾标注信息来源";

    if interactive || question.is_none() {
        run_chat_interactive(&db, &rt, &api_key, &base_url, &chat_model, rag_system_prompt,
            paper, limit, no_cite, verbose, stream, export_path, export_fmt)?;
    } else if let Some(q) = question {
        run_chat_single(q, &db, &rt, &api_key, &base_url, &chat_model,
            rag_system_prompt, paper, limit, no_cite, verbose, stream)?;
    }

    Ok(())
}

pub fn run_chat_single(
    question: &str,
    db: &Database,
    rt: &tokio::runtime::Runtime,
    api_key: &str,
    base_url: &str,
    chat_model: &str,
    rag_system_prompt: &str,
    _paper: Option<&str>,
    limit: usize,
    no_cite: bool,
    _verbose: bool,
    _stream: bool,
) -> Result<()> {
    let papers = db.search_papers(question, limit)?;
    if papers.is_empty() {
        eprintln!("No papers found matching your question.");
        return Ok(());
    }

    let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
        let abstract_text = if p.abstract_text.len() > 500 {
            format!("{}...", &p.abstract_text[..500])
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
    let user_prompt = format!(
        "基于以下论文内容回答问题。\n\n{context_str}\n\n问题: {question}"
    );

    println!("{}", "═".repeat(60));
    println!("💡 Answer:");

    let answer = rt.block_on(async {
        let client = rairos_llm::client_async::AsyncClient::new(
            api_key.to_string(),
            base_url.to_string(),
            chat_model.to_string(),
        );
        let messages = vec![
            std::collections::HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), user_prompt.clone()),
            ]),
        ];
        client.chat_completions(messages, None, Some(rag_system_prompt), false).await
    }).map_err(|e| anyhow::anyhow!("LLM call failed: {}", e))?;

    println!("{}", answer);
    println!("{}", "═".repeat(60));

    if !no_cite {
        println!("\n📖 引用来源");
        println!("{}", "-".repeat(60));
        for (i, p) in papers.iter().enumerate() {
            let preview: String = p.abstract_text.chars().take(150).collect();
            println!("\n[{}] {}", i + 1, p.title);
            println!("    ID: {}", p.id);
            println!("    > {}...", preview);
        }
    }

    Ok(())
}