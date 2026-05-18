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
        let papers = match db.search_papers(&question, limit) {
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

pub fn export_chat_history(history: &[(String, String)], path: &str, fmt: Option<&str>) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let format = fmt.unwrap_or(match ext {
        "html" | "htm" => "html",
        _ => "markdown",
    });
    let content = match format {
        "html" => export_chat_to_html(history),
        _ => export_chat_to_markdown(history),
    };
    let _ = std::fs::write(path, content);
}

pub fn export_chat_to_markdown(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut md = format!("# AI Research OS — Chat Export\n\n**Exported**: {now}\n\n---\n\n", now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        md.push_str(&format!("## Q{i}: {q}\n\n**A**: {a}\n\n---\n\n", i = i + 1, q = q, a = a));
    }
    md
}

pub fn export_chat_to_html(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang='zh-CN'>
<head>
<meta charset='UTF-8'>
<title>AI Research OS — Chat Export</title>
<style>
body {{ font-family: 'Segoe UI', Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
h1 {{ color: #1a1a2e; border-bottom: 2px solid #4a4a8a; padding-bottom: 10px; }}
.qa-block {{ background: #f8f9fa; border-radius: 8px; padding: 15px; margin: 15px 0; }}
.question {{ color: #2a5a2a; font-weight: bold; }}
.answer {{ color: #333; margin-top: 10px; line-height: 1.6; }}
.meta {{ color: #666; font-size: 0.85em; }}
</style>
</head>
<body>
<h1>AI Research OS — Chat Export</h1>
<p class='meta'>Exported: {now}</p>
"#, now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        html.push_str(&format!(
            r#"<div class='qa-block'>
<div class='question'>Q{i}: {q}</div>
<div class='answer'>{a}</div>
</div>
"#, i = i + 1, q = q, a = a));
    }
    html.push_str("</body>\n</html>");
    html
}

pub fn handle_chat_tui() -> Result<()> {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, List, ListItem, Paragraph},
        Terminal,
    };
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    let api_key = match std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
    {
        Ok(k) => k,
        Err(_) => {
            eprintln!("OPENAI_API_KEY not set. Please set it to enable chat.");
            return Ok(());
        }
    };
    let base_url = std::env::var("LLM_BASE_URL")
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let model = LLM_MODEL.to_string();

    let db_path = PathBuf::from("rairos.db");
    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            return Ok(());
        }
    };
    let rt = tokio::runtime::Runtime::new()?;

    let rag_system_prompt = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 使用 > 块引用格式引用原文片段
3. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点
- 详细解释部分引用原文片段
- 结尾标注信息来源";

    // ── TUI setup ──
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── State ──
    #[derive(Clone)]
    struct ChatMsg {
        role: String,  // "user" | "assistant" | "error" | "info"
        content: String,
    }

    let mut messages: Vec<ChatMsg> = Vec::new();
    let mut input = String::new();
    let _scroll_offset: usize = 0;
    let mut loading = false;

    messages.push(ChatMsg {
        role: "info".to_string(),
        content: "Welcome to Rairos TUI Chat! Type a question and press Enter. Type /quit or Esc to exit.".to_string(),
    });

    let r = loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(size);

            // ── Message area ──
            let msg_items: Vec<ListItem> = messages.iter().map(|msg| {
                let style = match msg.role.as_str() {
                    "user" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    "assistant" => Style::default().fg(Color::Green),
                    "error" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::DarkGray),
                };
                let prefix = match msg.role.as_str() {
                    "user" => "❓ ",
                    "assistant" => "💡 ",
                    "error" => "⚠️ ",
                    _ => "",
                };
                let lines: Vec<Line> = msg.content.lines().map(|l| {
                    Line::from(Span::styled(format!("{}{}", prefix, l), style))
                }).collect();
                ListItem::new(lines)
            }).collect();

            let msg_list = List::new(msg_items)
                .block(Block::default()
                    .title("  AI Research OS Chat  ")
                    .borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(msg_list, chunks[0]);

            // ── Input area ──
            let input_style = if loading {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let input_para = Paragraph::new(Text::from(input.as_str()))
                .style(input_style)
                .block(Block::default()
                    .title(if loading { "  ⏳ Thinking...  " } else { "  Type your question  " })
                    .borders(Borders::ALL));
            f.render_widget(input_para, chunks[1]);

            // Move cursor to end of input
            if !loading {
                let x = chunks[1].x + 1 + input.len() as u16;
                let y = chunks[1].y + 1;
                if x < chunks[1].x + chunks[1].width - 1 {
                    f.set_cursor_position(ratatui::layout::Position::new(x, y));
                }
            }
        })?;

        // ── Event handling ──
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => break Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        break Ok(());
                    }
                    KeyCode::Enter if !loading => {
                        let q = input.trim().to_string();
                        if q.is_empty() {
                            continue;
                        }
                        if q == "/quit" || q == "/exit" {
                            break Ok(());
                        }

                        messages.push(ChatMsg {
                            role: "user".to_string(),
                            content: q.clone(),
                        });
                        input.clear();

                        // Search papers and call LLM
                        let papers = db.search_papers(&q, 5).unwrap_or_default();
                        if papers.is_empty() {
                            messages.push(ChatMsg {
                                role: "info".to_string(),
                                content: "No matching papers found. Try a different question.".to_string(),
                            });
                            loading = false;
                            continue;
                        }

                        let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
                            let abs = if p.abstract_text.len() > 400 {
                                format!("{}...", &p.abstract_text[..400])
                            } else {
                                p.abstract_text.clone()
                            };
                            format!(
                                "[Paper {}] Title: {}\nAuthors: {}\nAbstract: {}",
                                i + 1, p.title, p.authors.join(", "), abs
                            )
                        }).collect();
                        let context_str = context_parts.join("\n\n");
                        let user_prompt = format!(
                            "基于以下论文内容回答问题。\n\n{context_str}\n\n问题: {q}"
                        );

                        let api_key = api_key.clone();
                        let base_url = base_url.clone();
                        let model = model.clone();
                        let rag_system_prompt = rag_system_prompt.to_string();

                        let answer_result = rt.block_on(async {
                            let client = rairos_llm::client_async::AsyncClient::new(
                                api_key, base_url, model,
                            );
                            let msgs = vec![
                                std::collections::HashMap::from([
                                    ("role".to_string(), "user".to_string()),
                                    ("content".to_string(), user_prompt),
                                ]),
                            ];
                            client.chat_completions(msgs, None, Some(&rag_system_prompt), false).await
                        });

                        match answer_result {
                            Ok(answer) => {
                                let mut response = answer;
                                if !papers.is_empty() {
                                    response.push_str("\n\n─── Citations ───\n");
                                    for (i, p) in papers.iter().enumerate() {
                                        response.push_str(&format!(
                                            "[{}] {} (ID: {})\n", i + 1, p.title, p.id
                                        ));
                                    }
                                }
                                messages.push(ChatMsg {
                                    role: "assistant".to_string(),
                                    content: response,
                                });
                            }
                            Err(e) => {
                                messages.push(ChatMsg {
                                    role: "error".to_string(),
                                    content: format!("LLM call failed: {}", e),
                                });
                            }
                        }
                        loading = false;
                    }
                    KeyCode::Char(c) if !loading => {
                        input.push(c);
                    }
                    KeyCode::Backspace if !loading => {
                        input.pop();
                    }
                    _ => {}
                }
            }
        }
    };

    // ── Cleanup ──
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    r
}
