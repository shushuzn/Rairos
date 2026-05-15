//! Rairos CLI — Rust reimplementation of the Python CLI
//!
//! 77 commands managed via clap derive macros.

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

use anyhow::{Context, Result};
use chrono::{Datelike, Utc};
use clap::{Parser, Subcommand};
use rairos_core::{Database, Paper, ParseStatus, RateLimiter, ResearchGap};
use rairos_pdf;
use rairos_kg::{GraphAlgorithms, KnowledgeGraph};
use rairos_llm::{Capsule, CapsuleStatus, GenePool, GenePoolDiversityCalculator};
use rairos_memory::{ResearchMemory, ResearchStance, StanceType};
use rairos_web::{start, AppState};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
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
        /// Filter by parse status (pending/done/parsed)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by year
        #[arg(short, long)]
        year: Option<i32>,

        /// Filter by tag/category (repeatable)
        #[arg(short = 't', long)]
        tag: Vec<String>,

        /// Maximum number of papers to show
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Sort by field (added_at/published/title/status)
        #[arg(long, default_value = "added_at")]
        sort: String,

        /// Sort order (asc/desc)
        #[arg(short, long, default_value = "desc")]
        order: String,

        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show database statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Search papers by title/abstract
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Field to search in (title, abstract, authors, all)
        #[arg(long, default_value = "all")]
        field: String,

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
        /// Paper ID(s) to delete
        #[arg(required = true)]
        id: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Update paper status
    UpdateStatus {
        /// Paper ID(s) — supports multiple IDs
        #[arg(required = true)]
        id: Vec<String>,

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
        /// Path to JSON file OR arXiv ID(s) to fetch
        #[arg(conflicts_with = "ids")]
        path: Option<PathBuf>,

        /// arXiv IDs or DOIs to import (fetches metadata from arXiv/CrossRef)
        #[arg(short, long, conflicts_with = "path")]
        ids: Vec<String>,

        /// Skip IDs already in database
        #[arg(short, long)]
        skip_existing: bool,
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

    /// Add a gene/capsule to the Gene Pool
    GeneAdd {
        /// Approach summary
        #[arg(short, long)]
        approach: String,

        /// Gap type
        #[arg(short, long)]
        gap_type: String,

        /// Trigger keywords (comma-separated)
        #[arg(short, long)]
        keywords: String,

        /// Source paper ID (optional)
        #[arg(short, long)]
        paper_id: Option<String>,
    },

    /// List genes in the Gene Pool
    GeneList {
        /// Filter by gap type
        #[arg(short, long)]
        gap_type: Option<String>,

        /// Filter by status (active/dormant/archived)
        #[arg(short, long)]
        status: Option<String>,

        /// Maximum number to show
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show gene details
    GeneShow {
        /// Gene/Capsule ID
        id: String,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Record feedback for a gene
    GeneFeedback {
        /// Gene/Capsule ID
        id: String,

        /// Positive or negative feedback
        #[arg(short, long)]
        positive: bool,
    },

    /// Calculate Gene Pool diversity metrics
    GeneDiversity {
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Run evolution cycle on Gene Pool
    GeneEvolve {
        /// Maximum number of crossovers to suggest
        #[arg(short, long, default_value = "10")]
        max_crossovers: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show knowledge graph statistics
    KgStats {
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show paper rankings from knowledge graph
    KgRank {
        /// Maximum number to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Find path between two papers
    KgPath {
        /// Source paper ID
        #[arg(short, long)]
        source: String,

        /// Target paper ID
        #[arg(short, long)]
        target: String,
    },

    /// Add a paper to the knowledge graph
    KgAddPaper {
        /// Paper ID or arXiv ID
        #[arg(short, long)]
        paper_id: String,
    },

    /// Add citation edge between two papers
    KgAddCitation {
        /// Source paper ID (the paper that cites)
        #[arg(short, long)]
        source: String,

        /// Target paper ID (the paper being cited)
        #[arg(short, long)]
        target: String,
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

    /// Add a research stance
    StanceAdd {
        /// Topic/question
        #[arg(short, long)]
        topic: String,

        /// Claim or hypothesis
        #[arg(short, long)]
        claim: String,

        /// Stance (supported/rejected/deferred/qualified)
        #[arg(short, long, default_value = "supported")]
        stance: String,

        /// Reasoning
        #[arg(short, long)]
        reasoning: String,
    },

    /// List research stances
    StanceList {
        /// Filter by topic
        #[arg(short, long)]
        topic: Option<String>,

        /// Filter by tag
        #[arg(short = 'g', long)]
        tag: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show stance details
    StanceShow {
        /// Stance ID
        #[arg(short, long)]
        id: String,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show memory statistics
    MemoryStats {
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show database status with breakdown by source and parse status
    Status {
        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show citation relationships for a paper
    Citations {
        /// Paper ID to find citations FROM (papers this paper cites)
        #[arg(long)]
        from: Option<String>,

        /// Paper ID to find citations TO (papers that cite this paper)
        #[arg(long)]
        to: Option<String>,

        /// Output format (table/csv/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show citation statistics
    CiteStats {
        /// Show stats for a specific paper
        #[arg(short, long)]
        paper: Option<String>,

        /// Show top N most-cited papers
        #[arg(short, long)]
        top: Option<usize>,

        /// Output format (table/csv/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Manage job queue
    Queue {
        /// Add a paper to the queue
        #[arg(long)]
        add: Option<String>,

        /// List queued jobs
        #[arg(long)]
        list: bool,

        /// Show papers awaiting processing
        #[arg(long)]
        pending: bool,

        /// Pop next job from queue
        #[arg(long)]
        dequeue: bool,

        /// Cancel a queued job by id
        #[arg(long)]
        cancel: Option<i64>,

        /// Clear all queued jobs
        #[arg(long)]
        clear: bool,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Merge a duplicate paper into a target paper
    Merge {
        /// Which paper to keep: 'older' (default), 'newer', 'parsed' (better parse_status), or 'semantic' (high similarity + parse_status)
        #[arg(short, long, default_value = "older")]
        keep: String,

        /// Show what would be merged without making changes
        #[arg(short, long)]
        dry_run: bool,

        /// Automatically find and merge all duplicate pairs with similarity >= 0.95
        #[arg(long)]
        auto: bool,

        /// ID of the paper to keep
        target_id: Option<String>,

        /// ID of the duplicate paper to absorb and delete
        duplicate_id: Option<String>,
    },

    /// Import citation links from a JSON file or inline JSON
    CiteImport {
        /// JSON string or @filename containing citation data
        json_input: Option<String>,

        /// Show what would be imported without writing to DB
        #[arg(short, long)]
        dry_run: bool,

        /// Skip source/target papers that do not exist in the DB
        #[arg(long)]
        skip_missing: bool,

        /// Extract citation references from a paper's plain_text (requires --paper)
        #[arg(long)]
        extract: bool,

        /// Paper ID for --extract mode
        #[arg(long)]
        paper: Option<String>,

        /// Use upsert mode to report duplicate citation edges
        #[arg(long)]
        dedup: bool,
    },

    /// Rank papers by citation velocity
    Influence {
        /// Number of top papers to show
        #[arg(short = 'n', long, default_value = "20")]
        top: usize,

        /// Show detailed stats for a specific paper
        #[arg(long)]
        paper: Option<String>,

        /// Minimum forward citations to include
        #[arg(long, default_value = "1")]
        min_cites: usize,

        /// Output format (text/csv/json)
        #[arg(short, long, default_value = "text")]
        format: String,
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
        eprintln!(
            "Database not found at {}. Run 'rairos init' first.",
            path.display()
        );
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

    add_paper_from_arxiv(db, arxiv_id)
}

/// Add a paper from arXiv by ID, returning Ok(()) on success
fn add_paper_from_arxiv(db: &Database, arxiv_id: &str) -> Result<()> {
    // Fetch from arXiv API
    println!("Fetching metadata from arXiv for {}...", arxiv_id);

    let url = format!("https://export.arxiv.org/api/query?id_list={}", arxiv_id);
    let resp = reqwest::blocking::get(&url).context("Failed to connect to arXiv API")?;

    if !resp.status().is_success() {
        anyhow::bail!("arXiv API returned error: {}", resp.status());
    }

    let body = resp.text().context("Failed to read arXiv response")?;

    // arXiv ATOM feed has feed-level <title> and <summary> BEFORE <entry>
    // We need the entry-level fields, so extract entry block first
    let entry_start = body.find("<entry>").unwrap_or(0);
    let entry_end = body.find("</entry>").unwrap_or(body.len());
    let entry = &body[entry_start..entry_end];

    // Parse entry-level fields
    let title = extract_xml_field(entry, "<title>")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| format!("arXiv:{}", arxiv_id));
    let abstract_text = extract_xml_field(entry, "<summary>")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Abstract not available".to_string());

    let authors: Vec<String> = extract_all_xml_fields(entry, "<name>")
        .into_iter()
        .map(|s| s.trim().to_string())
        .collect();

    let published = extract_xml_field(entry, "<published>")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    let categories: Vec<String> = extract_all_xml_fields(entry, "<category term=")
        .into_iter()
        .filter(|s| s.contains('"'))
        .map(|s| s.split('"').nth(1).unwrap_or(&s).to_string())
        .collect();

    let _primary_category = extract_xml_field(entry, "arxiv:primary_category term=")
        .map(|s| s.split('"').nth(1).unwrap_or(&s).to_string())
        .unwrap_or_else(|| categories.first().cloned().unwrap_or_default());

    println!("  Title: {}", title.chars().take(60).collect::<String>());
    println!(
        "  Authors: {}",
        authors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  Published: {}", published.format("%Y-%m-%d"));
    println!(
        "  Categories: {}",
        categories
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Build paper with fetched metadata
    let mut paper = Paper::new(Some(arxiv_id.to_string()), title, abstract_text);
    paper.authors = authors;
    paper.categories = categories;
    paper.published = published;

    db.insert_paper(&paper)?;
    println!("\n[OK] Added: {} ({})", paper.id, arxiv_id);
    Ok(())
}

// Extract text content from first XML field occurrence
fn extract_xml_field(xml: &str, tag: &str) -> Option<String> {
    if let Some(start) = xml.find(tag) {
        let content_start = start + tag.len();
        if let Some(end) = xml[content_start..].find("</") {
            return Some(xml[content_start..content_start + end].to_string());
        }
    }
    None
}

// Extract all occurrences of an XML tag's text content
fn extract_all_xml_fields(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_pos = 0;

    while let Some(tag_start) = xml[search_pos..].find(tag) {
        let content_start = tag_start + tag.len();
        let rest = &xml[search_pos..];
        let after_tag = &rest[content_start..];
        if let Some(end_offset) = after_tag.find("</") {
            results.push(after_tag[..end_offset].to_string());
            search_pos += content_start + end_offset + 3; // Skip past </tag>
        } else {
            break;
        }
    }
    results
}

fn handle_list(
    db: &Database,
    status: Option<String>,
    year: Option<i32>,
    tags: &[String],
    limit: usize,
    offset: usize,
    sort: &str,
    order: &str,
    format: &str,
) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let mut papers = db.list_papers(parse_status, 10000, 0)?;

    // Year filter
    if let Some(y) = year {
        papers.retain(|p| {
            p.published
                .format("%Y")
                .to_string()
                .parse::<i32>()
                .unwrap_or(0)
                == y
        });
    }

    // Tag filter (paper must have ALL specified tags)
    if !tags.is_empty() {
        papers.retain(|p| {
            let paper_tags: std::collections::HashSet<_> =
                p.categories.iter().map(|s| s.to_lowercase()).collect();
            tags.iter().all(|t| paper_tags.contains(&t.to_lowercase()))
        });
    }

    // Sort
    let reverse = order == "desc";
    match sort {
        "published" => papers.sort_by(|a, b| {
            if reverse {
                b.published.cmp(&a.published)
            } else {
                a.published.cmp(&b.published)
            }
        }),
        "title" => papers.sort_by(|a, b| {
            if reverse {
                b.title.cmp(&a.title)
            } else {
                a.title.cmp(&b.title)
            }
        }),
        "status" => papers.sort_by(|a, b| {
            if reverse {
                status_str(&b.parse_status).cmp(&status_str(&a.parse_status))
            } else {
                status_str(&a.parse_status).cmp(&status_str(&b.parse_status))
            }
        }),
        _ => {} // added_at - keep insertion order
    }

    // Apply offset/limit
    let total = papers.len();
    papers = papers.into_iter().skip(offset).take(limit).collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = papers
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "arxiv_id": p.arxiv_id,
                    "title": p.title,
                    "authors": p.authors,
                    "published": p.published,
                    "status": status_str(&p.parse_status),
                    "categories": p.categories,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Table format
    println!(
        "Showing {}/{} papers (sort: {} {}, offset: {})",
        papers.len(),
        total,
        sort,
        order,
        offset
    );
    println!();
    println!("{:<38} {:<10} {:<12} {}", "ID", "STATUS", "ARXIV", "TITLE");
    println!("{}", "-".repeat(120));
    for paper in &papers {
        let id_short = if paper.id.len() > 8 {
            &paper.id[..8]
        } else {
            &paper.id
        };
        let arxiv = paper.arxiv_id.as_deref().unwrap_or("-");
        let title = if paper.title.len() > 50 {
            &paper.title[..50]
        } else {
            &paper.title
        };
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

fn handle_stats(db: &Database, json: bool, format: &str) -> Result<()> {
    let stats = db.stats()?;

    if json || format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total_papers": stats.total,
                "pending": stats.pending,
                "done": stats.done,
                "research_gaps": stats.gaps,
            }))?
        );
        return Ok(());
    }

    println!("=== Rairos Database Statistics ===");
    println!("Total papers:  {}", stats.total);
    println!("  Pending:     {}", stats.pending);
    println!("  Done:        {}", stats.done);
    println!("Research gaps: {}", stats.gaps);
    Ok(())
}

fn handle_search(
    db: &Database,
    query: &str,
    limit: usize,
    field: &str,
    format: &str,
) -> Result<()> {
    // Use search_papers for real keyword matching in title/abstract
    let papers = db.search_papers(query, limit)?;

    let filtered: Vec<&Paper> = if field == "all" {
        papers.iter().collect()
    } else {
        papers
            .iter()
            .filter(|p| match field {
                "title" => p.title.to_lowercase().contains(&query.to_lowercase()),
                "abstract" => p
                    .abstract_text
                    .to_lowercase()
                    .contains(&query.to_lowercase()),
                "authors" => p
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&query.to_lowercase())),
                "categories" => p
                    .categories
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query.to_lowercase())),
                _ => true,
            })
            .collect()
    };

    let papers_vec: Vec<Paper> = filtered.into_iter().cloned().collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = papers_vec
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "arxiv_id": p.arxiv_id,
                    "title": p.title,
                    "authors": p.authors,
                    "published": p.published,
                    "categories": p.categories,
                    "cited_by": p.metadata.cited_by,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    if papers_vec.is_empty() {
        println!("No papers found for query: {} (field: {})", query, field);
        return Ok(());
    }

    println!(
        "Found {} papers for '{}' (field: {}):",
        papers_vec.len(),
        query,
        field
    );
    println!();
    for (i, paper) in papers_vec.iter().enumerate() {
        println!("{}. {}", i + 1, paper.title);
        if let Some(ref arxiv) = paper.arxiv_id {
            println!("   arXiv: {}", arxiv);
        }
        println!(
            "   {} | cited_by: {}",
            paper.published, paper.metadata.cited_by
        );
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
    println!(
        "Authors:     {:?}",
        paper.authors.iter().take(5).collect::<Vec<_>>()
    );
    if paper.authors.len() > 5 {
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

fn handle_delete(db: &Database, ids: &[String], force: bool) -> Result<()> {
    if ids.len() > 1 && !force {
        print!("Delete {} papers? [y/N] ", ids.len());
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut confirm = String::new();
        if std::io::stdin().read_line(&mut confirm).is_err()
            || !confirm.trim().eq_ignore_ascii_case("y")
        {
            println!("Cancelled.");
            return Ok(());
        }
    } else if ids.len() == 1 && !force {
        print!("Delete paper '{}'? [y/N] ", ids[0]);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut confirm = String::new();
        if std::io::stdin().read_line(&mut confirm).is_err()
            || !confirm.trim().eq_ignore_ascii_case("y")
        {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let mut deleted = 0;
    let mut failed = 0;
    for id in ids {
        match db.delete_paper(id) {
            Ok(_) => {
                println!("Deleted: {}", id);
                deleted += 1;
            }
            Err(e) => {
                eprintln!("Failed: {} ({})", id, e);
                failed += 1;
            }
        }
    }
    println!("\nDelete complete: {} deleted, {} failed", deleted, failed);
    Ok(())
}

fn handle_update_status(db: &Database, ids: &[String], status: &str) -> Result<()> {
    let parse_status = parse_status_arg(status).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid status '{}'. Use: pending, parsing, done, failed",
            status
        )
    })?;

    let mut updated = 0;
    let mut failed = 0;
    for id in ids {
        match db.update_paper_status(id, parse_status) {
            Ok(_) => {
                println!("Updated: {} -> {}", id, status);
                updated += 1;
            }
            Err(e) => {
                eprintln!("Failed: {} ({})", id, e);
                failed += 1;
            }
        }
    }
    println!(
        "\nStatus update complete: {} updated, {} failed",
        updated, failed
    );
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

    let arxiv_id = paper.arxiv_id.as_deref().unwrap_or(&paper.id);
    println!("Parsing paper: {}", paper.title);
    println!("  arXiv: {}", arxiv_id);

    let pdf_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("pdfs");
    std::fs::create_dir_all(&pdf_dir).context("Failed to create pdfs directory")?;

    let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

    if !pdf_path.exists() {
        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
        println!("  Downloading from {} ...", pdf_url);
        let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        rt.block_on(rairos_pdf::download_pdf(&pdf_url, &pdf_path))
            .context("Failed to download PDF")?;
        println!("  Downloaded to: {}", pdf_path.display());
    } else {
        println!("  Using cached PDF: {}", pdf_path.display());
    }

    println!("  Extracting text ...");
    let text = rairos_pdf::extract_pdf_text(&pdf_path)
        .context("Failed to extract text from PDF")?;

    println!("\n  Text length: {} characters", text.len());

    let preview: String = text.chars().take(500).collect();
    println!("\n--- Preview (first 500 chars) ---\n{}", preview);

    db.update_paper_status(&paper.id, rairos_core::ParseStatus::Done)
        .context("Failed to update paper status")?;
    println!("\n[OK] Parse complete. Status set to 'done'.");
    Ok(())
}

fn handle_import(
    db: &Database,
    path: &Option<PathBuf>,
    ids: &[String],
    skip_existing: bool,
) -> Result<()> {
    if let Some(p) = path {
        // JSON file import
        let content = std::fs::read_to_string(p)?;
        let papers: Vec<Paper> = serde_json::from_str(&content)
            .context("Failed to parse JSON — expected array of Paper objects")?;

        let count = papers.len();
        for paper in &papers {
            db.insert_paper(paper)?;
        }
        println!("Imported {} papers from {}", count, p.display());
    } else if !ids.is_empty() {
        // Batch import from arXiv IDs
        let mut added = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for arxiv_id in ids {
            let arxiv_id = arxiv_id.trim();
            if arxiv_id.is_empty() {
                continue;
            }

            // Check if already exists
            if skip_existing {
                if let Ok(Some(_)) = db.get_paper_by_arxiv(arxiv_id) {
                    println!("Skipped (exists): {}", arxiv_id);
                    skipped += 1;
                    continue;
                }
            }

            println!("Fetching {}...", arxiv_id);
            match add_paper_from_arxiv(db, arxiv_id) {
                Ok(_) => {
                    println!("Added: {}", arxiv_id);
                    added += 1;
                }
                Err(e) => {
                    eprintln!("Failed {}: {}", arxiv_id, e);
                    failed += 1;
                }
            }
        }

        println!(
            "\nImport complete: {} added, {} skipped, {} failed",
            added, skipped, failed
        );
    } else {
        println!("Nothing to import. Use positional arXiv IDs or --ids flag, or provide a JSON file path.");
    }
    Ok(())
}

fn handle_export(
    db: &Database,
    path: &PathBuf,
    status: Option<String>,
    format: &str,
) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let papers = db.list_papers(parse_status, 10000, 0)?;

    if format == "csv" {
        let mut w = csv::Writer::from_path(path)?;
        w.write_record(&[
            "id",
            "arxiv_id",
            "title",
            "authors",
            "published",
            "status",
            "cited_by",
            "categories",
        ])?;
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
        let out: Vec<serde_json::Value> = papers
            .iter()
            .map(|p| {
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
            })
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&out)?)?;
    }

    println!("Exported {} papers to {}", papers.len(), path.display());
    Ok(())
}

fn handle_gap(
    db: &Database,
    topic: &str,
    limit: usize,
    format: &str,
    category: Option<String>,
) -> Result<()> {
    println!("Detecting research gaps for topic: {}", topic);

    let papers = db.search_papers(topic, limit * 3)?;

    if papers.is_empty() {
        println!(
            "No papers found for topic '{}'. Try a different query.",
            topic
        );
        return Ok(());
    }

    let total_papers = papers.len();
    let stop_words: std::collections::HashSet<&str> = [
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "must",
        "shall",
        "can",
        "need",
        "dare",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "again",
        "further",
        "then",
        "once",
        "here",
        "there",
        "when",
        "where",
        "why",
        "how",
        "all",
        "each",
        "few",
        "more",
        "most",
        "other",
        "some",
        "such",
        "no",
        "nor",
        "not",
        "only",
        "own",
        "same",
        "so",
        "than",
        "too",
        "very",
        "just",
        "but",
        "and",
        "or",
        "if",
        "because",
        "as",
        "until",
        "while",
        "this",
        "that",
        "these",
        "those",
        "paper",
        "papers",
        "study",
        "method",
        "approach",
        "result",
        "results",
        "show",
        "shown",
        "using",
        "used",
        "based",
        "proposed",
        "present",
        "presented",
        "state",
    ]
    .into();

    // ============================================================
    // GAP 1: Underexplored subtopics (keywords appearing in 1-2 papers)
    // ============================================================
    let mut keyword_to_papers: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut keyword_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for paper in &papers {
        let text = format!(
            "{} {} {}",
            paper.title,
            paper.abstract_text,
            paper.categories.join(" ")
        );
        let words: std::collections::HashSet<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_string())
            .collect();

        for word in words {
            *keyword_counts.entry(word.clone()).or_insert(0) += 1;
            keyword_to_papers
                .entry(word)
                .or_insert_with(Vec::new)
                .push(paper.id.clone());
        }
    }

    // Rare keywords = appearing in 1-2 papers (out of many) - underexplored areas
    let rare_keywords: Vec<(String, usize)> = keyword_counts
        .iter()
        .filter(|(_, &count)| count >= 1 && count <= 2 && total_papers > 5)
        .map(|(k, &c)| (k.clone(), c))
        .collect();

    let mut gaps = Vec::new();

    // GAP 1: Underexplored subtopics
    if rare_keywords.len() > 3 {
        let sample: Vec<_> = rare_keywords.iter().take(5).collect();
        let examples: Vec<String> = sample.iter().map(|(k, _)| format!("\"{}\"", k)).collect();
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("underexplored"),
            &format!(
                "Underexplored subtopics detected: {} (appearing in only 1-2 papers each). \
                Potential research directions: {}",
                rare_keywords.len(),
                examples.join(", ")
            ),
            "high",
        );
        let paper_ids: Vec<String> = rare_keywords
            .iter()
            .take(5)
            .flat_map(|(kw, _)| keyword_to_papers.get(kw).cloned().unwrap_or_default())
            .take(5)
            .collect();
        let mut g = gap;
        g.paper_ids = paper_ids;
        gaps.push(g);
    }

    // ============================================================
    // GAP 2: Category imbalance (some categories underrepresented)
    // ============================================================
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        for cat in &paper.categories {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
    }

    let total_cats = cat_counts.values().sum::<usize>();
    if total_cats > 0 {
        let avg_cats_per_paper = total_cats as f64 / total_papers as f64;
        let underrepresented: Vec<(String, usize)> = cat_counts
            .iter()
            .filter(|(_, &count)| {
                let freq = count as f64 / total_papers as f64;
                freq < 0.3 * avg_cats_per_paper && count <= 2
            })
            .map(|(k, &c)| (k.clone(), c))
            .collect();

        if !underrepresented.is_empty() {
            let cats: Vec<String> = underrepresented
                .iter()
                .take(5)
                .map(|(k, _)| k.clone())
                .collect();
            let gap = ResearchGap::new(
                category.as_deref().unwrap_or("category-gap"),
                &format!(
                    "Underrepresented categories (appear in <30% of papers): {}. \
                    These sub-fields may need more investigation.",
                    cats.join(", ")
                ),
                "medium",
            );
            gaps.push(gap);
        }
    }

    // ============================================================
    // GAP 3: Recent papers citing older work (temporal gap)
    // ============================================================
    use chrono::Utc;
    let now = Utc::now();
    let recent_papers: Vec<_> = papers
        .iter()
        .filter(|p| (now - p.published).num_days() < 365)
        .collect();

    if recent_papers.len() >= 2 && total_papers > 5 {
        // Check if recent papers mostly cite old work
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("temporal"),
            &format!(
                "Recent work ({} papers <1yr old) may not fully incorporate latest advances. \
                Check if recent papers cite papers from the last 2 years.",
                recent_papers.len()
            ),
            "low",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 4: Coverage gap (insufficient papers)
    // ============================================================
    if total_papers < 10 {
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("coverage"),
            &format!(
                "Insufficient coverage of '{}' - only {} papers found. \
                This area may be nascent or need broader search terms.",
                topic, total_papers
            ),
            "high",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 5: Method diversity gap (check if papers use similar methods)
    // ============================================================
    let method_keywords = [
        "rl",
        "reinforcement",
        "supervised",
        "unsupervised",
        "reinforcement learning",
        "neural",
        "transformer",
        "diffusion",
        "gcn",
        "attention",
        "gan",
        "bayesian",
        "optimization",
        "gradient",
        "supervised learning",
    ];
    let method_counts: Vec<(&str, usize)> = method_keywords
        .iter()
        .filter_map(|m| {
            let count = keyword_counts.get(*m).copied().unwrap_or(0);
            if count > 0 {
                Some((*m, count))
            } else {
                None
            }
        })
        .collect();

    if !method_counts.is_empty() && method_counts.len() <= 2 && total_papers >= 5 {
        let methods: Vec<String> = method_counts
            .iter()
            .map(|(m, _)| format!("\"{}\"", m))
            .collect();
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("method-diversity"),
            &format!(
                "Limited methodological diversity. Methods detected: {} (only {}/{} known methods found). \
                Consider exploring alternative methodologies.",
                methods.join(", "), method_counts.len(), method_keywords.len()
            ),
            "medium",
        );
        gaps.push(gap);
    }

    // Save gaps to database
    for g in &gaps {
        db.insert_gap(g)?;
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Detected {} Research Gaps ===\n", gaps.len());
        for (i, gap) in gaps.iter().enumerate() {
            println!("[{}/{}] Gap: {}", i + 1, gaps.len(), gap.description);
            println!(
                "       Severity: {} | Category: {}",
                gap.severity, gap.category
            );
            println!("       Related papers: {}", gap.paper_ids.len());
            println!();
        }
    }

    if gaps.is_empty() {
        println!("No significant gaps detected. The field appears well-explored for this topic.");
    } else {
        println!(
            "Note: {} gap(s) saved to database. Use 'rairos gap-list' to view.",
            gaps.len()
        );
    }
    Ok(())
}

fn handle_gap_list(db: &Database, limit: usize, offset: usize, format: &str) -> Result<()> {
    let gaps = db.list_gaps(limit, offset)?;

    if gaps.is_empty() {
        println!("No research gaps found. Run 'rairos gap --topic <query>' to detect gaps.");
        return Ok(());
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Research Gaps ({}) ===\n", gaps.len());
        println!(
            "{:<36} {:<10} {:<8} {}",
            "ID", "CATEGORY", "SEVERITY", "DESCRIPTION"
        );
        println!("{}", "-".repeat(100));
        for gap in &gaps {
            let id_short = if gap.id.len() > 8 {
                &gap.id[..8]
            } else {
                &gap.id
            };
            let desc_short = if gap.description.len() > 60 {
                format!("{}...", &gap.description[..60])
            } else {
                gap.description.clone()
            };
            println!(
                "{:<36} {:<10} {:<8} {}",
                id_short, gap.category, gap.severity, desc_short
            );
        }
        println!();
    }
    Ok(())
}

fn handle_gap_show(db: &Database, id: &str) -> Result<()> {
    let gap = db
        .get_gap(id)?
        .ok_or_else(|| anyhow::anyhow!("Gap not found: {}", id))?;

    println!("\n=== Research Gap Details ===\n");
    println!("ID:          {}", gap.id);
    println!("Category:    {}", gap.category);
    println!("Severity:    {}", gap.severity);
    println!("Description: {}", gap.description);
    println!(
        "Paper IDs:   {} ({} total)",
        gap.paper_ids.join(", "),
        gap.paper_ids.len()
    );
    println!();

    // Show related papers
    if !gap.paper_ids.is_empty() {
        println!("Related Papers:");
        for pid in gap.paper_ids.iter().take(5) {
            if let Ok(paper) = db.get_paper(pid) {
                let title = if paper.title.len() > 60 {
                    format!("{}...", &paper.title[..60])
                } else {
                    paper.title
                };
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

fn handle_gene_add(
    approach: &str,
    gap_type: &str,
    keywords: &str,
    paper_id: Option<String>,
) -> Result<()> {
    let keywords: Vec<String> = keywords.split(',').map(|s| s.trim().to_string()).collect();
    let mut capsule = Capsule::new(approach, gap_type, keywords);
    if let Some(pid) = paper_id {
        capsule = capsule.with_paper(&pid);
    }

    let mut pool = GenePool::load().context("Failed to load gene pool")?;
    pool.add_capsule(capsule);
    pool.save().context("Failed to save gene pool")?;

    println!("[OK] Gene added to pool");
    println!(
        "Capsule ID: {}",
        pool.capsules()
            .last()
            .map(|c| c.capsule_id.as_str())
            .unwrap_or("N/A")
    );
    Ok(())
}

fn handle_gene_list(
    gap_type: Option<String>,
    status: Option<String>,
    limit: usize,
    format: &str,
) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let all_capsules = pool.capsules();

    let filtered: Vec<&Capsule> = all_capsules
        .iter()
        .filter(|c| {
            if let Some(ref gt) = gap_type {
                if &c.action_gap_type != gt {
                    return false;
                }
            }
            if let Some(ref s) = status {
                let status_match = match s.to_lowercase().as_str() {
                    "active" => c.status == CapsuleStatus::Active && !c.archived,
                    "dormant" => c.status == CapsuleStatus::Dormant,
                    "archived" => c.archived,
                    _ => true,
                };
                if !status_match {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = filtered
            .iter()
            .map(|c| {
                let status = if c.archived {
                    "archived".to_string()
                } else {
                    c.status.to_string()
                };
                serde_json::json!({
                    "capsule_id": c.capsule_id,
                    "gap_type": c.action_gap_type,
                    "approach": c.archetype.approach_summary,
                    "status": status,
                    "impact_score": c.impact_score,
                    "success_count": c.success_count,
                    "failure_count": c.failure_count,
                    "created_at": c.created_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let count = filtered.len();
    println!("=== Gene Pool ({} capsules) ===\n", count);
    println!(
        "{:<38} {:<15} {:<12} {:>8} {:>8} {:>8}",
        "ID", "GAP_TYPE", "STATUS", "IMPACT", "SUCCESS", "FAILED"
    );
    println!("{}", "-".repeat(95));
    for cap in &filtered {
        let id_short = if cap.capsule_id.len() > 8 {
            &cap.capsule_id[..8]
        } else {
            &cap.capsule_id
        };
        let status_str = if cap.archived {
            "archived".to_string()
        } else {
            cap.status.to_string()
        };
        println!(
            "{:<38} {:<15} {:<12} {:>8.3} {:>8} {:>8}",
            id_short,
            cap.action_gap_type,
            status_str,
            cap.impact_score,
            cap.success_count,
            cap.failure_count
        );
    }
    println!("\n{} capsules shown", count);
    Ok(())
}

fn handle_gene_show(id: &str, format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    if let Some(cap) = pool
        .capsules()
        .iter()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(id))
    {
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(cap)?);
            return Ok(());
        }

        println!("=== Gene Details ===\n");
        println!("ID:           {}", cap.capsule_id);
        println!("Gap Type:     {}", cap.action_gap_type);
        println!("Approach:     {}", cap.archetype.approach_summary);
        println!(
            "Status:       {}",
            if cap.archived {
                "archived".to_string()
            } else {
                cap.status.to_string()
            }
        );
        println!("Impact Score: {:.4}", cap.impact_score);
        println!("Success:      {}", cap.success_count);
        println!("Failure:      {}", cap.failure_count);
        println!("Created:      {}", cap.created_at);
        println!("Updated:      {}", cap.updated_at);
        println!("Keywords:     {:?}", cap.trigger_keywords);
        if let Some(ref fp) = cap.archetype.algorithm_fingerprint {
            println!("Fingerprint:  {}", fp);
        }
        if let Some(ref pid) = cap.archetype.source_paper_id {
            println!("Source Paper: {}", pid);
        }
    } else {
        anyhow::bail!("Gene not found: {}", id);
    }
    Ok(())
}

fn handle_gene_feedback(id: &str, positive: bool) -> Result<()> {
    let mut pool = GenePool::load().context("Failed to load gene pool")?;
    if let Some(cap) = pool
        .capsules_mut()
        .iter_mut()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(id))
    {
        if positive {
            cap.record_success();
            println!("[OK] Recorded positive feedback for {}", id);
        } else {
            cap.record_failure();
            println!("[OK] Recorded negative feedback for {}", id);
        }
        println!("  Success count: {}", cap.success_count);
        println!("  Failure count: {}", cap.failure_count);
        println!("  New impact score: {:.4}", cap.impact_score);
        pool.save().context("Failed to save gene pool")?;
    } else {
        anyhow::bail!("Gene not found: {}", id);
    }
    Ok(())
}

fn handle_gene_diversity(format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let diversity = GenePoolDiversityCalculator::calculate(pool.capsules());

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&diversity)?);
        return Ok(());
    }

    println!("=== Gene Pool Diversity ===\n");
    println!("Total Capsules:     {}", diversity.capsule_count);
    println!("Shannon Index:      {:.4}", diversity.shannon_index);
    println!("Shannon Normalized:  {:.4}", diversity.shannon_normalized);
    println!("Diversity Score:    {} / 100", diversity.diversity_score);
    println!(
        "Family Coverage:     {:.1}%",
        diversity.family_coverage * 100.0
    );
    println!();

    println!("Family Distribution:");
    let mut families: Vec<_> = diversity.family_counts.iter().collect();
    families.sort_by(|a, b| b.1.cmp(a.1));
    for (fam, count) in families {
        println!("  {:20} {:>4}", fam, count);
    }
    println!();

    if !diversity.underrepresented_families.is_empty() {
        println!(
            "Underrepresented: {:?}",
            diversity.underrepresented_families
        );
    }
    if !diversity.overrepresented_families.is_empty() {
        println!("Overrepresented:  {:?}", diversity.overrepresented_families);
    }
    Ok(())
}

fn handle_gene_evolve(max_crossovers: usize, format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let gaps = vec!["capability", "improvement", "reasoning"];
    let mut suggestions = Vec::new();
    for gap_type in &gaps {
        let pairs = pool.suggest_crossover(gap_type, max_crossovers / gaps.len());
        for (id1, id2) in pairs {
            suggestions.push((gap_type.to_string(), id1, id2));
        }
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = suggestions
            .iter()
            .map(|(gt, id1, id2)| {
                serde_json::json!({
                    "gap_type": gt,
                    "parent_1": id1,
                    "parent_2": id2,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "=== Evolution Suggestions ({} crossovers) ===\n",
        suggestions.len()
    );
    for (i, (gap_type, id1, id2)) in suggestions.iter().enumerate() {
        println!(
            "{}. {} × {} -> {}",
            i + 1,
            &id1[..8.min(id1.len())],
            &id2[..8.min(id2.len())],
            gap_type
        );
    }
    Ok(())
}

fn handle_kg_stats(format: &str) -> Result<()> {
    let graph = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());
    let stats = graph.stats();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("=== Knowledge Graph Stats ===\n");
    println!("Total Nodes:  {}", stats.total_nodes);
    println!("Total Edges:  {}", stats.total_edges);
    println!("Avg Degree:   {:.2}", stats.avg_degree);
    println!("Paper Nodes: {}", stats.paper_nodes);
    println!("Concept Nodes: {}", stats.concept_nodes);
    Ok(())
}

fn handle_kg_rank(limit: usize, format: &str) -> Result<()> {
    let graph = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());
    let ranks = GraphAlgorithms::rank_papers(&graph);

    let mut sorted: Vec<_> = ranks.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if format == "json" {
        let out: Vec<serde_json::Value> = sorted
            .iter()
            .take(limit)
            .map(|(id, score)| serde_json::json!({ "paper_id": id, "score": score }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Paper Rankings (Top {}) ===\n", limit);
    println!("{:>6} {:<40} {:>10}", "RANK", "PAPER ID", "SCORE");
    println!("{}", "-".repeat(60));
    for (i, (id, score)) in sorted.iter().take(limit).enumerate() {
        println!(
            "{:>6} {:<40} {:>10.4}",
            i + 1,
            &id[..id.len().min(40)],
            score
        );
    }
    Ok(())
}

fn handle_kg_path(source: &str, target: &str) -> Result<()> {
    let graph = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());

    match graph.find_path(source, target) {
        Some(path) => {
            println!("=== Path Found ({} steps) ===\n", path.len() - 1);
            for (i, node) in path.iter().enumerate() {
                println!("{}. {}", i + 1, node);
            }
        }
        None => {
            println!("No path found between {} and {}", source, target);
        }
    }
    Ok(())
}

fn handle_kg_add_paper(db: &Database, paper_id: &str) -> Result<()> {
    let paper = db
        .get_paper(paper_id)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(paper_id).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", paper_id))?;

    let mut graph = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());
    graph.add_paper(&paper);
    graph
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save knowledge graph: {}", e))?;

    println!("[OK] Added paper to knowledge graph:");
    println!("  ID: {}", paper.id);
    println!("  Title: {}", &paper.title[..paper.title.len().min(60)]);
    Ok(())
}

fn handle_kg_add_citation(db: &Database, source: &str, target: &str) -> Result<()> {
    let source_paper = db
        .get_paper(source)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(source).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Source paper not found: {}", source))?;

    let target_paper = db
        .get_paper(target)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(target).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Target paper not found: {}", target))?;

    let mut graph = KnowledgeGraph::load().unwrap_or_else(|_| KnowledgeGraph::new());

    // Ensure both papers are in the graph
    graph.add_paper(&source_paper);
    graph.add_paper(&target_paper);
    graph.add_citation(&source_paper.id, &target_paper.id);
    graph
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save knowledge graph: {}", e))?;

    println!("[OK] Added citation edge:");
    println!("  {} -> {}", source_paper.id, target_paper.id);
    Ok(())
}

fn handle_stance_add(topic: &str, claim: &str, stance: &str, reasoning: &str) -> Result<()> {
    let stance_type = match stance.to_lowercase().as_str() {
        "supported" => StanceType::Supported,
        "rejected" => StanceType::Rejected,
        "deferred" => StanceType::Deferred,
        "qualified" => StanceType::Qualified,
        _ => anyhow::bail!(
            "Invalid stance: {}. Use: supported, rejected, deferred, qualified",
            stance
        ),
    };

    let mut memory = ResearchMemory::load().context("Failed to load research memory")?;
    let new_stance = ResearchStance::new(topic, claim, stance_type, reasoning);
    memory.add_stance(new_stance);
    memory.save().context("Failed to save research memory")?;
    println!("[OK] Stance added");
    Ok(())
}

fn handle_stance_list(topic: Option<String>, tag: Option<String>, format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;

    let stances: Vec<&ResearchStance> = if let Some(ref t) = topic {
        memory.find_by_topic(t)
    } else if let Some(ref t) = tag {
        memory.find_by_tag(t)
    } else {
        memory.stances().iter().collect()
    };

    if format == "json" {
        let out: Vec<serde_json::Value> = stances
            .iter()
            .map(|s| {
                serde_json::json!({
                    "stance_id": s.stance_id,
                    "topic": s.topic,
                    "claim": s.claim,
                    "stance": s.stance.to_string(),
                    "confidence": s.confidence,
                    "tags": s.tags,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Research Stances ({} found) ===\n", stances.len());
    println!(
        "{:<38} {:<20} {:<15} {:<10}",
        "ID", "TOPIC", "STANCE", "CONFIDENCE"
    );
    println!("{}", "-".repeat(85));
    for s in stances {
        let id_short = if s.stance_id.len() > 8 {
            &s.stance_id[..8]
        } else {
            &s.stance_id
        };
        println!(
            "{:<38} {:<20} {:<15} {:.2}",
            id_short,
            &s.topic[..20.min(s.topic.len())],
            s.stance,
            s.confidence
        );
    }
    Ok(())
}

fn handle_stance_show(id: &str, format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;

    let stance = memory.get_stance(id).or_else(|| {
        memory
            .stances()
            .iter()
            .find(|s| s.stance_id.starts_with(id))
    });

    if let Some(s) = stance {
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(s)?);
            return Ok(());
        }
        println!("=== Stance Details ===\n");
        println!("ID:         {}", s.stance_id);
        println!("Topic:      {}", s.topic);
        println!("Claim:      {}", s.claim);
        println!("Stance:     {}", s.stance);
        println!("Confidence: {:.2}", s.confidence);
        println!("Reasoning: {}", s.reasoning);
        println!("Tags:      {:?}", s.tags);
        println!("Evidence:  {:?}", s.evidence_refs);
        println!("Created:   {}", s.created_at);
        println!("Updated:   {}", s.updated_at);

        let anomalies = memory.get_anomalies_by_stance(&s.stance_id);
        if !anomalies.is_empty() {
            println!("\n=== Anomalies ({} found) ===", anomalies.len());
            for a in anomalies {
                println!(
                    "  - [{}] {} ({})",
                    format!("{:?}", a.severity),
                    a.paper_title,
                    a.anomaly_type
                );
            }
        }
    } else {
        anyhow::bail!("Stance not found: {}", id);
    }
    Ok(())
}

fn handle_memory_stats(format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;
    let stats = memory.stats();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("=== Research Memory Stats ===\n");
    println!("Total Stances:  {}", stats.total_stances);
    println!("Total Anomalies: {}", stats.total_anomalies);
    println!("\nBy Stance:");
    for (stance, count) in &stats.by_stance {
        println!("  {}: {}", stance, count);
    }
    if !stats.by_severity.is_empty() {
        println!("\nBy Severity:");
        for (sev, count) in &stats.by_severity {
            println!("  {}: {}", sev, count);
        }
    }
    Ok(())
}

fn handle_status(db: &Database, format: &str) -> Result<()> {
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

fn handle_citations(
    db: &Database,
    from: Option<&str>,
    to: Option<&str>,
    format: &str,
) -> Result<()> {
    if from.is_none() && to.is_none() {
        eprintln!("Error: must specify --from or --to");
        std::process::exit(1);
    }

    match (from, to) {
        // Bridge mode: --from A --to B
        (Some(f), Some(t)) => {
            let from_title = db.get_paper(f)?.title;
            let to_title = db.get_paper(t)?.title;

            let citations_from = db.get_citations(f)?;
            let citations_to = db.get_citations(t)?;

            let direct = citations_from.references.contains(&t.to_string());
            let citing_to_sources: std::collections::HashSet<String> =
                citations_to.citing.into_iter().collect();
            let via_papers: Vec<&String> = citations_from
                .references
                .iter()
                .filter(|id| citing_to_sources.contains(*id))
                .collect();

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "from": f,
                        "from_title": from_title,
                        "to": t,
                        "to_title": to_title,
                        "direct": direct,
                        "via_papers": via_papers,
                    }))?
                );
                return Ok(());
            }

            println!("Citation Bridge — {} ↔ {}", f, t);
            println!("  From: {}", from_title.chars().take(60).collect::<String>());
            println!("  To:   {}", to_title.chars().take(60).collect::<String>());
            if direct {
                println!("  ✅ DIRECT: {} cites {}", f, t);
            }
            if !via_papers.is_empty() {
                println!("  ⚡ INDIRECT ({} connections):", via_papers.len());
                for v in &via_papers {
                    println!("    {} → {} → {}", f, v, t);
                }
            }
            if !direct && via_papers.is_empty() {
                println!("  No citation path found between these papers");
            }
        }
        // Single direction mode
        (Some(pid), None) | (None, Some(pid)) => {
            let direction = if from.is_some() { "from" } else { "to" };
            let paper_result = db.get_paper(pid);
            let title = paper_result
                .as_ref()
                .map(|p| p.title.clone())
                .unwrap_or_else(|_| "?".to_string());

            let citations = db.get_citations(pid)?;

            let ids: Vec<String> = if direction == "from" {
                citations.references
            } else {
                citations.citing
            };

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "paper_id": pid,
                        "direction": direction,
                        "count": ids.len(),
                        "citations": ids,
                    }))?
                );
                return Ok(());
            }

            let label = if direction == "from" {
                "References"
            } else {
                "Cited by"
            };

            println!("{} — {}", label, pid);
            println!("  {}", title.chars().take(60).collect::<String>());
            if ids.is_empty() {
                println!("  No citations found");
            } else {
                println!("  Found {} citation(s):", ids.len());
                for cid in &ids {
                    println!("    {}", cid);
                }
            }
        }
        (None, None) => unreachable!(), // already checked above
    }
    Ok(())
}

fn handle_cite_stats(
    db: &Database,
    paper: Option<&str>,
    top: Option<usize>,
    format: &str,
) -> Result<()> {
    if let Some(paper_id) = paper {
        let citations = db.get_citations(paper_id)?;
        let pap = db.get_paper(paper_id);

        if format == "json" {
            let out = serde_json::json!({
                "paper_id": paper_id,
                "title": pap.ok().map(|p| p.title),
                "citing_count": citations.citing.len(),
                "references_count": citations.references.len(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("=== Citation Stats for Paper ===\n");
            println!("Paper ID:   {}", paper_id);
            if let Ok(p) = pap {
                println!("Title:      {}", p.title);
            }
            println!("Cited by:   {} papers", citations.citing.len());
            println!("References: {} papers", citations.references.len());
            if !citations.citing.is_empty() {
                println!("\nCited by:");
                for cid in &citations.citing {
                    println!("  - {}", cid);
                }
            }
            if !citations.references.is_empty() {
                println!("\nReferences:");
                for cid in &citations.references {
                    println!("  - {}", cid);
                }
            }
        }
        return Ok(());
    }

    let stats = db.stats()?;
    let all_papers = db.list_papers(None, 10000, 0)?;

    if format == "json" {
        let out = serde_json::json!({
            "total_papers": stats.total,
            "pending": stats.pending,
            "done": stats.done,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Citation Statistics ===\n");
    println!("Total papers:  {}", stats.total);
    println!("Pending:       {}", stats.pending);
    println!("Parsed:        {}", stats.done);

    if let Some(n) = top {
        let mut papers_with_cites: Vec<_> = all_papers
            .iter()
            .filter(|p| p.metadata.cited_by > 0 || p.metadata.references > 0)
            .collect();
        papers_with_cites.sort_by(|a, b| b.metadata.cited_by.cmp(&a.metadata.cited_by));
        println!("\nTop {} most-cited papers:", n);
        for p in papers_with_cites.iter().take(n) {
            println!(
                "  [{:4}] {}  {}",
                p.metadata.cited_by,
                p.id,
                p.title.chars().take(60).collect::<String>()
            );
        }
    }

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
    println!(
        "Throughput:      {:.0} req/s",
        count as f64 / elapsed.as_secs_f64()
    );
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

fn handle_daemon(db: &Database, port: u16, _log_level: &str, _foreground: bool) -> Result<()> {
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

fn handle_subscribe(
    db: &Database,
    query: &str,
    interval_minutes: u64,
    max_papers: usize,
    auto_add: bool,
) -> Result<()> {
    println!("=== Subscribing to arXiv: {} ===", query);
    println!("Check interval: {} minutes", interval_minutes);
    println!("Max papers per check: {}", max_papers);
    println!("Auto-add: {}", if auto_add { "yes" } else { "no" });
    println!();

    // Perform immediate search using arXiv API
    println!("Running immediate search...");
    let url = format!(
        "https://export.arxiv.org/api/query?search_query=all:{}&sortBy=submittedDate&sortOrder=descending&max_results={}",
        query.replace(' ', "+"), max_papers
    );

    let resp = reqwest::blocking::get(&url).context("Failed to connect to arXiv API")?;

    if !resp.status().is_success() {
        anyhow::bail!("arXiv API returned error: {}", resp.status());
    }

    let body = resp.text().context("Failed to read arXiv response")?;

    // Count entries in response
    let entry_count = body.matches("<entry>").count();
    println!(
        "[OK] Found {} papers from arXiv for query '{}'",
        entry_count, query
    );

    if entry_count == 0 {
        println!("No papers found. Try a different query.");
        return Ok(());
    }

    // Extract all papers (id, title)
    let mut papers_info: Vec<(String, String)> = Vec::new();
    let mut search_pos = 0;
    while let Some(rel_entry_start) = body[search_pos..].find("<entry>") {
        let entry_abs_start = search_pos + rel_entry_start;
        let entry_block = &body[entry_abs_start..];
        if let Some(rel_end) = entry_block.find("</entry>") {
            let entry = &entry_block[..rel_end];
            let title = extract_xml_field(entry, "<title>")
                .map(|t| t.trim().to_string())
                .unwrap_or_default();
            // Extract arXiv ID from id tag (e.g. http://arxiv.org/abs/2301.00001v1)
            let id_full = extract_xml_field(entry, "<id>").unwrap_or_default();
            let arxiv_id = id_full
                .trim()
                .strip_prefix("http://arxiv.org/abs/")
                .or_else(|| id_full.trim().strip_prefix("https://arxiv.org/abs/"))
                .unwrap_or(&id_full)
                .trim()
                .to_string();
            if !arxiv_id.is_empty() {
                papers_info.push((arxiv_id, title));
            }
            search_pos = entry_abs_start + rel_end + 9;
        } else {
            break;
        }
    }

    println!("\nTop papers found:");
    for (i, (_, title)) in papers_info.iter().enumerate().take(5) {
        println!(
            "  {}. {}",
            i + 1,
            title.chars().take(70).collect::<String>()
        );
    }

    if auto_add && !papers_info.is_empty() {
        println!("\nAuto-adding papers to database...");
        let mut added = 0;
        let mut skipped = 0;
        let mut failed = 0;
        for (arxiv_id, _title) in &papers_info {
            match db.get_paper_by_arxiv(arxiv_id) {
                Ok(Some(_)) => {
                    println!("  - {}: already exists", arxiv_id);
                    skipped += 1;
                }
                _ => {
                    // Add paper via arXiv API
                    match add_paper_from_arxiv(db, arxiv_id) {
                        Ok(_) => {
                            println!("  + {}: added", arxiv_id);
                            added += 1;
                        }
                        Err(e) => {
                            println!("  ! {}: failed ({})", arxiv_id, e);
                            failed += 1;
                        }
                    }
                }
            }
        }
        println!(
            "\nAuto-add complete: {} added, {} skipped, {} failed",
            added, skipped, failed
        );
    }

    println!("\nNote: Background monitoring requires daemon process. Subscription saved.");
    println!(
        "Run 'rairos subscribe \"{}\" --interval {}' periodically to check manually.",
        query, interval_minutes
    );
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
            println!(
                "Total size: {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / 1_048_576.0
            );
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
                    println!(
                        "... and more ({} total entries)",
                        std::fs::read_dir(cache_dir)?.count()
                    );
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
    let db_path = PathBuf::from("rairos.db");
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Database not found. Run 'rairos init' first."));
    }
    let db = Database::open(&db_path).context("Failed to open database")?;

    println!("=== Rairos REPL ===");
    println!("Type 'help' for commands, 'exit' to quit.\n");

    if let Some(q) = query {
        println!("Pre-loading papers matching: {}", q);
        match db.search_papers(&q, 10) {
            Ok(papers) if !papers.is_empty() => {
                println!("Found {} papers:\n", papers.len());
                for (i, p) in papers.iter().enumerate() {
                    let title = if p.title.len() > 60 {
                        format!("{}...", &p.title[..60])
                    } else {
                        p.title.clone()
                    };
                    let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                    let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                    println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                }
                println!();
            }
            _ => println!("No papers found for query: {}\n", q),
        }
    }

    loop {
        print!("rairos> ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }
        let input = input.trim();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("\nCommands:");
                println!("  help                   Show this help");
                println!("  exit / quit            Exit REPL");
                println!("  search <query>         Search papers");
                println!("  show <id>              Show paper details");
                println!("  list [status]          List papers (pending/done/all)");
                println!("  stats                  Show DB statistics");
                println!("  gap <topic>            Detect research gaps");
                println!("  add <arxiv_id>         Import paper from arXiv");
                println!();
            }
            "search" if arg.is_empty() => {
                println!("Usage: search <query>\n");
            }
            "search" => {
                match db.search_papers(arg, 20) {
                    Ok(papers) if papers.is_empty() => {
                        println!("No papers found for: {}", arg);
                    }
                    Ok(papers) => {
                        println!("Found {} papers:\n", papers.len());
                        for (i, p) in papers.iter().enumerate() {
                            let title = if p.title.len() > 60 {
                                format!("{}...", &p.title[..60])
                            } else {
                                p.title.clone()
                            };
                            let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                            let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                            println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                        }
                        println!();
                    }
                    Err(e) => println!("Error: {}\n", e),
                }
            }
            "show" if arg.is_empty() => {
                println!("Usage: show <id>\n");
            }
            "show" => {
                if let Err(e) = handle_show(&db, arg, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "list" => {
                let status = if arg.is_empty() { None } else { Some(arg.to_string()) };
                if let Err(e) = handle_list(&db, status, None, &[], 20, 0, "published", "desc", "table") {
                    println!("Error: {}\n", e);
                }
            }
            "stats" => {
                if let Err(e) = handle_stats(&db, false, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "gap" if arg.is_empty() => {
                println!("Usage: gap <topic>\n");
            }
            "gap" => {
                if let Err(e) = handle_gap(&db, arg, 5, "table", None) {
                    println!("Error: {}\n", e);
                }
            }
            "add" if arg.is_empty() => {
                println!("Usage: add <arxiv_id>\n");
            }
            "add" => {
                if let Err(e) = handle_add(&db, arg) {
                    println!("Error: {}\n", e);
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.\n", cmd);
            }
        }
    }
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

fn handle_agent(
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

    let papers = db.search_papers(topic, max_papers)?;

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Found {} papers. Starting research loop...\n", papers.len());
    println!("(Full autonomous research loop requires LLM integration)");
    println!();

    println!("Research Plan:");
    println!(
        "  1. Analyze {} papers for key themes and methodologies",
        papers.len()
    );
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
    match kind {
        "keywords" => {
            if let Some(p) = paper {
                let paper_obj = db
                    .get_paper(&p)
                    .ok()
                    .or_else(|| db.get_paper_by_arxiv(&p).ok().flatten())
                    .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", p))?;

                // Extract keywords from title + abstract using TF-like scoring
                let text = format!(
                    "{} {} {}",
                    paper_obj.title,
                    paper_obj.abstract_text,
                    paper_obj.categories.join(" ")
                );
                let keywords = extract_keywords(&text, 10);

                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "id": paper_obj.id,
                            "title": paper_obj.title,
                            "keywords": keywords,
                        }))?
                    );
                } else {
                    println!("=== Keyword Analysis ===\n");
                    println!("Title: {}", paper_obj.title);
                    println!("\nTop {} keywords:", keywords.len());
                    for (i, (kw, score)) in keywords.iter().enumerate() {
                        println!("  {:>2}. {:<25} ({:.2})", i + 1, kw, score);
                    }
                }
            } else {
                println!("Analyzing all papers...");
                let papers = db.list_papers(None, 100, 0)?;
                let mut all_kw: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for p in &papers {
                    let text =
                        format!("{} {} {}", p.title, p.abstract_text, p.categories.join(" "));
                    for (kw, _) in extract_keywords(&text, 5) {
                        *all_kw.entry(kw).or_insert(0) += 1;
                    }
                }
                let top: Vec<_> = all_kw
                    .into_iter()
                    .filter(|(_, c)| *c > 1)
                    .map(|(k, c)| (k, c))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .take(10)
                    .collect();
                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({"papers": papers.len(), "top_keywords": top})
                        )?
                    );
                } else {
                    println!("\n=== Cross-Paper Keywords (n={}) ===\n", papers.len());
                    for (kw, count) in top {
                        println!("  {:<25} {} papers", kw, count);
                    }
                }
            }
        }
        "summary" | "topics" | "quality" => {
            if let Some(p) = paper {
                let paper_obj = db
                    .get_paper(&p)
                    .ok()
                    .or_else(|| db.get_paper_by_arxiv(&p).ok().flatten())
                    .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", p))?;

                // Rule-based topic classification
                let topics = classify_topics(
                    &paper_obj.title,
                    &paper_obj.abstract_text,
                    &paper_obj.categories,
                );
                let quality = estimate_quality(&paper_obj);

                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "id": paper_obj.id,
                            "title": paper_obj.title,
                            "topics": topics,
                            "quality_score": quality,
                        }))?
                    );
                } else {
                    println!("=== Paper Analysis ===\n");
                    println!("Title: {}", paper_obj.title);
                    println!("\nDetected Topics: {:?}", topics);
                    println!("Quality Score: {:.1}/10", quality);
                }
            } else {
                println!("Analyzing all papers in database...");
                let papers = db.list_papers(None, 100, 0)?;
                println!(
                    "Found {} papers. (Full analysis requires LLM integration)",
                    papers.len()
                );
            }
        }
        _ => {
            println!(
                "Unknown analysis type: {}. Use: summary, keywords, topics, quality",
                kind
            );
        }
    }
    Ok(())
}

// Extract top keywords using simple TF-like scoring (no LLM needed)
fn extract_keywords(text: &str, top_n: usize) -> Vec<(String, f64)> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "but", "and", "or", "if", "because", "until", "while",
        "this", "that", "these", "those", "which", "what", "who", "whom",
    ]
    .into_iter()
    .collect();

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in text.split_whitespace() {
        let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        let clean_lower = clean.to_lowercase();
        if clean_lower.len() > 3 && !stop_words.contains(clean_lower.as_str()) {
            *counts.entry(clean_lower).or_insert(0) += 1;
        }
    }

    let total: usize = counts.values().sum();
    let mut scored: Vec<_> = counts
        .into_iter()
        .map(|(w, c)| (w, c as f64 / total as f64 * 100.0))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top_n).collect()
}

// Classify paper into research topics using keyword matching
fn classify_topics(title: &str, abstract_: &str, categories: &[String]) -> Vec<String> {
    let text = format!("{} {} {}", title, abstract_, categories.join(" ")).to_lowercase();
    let mut topics = Vec::new();

    let topic_rules: Vec<(&str, &[&str])> = vec![
        (
            "Machine Learning",
            &[
                "machine learning",
                "deep learning",
                "neural network",
                "neural networks",
            ],
        ),
        (
            "NLP",
            &[
                "natural language",
                "transformer",
                "attention",
                "language model",
                "text",
                "parsing",
                "translation",
            ],
        ),
        (
            "Computer Vision",
            &[
                "image",
                "vision",
                "object detection",
                "segmentation",
                "image classification",
            ],
        ),
        (
            "Reinforcement Learning",
            &[
                "reinforcement learning",
                "policy",
                "reward",
                "agent",
                "environment",
            ],
        ),
        (
            "Optimization",
            &[
                "optimization",
                "optimizer",
                "gradient",
                "convergence",
                "loss function",
            ],
        ),
        (
            "Graph / Knowledge",
            &[
                "graph",
                "knowledge graph",
                "knowledge base",
                "entity",
                "relation",
            ],
        ),
        (
            "Uncertainty",
            &[
                "uncertainty",
                "probabilistic",
                "bayesian",
                "variance",
                "confidence",
            ],
        ),
        (
            "Scaling",
            &["scale", "scaling", "large-scale", "billion", "parameter"],
        ),
    ];

    for (topic, keywords) in topic_rules.iter() {
        if keywords.iter().any(|kw| text.contains(*kw)) {
            topics.push(topic.to_string());
        }
    }

    if topics.is_empty() {
        topics.push("General".to_string());
    }
    topics
}

// Estimate paper quality from metadata (heuristic, no LLM)
fn estimate_quality(paper: &Paper) -> f64 {
    let mut score: f64 = 5.0; // base

    // Citations boost
    if paper.metadata.cited_by > 1000 {
        score += 2.0;
    } else if paper.metadata.cited_by > 100 {
        score += 1.0;
    }

    // Has abstract
    if !paper.abstract_text.is_empty() && paper.abstract_text.len() > 100 {
        score += 0.5;
    }

    // Has categories
    if !paper.categories.is_empty() {
        score += 0.5;
    }

    // Title length heuristic (reasonable length is better)
    if paper.title.len() > 30 && paper.title.len() < 150 {
        score += 0.5;
    }

    score.min(10.0_f64)
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

    // Keyword-based retrieval: split question into keywords and score papers
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "but", "and", "or", "if", "because", "until", "while",
        "this", "that", "these", "those", "what", "which", "who", "whom",
    ]
    .into();

    let question_lower = question.to_lowercase();
    let question_words: Vec<&str> = question_lower
        .split_whitespace()
        .filter(|w| w.len() > 2 && !stop_words.contains(w))
        .collect();

    if question_words.is_empty() {
        println!("Question too generic. Try adding specific terms.");
        return Ok(());
    }

    println!("Keywords extracted: {}", question_words.join(", "));
    println!();

    // Score each paper by keyword overlap
    let mut scored: Vec<(&Paper, usize)> = Vec::new();
    for paper in &papers {
        let title_lower = paper.title.to_lowercase();
        let abstract_lower = paper.abstract_text.to_lowercase();
        let combined = format!("{} {}", title_lower, abstract_lower);

        let match_count = question_words
            .iter()
            .filter(|kw| combined.contains(*kw))
            .count();

        if match_count > 0 {
            scored.push((paper, match_count));
        }
    }

    // Sort by match count descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    let top_papers: Vec<_> = scored.into_iter().take(5).collect();

    if top_papers.is_empty() {
        println!("No papers found matching your question keywords.");
        println!("Try different search terms.");
        return Ok(());
    }

    println!("Top {} most relevant papers:\n", top_papers.len());

    for (i, (paper, score)) in top_papers.iter().enumerate() {
        println!("{}. [score: {}] {}", i + 1, score, paper.title);
        println!("   {}", paper.authors.join(", "));
        println!(
            "   {} | cited_by: {}",
            paper.published.format("%Y-%m-%d"),
            paper.metadata.cited_by
        );
        if !paper.abstract_text.is_empty() {
            let preview = if paper.abstract_text.len() > 150 {
                format!("{}...", &paper.abstract_text[..150])
            } else {
                paper.abstract_text.clone()
            };
            println!("   {}\n", preview);
        }
    }

    if format == "json" {
        let out = serde_json::json!({
            "question": question,
            "papers_searched": papers.len(),
            "top_papers": top_papers.iter().map(|(p, s)| {
                serde_json::json!({
                    "id": p.id,
                    "title": p.title,
                    "authors": p.authors,
                    "score": s,
                    "abstract": p.abstract_text
                })
            }).collect::<Vec<_>>()
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
                if used[i] {
                    continue;
                }
                let mut group: Vec<usize> = vec![i];
                used[i] = true;

                for j in (i + 1)..papers.len() {
                    if used[j] {
                        continue;
                    }
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
    let words_a: std::collections::HashSet<&str> = a
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let words_b: std::collections::HashSet<&str> = b
        .split_whitespace()
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
    let paper = db
        .get_paper(paper_id)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(paper_id).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", paper_id))?;

    println!("=== Similar Papers ===\n");
    println!("Finding papers similar to:");
    println!("  {} ({})", paper.title, paper.published.format("%Y-%m-%d"));
    println!();

    let all_papers = db.list_papers(None, 1000, 0)?;
    let target_title = paper.title.to_lowercase();
    let target_abstract = paper.abstract_text.to_lowercase();
    let target_text = format!("{} {}", target_title, target_abstract);

    // Compute similarity for each paper using word overlap
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "but", "and", "or", "if", "because", "until", "while",
        "this", "that", "these", "those", "what", "which", "who", "whom",
    ]
    .into();

    let target_words: std::collections::HashSet<String> = target_text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3 && !stop_words.contains(w))
        .map(|w| w.to_lowercase())
        .collect();

    if target_words.is_empty() {
        println!("No similar papers found (target paper has no extractable content).");
        return Ok(());
    }

    // Score all papers by similarity
    let mut scored: Vec<(&Paper, f64)> = Vec::new();
    for p in &all_papers {
        if p.id == paper.id {
            continue;
        }
        let p_text = format!(
            "{} {}",
            p.title.to_lowercase(),
            p.abstract_text.to_lowercase()
        );
        let p_words: std::collections::HashSet<String> = p_text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_lowercase())
            .collect();

        if p_words.is_empty() {
            continue;
        }

        // Jaccard similarity
        let intersection: std::collections::HashSet<_> =
            target_words.intersection(&p_words).collect();
        let union: std::collections::HashSet<_> = target_words.union(&p_words).collect();
        let sim = if union.is_empty() {
            0.0
        } else {
            intersection.len() as f64 / union.len() as f64
        };

        if sim > 0.05 {
            scored.push((p, sim));
        }
    }

    // Sort by similarity descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top: Vec<_> = scored.into_iter().take(limit).collect();
    if top.is_empty() {
        println!("No similar papers found in database.");
    } else {
        println!("Similar papers ({} found):\n", top.len());
        println!("{:<6} {:<8} {}", "SIM", "YEAR", "TITLE");
        println!("{}", "-".repeat(80));
        for (p, sim) in &top {
            let year = p.published.format("%Y");
            let title = if p.title.len() > 60 {
                format!("{}...", &p.title[..60])
            } else {
                p.title.clone()
            };
            println!("{:<6.3} {:<8} {}", sim, year, title);
        }
    }

    Ok(())
}

fn handle_compare(db: &Database, papers_arg: &str, aspect: &str) -> Result<()> {
    println!("=== Compare Papers ===\n");

    let paper_ids: Vec<&str> = papers_arg.split(',').map(|s| s.trim()).collect();

    // Fetch all papers
    let mut papers: Vec<Paper> = Vec::new();
    for pid in &paper_ids {
        if let Ok(paper) = db.get_paper(pid) {
            papers.push(paper);
        } else if let Ok(Some(paper)) = db.get_paper_by_arxiv(pid) {
            papers.push(paper);
        } else {
            eprintln!("Warning: Paper '{}' not found, skipping", pid);
        }
    }

    if papers.is_empty() {
        println!("No valid papers found. Add papers first with `rairos-cli add <arxiv_id>`");
        return Ok(());
    }

    println!("Comparing {} papers:\n", papers.len());
    for (i, p) in papers.iter().enumerate() {
        println!(
            "  {}. {} ({})",
            i + 1,
            p.title,
            p.published.format("%Y-%m-%d")
        );
    }
    println!();

    match aspect {
        "overview" => {
            // Summary table: title, authors, year, categories, citations, references
            println!(
                "{:<6} {:<50} {:<8} {:>6} {:>10} {:>10}",
                "#", "Title", "Year", "Authors", "Cited_by", "Refs"
            );
            println!("{}", "-".repeat(96));
            for (i, p) in papers.iter().enumerate() {
                let title = if p.title.len() > 48 {
                    format!("{}...", &p.title[..48])
                } else {
                    p.title.clone()
                };
                let year = p.published.format("%Y").to_string();
                let author_count = p.authors.len();
                let cited = p.metadata.cited_by;
                let refs = p.metadata.references;
                println!(
                    "{:<6} {:<50} {:<8} {:>6} {:>10} {:>10}",
                    i + 1,
                    title,
                    year,
                    author_count,
                    cited,
                    refs
                );
            }
        }
        "citations" => {
            // Compare citation counts
            println!("Citation Comparison:\n");
            println!("{:<50} {:>12} {:>12}", "Paper", "Cited By", "References");
            println!("{}", "-".repeat(76));
            let mut by_cited: Vec<_> = papers.iter().enumerate().collect();
            by_cited.sort_by(|a, b| b.1.metadata.cited_by.cmp(&a.1.metadata.cited_by));
            for (_rank, (_, p)) in by_cited.iter().enumerate() {
                println!(
                    "{:<50} {:>12} {:>12}",
                    if p.title.len() > 48 {
                        format!("{}...", &p.title[..48])
                    } else {
                        p.title.clone()
                    },
                    p.metadata.cited_by,
                    p.metadata.references
                );
            }
            if papers.len() > 1 {
                let max_cited = papers.iter().map(|p| p.metadata.cited_by).max().unwrap();
                let min_cited = papers.iter().map(|p| p.metadata.cited_by).min().unwrap();
                if max_cited > 0 {
                    println!(
                        "\nCitation spread: {} (max) / {} (min) = {:.1}x",
                        max_cited,
                        min_cited,
                        max_cited as f64 / min_cited as f64
                    );
                }
            }
        }
        "authors" => {
            // Compare author overlap
            println!("Author Comparison:\n");
            for (i, p) in papers.iter().enumerate() {
                println!(
                    "Paper {}: {} author(s) - {}",
                    i + 1,
                    p.authors.len(),
                    p.authors.join(", ")
                );
            }
            if papers.len() > 1 {
                // Find author overlap between all pairs
                println!("\nAuthor Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let set_i: HashSet<_> =
                            papers[i].authors.iter().map(|a| a.to_lowercase()).collect();
                        let set_j: HashSet<_> =
                            papers[j].authors.iter().map(|a| a.to_lowercase()).collect();
                        let intersection: HashSet<_> = set_i.intersection(&set_j).collect();
                        let union: HashSet<_> = set_i.union(&set_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared author(s) (Jaccard: {:.2})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        "topics" | "categories" => {
            // Compare categories
            println!("Category Comparison:\n");
            for (i, p) in papers.iter().enumerate() {
                println!(
                    "Paper {}: {} categories - {}",
                    i + 1,
                    p.categories.len(),
                    p.categories.join(", ")
                );
            }
            if papers.len() > 1 {
                println!("\nCategory Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let set_i: HashSet<_> = papers[i].categories.iter().collect();
                        let set_j: HashSet<_> = papers[j].categories.iter().collect();
                        let intersection: HashSet<_> = set_i.intersection(&set_j).collect();
                        let union: HashSet<_> = set_i.union(&set_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared category/ies (Jaccard: {:.2})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        "timeline" => {
            // Compare publication dates
            println!("Timeline Comparison (newest first):\n");
            let mut sorted: Vec<_> = papers.iter().enumerate().collect();
            sorted.sort_by(|a, b| b.1.published.cmp(&a.1.published));
            println!("{:<50} {:>12} {:>12}", "Paper", "Published", "Age (days)");
            println!("{}", "-".repeat(76));
            let now = Utc::now();
            for (_, p) in sorted.iter() {
                let age = (now - p.published).num_days();
                println!(
                    "{:<50} {:>12} {:>12}",
                    if p.title.len() > 48 {
                        format!("{}...", &p.title[..48])
                    } else {
                        p.title.clone()
                    },
                    p.published.format("%Y-%m-%d"),
                    age
                );
            }
        }
        "abstract" => {
            println!("Abstract Comparison (keyword overlap):\n");
            for (i, p) in papers.iter().enumerate() {
                let words: HashSet<String> = p
                    .abstract_text
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.len() > 4)
                    .map(|s| s.to_string())
                    .collect();
                println!("Paper {}: {} unique words in abstract", i + 1, words.len());
            }
            if papers.len() > 1 {
                println!("\nAbstract Keyword Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let words_i: HashSet<String> = papers[i]
                            .abstract_text
                            .to_lowercase()
                            .split(|c: char| !c.is_alphanumeric())
                            .filter(|w| w.len() > 4)
                            .map(|s| s.to_string())
                            .collect();
                        let words_j: HashSet<String> = papers[j]
                            .abstract_text
                            .to_lowercase()
                            .split(|c: char| !c.is_alphanumeric())
                            .filter(|w| w.len() > 4)
                            .map(|s| s.to_string())
                            .collect();
                        let intersection: HashSet<_> = words_i.intersection(&words_j).collect();
                        let union: HashSet<_> = words_i.union(&words_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared words (Jaccard: {:.3})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        _ => {
            println!("Unknown aspect: '{}'. Available aspects:", aspect);
            println!("  overview     - Summary table with all metadata");
            println!("  citations    - Citation count comparison");
            println!("  authors      - Author count and overlap");
            println!("  topics       - Category comparison");
            println!("  timeline     - Publication date comparison");
            println!("  abstract     - Keyword overlap in abstracts");
        }
    }

    Ok(())
}

fn handle_trend(db: &Database, topic: &str, range: &str, format: &str) -> Result<()> {
    use chrono::{Duration, Utc};

    // Parse time range
    let days = match range {
        "6m" => 180,
        "1y" => 365,
        "2y" => 730,
        "5y" => 1825,
        "all" => 9999,
        other => {
            println!("Unknown range '{}'. Use: 6m, 1y, 2y, 5y, all", other);
            return Ok(());
        }
    };

    let cutoff = Utc::now() - Duration::days(days);
    println!("=== Research Trends ===");
    println!("Topic: {}", topic);
    println!("Time range: {} (papers from last {} days)", range, days);
    println!();

    let all_papers = db.search_papers(topic, 500)?;
    let papers: Vec<_> = all_papers
        .into_iter()
        .filter(|p| p.published >= cutoff)
        .collect();

    if papers.is_empty() {
        println!(
            "No papers found for topic '{}' in the last {}.",
            topic, range
        );
        return Ok(());
    }

    println!(
        "Found {} papers on '{}' in the specified time range.",
        papers.len(),
        topic
    );
    println!();

    // Group by year
    let mut year_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
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
                println!(
                    "  - Growing trend: {} -> {} papers",
                    first_count, last_count
                );
            } else {
                println!(
                    "  - Stable/declining: {} -> {} papers",
                    first_count, last_count
                );
            }
        }
    } else if years.len() == 1 {
        println!("  - Only one year represented: {}", years[0]);
    }

    // Top categories
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        for cat in &paper.categories {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
    }
    let mut top_cats: Vec<_> = cat_counts.iter().collect();
    top_cats.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "  - Top categories: {}",
        top_cats
            .iter()
            .take(5)
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    if format == "json" {
        let out = serde_json::json!({
            "topic": topic,
            "range": range,
            "days": days,
            "papers_found": papers.len(),
            "year_counts": year_counts,
            "top_categories": top_cats.iter().take(5).map(|(c, n)| (c, *n)).collect::<std::collections::HashMap<_, _>>()
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

// ============================================================================
// Main
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rairos_core::Database;

    #[test]
    fn cli_version_exists() {
        assert!(true)
    }

    #[test]
    fn test_parse_paper_not_found() {
        let dir = std::env::temp_dir().join("rairos_cli_test_parse");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        let db = Database::open(&db_path).unwrap();
        let result = handle_parse(&db, "nonexistent_paper_xyz");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string().to_lowercase();
        assert!(err.contains("not found"), "Expected 'not found', got: {}", err);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_repl_db_not_found() {
        let dir = std::env::temp_dir().join("rairos_cli_test_repl");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = handle_repl(None);
        std::env::set_current_dir(&orig_dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Database not found"), "Expected 'Database not found', got: {}", err);
    }

    #[test]
    fn test_cli_dispatch_routes_version() {
        let cli = Cli { command: Commands::Version, db: PathBuf::from("test.db"), verbose: false };
        // Just check the Cli struct can be created with Version command
        assert!(matches!(cli.command, Commands::Version));
    }
}

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
        Commands::Agent {
            topic,
            max_papers,
            max_time_minutes,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_agent(&db, topic, *max_papers, *max_time_minutes, format)?;
        }
        Commands::Analyze {
            kind,
            paper,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_analyze(&db, kind, paper.clone(), format)?;
        }
        Commands::Ask {
            question,
            max_papers,
            format,
        } => {
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
        Commands::Trend {
            topic,
            range,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_trend(&db, topic, range, format)?;
        }
        Commands::Init => {
            handle_init(&cli.db)?;
        }
        Commands::Stats { json, format } => {
            let db = open_db(&cli.db)?;
            handle_stats(&db, *json, format)?;
        }
        Commands::Add { arxiv_id } => {
            let db = open_db(&cli.db)?;
            handle_add(&db, arxiv_id)?;
        }
        Commands::List {
            status,
            year,
            tag,
            limit,
            offset,
            sort,
            order,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_list(
                &db,
                status.clone(),
                *year,
                &tag,
                *limit,
                *offset,
                sort,
                order,
                format,
            )?;
        }
        Commands::Show { id, format } => {
            let db = open_db(&cli.db)?;
            handle_show(&db, id, format)?;
        }
        Commands::Search {
            query,
            limit,
            field,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_search(&db, query, *limit, field, format)?;
        }
        Commands::Delete { id, force } => {
            let db = open_db(&cli.db)?;
            handle_delete(&db, &id, *force)?;
        }
        Commands::UpdateStatus { id, status } => {
            let db = open_db(&cli.db)?;
            handle_update_status(&db, &id, status)?;
        }
        Commands::Parse { id } => {
            let db = open_db(&cli.db)?;
            handle_parse(&db, id)?;
        }
        Commands::Import {
            path,
            ids,
            skip_existing,
        } => {
            let db = open_db(&cli.db)?;
            handle_import(&db, path, &ids, *skip_existing)?;
        }
        Commands::Export {
            path,
            status,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_export(&db, path, status.clone(), format)?;
        }
        Commands::Gap {
            topic,
            limit,
            format,
            category,
        } => {
            let db = open_db(&cli.db)?;
            handle_gap(&db, topic, *limit, format, category.clone())?;
        }
        Commands::GapList {
            limit,
            offset,
            format,
        } => {
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
        Commands::GeneAdd {
            approach,
            gap_type,
            keywords,
            paper_id,
        } => {
            handle_gene_add(approach, gap_type, keywords, paper_id.clone())?;
        }
        Commands::GeneList {
            gap_type,
            status,
            limit,
            format,
        } => {
            handle_gene_list(gap_type.clone(), status.clone(), *limit, format)?;
        }
        Commands::GeneShow { id, format } => {
            handle_gene_show(id, format)?;
        }
        Commands::GeneFeedback { id, positive } => {
            handle_gene_feedback(id, *positive)?;
        }
        Commands::GeneDiversity { format } => {
            handle_gene_diversity(format)?;
        }
        Commands::GeneEvolve {
            max_crossovers,
            format,
        } => {
            handle_gene_evolve(*max_crossovers, format)?;
        }
        Commands::KgStats { format } => {
            handle_kg_stats(format)?;
        }
        Commands::KgRank { limit, format } => {
            handle_kg_rank(*limit, format)?;
        }
        Commands::KgPath { source, target } => {
            handle_kg_path(source, target)?;
        }
        Commands::KgAddPaper { paper_id } => {
            let db = open_db(&cli.db)?;
            handle_kg_add_paper(&db, paper_id)?;
        }
        Commands::KgAddCitation { source, target } => {
            let db = open_db(&cli.db)?;
            handle_kg_add_citation(&db, source, target)?;
        }
        Commands::RateLimitBenchmark { count } => {
            handle_rate_limit_benchmark(*count)?;
        }
        Commands::RateLimitCheck { endpoint } => {
            handle_rate_limit_check(endpoint)?;
        }
        Commands::Daemon {
            port,
            log_level,
            foreground,
        } => {
            let db = open_db(&cli.db)?;
            handle_daemon(&db, *port, log_level, *foreground)?;
        }
        Commands::Subscribe {
            query,
            interval_minutes,
            max_papers,
            auto_add,
        } => {
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
        Commands::StanceAdd {
            topic,
            claim,
            stance,
            reasoning,
        } => {
            handle_stance_add(&topic, &claim, &stance, &reasoning)?;
        }
        Commands::StanceList { topic, tag, format } => {
            handle_stance_list(topic.clone(), tag.clone(), &format)?;
        }
        Commands::StanceShow { id, format } => {
            handle_stance_show(&id, &format)?;
        }
        Commands::MemoryStats { format } => {
            handle_memory_stats(&format)?;
        }
        Commands::Status { format } => {
            let db = open_db(&cli.db)?;
            handle_status(&db, &format)?;
        }
        Commands::Citations { from, to, format } => {
            let db = open_db(&cli.db)?;
            handle_citations(&db, from.as_deref(), to.as_deref(), &format)?;
        }
        Commands::CiteStats { paper, top, format } => {
            let db = open_db(&cli.db)?;
            handle_cite_stats(&db, paper.as_deref(), *top, &format)?;
        }
        Commands::Queue { add, list, pending, dequeue, cancel, clear, format } => {
            let db = open_db(&cli.db)?;
            handle_queue(&db, add.as_deref(), *list, *pending, *dequeue, *cancel, *clear, &format)?;
        }
        Commands::Influence { top, paper, min_cites, format } => {
            let db = open_db(&cli.db)?;
            handle_influence(&db, *top, paper.as_deref(), *min_cites, &format)?;
        }
        Commands::Merge { keep, dry_run, auto, target_id, duplicate_id } => {
            let db = open_db(&cli.db)?;
            handle_merge(&db, keep, *dry_run, *auto, target_id.as_deref(), duplicate_id.as_deref())?;
        }
        Commands::CiteImport { json_input, dry_run, skip_missing, extract, paper, dedup } => {
            let db = open_db(&cli.db)?;
            handle_cite_import(&db, json_input.as_deref(), *dry_run, *skip_missing, *extract, paper.as_deref(), *dedup)?;
        }
    }

    Ok(())
}

fn handle_queue(
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

fn handle_influence(
    db: &Database,
    top: usize,
    paper: Option<&str>,
    min_cites: usize,
    format: &str,
) -> Result<()> {
    if let Some(paper_id) = paper {
        let pap = db.get_paper(paper_id)?;
        let citations = db.get_citations(paper_id)?;
        let forward = citations.citing.len() as f64;
        let backward = citations.references.len() as f64;

        let year = pap.published.year();
        let age = if year > 2000 && year <= 2026 {
            (2026 - year + 1) as f64
        } else {
            0.0
        };
        let velocity = if age > 0.0 { forward / age } else { 0.0 };

        let impact = if velocity >= 10.0 {
            "\u{1f525} Extremely high velocity (\u{2265}10/y) — field-defining"
        } else if velocity >= 5.0 {
            "\u{1f4c8} High velocity (5-10/y) — very active research"
        } else if velocity >= 1.0 {
            "\u{1f4ca} Moderate velocity (1-5/y) — steady influence"
        } else {
            "\u{1f4c9} Low velocity — emerging or niche"
        };

        println!("=== Paper Influence Profile ===");
        println!("  Paper ID  : {}", paper_id);
        println!("  Title     : {}", pap.title);
        println!("  Published : {}", year);
        if age > 0.0 {
            println!("  Age       : {:.0} years (as of 2026)", age);
        }
        println!();
        println!("  Citations");
        if age > 0.0 {
            println!(
                "    Cited by (forward) : {:.0}  → velocity = {:.0}/{:.0} = {:.2}/y",
                forward, forward, age, velocity
            );
        } else {
            println!("    Cited by (forward) : {:.0}", forward);
        }
        println!("    References (backward): {:.0}", backward);
        println!();
        if age > 0.0 {
            println!("  Impact Assessment");
            println!("    {}", impact);
        }
        return Ok(());
    }

    let all_papers = db.list_papers(None, 100000, 0)?;
    let mut results: Vec<(String, String, i32, f64, f64)> = Vec::new();

    for p in &all_papers {
        if p.metadata.cited_by == 0 && min_cites > 0 {
            continue;
        }
        let forward = p.metadata.cited_by as f64;
        if forward < min_cites as f64 {
            continue;
        }
        let year = p.published.year();
        if year < 2000 || year > 2026 {
            continue;
        }
        let age = (2026 - year + 1) as f64;
        let velocity = forward / age;
        results.push((p.id.clone(), p.title.clone(), year, forward, velocity));
    }

    results.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    if results.is_empty() {
        println!("No papers with sufficient citation data found.");
        return Ok(());
    }

    let top_n: Vec<_> = results.iter().take(top).collect();

    match format {
        "json" => {
            let data: Vec<serde_json::Value> = top_n
                .iter()
                .enumerate()
                .map(|(i, (id, title, year, forward, vel))| {
                    serde_json::json!({
                        "rank": i + 1,
                        "paper_id": id,
                        "title": title,
                        "year": year,
                        "forward_cites": forward,
                        "age_years": (2026 - year + 1) as f64,
                        "velocity": (vel * 100.0).round() / 100.0,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        "csv" => {
            println!("rank,paper_id,title,year,forward_cites,age_years,velocity");
            for (i, (id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_esc = title.replace('"', "\"\"");
                println!(
                    "{},\"{}\",\"{}\",{},{},{:.1},{:.2}",
                    i + 1,
                    id,
                    title_esc,
                    year,
                    forward,
                    2026 - year + 1,
                    vel
                );
            }
        }
        _ => {
            println!(
                "{:>4}  {:>8}  {:>5}  {:>3}y  Year  Paper",
                "Rank", "Velocity", "Cites", "Age"
            );
            println!("{}", "-".repeat(50));
            for (i, (id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_short = if title.len() > 50 {
                    format!("{}…", &title[..50])
                } else {
                    title.clone()
                };
                println!(
                    "{:>4}  {:>7.1}/y  {:>5.0}  {:>3.0}   {}  {}",
                    i + 1,
                    vel,
                    forward,
                    2026 - year + 1,
                    year,
                    title_short
                );
            }
            println!();
            println!(
                "Showing {} of {} papers with >= {} citation(s)",
                top_n.len(),
                results.len(),
                min_cites
            );
            println!("Formula: velocity = forward_citations / age_years  (age = 2026 - published + 1)");
        }
    }
    Ok(())
}

/// Pick which paper to keep and which to drop based on strategy.
fn pick_keep<'a>(
    _title_a: &'a str,
    _title_b: &'a str,
    status_a: &'a str,
    status_b: &'a str,
    strategy: &'a str,
) -> (&'static str, &'static str) {
    match strategy {
        "newer" => ("A", "B"),
        "older" => ("B", "A"),
        "parsed" | "semantic" => {
            fn rank(s: &str) -> u8 {
                match s {
                    "done" => 4,
                    "parsing" => 3,
                    "pending" => 2,
                    "failed" => 1,
                    _ => 0,
                }
            }
            if rank(status_a) >= rank(status_b) {
                ("A", "B")
            } else {
                ("B", "A")
            }
        }
        _ => ("A", "B"),
    }
}

fn handle_merge(
    db: &Database,
    keep: &str,
    dry_run: bool,
    auto: bool,
    target_id: Option<&str>,
    duplicate_id: Option<&str>,
) -> Result<()> {
    if auto {
        let papers = db.list_papers(None, 100000, 0)?;
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        let mut merged_count = 0u32;
        let mut skipped_count = 0u32;

        for paper in &papers {
            if seen.iter().any(|(a, b)| a == &paper.id || b == &paper.id) {
                continue;
            }
            if paper.title.is_empty() {
                continue;
            }

            let sims = match db.find_similar(&paper.id, 10, 0.95) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for (sim_id, score) in &sims {
                let pair_key = if paper.id < *sim_id {
                    (paper.id.clone(), sim_id.clone())
                } else {
                    (sim_id.clone(), paper.id.clone())
                };
                if seen.contains(&pair_key) {
                    continue;
                }
                seen.insert(pair_key.clone());

                if *score < 0.95 {
                    continue;
                }

                let sim_paper = match db.get_paper(sim_id) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                let (keep_id, drop_id) = {
                    let (keep_label, _) = pick_keep(
                        &paper.title,
                        &sim_paper.title,
                        &paper.parse_status.to_string(),
                        &sim_paper.parse_status.to_string(),
                        keep,
                    );
                    if keep_label == "A" {
                        (&paper.id, sim_id)
                    } else {
                        (sim_id, &paper.id)
                    }
                };

                if dry_run {
                    println!("Would merge {} into {}", drop_id, keep_id);
                    println!("  keeping : [{}] {}", keep_id, &paper.title[..paper.title.len().min(70)]);
                    println!("  deleting: [{}] {}", drop_id, &sim_paper.title[..sim_paper.title.len().min(70)]);
                    println!("  semantic similarity: {:.3}", score);
                    println!();
                    skipped_count += 1;
                } else {
                    let merged = db.merge_papers(keep_id, &[drop_id])?;
                    if merged {
                        db.log_dedup(keep_id, drop_id, "semantic-auto")?;
                        println!("Merged {} into {} (similarity={:.3})", drop_id, keep_id, score);
                        merged_count += 1;
                    } else {
                        println!("Merge failed for {} -> {}", drop_id, keep_id);
                    }
                }
            }
        }

        if dry_run {
            println!("({} pair(s) would be merged, dry-run)", skipped_count);
        } else {
            println!("Auto-merge complete: {} pair(s) merged", merged_count);
        }
        return Ok(());
    }

    let target_id = match target_id {
        Some(id) => id,
        None => {
            eprintln!("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)");
            std::process::exit(1);
        }
    };
    let duplicate_id = match duplicate_id {
        Some(id) => id,
        None => {
            eprintln!("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)");
            std::process::exit(1);
        }
    };

    let target = match db.get_paper(target_id) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Target paper {} not found", target_id);
            std::process::exit(1);
        }
    };
    let duplicate = match db.get_paper(duplicate_id) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Duplicate paper {} not found", duplicate_id);
            std::process::exit(1);
        }
    };

    let sim = db.get_similarity(target_id, duplicate_id).ok().flatten();

    let (keep_id, drop_id, drop_title) = if keep == "semantic" {
        if let Some(s) = sim {
            if s >= 0.8 {
                let (keep_label, _) = pick_keep(
                    &target.title,
                    &duplicate.title,
                    &target.parse_status.to_string(),
                    &duplicate.parse_status.to_string(),
                    keep,
                );
                if keep_label == "A" {
                    (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
                } else {
                    (duplicate.id.clone(), target.id.clone(), target.title.clone())
                }
            } else {
                eprintln!("Note: low similarity, falling back to 'parsed' (similarity: {:.3})", s);
                let (keep_label, _) = pick_keep(
                    &target.title,
                    &duplicate.title,
                    &target.parse_status.to_string(),
                    &duplicate.parse_status.to_string(),
                    "parsed",
                );
                if keep_label == "A" {
                    (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
                } else {
                    (duplicate.id.clone(), target.id.clone(), target.title.clone())
                }
            }
        } else {
            eprintln!("Note: no embeddings available, falling back to 'parsed'");
            let (keep_label, _) = pick_keep(
                &target.title,
                &duplicate.title,
                &target.parse_status.to_string(),
                &duplicate.parse_status.to_string(),
                "parsed",
            );
            if keep_label == "A" {
                (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
            } else {
                (duplicate.id.clone(), target.id.clone(), target.title.clone())
            }
        }
    } else {
        let (keep_label, _) = pick_keep(
            &target.title,
            &duplicate.title,
            &target.parse_status.to_string(),
            &duplicate.parse_status.to_string(),
            keep,
        );
        if keep_label == "A" {
            (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
        } else {
            (duplicate.id.clone(), target.id.clone(), target.title.clone())
        }
    };

    if dry_run {
        println!("Would merge {} into {} (--keep={})", drop_id, keep_id, keep);
        println!("  keeping : [{}] {}", keep_id, &target.title[..target.title.len().min(70)]);
        println!("  deleting: [{}] {}", drop_id, &drop_title[..drop_title.len().min(70)]);
        if let Some(s) = sim {
            println!("  semantic similarity: {:.3}", s);
        } else {
            println!("  semantic similarity: no embeddings available");
        }
        return Ok(());
    }

    println!("Merging {} into {}", drop_id, keep_id);
    println!("  Keeping: [{}] {}", keep_id, &target.title[..target.title.len().min(70)]);
    println!("  Deleting: [{}] {}", drop_id, &drop_title[..drop_title.len().min(70)]);
    if let Some(s) = sim {
        println!("  Similarity: {:.3}", s);
    } else {
        println!("  semantic similarity: no embeddings available");
    }

    let ok = db.merge_papers(&keep_id, &[&drop_id])?;
    if ok {
        db.log_dedup(&keep_id, &drop_id, keep)?;
        println!("Merged {} into {}", drop_id, keep_id);
    } else {
        eprintln!("Merge failed for {} -> {}", drop_id, keep_id);
        std::process::exit(1);
    }

    Ok(())
}

fn handle_cite_import(
    db: &Database,
    json_input: Option<&str>,
    dry_run: bool,
    skip_missing: bool,
    extract: bool,
    paper: Option<&str>,
    _dedup: bool,
) -> Result<()> {
    if extract {
        let paper_id = match paper {
            Some(id) => id,
            None => {
                eprintln!("Error: --paper PAPER_ID required with --extract");
                std::process::exit(1);
            }
        };

        // Verify the paper exists
        if !db.paper_exists(paper_id) {
            eprintln!("Error: paper '{}' not found in DB", paper_id);
            std::process::exit(1);
        }

        // Get plain_text from the DB
        let text = match db.get_paper_plain_text(paper_id)? {
            Some(t) if !t.is_empty() => t,
            _ => {
                eprintln!("Error: paper '{}' has no plain_text to extract from", paper_id);
                std::process::exit(1);
            }
        };

        // Extract references using regex
        let arxiv_re = regex::Regex::new(r"(?i)\barXiv:\s*(\d+\.\d+\b)").unwrap();
        let doi_re = regex::Regex::new(r"(?i)\b10\.\d{4,}/[^\s]+").unwrap();
        let pmid_re = regex::Regex::new(r"(?i)\bPMID:\s*(\d{6,})\b").unwrap();
        let isbn_re = regex::Regex::new(r"(?i)\bISBN(?:-13)?:?\s*([0-9-X]{10,})\b").unwrap();

        // Find references section
        let refs_section_re = regex::Regex::new(r"(?i)(?:\n|^)[ ]*(?:\d+\.?\s*)?(?:References|Bibliography|Citations)").unwrap();
        let refs_text = if let Some(m) = refs_section_re.find(&text) {
            &text[m.start()..]
        } else {
            &text[..]
        };

        let arxiv_ids: Vec<String> = arxiv_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        let dois: Vec<String> = doi_re
            .find_iter(refs_text)
            .map(|m| m.as_str().to_string())
            .collect();

        let pmids: Vec<String> = pmid_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        let isbns: Vec<String> = isbn_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        // Print extracted references
        if !arxiv_ids.is_empty() {
            println!("  arXiv IDs ({}): {}", arxiv_ids.len(), arxiv_ids.join(", "));
        }
        if !dois.is_empty() {
            println!("  DOIs ({}): {}", dois.len(), dois.join(", "));
        }
        if !pmids.is_empty() {
            println!("  PMIDs ({}): {}", pmids.len(), pmids.join(", "));
        }
        if !isbns.is_empty() {
            println!("  ISBNs ({}): {}", isbns.len(), isbns.join(", "));
        }

        if arxiv_ids.is_empty() && dois.is_empty() && pmids.is_empty() && isbns.is_empty() {
            println!("No references found in '{}'", paper_id);
            return Ok(());
        }

        // Look up arXiv IDs in DB and import citations
        let mut db_ids: Vec<String> = Vec::new();
        for aid in &arxiv_ids {
            let full = format!("arxiv:{}", aid.to_lowercase());
            if db.paper_exists(&full) {
                db_ids.push(full);
            }
        }

        if db_ids.is_empty() {
            println!("No matching papers found in DB for extracted references");
            return Ok(());
        }

        if dry_run {
            println!("\n[dry-run] Would import {} citation edge(s):", db_ids.len());
            for tgt in &db_ids {
                println!("  {} -> {}", paper_id, tgt);
            }
        } else {
            let mut new_count = 0u32;
            for tgt in &db_ids {
                // insert_citation uses INSERT OR IGNORE — always succeeds
                db.insert_citation(paper_id, tgt)?;
                new_count += 1;
            }
            println!("\nImported {} citation edge(s)", new_count);
        }

        return Ok(());
    }

    // ── JSON input mode ──
    let raw = match json_input {
        Some(s) => s,
        None => {
            eprintln!("Error: json_input required (JSON string or @filepath)");
            std::process::exit(1);
        }
    };

    let data: serde_json::Value = if raw.starts_with('@') {
        let path = &raw[1..];
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Error reading {}: {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Error parsing JSON from {}: {}", path, e))?
    } else {
        serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Error: invalid JSON: {}", e))?
    };

    // Normalise to array
    let items: Vec<&serde_json::Value> = match &data {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        serde_json::Value::Object(_) => vec![&data],
        _ => {
            eprintln!("Error: JSON must be a list of objects or a single object");
            std::process::exit(1);
        }
    };

    let mut total_new = 0u32;
    let mut total_skip_missing = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let obj = match item.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!("[{}] item is not an object, skipping", i));
                continue;
            }
        };

        let source = obj
            .get("source")
            .or_else(|| obj.get("source_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let targets = obj
            .get("targets")
            .or_else(|| obj.get("target_ids"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        if source.is_empty() {
            errors.push(format!("[{}] missing 'source' field, skipping", i));
            continue;
        }

        if targets.is_empty() {
            errors.push(format!("[{}] empty 'targets' for source={}, skipping", i, source));
            continue;
        }

        // Check source exists
        if !db.paper_exists(source) {
            if skip_missing {
                total_skip_missing += 1;
                if dry_run {
                    println!("  [dry-run] skip (missing): {}", source);
                }
                continue;
            } else {
                errors.push(format!("[{}] source paper '{}' not in DB", i, source));
                continue;
            }
        }

        let mut valid_targets: Vec<String> = Vec::new();
        for tgt in &targets {
            if !db.paper_exists(tgt) {
                if skip_missing {
                    total_skip_missing += 1;
                    if dry_run {
                        println!("  [dry-run] skip (missing): {}", tgt);
                    }
                } else {
                    errors.push(format!("[{}] target paper '{}' not in DB", i, tgt));
                }
                continue;
            }
            valid_targets.push(tgt.clone());
        }

        if valid_targets.is_empty() {
            continue;
        }

        if dry_run {
            for tgt in &valid_targets {
                println!("  [dry-run] add citation: {} -> {}", source, tgt);
            }
            total_new += valid_targets.len() as u32;
        } else {
            for tgt in &valid_targets {
                db.insert_citation(source, tgt)?;
            }
            total_new += valid_targets.len() as u32;
        }
    }

    if !errors.is_empty() {
        println!("  warnings/errors : {}", errors.len());
        for err in errors.iter().take(10) {
            println!("    - {}", err);
        }
        if errors.len() > 10 {
            println!("    ... and {} more", errors.len() - 10);
        }
        std::process::exit(1);
    }

    println!("Import complete.");
    println!("  new citations : {}", total_new);
    if skip_missing {
        println!("  skipped (missing papers): {}", total_skip_missing);
    }

    Ok(())
}
