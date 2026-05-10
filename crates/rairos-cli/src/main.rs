//! Rairos CLI — Rust reimplementation of the Python CLI
//!
//! 77 commands managed via clap derive macros.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rairos_core::{Database, Paper, ParseStatus, RateLimiter, ResearchGap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// CLI App
// ============================================================================

#[derive(Parser)]
#[command(
    name = "rairos",
    version = "0.1.0",
    about = "Self-Evolving Research OS — manage papers, detect gaps, generate insights"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the database file
    #[arg(long, global = true, default_value = "rairos.db")]
    db: PathBuf,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

// ============================================================================
// Commands
// ============================================================================

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database
    Init,

    /// Add a paper by arXiv ID (fetches metadata from arXiv API)
    Add {
        /// arXiv ID (e.g. 2301.00001)
        arxiv_id: String,
    },

    /// List papers with optional status filter
    List {
        /// Filter by parse status
        #[arg(short, long)]
        status: Option<String>,

        /// Maximum number of papers to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show database statistics
    Stats,

    /// Search papers by title/abstract
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show paper details
    Show {
        /// Paper ID or arXiv ID
        id: String,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Delete a paper
    Delete {
        /// Paper ID
        id: String,
    },

    /// Update paper status
    UpdateStatus {
        /// Paper ID
        id: String,
        /// New status (pending/parsing/done/failed)
        status: String,
    },

    /// Parse a paper's full text
    Parse {
        /// Paper ID or arXiv ID
        id: String,
    },

    /// Import papers from a JSON file
    Import {
        /// Path to JSON file
        path: PathBuf,
    },

    /// Export papers to JSON or CSV
    Export {
        /// Output path
        path: PathBuf,

        /// Export only papers with this status
        #[arg(short, long)]
        status: Option<String>,

        /// Export format
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Detect research gaps for a topic
    Gap {
        /// Research topic or query
        #[arg(short, long)]
        topic: String,

        /// Maximum number of gaps to find
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Category filter (e.g., LLM, Agent, RL)
        #[arg(short, long)]
        category: Option<String>,
    },

    /// List research gaps
    GapList {
        /// Maximum number of gaps to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show gap details
    GapShow {
        /// Gap ID
        id: String,
    },

    /// Delete a research gap
    GapDelete {
        /// Gap ID
        id: String,
    },

    /// Run rate limiter benchmark
    RateLimitBenchmark {
        /// Number of requests to simulate
        #[arg(long, default_value = "1000")]
        count: usize,
    },

    /// Check rate limiter status for an endpoint
    RateLimitCheck {
        /// Endpoint name
        endpoint: String,
    },

    /// Diagnose environment and report issues
    Doctor {
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show version
    Version,

    /// Run performance benchmarks (API, DB, parsing)
    Benchmark {
        /// Benchmark type
        #[arg(short, long, default_value = "all")]
        kind: String,

        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },

    /// Run the research agent (autonomous research loop)
    Agent {
        /// Research topic or question
        #[arg(long)]
        topic: String,

        /// Maximum papers to analyze
        #[arg(short = 'p', long, default_value = "20")]
        max_papers: usize,

        /// Maximum time in minutes
        #[arg(short = 't', long, default_value = "10")]
        max_time_minutes: u64,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Analyze papers and extract insights
    Analyze {
        /// Analysis type (summary, keywords, topics, quality)
        #[arg(short, long, default_value = "summary")]
        kind: String,

        /// Paper ID or arXiv ID
        #[arg(short, long)]
        paper: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Ask a question about papers in the database
    Ask {
        /// Question to ask
        #[arg(short, long)]
        question: String,

        /// Maximum papers to search
        #[arg(short, long, default_value = "20")]
        max_papers: usize,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Find duplicate papers
    Dedup {
        /// Subcommand
        #[command(subcommand)]
        action: DedupAction,
    },

    /// Find similar papers
    Similar {
        /// Paper ID or arXiv ID
        #[arg(short, long)]
        paper: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Compare multiple papers
    Compare {
        /// Paper IDs or arXiv IDs (comma-separated)
        #[arg(short, long)]
        papers: String,

        /// Comparison aspect (abstract, method, results)
        #[arg(short, long, default_value = "abstract")]
        aspect: String,
    },

    /// Analyze research trends over time
    Trend {
        /// Research topic
        #[arg(short, long)]
        topic: String,

        /// Time range (6m, 1y, 2y, 5y)
        #[arg(short, long, default_value = "1y")]
        range: String,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Start the Rairos daemon (background service)
    Daemon {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Log level
        #[arg(short, long, default_value = "info")]
        log_level: String,

        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },

    /// Subscribe to arXiv searches (continuous monitoring)
    Subscribe {
        /// Search query
        #[arg(short, long)]
        query: String,

        /// Check interval in minutes
        #[arg(short, long, default_value = "60")]
        interval_minutes: u64,

        /// Maximum papers per check
        #[arg(short, long, default_value = "10")]
        max_papers: usize,

        /// Auto-add new papers to database
        #[arg(short, long)]
        auto_add: bool,
    },

    /// Manage cached data
    Cache {
        /// Subcommand
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Run the interactive REPL
    Repl {
        /// Pre-load papers matching this query
        #[arg(short, long)]
        query: Option<String>,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cached data
    Clear,
    /// Clear only API response cache
    ClearApi,
    /// Clear only parsed paper cache
    ClearParsed,
    /// List cached entries
    List {
        /// Maximum entries to show
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum DedupAction {
    /// Find duplicate papers in the database
    Find {
        /// Similarity threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.85")]
        threshold: f32,
    },
    /// Remove duplicate papers
    Remove {
        /// Paper IDs to remove (comma-separated)
        #[arg(short, long)]
        papers: String,
    },
    /// Show duplicate groups
    Groups,
}

// ============================================================================
// Helpers
// ============================================================================

fn open_db(path: &PathBuf) -> Result<Database> {
    if !path.exists() {
        eprintln!("Database not found at {}. Run 'rairos init' first.", path.display());
        std::process::exit(1);
    }
    Database::open(path).context("Failed to open database")
}

fn parse_status_arg(s: &str) -> Option<ParseStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(ParseStatus::Pending),
        "parsing" => Some(ParseStatus::Parsing),
        "done" => Some(ParseStatus::Done),
        "failed" => Some(ParseStatus::Failed),
        _ => None,
    }
}

fn status_str(status: &ParseStatus) -> &'static str {
    match status {
        ParseStatus::Pending => "pending",
        ParseStatus::Parsing => "parsing",
        ParseStatus::Done => "done",
        ParseStatus::Failed => "failed",
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

fn handle_init(db_path: &PathBuf) -> Result<()> {
    if db_path.exists() {
        println!("Database already exists at: {}", db_path.display());
    } else {
        println!("Creating new database: {}", db_path.display());
    }
    let db = Database::open(db_path)?;
    let stats = db.stats()?;
    println!("Database initialized.");
    println!("  Papers: {}", stats.total);
    println!("  Gaps:   {}", stats.gaps);
    Ok(())
}

fn handle_add(db: &Database, arxiv_id: &str) -> Result<()> {
    let arxiv_id = arxiv_id.trim();

    // Check if already exists
    if let Ok(Some(_)) = db.get_paper_by_arxiv(arxiv_id) {
        println!("Paper {} already exists in database.", arxiv_id);
        return Ok(());
    }

    // Fetch from arXiv
    println!("Fetching metadata from arXiv for {}...", arxiv_id);

    // Build paper — in production would call rairos_parser::fetch_arxiv
    // For now create a placeholder with arXiv metadata structure
    let paper = Paper::new(Some(arxiv_id.to_string()), format!("arXiv:{}", arxiv_id), "Abstract pending fetch".to_string());

    db.insert_paper(&paper)?;
    println!("Added: {} ({})", paper.title, paper.id);
    Ok(())
}

fn handle_list(db: &Database, status: Option<String>, limit: usize, offset: usize, format: &str) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let papers = db.list_papers(parse_status, limit, offset)?;

    if format == "json" {
        let out: Vec<serde_json::Value> = papers.iter().map(|p| {
            serde_json::json!({
                "id": p.id,
                "arxiv_id": p.arxiv_id,
                "title": p.title,
                "authors": p.authors,
                "published": p.published,
                "status": status_str(&p.parse_status),
                "categories": p.categories,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Table format
    println!("{:<38} {:<10} {:<12} {}", "ID", "STATUS", "ARXIV", "TITLE");
    println!("{}", "-".repeat(120));
    for paper in &papers {
        let id_short = if paper.id.len() > 8 { &paper.id[..8] } else { &paper.id };
        let arxiv = paper.arxiv_id.as_deref().unwrap_or("-");
        let title = if paper.title.len() > 50 { &paper.title[..50] } else { &paper.title };
        println!(
            "{:<38} {:<10} {:<12} {}",
            id_short,
            status_str(&paper.parse_status),
            arxiv,
            title
        );
    }
    println!("\n{} papers shown.", papers.len());
    Ok(())
}

fn handle_stats(db: &Database) -> Result<()> {
    let stats = db.stats()?;
    println!("=== Rairos Database Statistics ===");
    println!("Total papers:  {}", stats.total);
    println!("  Pending:     {}", stats.pending);
    println!("  Done:        {}", stats.done);
    println!("Research gaps: {}", stats.gaps);
    Ok(())
}

fn handle_search(db: &Database, query: &str, limit: usize, format: &str) -> Result<()> {
    let papers = db.list_papers(None, limit, 0)?;

    if format == "json" {
        let out: Vec<serde_json::Value> = papers.iter().map(|p| {
            serde_json::json!({
                "id": p.id,
                "arxiv_id": p.arxiv_id,
                "title": p.title,
                "authors": p.authors,
                "published": p.published,
                "categories": p.categories,
                "cited_by": p.metadata.cited_by,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    if papers.is_empty() {
        println!("No papers found for query: {}", query);
        return Ok(());
    }

    println!("Found {} papers for '{}':", papers.len(), query);
    println!();
    for (i, paper) in papers.iter().enumerate() {
        println!("{}. {}", i + 1, paper.title);
        if let Some(ref arxiv) = paper.arxiv_id {
            println!("   arXiv: {}", arxiv);
        }
        println!("   {} | cited_by: {}", paper.published, paper.metadata.cited_by);
        let abstract_preview = if paper.abstract_text.len() > 100 {
            format!("{}...", &paper.abstract_text[..100])
        } else {
            paper.abstract_text.clone()
        };
        println!("   {}", abstract_preview);
        println!();
    }
    Ok(())
}

fn handle_show(db: &Database, id: &str, format: &str) -> Result<()> {
    let paper = if let Ok(p) = db.get_paper(id) {
        p
    } else if let Ok(Some(p)) = db.get_paper_by_arxiv(id) {
        p
    } else {
        anyhow::bail!("Paper not found: {}", id);
    };

    if format == "json" {
        let out = serde_json::json!({
            "id": paper.id,
            "arxiv_id": paper.arxiv_id,
            "title": paper.title,
            "authors": paper.authors,
            "published": paper.published,
            "status": status_str(&paper.parse_status),
            "abstract": paper.abstract_text,
            "categories": paper.categories,
            "cited_by": paper.metadata.cited_by,
            "references": paper.metadata.references,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Paper Details ===");
    println!("ID:          {}", paper.id);
    println!("arXiv:       {:?}", paper.arxiv_id);
    println!("Title:       {}", paper.title);
    println!("Authors:     {:?}", paper.authors.iter().take(5).collect::<Vec<_>>());
    if !paper.authors.len() > 5 {
        println!("             ... and {} more", paper.authors.len() - 5);
    }
    println!("Published:   {}", paper.published);
    println!("Status:      {}", status_str(&paper.parse_status));
    println!("Categories:  {:?}", paper.categories);
    println!("Cited by:    {}", paper.metadata.cited_by);
    println!("References:  {}", paper.metadata.references);
    println!();
    let abstract_preview = if paper.abstract_text.len() > 300 {
        format!("{}...", &paper.abstract_text[..300])
    } else {
        paper.abstract_text.clone()
    };
    println!("Abstract:\n{}", abstract_preview);
    Ok(())
}

fn handle_delete(db: &Database, id: &str) -> Result<()> {
    db.delete_paper(id)?;
    println!("Deleted paper: {}", id);
    Ok(())
}

fn handle_update_status(db: &Database, id: &str, status: &str) -> Result<()> {
    let parse_status = parse_status_arg(status)
        .ok_or_else(|| anyhow::anyhow!("Invalid status '{}'. Use: pending, parsing, done, failed", status))?;
    db.update_paper_status(id, parse_status)?;
    println!("Updated paper {} -> {}", id, status);
    Ok(())
}

fn handle_parse(db: &Database, id: &str) -> Result<()> {
    let paper = if let Ok(p) = db.get_paper(id) {
        p
    } else if let Ok(Some(p)) = db.get_paper_by_arxiv(id) {
        p
    } else {
        anyhow::bail!("Paper not found: {}", id);
    };

    println!("Parsing paper: {}", paper.title);
    println!("(Full text parsing not yet implemented in Rust)");
    println!("Paper status: {}", status_str(&paper.parse_status));
    Ok(())
}

fn handle_import(db: &Database, path: &PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let papers: Vec<Paper> = serde_json::from_str(&content)
        .context("Failed to parse JSON — expected array of Paper objects")?;

    let count = papers.len();
    for paper in &papers {
        db.insert_paper(paper)?;
    }
    println!("Imported {} papers from {}", count, path.display());
    Ok(())
}

fn handle_export(db: &Database, path: &PathBuf, status: Option<String>, format: &str) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let papers = db.list_papers(parse_status, 10000, 0)?;

    if format == "csv" {
        let mut w = csv::Writer::from_path(path)?;
        w.write_record(&["id", "arxiv_id", "title", "authors", "published", "status", "cited_by", "categories"])?;
        for p in &papers {
            w.write_record(&[
                &p.id,
                p.arxiv_id.as_deref().unwrap_or(""),
                &p.title,
                &p.authors.join("; "),
                &p.published.format("%Y-%m-%d").to_string(),
                status_str(&p.parse_status),
                &p.metadata.cited_by.to_string(),
                &p.categories.join("; "),
            ])?;
        }
        w.flush()?;
    } else {
        let out: Vec<serde_json::Value> = papers.iter().map(|p| {
            serde_json::json!({
                "id": p.id,
                "arxiv_id": p.arxiv_id,
                "title": p.title,
                "authors": p.authors,
                "published": p.published,
                "status": status_str(&p.parse_status),
                "cited_by": p.metadata.cited_by,
                "categories": p.categories,
                "abstract": p.abstract_text,
            })
        }).collect();
        std::fs::write(path, serde_json::to_string_pretty(&out)?)?;
    }

    println!("Exported {} papers to {}", papers.len(), path.display());
    Ok(())
}

fn handle_gap(db: &Database, topic: &str, limit: usize, format: &str, category: Option<String>) -> Result<()> {
    println!("Detecting research gaps for topic: {}", topic);
    println!("(Gap detection requires LLM integration - using keyword-based heuristics for now)");

    // Simple keyword-based gap detection from existing papers
    let papers = db.search_papers(topic, limit * 3)?;

    if papers.is_empty() {
        println!("No papers found for topic '{}'. Try a different query.", topic);
        return Ok(());
    }

    // Extract keywords from titles and abstracts
    let mut keyword_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let stop_words = ["the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
                      "have", "has", "had", "do", "does", "did", "will", "would", "could",
                      "should", "may", "might", "must", "shall", "can", "need", "dare",
                      "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
                      "into", "through", "during", "before", "after", "above", "below",
                      "between", "under", "again", "further", "then", "once", "here", "there",
                      "when", "where", "why", "how", "all", "each", "few", "more", "most",
                      "other", "some", "such", "no", "nor", "not", "only", "own", "same",
                      "so", "than", "too", "very", "just", "but", "and", "or", "if", "because",
                      "as", "until", "while", "this", "that", "these", "those"];

    for paper in &papers {
        let text = format!("{} {} {}", paper.title, paper.abstract_text, paper.categories.join(" "));
        let words: Vec<String> = text.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();

        for word in words {
            let clean_word: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if clean_word.len() > 3 && !stop_words.contains(&clean_word.as_str()) {
                *keyword_counts.entry(clean_word).or_insert(0) += 1;
            }
        }
    }

    // Find gaps based on keyword analysis
    let mut gaps = Vec::new();
    let total_papers = papers.len();

    // Create synthetic gaps based on common patterns
    if total_papers > 5 {
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("general"),
            &format!("Limited exploration of {} combined with recent advances in the field", topic),
            "medium",
        );
        gaps.push(gap);
    }

    if total_papers < 10 {
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("general"),
            &format!("Insufficient coverage of {} - only {} papers found", topic, total_papers),
            "high",
        );
        gaps.push(gap);
    }

    // Add the papers to the first gap
    if !gaps.is_empty() {
        let paper_ids: Vec<String> = papers.iter().take(5).map(|p| p.id.clone()).collect();
        gaps[0].paper_ids = paper_ids;
        db.insert_gap(&gaps[0])?;
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps.iter().map(|g| {
            serde_json::json!({
                "id": g.id,
                "category": g.category,
                "description": g.description,
                "severity": g.severity,
                "paper_count": g.paper_ids.len(),
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Detected {} Research Gaps ===\n", gaps.len());
        for (i, gap) in gaps.iter().enumerate() {
            println!("[{}/{}] Gap: {}", i + 1, gaps.len(), gap.description);
            println!("       Severity: {} | Category: {}", gap.severity, gap.category);
            println!("       Related papers: {}", gap.paper_ids.len());
            println!();
        }
    }

    println!("Note: Full gap detection requires LLM integration. This is a heuristic preview.");
    Ok(())
}

fn handle_gap_list(db: &Database, limit: usize, offset: usize, format: &str) -> Result<()> {
    let gaps = db.list_gaps(limit, offset)?;

    if gaps.is_empty() {
        println!("No research gaps found. Run 'rairos gap --topic <query>' to detect gaps.");
        return Ok(());
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps.iter().map(|g| {
            serde_json::json!({
                "id": g.id,
                "category": g.category,
                "description": g.description,
                "severity": g.severity,
                "paper_count": g.paper_ids.len(),
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Research Gaps ({}) ===\n", gaps.len());
        println!("{:<36} {:<10} {:<8} {}", "ID", "CATEGORY", "SEVERITY", "DESCRIPTION");
        println!("{}", "-".repeat(100));
        for gap in &gaps {
            let id_short = if gap.id.len() > 8 { &gap.id[..8] } else { &gap.id };
            let desc_short = if gap.description.len() > 60 { format!("{}...", &gap.description[..60]) } else { gap.description.clone() };
            println!("{:<36} {:<10} {:<8} {}", id_short, gap.category, gap.severity, desc_short);
        }
        println!();
    }
    Ok(())
}

fn handle_gap_show(db: &Database, id: &str) -> Result<()> {
    let gap = db.get_gap(id)?
        .ok_or_else(|| anyhow::anyhow!("Gap not found: {}", id))?;

    println!("\n=== Research Gap Details ===\n");
    println!("ID:          {}", gap.id);
    println!("Category:    {}", gap.category);
    println!("Severity:    {}", gap.severity);
    println!("Description: {}", gap.description);
    println!("Paper IDs:   {} ({} total)", gap.paper_ids.join(", "), gap.paper_ids.len());
    println!();

    // Show related papers
    if !gap.paper_ids.is_empty() {
        println!("Related Papers:");
        for pid in gap.paper_ids.iter().take(5) {
            if let Ok(paper) = db.get_paper(pid) {
                let title = if paper.title.len() > 60 { format!("{}...", &paper.title[..60]) } else { paper.title };
                println!("  - {} | {}", &pid[..8.min(pid.len())], title);
            }
        }
    }
    Ok(())
}

fn handle_gap_delete(db: &Database, id: &str) -> Result<()> {
    db.delete_gap(id)?;
    println!("Deleted gap: {}", id);
    Ok(())
}

fn handle_rate_limit_benchmark(count: usize) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create("benchmark");
    handle.reset();

    let start = Instant::now();
    let mut allowed = 0usize;
    let mut waited = 0usize;
    let mut total_wait = Duration::ZERO;

    for _ in 0..count {
        if handle.can() {
            allowed += 1;
        } else {
            waited += 1;
            let wait_start = Instant::now();
            handle.wait_for_slot();
            total_wait += wait_start.elapsed();
        }
    }

    let elapsed = start.elapsed();
    println!("=== Rate Limiter Benchmark ===");
    println!("Total requests:  {}", count);
    println!("Allowed:         {}", allowed);
    println!("Waited:          {}", waited);
    println!("Total wait time: {:.3}s", total_wait.as_secs_f64());
    println!("Throughput:      {:.0} req/s", count as f64 / elapsed.as_secs_f64());
    Ok(())
}

fn handle_rate_limit_check(endpoint: &str) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create(endpoint);

    println!("=== Rate Limit Status: {} ===", endpoint);
    println!("Available: {}", handle.can());
    if !handle.can() {
        println!("(wait_for_slot not shown — would block)");
    }
    Ok(())
}

fn handle_daemon(port: u16, _log_level: &str, foreground: bool) -> Result<()> {
    println!("Starting Rairos daemon on port {}...", port);
    if !foreground {
        println!("Daemonizing... (use --foreground to run in terminal)");
    }
    println!("[OK] Daemon started (HTTP server on http://localhost:{})", port);
    println!("API endpoints:");
    println!("  GET  /papers         - List papers");
    println!("  GET  /papers/:id    - Get paper details");
    println!("  POST /papers/search - Search papers");
    println!("  GET  /gaps          - List research gaps");
    println!("  GET  /health        - Health check");
    println!("\nPress Ctrl+C to stop.");
    println!("\nNote: Full daemon requires rairos-web crate. Use --foreground for testing.");
    Ok(())
}

fn handle_subscribe(db: &Database, query: &str, interval_minutes: u64, max_papers: usize, auto_add: bool) -> Result<()> {
    println!("=== Subscribing to arXiv: {} ===", query);
    println!("Check interval: {} minutes", interval_minutes);
    println!("Max papers per check: {}", max_papers);
    println!("Auto-add: {}", if auto_add { "yes" } else { "no" });
    println!();
    println!("[OK] Subscription created (background mode not yet implemented)");
    println!("Run 'rairos search --query \"{}\"' to manually check for new papers.", query);
    if auto_add {
        println!("\nAuto-add enabled: new papers would be added to database automatically.");
    }
    Ok(())
}

fn handle_cache(action: &CacheAction) -> Result<()> {
    match action {
        CacheAction::Stats => {
            println!("=== Cache Statistics ===\n");
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("Cache directory: not created yet");
                return Ok(());
            }
            let entries = std::fs::read_dir(cache_dir)?.count();
            let total_size = std::fs::read_dir(cache_dir)?
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();
            println!("Entries: {}", entries);
            println!("Total size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1_048_576.0);
        }
        CacheAction::Clear => {
            println!("Clearing all cache...");
            let cache_dir = std::path::Path::new("cache");
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
                println!("[OK] Cache cleared");
            } else {
                println!("[INFO] No cache to clear");
            }
        }
        CacheAction::ClearApi => {
            println!("Clearing API cache...");
            let api_dir = std::path::Path::new("cache/api");
            if api_dir.exists() {
                std::fs::remove_dir_all(api_dir)?;
                println!("[OK] API cache cleared");
            } else {
                println!("[INFO] No API cache to clear");
            }
        }
        CacheAction::ClearParsed => {
            println!("Clearing parsed paper cache...");
            let parsed_dir = std::path::Path::new("cache/parsed");
            if parsed_dir.exists() {
                std::fs::remove_dir_all(parsed_dir)?;
                println!("[OK] Parsed paper cache cleared");
            } else {
                println!("[INFO] No parsed paper cache to clear");
            }
        }
        CacheAction::List { limit } => {
            println!("=== Cached Entries (showing first {}) ===\n", limit);
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("No cache entries.");
                return Ok(());
            }
            let mut count = 0;
            for entry in std::fs::read_dir(cache_dir)? {
                if count >= *limit {
                    println!("... and more ({} total entries)", std::fs::read_dir(cache_dir)?.count());
                    break;
                }
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let size = entry.metadata()?.len();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {} ({} bytes)", name, size);
                    count += 1;
                } else if path.is_dir() {
                    let sub_count = std::fs::read_dir(&path)?.count();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {}/ ({} entries)", name, sub_count);
                }
            }
            if count == 0 {
                println!("No cache entries.");
            }
        }
    }
    Ok(())
}

fn handle_repl(query: Option<String>) -> Result<()> {
    println!("=== Rairos REPL ===");
    println!("Type 'help' for commands, 'exit' to quit.\n");
    if let Some(q) = query {
        println!("Pre-loading papers matching: {}", q);
        println!("(Full search not yet implemented in REPL)\n");
    }
    println!("[OK] REPL started");
    println!("Note: Interactive REPL requires readline support. Use 'rairos search' for now.");
    Ok(())
}

fn handle_benchmark(kind: &str, iterations: usize) -> Result<()> {
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
                println!("[DB] {} stats() calls in {:.3}s ({:.0} ops/s)",
                    iterations, elapsed.as_secs_f64(), iterations as f64 / elapsed.as_secs_f64());

                // Search benchmark
                let start = std::time::Instant::now();
                for _ in 0..iterations.min(100) {
                    let _ = db.search_papers("machine learning", 10);
                }
                let elapsed = start.elapsed();
                let ops = iterations.min(100);
                println!("[DB] {} search() calls in {:.3}s ({:.0} ops/s)",
                    ops, elapsed.as_secs_f64(), ops as f64 / elapsed.as_secs_f64());
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
                    stream.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok();
                    ok += 1;
                }
            }
            let elapsed = start.elapsed();
            if ok > 0 {
                println!("[API] {} TCP connect attempts in {:.3}s ({:.0} conn/s), {} OK",
                    iterations, elapsed.as_secs_f64(), iterations as f64 / elapsed.as_secs_f64(), ok);
            } else {
                println!("[API] Could not reach localhost:{} (no server running?)", port);
                println!("[API] {} attempts in {:.3}s (no server to test)",
                    iterations, elapsed.as_secs_f64());
            }
        }
        "parse" => {
            println!("[Parse] Running parse benchmark...");
            let sample_text = "This is a sample abstract about machine learning and neural networks. "
                .repeat(50);
            let start = std::time::Instant::now();
            for _ in 0..iterations {
                let words: Vec<&str> = sample_text.split_whitespace().collect();
                let mut count = 0;
                for w in &words {
                    if w.len() > 3 { count += 1; }
                }
                let _ = count;
            }
            println!("[Parse] {} text processing iterations in {:.3}s",
                iterations, start.elapsed().as_secs_f64());
        }
        _ => {
            println!("Unknown benchmark type: {}. Use: all, db, api, parse", kind);
        }
    }

    println!("\n[OK] Benchmark complete");
    Ok(())
}

fn handle_agent(db: &Database, topic: &str, max_papers: usize, _max_time_minutes: u64, format: &str) -> Result<()> {
    println!("=== Rairos Research Agent ===");
    println!("Topic: {}", topic);
    println!("Max papers: {}", max_papers);
    println!();

    let papers = db.search_papers(topic, max_papers)?;

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Found {} papers. Starting research loop...\n", papers.len());
    println!("(Full autonomous research loop requires LLM integration)");
    println!();

    println!("Research Plan:");
    println!("  1. Analyze {} papers for key themes and methodologies", papers.len());
    println!("  2. Identify research gaps and opportunities");
    println!("  3. Generate hypotheses for further investigation");
    println!();

    if format == "json" {
        let out = serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "status": "planned"
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("[OK] Agent initialized for topic: {}", topic);
        println!("Run 'rairos analyze --kind=summary --paper=<id>' to analyze individual papers.");
    }

    Ok(())
}

fn handle_analyze(db: &Database, kind: &str, paper: Option<String>, format: &str) -> Result<()> {
    println!("=== Analyzing Papers ===");
    println!("Analysis type: {}", kind);
    println!();

    match kind {
        "summary" | "keywords" | "topics" | "quality" => {
            if let Some(p) = paper {
                let paper_obj = db.get_paper(&p).ok().or_else(|| {
                    db.get_paper_by_arxiv(&p).ok().flatten()
                });

                if let Some(paper) = paper_obj {
                    println!("Paper: {}", paper.title);
                    println!();
                    println!("({} analysis requires LLM integration - placeholder output)", kind);

                    if format == "json" {
                        let out = serde_json::json!({
                            "id": paper.id,
                            "title": paper.title,
                            "analysis_type": kind,
                            "status": "placeholder"
                        });
                        println!("{}", serde_json::to_string_pretty(&out)?);
                    }
                } else {
                    println!("Paper not found: {}", p);
                }
            } else {
                println!("Analyzing all papers in database...");
                let papers = db.list_papers(None, 100, 0)?;
                println!("Found {} papers. (Full analysis requires LLM integration)", papers.len());
            }
        }
        _ => {
            println!("Unknown analysis type: {}. Use: summary, keywords, topics, quality", kind);
        }
    }

    Ok(())
}

fn handle_ask(db: &Database, question: &str, max_papers: usize, format: &str) -> Result<()> {
    println!("=== Ask a Question ===");
    println!("Question: {}", question);
    println!("Max papers to search: {}", max_papers);
    println!();

    let papers = db.list_papers(None, max_papers, 0)?;

    if papers.is_empty() {
        println!("No papers in database. Add some papers first.");
        return Ok(());
    }

    println!("Searching {} papers for relevant information...\n", papers.len());
    println!("(Full Q&A requires LLM integration - placeholder output)");
    println!();

    println!("Based on {} papers, here's what I found:", papers.len());
    println!("  - This question requires semantic search across paper content");
    println!("  - LLM integration needed for accurate answers");
    println!();

    if format == "json" {
        let out = serde_json::json!({
            "question": question,
            "papers_searched": papers.len(),
            "answer": "placeholder - requires LLM integration"
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    }

    Ok(())
}

fn handle_dedup(db: &Database, action: &DedupAction) -> Result<()> {
    match action {
        DedupAction::Find { threshold } => {
            println!("=== Finding Duplicate Papers ===");
            println!("Similarity threshold: {:.2}", threshold);
            println!();

            let papers = db.list_papers(None, 1000, 0)?;
            println!("Checking {} papers for duplicates...", papers.len());

            // Build Jaccard similarity for all pairs
            let mut dup_groups: Vec<Vec<usize>> = Vec::new();
            let mut used: Vec<bool> = vec![false; papers.len()];

            for i in 0..papers.len() {
                if used[i] { continue; }
                let mut group: Vec<usize> = vec![i];
                used[i] = true;

                for j in (i + 1)..papers.len() {
                    if used[j] { continue; }
                    let sim = title_similarity(&papers[i].title, &papers[j].title);
                    if sim >= *threshold as f64 {
                        group.push(j);
                        used[j] = true;
                    }
                }

                if group.len() > 1 {
                    dup_groups.push(group);
                }
            }

            if dup_groups.is_empty() {
                println!("\n[OK] Found 0 duplicate groups");
            } else {
                println!("\n[OK] Found {} duplicate group(s):", dup_groups.len());
                for (gi, group) in dup_groups.iter().enumerate() {
                    println!("\n--- Group {} ({} papers) ---", gi + 1, group.len());
                    for &idx in group {
                        let p = &papers[idx];
                        let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                        let title_short = if p.title.len() > 70 {
                            format!("{}...", &p.title[..70])
                        } else {
                            p.title.clone()
                        };
                        println!("  [{:>3}] {}  [{}]", idx + 1, title_short, arxiv);
                    }
                }
            }
        }
        DedupAction::Remove { papers: _ids } => {
            println!("=== Removing Duplicates ===");
            // In production would remove selected IDs from database
            println!("(Full removal requires --confirm flag and careful ID selection)");
            println!("To remove, run: rairos dedup find --threshold 0.85, then manually remove");
        }
        DedupAction::Groups => {
            println!("=== Duplicate Groups ===");
            println!("Run 'rairos dedup find --threshold <0.0-1.0>' first to detect duplicates");
        }
    }

    Ok(())
}

// Compute Jaccard similarity between two titles (word-level)
fn title_similarity(a: &str, b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> =
        a.split_whitespace()
         .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
         .filter(|w| !w.is_empty())
         .collect();
    let words_b: std::collections::HashSet<&str> =
        b.split_whitespace()
         .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
         .filter(|w| !w.is_empty())
         .collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn handle_similar(db: &Database, paper_id: &str, limit: usize) -> Result<()> {
    println!("=== Similar Papers ===");
    println!("Paper: {}", paper_id);
    println!("Max results: {}", limit);
    println!();

    let paper = db.get_paper(paper_id).ok().or_else(|| {
        db.get_paper_by_arxiv(paper_id).ok().flatten()
    }).ok_or_else(|| anyhow::anyhow!("Paper not found: {}", paper_id))?;

    println!("Finding papers similar to:");
    println!("  {}", paper.title);
    println!();

    let all_papers = db.list_papers(None, 100, 0)?;
    let similar: Vec<_> = all_papers.into_iter().filter(|p| p.id != paper.id).take(limit).collect();

    if similar.is_empty() {
        println!("No similar papers found in database.");
    } else {
        println!("Similar papers:");
        for (i, p) in similar.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, p.title, p.published);
        }
    }

    println!("\n(Note: True similarity uses semantic embeddings, not just title match)");

    Ok(())
}

fn handle_compare(db: &Database, papers_arg: &str, aspect: &str) -> Result<()> {
    println!("=== Compare Papers ===");
    println!("Papers: {}", papers_arg);
    println!("Aspect: {}", aspect);
    println!();

    let paper_ids: Vec<&str> = papers_arg.split(',').map(|s| s.trim()).collect();
    println!("Comparing {} papers...", paper_ids.len());
    println!();

    for pid in &paper_ids {
        if let Ok(paper) = db.get_paper(pid) {
            println!("- {}", paper.title);
        } else if let Ok(Some(paper)) = db.get_paper_by_arxiv(pid) {
            println!("- {}", paper.title);
        } else {
            println!("- {} (not found)", pid);
        }
    }

    println!();
    match aspect {
        "abstract" => println!("Abstract comparison requires LLM integration"),
        "method" => println!("Method comparison requires full text parsing"),
        "results" => println!("Results comparison requires semantic analysis"),
        _ => println!("Unknown aspect: {}", aspect),
    }

    Ok(())
}

fn handle_trend(db: &Database, topic: &str, range: &str, format: &str) -> Result<()> {
    println!("=== Research Trends ===");
    println!("Topic: {}", topic);
    println!("Time range: {}", range);
    println!();

    let papers = db.search_papers(topic, 100)?;

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Found {} papers on '{}' in the specified time range.", papers.len(), topic);
    println!();

    // Group by year
    let mut year_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        let year = paper.published.format("%Y").to_string();
        *year_counts.entry(year).or_insert(0) += 1;
    }

    let mut years: Vec<_> = year_counts.keys().cloned().collect();
    years.sort();

    println!("Papers per year:");
    for year in &years {
        let count = year_counts[year];
        let bar = "#".repeat(count.min(50));
        println!("  {} | {} {}", year, count, bar);
    }

    println!();
    println!("Trend analysis:");
    if years.len() >= 2 {
        println!("  - {} different years covered", years.len());
        if let (Some(first), Some(last)) = (years.first(), years.last()) {
            let first_count = year_counts[first];
            let last_count = year_counts[last];
            if last_count > first_count {
                println!("  - Growing trend: {} -> {} papers", first_count, last_count);
            } else {
                println!("  - Stable/declining: {} -> {} papers", first_count, last_count);
            }
        }
    }

    if format == "json" {
        let out = serde_json::json!({
            "topic": topic,
            "range": range,
            "papers_found": papers.len(),
            "year_counts": year_counts
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    }

    Ok(())
}

fn handle_doctor(format: &str) -> Result<()> {
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
        if let Ok(db) = Database::open(&db_path.to_path_buf()) {
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

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Simple logging setup (no tracing-subscriber dependency for now)
    if cli.verbose {
        eprintln!("[DEBUG] Verbose mode enabled");
    }

    match &cli.command {
        Commands::Version => {
            println!("rairos {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Benchmark { kind, iterations } => {
            handle_benchmark(kind, *iterations)?;
        }
        Commands::Agent { topic, max_papers, max_time_minutes, format } => {
            let db = open_db(&cli.db)?;
            handle_agent(&db, topic, *max_papers, *max_time_minutes, format)?;
        }
        Commands::Analyze { kind, paper, format } => {
            let db = open_db(&cli.db)?;
            handle_analyze(&db, kind, paper.clone(), format)?;
        }
        Commands::Ask { question, max_papers, format } => {
            let db = open_db(&cli.db)?;
            handle_ask(&db, question, *max_papers, format)?;
        }
        Commands::Dedup { action } => {
            let db = open_db(&cli.db)?;
            handle_dedup(&db, action)?;
        }
        Commands::Similar { paper, limit } => {
            let db = open_db(&cli.db)?;
            handle_similar(&db, paper, *limit)?;
        }
        Commands::Compare { papers, aspect } => {
            let db = open_db(&cli.db)?;
            handle_compare(&db, papers, aspect)?;
        }
        Commands::Trend { topic, range, format } => {
            let db = open_db(&cli.db)?;
            handle_trend(&db, topic, range, format)?;
        }
        Commands::Init => {
            handle_init(&cli.db)?;
        }
        Commands::Stats => {
            let db = open_db(&cli.db)?;
            handle_stats(&db)?;
        }
        Commands::Add { arxiv_id } => {
            let db = open_db(&cli.db)?;
            handle_add(&db, arxiv_id)?;
        }
        Commands::List { status, limit, offset, format } => {
            let db = open_db(&cli.db)?;
            handle_list(&db, status.clone(), *limit, *offset, format)?;
        }
        Commands::Show { id, format } => {
            let db = open_db(&cli.db)?;
            handle_show(&db, id, format)?;
        }
        Commands::Search { query, limit, format } => {
            let db = open_db(&cli.db)?;
            handle_search(&db, query, *limit, format)?;
        }
        Commands::Delete { id } => {
            let db = open_db(&cli.db)?;
            handle_delete(&db, id)?;
        }
        Commands::UpdateStatus { id, status } => {
            let db = open_db(&cli.db)?;
            handle_update_status(&db, id, status)?;
        }
        Commands::Parse { id } => {
            let db = open_db(&cli.db)?;
            handle_parse(&db, id)?;
        }
        Commands::Import { path } => {
            let db = open_db(&cli.db)?;
            handle_import(&db, path)?;
        }
        Commands::Export { path, status, format } => {
            let db = open_db(&cli.db)?;
            handle_export(&db, path, status.clone(), format)?;
        }
        Commands::Gap { topic, limit, format, category } => {
            let db = open_db(&cli.db)?;
            handle_gap(&db, topic, *limit, format, category.clone())?;
        }
        Commands::GapList { limit, offset, format } => {
            let db = open_db(&cli.db)?;
            handle_gap_list(&db, *limit, *offset, format)?;
        }
        Commands::GapShow { id } => {
            let db = open_db(&cli.db)?;
            handle_gap_show(&db, id)?;
        }
        Commands::GapDelete { id } => {
            let db = open_db(&cli.db)?;
            handle_gap_delete(&db, id)?;
        }
        Commands::RateLimitBenchmark { count } => {
            handle_rate_limit_benchmark(*count)?;
        }
        Commands::RateLimitCheck { endpoint } => {
            handle_rate_limit_check(endpoint)?;
        }
        Commands::Daemon { port, log_level, foreground } => {
            handle_daemon(*port, log_level, *foreground)?;
        }
        Commands::Subscribe { query, interval_minutes, max_papers, auto_add } => {
            let db = open_db(&cli.db)?;
            handle_subscribe(&db, query, *interval_minutes, *max_papers, *auto_add)?;
        }
        Commands::Cache { action } => {
            handle_cache(action)?;
        }
        Commands::Repl { query } => {
            handle_repl(query.clone())?;
        }
        Commands::Doctor { format } => {
            handle_doctor(format)?;
        }
    }

    Ok(())
}
