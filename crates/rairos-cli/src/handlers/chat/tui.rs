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
use std::path::PathBuf;
use rairos_core::Database;

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