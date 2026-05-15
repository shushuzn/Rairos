//! CLI command handler implementations.
//!
//! Extracted from main.rs for maintainability. Each handler
//! corresponds to one Commands variant from the parent module.

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
use chrono::Datelike;
use rairos_core::{Database, ParseStatus};
use rairos_mcp_jin10::Jin10Client;
use std::sync::Arc;

use rairos_web::{start, AppState};
use crate::Jin10Action;


// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

pub fn handle_status(db: &Database, format: &str) -> Result<()> {
    let papers = db.list_papers(None, 10000, 0)?;
    let total = papers.len();

    let mut by_status: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for p in &papers {
        let status = p.parse_status.to_string();
        *by_status.entry(status).or_default() += 1;
    }

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total_papers": total,
                "by_status": by_status,
            }))?
        );
        return Ok(());
    }

    println!("Total papers: {}", total);
    println!(
        "By status: {}",
        by_status
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

pub fn handle_daemon(db: &Database, port: u16, _log_level: &str, _foreground: bool) -> Result<()> {
    println!("Starting Rairos web server on port {}...", port);
    println!();
    println!("API endpoints:");
    println!("  GET  /              - Web UI");
    println!("  GET  /health        - Health check");
    println!("  GET  /stats         - Database stats");
    println!("  GET  /papers        - List papers");
    println!("  GET  /papers/:id    - Get paper details");
    println!("  GET  /papers/search - Search papers");
    println!("  GET  /gaps          - List research gaps");
    println!("  GET  /genes         - List gene pool");
    println!("  GET  /genes/diversity - Gene diversity metrics");
    println!("  GET  /kg/stats      - Knowledge graph stats");
    println!("  GET  /kg/rank       - Paper rankings");
    println!();
    println!("Press Ctrl+C to stop.");
    println!();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let state = Arc::new(AppState::new(db.clone()));
        start(&format!("127.0.0.1:{}", port), state).await
    })?;
    Ok(())
}

pub fn handle_benchmark(kind: &str, iterations: usize) -> Result<()> {
    println!("=== Rairos Benchmark ===");
    println!("Type: {} | Iterations: {}\n", kind, iterations);

    match kind {
        "all" | "db" => {
            println!("[DB] Running database benchmark...");
            let db_path = std::path::PathBuf::from("rairos.db");
            if !db_path.exists() {
                println!("No database at rairos.db, skipping DB benchmark");
            } else if let Ok(db) = Database::open(&db_path) {
                let start = std::time::Instant::now();
                for _ in 0..iterations {
                    let _ = db.stats();
                }
                let elapsed = start.elapsed();
                println!(
                    "[DB] {} stats() calls in {:.3}s ({:.0} ops/s)",
                    iterations,
                    elapsed.as_secs_f64(),
                    iterations as f64 / elapsed.as_secs_f64()
                );

                // Search benchmark
                let start = std::time::Instant::now();
                for _ in 0..iterations.min(100) {
                    let _ = db.search_papers("machine learning", 10);
                }
                let elapsed = start.elapsed();
                let ops = iterations.min(100);
                println!(
                    "[DB] {} search() calls in {:.3}s ({:.0} ops/s)",
                    ops,
                    elapsed.as_secs_f64(),
                    ops as f64 / elapsed.as_secs_f64()
                );
            } else {
                println!("Could not open database");
            }
        }
        "api" => {
            println!("[API] Running API benchmark (measuring TCP connection latency)...");
            let port = 8080u16;
            let start = std::time::Instant::now();
            let mut ok = 0;
            for _ in 0..iterations {
                if let Ok(stream) = std::net::TcpStream::connect_timeout(
                    &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    std::time::Duration::from_millis(500),
                ) {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
                        .ok();
                    ok += 1;
                }
            }
            let elapsed = start.elapsed();
            if ok > 0 {
                println!(
                    "[API] {} TCP connect attempts in {:.3}s ({:.0} conn/s), {} OK",
                    iterations,
                    elapsed.as_secs_f64(),
                    iterations as f64 / elapsed.as_secs_f64(),
                    ok
                );
            } else {
                println!(
                    "[API] Could not reach localhost:{} (no server running?)",
                    port
                );
                println!(
                    "[API] {} attempts in {:.3}s (no server to test)",
                    iterations,
                    elapsed.as_secs_f64()
                );
            }
        }
        "parse" => {
            println!("[Parse] Running parse benchmark...");
            let sample_text =
                "This is a sample abstract about machine learning and neural networks. ".repeat(50);
            let start = std::time::Instant::now();
            for _ in 0..iterations {
                let words: Vec<&str> = sample_text.split_whitespace().collect();
                let mut count = 0;
                for w in &words {
                    if w.len() > 3 {
                        count += 1;
                    }
                }
                let _ = count;
            }
            println!(
                "[Parse] {} text processing iterations in {:.3}s",
                iterations,
                start.elapsed().as_secs_f64()
            );
        }
        _ => {
            println!("Unknown benchmark type: {}. Use: all, db, api, parse", kind);
        }
    }

    println!("\n[OK] Benchmark complete");
    Ok(())
}

pub fn handle_doctor(format: &str) -> Result<()> {
    use std::env;
    use std::path::Path;

    let mut ok: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let issues: Vec<String> = Vec::new();

    // Python
    let ver = env!("CARGO_PKG_VERSION");
    ok.push(format!("rairos-cli {}", ver));

    // Platform
    #[cfg(windows)]
    ok.push("Platform: Windows (MSVC)".to_string());
    #[cfg(not(windows))]
    ok.push("Platform: non-Windows".to_string());

    // Database (check common paths)
    let db_paths = [
        Path::new("rairos.db"),
        Path::new("research.db"),
        Path::new(".ai_research_os/research.db"),
    ];
    let found_db = db_paths.iter().find(|p| p.exists());
    if let Some(db_path) = found_db {
        ok.push(format!("Database: {} exists", db_path.display()));
        // Try to open it
        if let Ok(db) = Database::open(db_path) {
            if let Ok(stats) = db.stats() {
                ok.push(format!("  {} papers, {} gaps", stats.total, stats.gaps));
            }
        }
    } else {
        warnings.push("No database found (run 'rairos init')".to_string());
    }

    // Config files
    for name in &[".env", ".env.example"] {
        if Path::new(name).exists() {
            ok.push(format!("{}: exists", name));
        } else {
            warnings.push(format!("{}: not found", name));
        }
    }

    // Git
    if Path::new(".git").exists() {
        ok.push("Git repository: yes".to_string());
    } else {
        warnings.push("Not a git repository".to_string());
    }

    // API health check (if daemon is running)
    match reqwest::blocking::get("http://127.0.0.1:8080/health") {
        Ok(resp) if resp.status().is_success() => {
            ok.push("API daemon: reachable".to_string());
        }
        Ok(resp) => {
            warnings.push(format!("API daemon: returned {}", resp.status()));
        }
        Err(_e) => {
            warnings.push(
                "API daemon: not reachable (run 'rairos daemon --foreground' to start)".to_string(),
            );
        }
    }

    // Rust toolchain
    ok.push(format!("Rust: {}", env::consts::ARCH));

    if format == "json" {
        let out = serde_json::json!({
            "ok": ok,
            "warnings": warnings,
            "issues": issues,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("=== Rairos Health Check ===");
        println!();

        println!("[OK] Checks passed ({}):", ok.len());
        for item in &ok {
            println!("  ✓ {}", item);
        }

        if !warnings.is_empty() {
            println!();
            println!("[WARN] Warnings ({}):", warnings.len());
            for w in &warnings {
                println!("  ⚠ {}", w);
            }
        }

        if !issues.is_empty() {
            println!();
            println!("[FAIL] Issues ({}):", issues.len());
            for i in &issues {
                println!("  ✗ {}", i);
            }
        }

        println!();
        if issues.is_empty() {
            println!("All checks passed.");
        } else {
            println!("{} issue(s) found.", issues.len());
        }
    }

    Ok(())
}

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

pub fn handle_queue(
    db: &Database,
    add: Option<&str>,
    list: bool,
    pending: bool,
    dequeue: bool,
    cancel: Option<i64>,
    clear: bool,
    format: &str,
) -> Result<()> {
    if list {
        let jobs = db.get_queue_jobs(None, 100)?;
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&jobs)?);
        } else {
            for j in &jobs {
                println!(
                    "[{}] {} ({}) priority={} status={}",
                    j.id, j.paper_id, j.job_type, j.priority, j.status
                );
            }
        }
        if jobs.is_empty() {
            println!("Queue empty");
        }
    } else if pending {
        let all_papers = db.list_papers(Some(ParseStatus::Pending), 200, 0)?;
        if all_papers.is_empty() {
            println!("No pending papers");
        } else {
            println!("{} paper(s) awaiting processing:", all_papers.len());
            for p in &all_papers {
                println!(
                    "  {} [{}]",
                    p.id,
                    p.arxiv_id.as_deref().unwrap_or("no-arxiv")
                );
            }
        }
    } else if dequeue {
        match db.dequeue_job()? {
            Some(job) => println!("Dequeued: {} (id={})", job.paper_id, job.id),
            None => println!("Queue empty"),
        }
    } else if let Some(paper_id) = add {
        db.enqueue_job(paper_id, "parse", 5)?;
        println!("Added {} to queue", paper_id);
    } else if let Some(job_id) = cancel {
        if db.cancel_job(job_id)? {
            println!("Cancelled job {}", job_id);
        } else {
            println!("No such job {}", job_id);
        }
    } else if clear {
        let n = db.clear_pending_papers()?;
        println!("Cleared {} pending paper(s)", n);
    } else {
        println!("Use --list, --dequeue, --add UID, --cancel JOB_ID, or --clear");
    }
    Ok(())
}

pub fn handle_story(db: &Database, topic: Option<&str>) -> Result<()> {
    let Some(topic) = topic else {
        eprintln!("❌ 请提供 topic");
        std::process::exit(1);
    };
    println!("📖 Weaving story for: {}", topic);

    let papers = db.search_papers(topic, 20)?;
    let inputs: Vec<crate::story::PaperInput> = papers
        .iter()
        .map(|p| crate::story::PaperInput {
            id: p.id.clone(),
            title: p.title.clone(),
            abstract_text: p.abstract_text.clone(),
            year: p.published.year(),
        })
        .collect();

    let weaver = crate::story::StoryWeaver;
    let result = weaver.weave(topic, inputs);
    println!("{}", result.summary);
    Ok(())
}

pub fn handle_argue(db: &Database, thesis: &[String]) -> Result<()> {
    let topic_text = if thesis.is_empty() {
        let papers = db.list_papers(None, 1, 0)?;
        if let Some(p) = papers.first() {
            p.title.clone()
        } else {
            "research".to_string()
        }
    } else {
        thesis.join(" ")
    };

    println!("🧠 Building argument for: {}", topic_text);
    println!("{}", rairos_argument_builder::render_argument(
        &rairos_argument_builder::ArgumentResult {
            topic: topic_text.clone(),
            argument: rairos_argument_builder::Argument {
                thesis: topic_text.clone(),
                claims: vec![],
                supporting_evidence: vec![],
                contradicting_evidence: vec![],
                related_gaps: vec![],
                paper_suggestions: vec![],
            },
            summary: String::new(),
            section_guidance: std::collections::HashMap::new(),
        }
    ));
    Ok(())
}

pub fn handle_intel(topic: &str, verbose: bool) -> Result<()> {
    let report = rairos_intelligence::IntelligenceGenerator::generate(topic, verbose);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn handle_dashboard(port: u16, host: &str, _no_browser: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let addr = format!("{}:{}", host, port);
        let db = Database::open("rairos.db")?;
        let state = Arc::new(rairos_web::AppState::new(db));
        println!("🚀 Rairos Web UI starting on http://{}", addr);
        rairos_web::start(&addr, state).await.map_err(|e| anyhow::anyhow!("Web UI failed: {}", e))
    })?;
    Ok(())
}

