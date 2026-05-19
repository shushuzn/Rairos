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
        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = model
        .map(|m| m.to_string())
        .or_else(|| std::env::var("LLM_MODEL").ok())
        .unwrap_or_else(|| LLM_MODEL.to_string());

    let db_path = PathBuf::from("rairos.db");
    let db = Database::open(&db_path)?;

    let rt = tokio::runtime::Runtime::new()?;

    let rag_system_prompt = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分\"原文明确说\"和\"可推断\"
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

fn run_chat_single(
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
    let stop_words: std::collections::HashSet<&str> = [
        "what", "are", "the", "a", "an", "is", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "must", "can", "need", "to", "of",
        "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
        "during", "before", "after", "above", "below", "between", "under",
        "again", "further", "then", "once", "here", "there", "when", "where",
        "why", "how", "all", "each", "few", "more", "most", "other", "some",
        "such", "no", "nor", "not", "only", "own", "same", "so", "than",
        "too", "very", "just", "but", "and", "or", "if", "because", "as",
        "until", "while", "this", "that", "these", "those", "about", "main",
        "findings", "find", "found", "research", "study", "studies", "paper",
        "papers", "your", "you", "i", "we", "they", "he", "she", "it",
    ].into();

    let query_terms: Vec<&str> = question
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 2 && !stop_words.contains(&w.to_lowercase().as_str()))
        .collect();

    let search_query = if query_terms.is_empty() {
        question.to_string()
    } else if query_terms.len() == 1 {
        query_terms[0].to_string()
    } else {
        query_terms.join(" ")
    };

    let papers = db.search_papers_smart(&search_query, limit)?;
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

    let verification = rt.block_on(async {
        verify_chat_answer(&answer, question, &context_str, api_key, &base_url, &chat_model).await
    });

    if let Some(warning) = verification {
        eprintln!("\n⚠️  Verification Warning: {}", warning);
    }

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

const VERIFY_ANSWER_PROMPT: &str = r#"你是一个答案验证助手。检查以下回答是否准确基于提供的上下文。

问题: {question}
上下文: {context}
回答: {answer}

检查：
1. 回答中的事实是否在上下文中找到支持？
2. 是否有捏造的内容？
3. 是否正确引用了原文？

请以JSON格式返回验证结果：
{{"is_valid": true/false, "issues": ["问题1", "问题2"]}}

如果回答有效，返回 {{"is_valid": true, "issues": []}}。
如果回答存在问题，返回 {{"is_valid": false, "issues": ["具体问题描述"]}}。"#;

async fn verify_chat_answer(
    answer: &str,
    question: &str,
    context: &str,
    api_key: &str,
    base_url: &str,
    chat_model: &str,
) -> Option<String> {
    let prompt = VERIFY_ANSWER_PROMPT
        .replace("{question}", question)
        .replace("{context}", context)
        .replace("{answer}", answer);

    let client = rairos_llm::client_async::AsyncClient::new(
        api_key.to_string(),
        base_url.to_string(),
        chat_model.to_string(),
    );

    let messages = vec![
        std::collections::HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), prompt),
        ]),
    ];

    match client.chat_completions(messages, None, None, false).await {
        Ok(response) => {
            parse_verification_response(&response)
        }
        Err(_) => None,
    }
}

fn parse_verification_response(response: &str) -> Option<String> {
    let response = response.trim();

    if response.contains("\"is_valid\": true") || response.contains("\"is_valid\":true") {
        return None;
    }

    if response.contains("\"is_valid\": false") || response.contains("\"is_valid\":false") {
        let mut issues = Vec::new();

        if let Some(start) = response.find("\"issues\":") {
            let issues_str = &response[start..];
            if let Some(arr_start) = issues_str.find('[') {
                if let Some(arr_end) = issues_str.find(']') {
                    let items = &issues_str[arr_start + 1..arr_end];
                    for item in items.split(',') {
                        let item = item.trim().trim_matches('"').trim_matches(|c| c == '"' || c == ' ');
                        if !item.is_empty() && item != "[]" && item != "issues" {
                            issues.push(item.to_string());
                        }
                    }
                }
            }
        }

        if issues.is_empty() {
            Some("回答可能存在准确性问题".to_string())
        } else {
            Some(issues.join("; "))
        }
    } else {
        None
    }
}