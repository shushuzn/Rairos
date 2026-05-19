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
use rairos_core::Database;
use crate::handlers::*;

pub fn run_chat_interactive(
    db: &Database,
    rt: &tokio::runtime::Runtime,
    api_key: &str,
    base_url: &str,
    chat_model: &str,
    rag_system_prompt: &str,
    _paper: Option<&str>,
    limit: usize,
    no_cite: bool,
    verbose: bool,
    stream: bool,
    export_path: Option<&str>,
    export_fmt: Option<&str>,
) -> Result<()> {
    println!("{}", "═".repeat(60));
    println!("📚 AI Research OS — RAG Chat");
    println!("{}", "═".repeat(60));
    println!();
    println!("Commands:");
    println!("  q / quit / exit    Quit");
    println!("  clear              Clear history");
    println!("  help               Show help");
    println!();
    println!("Tip: Ask questions about papers in your library.");
    println!();

    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        let question = {
            print!("❓ ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => line.trim().to_string(),
            }
        };

        if question.is_empty() {
            continue;
        }

        match question.to_lowercase().as_str() {
            "q" | "quit" | "exit" => {
                if !history.is_empty() {
                    if let Some(path) = export_path {
                        export_chat_history(&history, path, export_fmt);
                        println!("✅ Exported to {}", path);
                    }
                }
                println!("\n再见！");
                break;
            }
            "clear" => {
                history.clear();
                println!("✅ History cleared");
                continue;
            }
            "help" => {
                println!("\nHelp:");
                println!("  Ask any question about papers in your library");
                println!("  Example questions:");
                println!("    How does self-attention work?");
                println!("    What are the main contributions?");
                println!("    What is Sparse MoE?");
                println!();
                continue;
            }
            _ => {}
        }

        if verbose {
            println!("🔍 Retrieving papers...");
        }
        let papers = match db.search_papers_smart(&question, limit) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Search failed: {}", e);
                continue;
            }
        };

        if papers.is_empty() {
            println!("No matching papers found. Try a different question.");
            continue;
        }

        let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
            let abstract_text = if p.abstract_text.len() > 400 {
                format!("{}...", &p.abstract_text[..400])
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

        println!("\n💡 Answer:");
        println!("{}", "─".repeat(60));

        let answer_result = rt.block_on(async {
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
            if stream {
                client.chat_completions_streaming(messages, None, Some(rag_system_prompt)).await
            } else {
                client.chat_completions(messages, None, Some(rag_system_prompt), false).await
            }
        });

        match answer_result {
            Ok(answer) => {
                println!("{}", answer);
                println!("{}", "─".repeat(60));
                if !no_cite {
                    println!("\n📖 引用来源");
                    for (i, p) in papers.iter().enumerate().take(5) {
                        println!("  [{}] {} (ID: {})", i + 1, p.title, p.id);
                    }
                }
                println!();
                history.push((question, answer));
            }
            Err(e) => {
                eprintln!("LLM call failed: {}", e);
            }
        }
    }

    Ok(())
}