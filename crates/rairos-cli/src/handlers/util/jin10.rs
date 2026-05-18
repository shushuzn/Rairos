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
use rairos_mcp_jin10::Jin10Client;

use crate::Jin10Action;

pub fn handle_jin10(action: &Jin10Action) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut client = Jin10Client::default();

        match action {
            Jin10Action::Quote { code } => {
                let data = client.get_quote(code).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                println!();
                println!("  \x1b[36m{} Quote\x1b[0m", data.get("name").and_then(|v| v.as_str()).unwrap_or(code));
                println!("  Time:   {}", data.get("time").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  Price:  {}", data.get("close").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  Open:   {}", data.get("open").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  High:   {}", data.get("high").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  Low:    {}", data.get("low").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  Volume: {}", data.get("volume").and_then(|v| v.as_str()).unwrap_or("?"));
                println!("  Change: {} ({}%)",
                    data.get("ups_price").and_then(|v| v.as_str()).unwrap_or("?"),
                    data.get("ups_percent").and_then(|v| v.as_str()).unwrap_or("?"));
                println!();
            }
            Jin10Action::Kline { code, time, count } => {
                let data = client.get_kline(code, *time, *count).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let name = data.get("name").and_then(|v| v.as_str()).unwrap_or(code);
                let klines = data.get("klines").or(data.get("data")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                println!("\n  \x1b[36m{} K-line ({})\x1b[0m", name, time);
                println!("  {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}", "Time", "Open", "High", "Low", "Close", "Vol");
                println!("  {} {} {} {} {} {}", "─".repeat(16), "─".repeat(8), "─".repeat(8), "─".repeat(8), "─".repeat(8), "─".repeat(8));
                for k in klines.iter().take(*count as usize) {
                    println!("  {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
                        k.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>(),
                        k.get("open").and_then(|v| v.as_str()).unwrap_or(""),
                        k.get("high").and_then(|v| v.as_str()).unwrap_or(""),
                        k.get("low").and_then(|v| v.as_str()).unwrap_or(""),
                        k.get("close").and_then(|v| v.as_str()).unwrap_or(""),
                        k.get("volume").and_then(|v| v.as_str()).unwrap_or(""));
                }
                println!();
            }
            Jin10Action::Flash { cursor } => {
                let data = client.list_flash(cursor.as_deref()).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let items = data.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let nc = data.get("next_cursor").and_then(|v| v.as_str()).unwrap_or("");
                println!("\n  \x1b[36mFlash News\x1b[0m ({} items)", items.len());
                if !nc.is_empty() {
                    println!("  Next cursor: {}", nc);
                }
                println!();
                for item in &items {
                    let ts = item.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                    let content = item.get("content").or(item.get("title")).and_then(|v| v.as_str()).unwrap_or("").chars().take(80).collect::<String>();
                    println!("  [{}] {}", ts, content);
                }
                println!();
            }
            Jin10Action::SearchFlash { keyword } => {
                let data = client.search_flash(keyword).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let items = data.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                println!("\n  \x1b[36mFlash News: {}\x1b[0m ({} results)", keyword, items.len());
                println!();
                for item in &items {
                    let ts = item.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                    let content = item.get("content").or(item.get("title")).and_then(|v| v.as_str()).unwrap_or("").chars().take(80).collect::<String>();
                    println!("  [{}] {}", ts, content);
                }
                println!();
            }
            Jin10Action::News { cursor } => {
                let data = client.list_news(cursor.as_deref()).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let items = data.get("items").or(data.get("data")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let nc = data.get("next_cursor").and_then(|v| v.as_str()).unwrap_or("");
                println!("\n  \x1b[36mNews\x1b[0m ({} items)", items.len());
                if !nc.is_empty() {
                    println!("  Next cursor: {}", nc);
                }
                println!();
                for item in &items {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").chars().take(70).collect::<String>();
                    let ts = item.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                    let intro = item.get("introduction").and_then(|v| v.as_str()).unwrap_or("").chars().take(60).collect::<String>();
                    println!("  [{}] {}", id, title);
                    println!("       {} | {}", ts, intro);
                }
                println!();
            }
            Jin10Action::SearchNews { keyword, cursor } => {
                let data = client.search_news(keyword, cursor.as_deref()).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let items = data.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let nc = data.get("next_cursor").and_then(|v| v.as_str()).unwrap_or("");
                println!("\n  \x1b[36mNews: {}\x1b[0m ({} results)", keyword, items.len());
                if !nc.is_empty() {
                    println!("  Next cursor: {}", nc);
                }
                println!();
                for item in &items {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").chars().take(70).collect::<String>();
                    let ts = item.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                    println!("  [{}] {}", id, title);
                    if !ts.is_empty() {
                        println!("       {}", ts);
                    }
                }
                println!();
            }
            Jin10Action::NewsDetail { id } => {
                let data = client.get_news(id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("News Detail");
                let ts = data.get("time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                println!("\n  \x1b[36m{}\x1b[0m", title);
                println!("  ID: {}  |  Time: {}", id, ts);
                println!("  URL: {}", data.get("url").and_then(|v| v.as_str()).unwrap_or(""));
                let intro = data.get("introduction").and_then(|v| v.as_str()).unwrap_or("");
                let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                if !intro.is_empty() {
                    println!("\n  {}", intro);
                }
                if !content.is_empty() {
                    println!("\n  {}", content);
                }
            }
            Jin10Action::Calendar => {
                let data = client.list_calendar().await.map_err(|e| anyhow::anyhow!("{}", e))?;
                let items = data.get("items").or(data.get("data")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                println!("\n  \x1b[36mEconomic Calendar\x1b[0m ({} items)", items.len());
                println!();
                for item in &items {
                    let ts = item.get("pub_time").and_then(|v| v.as_str()).unwrap_or("").chars().take(16).collect::<String>();
                    let stars = item.get("star").and_then(|v| v.as_i64()).unwrap_or(0);
                    let star_str = "⭐".repeat(stars as usize);
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    println!("  [{}] {} {}", ts, star_str, title);
                    println!("       Previous: {}  |  Consensus: {}  |  Actual: {}",
                        item.get("previous").and_then(|v| v.as_str()).unwrap_or("-"),
                        item.get("consensus").and_then(|v| v.as_str()).unwrap_or("-"),
                        item.get("actual").and_then(|v| v.as_str()).unwrap_or("-"));
                    if let Some(affect) = item.get("affect_txt").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        println!("       Impact: {}", affect);
                    }
                }
                println!();
            }
            Jin10Action::Symbols => {
                let symbols = client.list_symbols().await.map_err(|e| anyhow::anyhow!("{}", e))?;
                println!("\n  \x1b[36mSupported Symbols\x1b[0m");
                println!();
                for s in &symbols {
                    println!("  {:<10} {}",
                        s.get("code").and_then(|v| v.as_str()).unwrap_or("?"),
                        s.get("name").and_then(|v| v.as_str()).unwrap_or(""));
                }
                println!();
            }
        }
        Ok(())
    })
}