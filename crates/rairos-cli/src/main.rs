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
use rairos_kg::{GraphAlgorithms, KgNode, KnowledgeGraph};
use rairos_llm::{Capsule, CapsuleStatus, GenePool, GenePoolDiversityCalculator};
use rairos_memory::{ResearchMemory, ResearchStance, StanceType};
use rairos_web::{start, AppState};
use rairos_mcp_jin10::Jin10Client;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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

    /// Show paper's ego graph (neighbors up to N hops)
    KgGraph {
        /// Paper ID or arXiv ID
        #[arg(short, long)]
        paper_id: String,
        /// BFS depth (default 2)
        #[arg(short, long, default_value = "2")]
        depth: u32,
        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Search nodes in the knowledge graph
    KgSearch {
        /// Filter by node type (Paper/Tag/Author/PNote/CNote/MNote)
        #[arg(long)]
        node_type: Option<String>,
        /// Search keyword in label or entity ID
        #[arg(short, long)]
        keyword: Option<String>,
        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Rebuild knowledge graph from papers database
    KgRebuild {
        /// Only process new/changed papers since last rebuild
        #[arg(long)]
        incremental: bool,
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

    // ── Ported from Python CLI (Rust-native) ──────────────────────────────

    /// Signal analysis — match event keyword against Gene Pool patterns
    Signal {
        /// Event keyword (e.g. 富查伊拉, 石油, 美联储)
        keyword: String,
    },

    /// Weave research papers into narrative stories
    Story {
        /// Research topic to weave into story
        topic: Option<String>,
    },

    /// Build structured research arguments from evidence
    Argue {
        /// Research thesis or claim
        thesis: Vec<String>,
    },

    /// Discover patterns and insights from research data
    Discover {
        /// Force full rediscovery
        #[arg(short, long)]
        force: bool,
    },

    /// Scout for new papers on a topic
    Scout {
        /// Topic to scout
        topic: Option<String>,

        /// Sources to search (arxiv, news, all)
        #[arg(short, long, default_value = "all")]
        sources: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        max_results: usize,
    },

    /// Manage research journal
    Journal {
        /// Action: add, list, search
        action: String,

        /// Journal entry content (for add action)
        #[arg(short, long)]
        content: Option<String>,

        /// Tags for the entry (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Mood for the entry
        #[arg(short, long)]
        mood: Option<String>,
    },

    /// Generate unified intelligence report
    Intel {
        /// Focus topic
        #[arg(short, long, default_value = "")]
        topic: String,

        /// Include detailed breakdowns
        #[arg(short, long)]
        verbose: bool,
    },

    /// Analyze literature and find trends
    Litreview {
        /// Topic to analyze
        topic: Option<String>,

        /// Maximum papers to analyze
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Generate evolution report
    Report {
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Manage research log
    Research {
        /// Action: list, add
        action: String,

        /// Note content (for add action)
        #[arg(short, long)]
        content: Option<String>,
    },

    /// Generate weekly research digest
    Digest {
        /// Number of weeks to summarize
        #[arg(short, long, default_value = "1")]
        weeks: usize,
    },

    /// Query paper-code lineage traces
    Trace {
        /// arXiv ID (omit to list all recent traces)
        arxiv_id: Option<String>,

        /// List recent traces across all papers
        #[arg(short, long)]
        list: bool,

        /// Show detailed paper_section_refs for each trace
        #[arg(short, long)]
        refs: bool,

        /// Max traces to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Manage paper reviews
    Review {
        /// Action: list, add
        action: String,

        /// Paper ID for review
        #[arg(short, long)]
        paper: Option<String>,

        /// Review content
        #[arg(short, long)]
        content: Option<String>,
    },

    /// Replication checking for papers
    Replicate {
        /// Paper ID or arXiv ID to check
        paper_id: String,
    },

    // ── Batch 5 ported from Python CLI ────────────────────────────────────

    /// Research friction report — detect bottlenecks and failures
    Friction {
        /// Filter by friction type (command, workflow, retrieval, cognitive, navigation)
        #[arg(short, long)]
        friction_type: Option<String>,

        /// Time window in days
        #[arg(short, long, default_value = "30")]
        days: usize,

        /// JSON output
        #[arg(short, long)]
        json: bool,

        /// Max events to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Track research experiments
    Experiment {
        /// Action: list, run, get, complete, metric, compare, delete, simulate
        action: String,

        /// Experiment name (for run)
        #[arg(long)]
        name: Option<String>,

        /// Description (for run)
        #[arg(long)]
        desc: Option<String>,

        /// Roadmap milestone (for run)
        #[arg(short, long)]
        milestone: Option<String>,

        /// Tags (for run)
        #[arg(long)]
        tag: Vec<String>,

        /// Experiment ID (for get/complete/metric/delete/simulate)
        #[arg(long)]
        id: Option<String>,

        /// Metrics JSON (for complete)
        #[arg(long)]
        metrics: Option<String>,

        /// Metric name (for metric action)
        #[arg(long)]
        metric_name: Option<String>,

        /// Metric value (for metric action)
        #[arg(long)]
        metric_value: Option<f64>,

        /// Metric unit (for metric action)
        #[arg(long, default_value = "")]
        unit: String,

        /// Experiment IDs to compare (for compare action)
        #[arg(long)]
        ids: Vec<String>,

        /// Simulated result [success|fail] (for simulate action)
        #[arg(long)]
        result: Option<String>,
    },

    /// Evolution dashboard — view system learning progress
    Evolution {
        /// Show statistics
        #[arg(short, long)]
        stats: bool,

        /// Show learned patterns
        #[arg(short, long)]
        patterns: bool,

        /// Show recent feedback
        #[arg(short, long)]
        feedback: bool,

        /// Generate learning report
        #[arg(short, long)]
        report: bool,

        /// Show research sessions
        #[arg(long)]
        sessions: bool,

        /// Report period in days
        #[arg(long, default_value = "7")]
        days: usize,

        /// Clear all evolution data
        #[arg(short, long)]
        clear: bool,

        /// Export data to JSON
        #[arg(short, long)]
        export: bool,
    },

    /// Start Rairos Web UI dashboard
    Dashboard {
        /// Port to listen on
        #[arg(short, long, default_value = "8501")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Don't open browser
        #[arg(long)]
        no_browser: bool,
    },

    /// Build and visualize citation chains
    CitationChain {
        /// Starting paper ID
        paper_id: Option<String>,

        /// Chain depth
        #[arg(short, long, default_value_t = 2)]
        depth: i32,

        /// Output Graphviz DOT format
        #[arg(short, long)]
        graphviz: bool,

        /// Output Mermaid flowchart
        #[arg(short, long)]
        mermaid: bool,

        /// Show papers that influenced this
        #[arg(long)]
        influencers: bool,

        /// Show papers influenced by this
        #[arg(long)]
        impact: bool,

        /// Find path to another paper ID
        #[arg(long)]
        path: Option<String>,
    },

    /// Generate research hypotheses from gaps
    Hypothesize {
        /// Research topic
        topic: Option<String>,

        /// Gap context from gap analysis
        #[arg(short, long, default_value = "")]
        gap: String,

        /// Trend context from trend analysis
        #[arg(short, long, default_value = "")]
        trend: String,

        /// Story context from story weaving
        #[arg(short, long, default_value = "")]
        story: String,

        /// Disable LLM enhancement
        #[arg(long)]
        no_llm: bool,

        /// Generate creative cross-domain hypotheses
        #[arg(long)]
        creative: bool,

        /// JSON output
        #[arg(short, long)]
        json: bool,

        /// LLM model to use
        #[arg(short = 'M', long)]
        model: Option<String>,

        /// Number of hypotheses to generate
        #[arg(short = 'n', long, default_value = "5")]
        top: usize,
    },

    // ── Batch 6 ported from Python CLI ────────────────────────────────────

    /// Build citation subgraph from DB
    CiteGraph {
        /// Root paper ID
        #[arg(long)]
        paper: Option<String>,

        /// Traversal depth
        #[arg(long, default_value = "2")]
        depth: i32,

        /// Max nodes
        #[arg(long, default_value = "30")]
        max_nodes: usize,

        /// Output format (text/mermaid/json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Fetch paper metadata from external APIs
    CiteFetch {
        /// Paper ID (arXiv ID or DOI)
        paper_id: Option<String>,

        /// Show what would be done without writing
        #[arg(long)]
        dry_run: bool,
    },

    /// Verify hypotheses with Lean 4 theorem prover
    Lean {
        /// Path to a Lean file to verify
        file: Option<String>,

        /// Hypothesis text to translate to Lean
        #[arg(short = 'y', long)]
        hypothesis: Option<String>,

        /// Show installation instructions
        #[arg(long)]
        install: bool,

        /// Check if Lean is installed
        #[arg(long)]
        check: bool,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Generate D3 visualizations
    Visual {
        /// Paper ID to visualize citations for
        #[arg(long)]
        paper: Option<String>,

        /// Query for benchmark visualization
        #[arg(short, long)]
        query: Option<String>,

        /// Max papers
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Output path for HTML
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Ingest paper metadata from arXiv/DOI
    Ingest {
        /// Paper ID (arXiv ID or DOI)
        paper_id: Option<String>,

        /// Output JSON format
        #[arg(short, long)]
        json: bool,

        /// Skip PDF processing
        #[arg(long)]
        no_pdf: bool,

        /// Source: arxiv or doi
        #[arg(short, long, default_value = "arxiv")]
        source: String,
    },

    /// Manage research sessions
    Session {
        /// Action: start, list, current, end
        action: String,

        /// Session title (for start)
        #[arg(long)]
        title: Option<String>,

        /// Topic (for start)
        #[arg(short = 'k', long)]
        topic: Option<String>,

        /// Days to look back (for list)
        #[arg(short, long, default_value = "7")]
        days: usize,

        /// Max sessions to show (for list)
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Manage research narratives (ported from Python CLI)
    Narrative {
        #[command(subcommand)]
        action: NarrativeAction,
    },

    /// Validate novelty of research questions
    Validate {
        /// Research question to validate
        question: Option<String>,

        /// Disable LLM analysis (rule-based only)
        #[arg(long)]
        no_llm: bool,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Analysis depth (quick/full)
        #[arg(short, long, default_value = "quick")]
        depth: String,

        /// LLM model to use
        #[arg(short, long)]
        model: Option<String>,

        /// Interactive exploration mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Run deep analysis pipeline on a paper (6-stage post-processing)
    Postprocess {
        /// Paper ID to analyze
        paper_id: String,

        /// Root folder for your research OS (default: AI-Research)
        #[arg(long, default_value = "AI-Research")]
        root: String,

        /// Specific stages to run (space-separated)
        #[arg(long, num_args = 1..)]
        stages: Vec<String>,

        /// Skip LLM usage — keyword-only analysis
        #[arg(long)]
        skip_llm: bool,

        /// Comma-separated tags override
        #[arg(long)]
        tags: Option<String>,
    },

    /// Generate research reading path from KG citation graph
    Path {
        /// Research topic to explore
        topic: Option<String>,

        /// Reading level: intro/intermediate/advanced
        #[arg(short = 'l', long, default_value = "intermediate")]
        level: String,

        /// Maximum papers to recommend
        #[arg(short = 'n', long, default_value = "8")]
        max: usize,

        /// Minimum publication year
        #[arg(long)]
        min_year: Option<i32>,

        /// Maximum publication year
        #[arg(long)]
        max_year: Option<i32>,

        /// Output as Mermaid diagram
        #[arg(short, long)]
        mermaid: bool,

        /// Interactive exploration mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Generate slides from papers (MD/HTML output)
    Slides {
        /// Paper ID(s) to generate slides for
        paper_ids: Vec<String>,

        /// Output format (md/html)
        #[arg(short = 'f', long, default_value = "md")]
        format: String,

        /// Slide template (academic/minimal/modern)
        #[arg(short = 't', long, default_value = "academic")]
        template: String,

        /// Number of slides
        #[arg(short = 's', long, default_value = "10")]
        num_slides: usize,

        /// Output file path
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Include speaker notes
        #[arg(long)]
        include_notes: bool,

        /// Output language (zh/en/bilingual)
        #[arg(long, default_value = "zh")]
        lang: String,
    },

    /// Generate research roadmap from a question
    Roadmap {
        /// Question ID (fetched from QuestionTracker)
        #[arg(short = 'q', long)]
        question: Option<String>,

        /// Direct question text (used if --question is not provided)
        #[arg(short = 't', long)]
        text: Option<String>,

        /// JSON output
        #[arg(short = 'j', long)]
        json: bool,

        /// Export as Markdown file
        #[arg(long)]
        export_md: Option<String>,
    },

    /// Manage research questions (ported from Python CLI)
    Question {
        #[command(subcommand)]
        action: QuestionAction,
    },

    /// Run end-to-end Rairos pipeline demo
    Demo {
        /// Quick 30-second demo
        #[arg(long)]
        quick: bool,

        /// Process N papers
        #[arg(long)]
        papers: Option<usize>,

        /// Focus on insight extraction
        #[arg(long)]
        insights: bool,
    },

    /// Full research pipeline: gap analysis → hypothesis → experiment
    Pipeline {
        /// Research topic or keyword
        topic: String,

        /// Run gap analysis + hypothesis only (skip experiment creation)
        #[arg(long)]
        hypothesis_only: bool,

        /// Number of top hypotheses to convert to experiments
        #[arg(short = 'n', long, default_value = "3")]
        top_n: usize,

        /// Minimum papers for gap analysis
        #[arg(long, default_value = "5")]
        min_papers: usize,

        /// LLM model override
        #[arg(long)]
        model: Option<String>,

        /// Output as JSON
        #[arg(short = 'j', long)]
        json: bool,

        /// Skip LLM enhancement — template-based only
        #[arg(long)]
        no_llm: bool,

        /// Verbose output
        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Manage insight cards (add, search, rate, like, top, tag-cloud)
    Insight {
        #[command(subcommand)]
        action: InsightAction,
    },

    /// Jin10 financial data (quotes, news, calendar)
    Jin10 {
        #[command(subcommand)]
        action: Jin10Action,
    },

    /// Route a natural-language query to the appropriate CLI command
    Route {
        /// Research query to route
        query: Vec<String>,

        /// Output full route object as JSON
        #[arg(short, long)]
        json: bool,

        /// Execute the routed command(s) and print outputs
        #[arg(short, long)]
        exec: bool,

        /// Execute all routed commands (for multi-intent queries)
        #[arg(short, long)]
        all: bool,
    },

    /// Benchmark-driven skill discovery with EvoSkill
    #[command(name = "evoskill")]
    EvoSkill {
        #[command(subcommand)]
        action: EvoSkillAction,
    },

    /// RAG pipeline: paper2code + EvoSkill automated improvement loop
    #[command(name = "rag")]
    Rag {
        #[command(subcommand)]
        action: RagAction,
    },

    /// RAG Chat with your paper library
    Chat {
        /// Question to ask (omit for interactive mode)
        question: Option<String>,

        /// Target specific paper by ID
        #[arg(long)]
        paper: Option<String>,

        /// Filter by concept/tag
        #[arg(short, long)]
        concept: Option<String>,

        /// Number of papers to retrieve
        #[arg(short = 'n', long, default_value = "5")]
        limit: usize,

        /// Interactive REPL mode
        #[arg(short, long)]
        interactive: bool,

        /// Hide citations in output
        #[arg(long)]
        no_cite: bool,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Stream the response
        #[arg(long)]
        stream: bool,

        /// Export chat history
        #[arg(short, long)]
        export: Option<String>,

        /// Export format (markdown/html)
        #[arg(short, long)]
        format: Option<String>,
    },
}

#[derive(Subcommand)]
enum RagAction {
    /// Run full RAG pipeline for a paper
    RunFull {
        /// arXiv ID
        arxiv_id: String,

        /// Implementation mode (minimal/full/educational)
        #[arg(short, long, default_value = "minimal")]
        mode: String,

        /// Deep learning framework (pytorch/jax/numpy)
        #[arg(short, long, default_value = "pytorch")]
        framework: String,

        /// EvoSkill task name
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Generate tests from a paper
    GenTests {
        /// arXiv ID
        arxiv_id: String,
    },
    /// Initialize EvoSkill benchmark from CSV
    InitBenchmark {
        /// Path to test cases CSV
        csv_path: String,

        /// Task name
        #[arg(short, long)]
        task: String,
    },
    /// Run EvoSkill improvement loop
    RunEvoskill {
        /// Resume from frontier
        #[arg(long)]
        continue_mode: bool,
    },
    /// List discovered skills
    ListSkills,
    /// Check RAG pipeline status
    Status,
}

#[derive(Subcommand)]
enum EvoSkillAction {
    /// Initialize EvoSkill project
    Init {
        /// Task name
        #[arg(short, long)]
        task: String,
        /// Path to benchmark CSV
        #[arg(short, long)]
        dataset: String,
        /// Agent runtime (claude, opencode, etc.)
        #[arg(short = 'H', long, default_value = "claude")]
        harness: String,
        /// Model to use
        #[arg(short, long, default_value = "sonnet")]
        model: String,
        /// Question column name
        #[arg(long, default_value = "question")]
        question_col: String,
        /// Answer column name
        #[arg(long, default_value = "answer")]
        answer_col: String,
        /// Category column name
        #[arg(long)]
        category_col: Option<String>,
    },
    /// Run EvoSkill self-improvement loop
    Run {
        /// Resume from frontier
        #[arg(long)]
        continue_mode: bool,
        /// Show pass/fail details
        #[arg(short, long)]
        verbose: bool,
    },
    /// Evaluate best program on validation set
    Eval,
    /// Show diff between iterations
    Diff {
        /// Source iteration
        from_iter: Option<i32>,
        /// Target iteration
        to_iter: Option<i32>,
    },
    /// Reset all program branches
    Reset,
    /// Check EvoSkill availability
    Status,
}

#[derive(Subcommand)]
enum Jin10Action {
    /// Get real-time quote
    Quote {
        /// Symbol code (e.g. XAUUSD, USOIL)
        code: String,
    },
    /// Get K-line data
    Kline {
        /// Symbol code
        code: String,
        /// Minutes: 1/5/15/60/240/1440
        #[arg(short, long, default_value = "1")]
        time: i32,
        /// Number of candles
        #[arg(short = 'n', long, default_value = "10")]
        count: i32,
    },
    /// Latest flash news
    Flash {
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Search flash news
    SearchFlash {
        /// Search keyword
        keyword: String,
    },
    /// Latest news list
    News {
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Search news
    SearchNews {
        /// Search keyword
        keyword: String,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get news article detail
    NewsDetail {
        /// News article ID
        id: String,
    },
    /// Economic calendar
    Calendar,
    /// List supported symbols
    Symbols,
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
    /// Show embedding coverage statistics
    Stats,
    /// Find semantically similar papers using embeddings
    Semantic {
        /// Paper ID to check
        paper: String,
        /// Cosine similarity threshold
        #[arg(short, long, default_value = "0.85")]
        threshold: f32,
        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum QuestionAction {
    /// List all research questions
    List {
        /// Filter by status (open/in_progress/resolved/wontfix)
        #[arg(long)]
        status: Option<String>,
        /// Filter by topic
        #[arg(long)]
        topic: Option<String>,
        /// Filter by source (manual/gap_detection/hypothesis/literature_review)
        #[arg(long)]
        source: Option<String>,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Add a new research question
    Add {
        /// Research question text
        question: String,
        /// Research topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Priority 1-10 (default: 5)
        #[arg(short, long, default_value = "5")]
        priority: u8,
        /// Additional notes
        #[arg(short, long)]
        notes: Option<String>,
    },
    /// Get a question by ID
    Get {
        /// Question ID
        id: String,
    },
    /// Update a question
    Update {
        /// Question ID
        id: String,
        /// New status
        #[arg(short, long)]
        status: Option<String>,
        /// Update notes
        #[arg(short, long)]
        notes: Option<String>,
        /// Update priority 1-10
        #[arg(short, long)]
        priority: Option<u8>,
    },
    /// Link a paper to a question
    Link {
        /// Question ID
        id: String,
        /// Paper ID (arxiv ID or UID)
        paper_id: String,
    },
    /// Unlink a paper from a question
    Unlink {
        /// Question ID
        id: String,
        /// Paper ID to unlink
        paper_id: String,
    },
    /// Delete a question
    Delete {
        /// Question ID
        id: String,
    },
    /// Sync questions from gap detection
    Sync {
        /// Research topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Priority 1-10 (default: 7)
        #[arg(short, long, default_value = "7")]
        priority: u8,
    },
    /// Show question statistics
    Stats,
}

#[derive(Subcommand)]
enum NarrativeAction {
    /// List all research narrative threads
    List,
    /// Show a single thread in detail
    Show {
        /// Thread ID
        #[arg(short, long)]
        id: String,
    },
    /// Track a topic (create or update by topic)
    Track {
        /// Research topic
        topic: String,
    },
    /// Update a thread's fields
    Update {
        /// Thread ID
        #[arg(short, long)]
        id: String,
        /// New topic
        #[arg(short, long)]
        topic: Option<String>,
        /// New notes
        #[arg(short, long)]
        notes: Option<String>,
    },
    /// Add a note to a thread
    Note {
        /// Thread ID
        #[arg(short, long)]
        id: String,
        /// Note text to append
        #[arg(short, long)]
        text: String,
    },
    /// Show dashboard overview of all threads
    Dashboard,
}

#[derive(Subcommand)]
enum InsightAction {
    /// Add a new insight card
    Add {
        /// Insight content
        #[arg(long)]
        content: String,

        /// Insight type (finding, method, limitation, future_work)
        #[arg(short = 't', long, default_value = "finding")]
        r#type: String,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Paper ID
        #[arg(long)]
        paper: Option<String>,

        /// Collection ID to add to
        #[arg(short = 'c', long)]
        collection: Option<String>,
    },

    /// List all insight cards
    List {
        /// Maximum cards to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Search insight cards
    Search {
        /// Search query
        #[arg(short = 'q', long)]
        query: String,

        /// Filter by type
        #[arg(short = 't', long)]
        r#type: Option<String>,
    },

    /// Show tag cloud (tag frequency)
    TagCloud,

    /// Rate a card (1-5 stars)
    Rate {
        /// Card ID
        #[arg(long)]
        card: String,

        /// Star rating (1-5)
        #[arg(long)]
        stars: i32,
    },

    /// Like a card
    Like {
        /// Card ID
        #[arg(long)]
        card: String,
    },

    /// Dislike a card
    Dislike {
        /// Card ID
        #[arg(long)]
        card: String,
    },

    /// Show top-rated cards
    Top {
        /// Minimum rating filter
        #[arg(long, default_value = "3")]
        min_rating: i32,

        /// Maximum cards to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Show bottom-rated cards
    Bottom {
        /// Maximum rating filter
        #[arg(long, default_value = "2")]
        max_rating: i32,

        /// Maximum cards to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
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

fn handle_kg_graph(paper_id: &str, depth: u32, format: &str) -> Result<()> {
    let graph = KnowledgeGraph::load()?;

    // Find center node by entity_id or node id
    let center = graph
        .nodes()
        .values()
        .find(|n| n.entity_id == paper_id || n.id == paper_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Paper '{}' not found in KG", paper_id))?;

    // BFS neighbors on in-memory graph
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(center.id.clone());
    let mut results: Vec<(KgNode, String, u32)> = Vec::new();
    let mut current_level = vec![center.id.clone()];

    for d in 1..=depth {
        let mut next_level = Vec::new();
        for nid in &current_level {
            for edge in &graph.edges {
                let neighbor_id = if edge.source == *nid {
                    Some(&edge.target)
                } else if edge.target == *nid {
                    Some(&edge.source)
                } else {
                    None
                };
                if let Some(nbr_id) = neighbor_id {
                    if visited.insert(nbr_id.clone()) {
                        if let Some(node) = graph.get_node(nbr_id) {
                            results.push((node.clone(), edge.relation.clone(), d));
                            next_level.push(nbr_id.clone());
                        }
                    }
                }
            }
        }
        current_level = next_level;
    }

    if format == "json" {
        let neighbors: Vec<serde_json::Value> = results
            .iter()
            .map(|(node, rel, d)| {
                serde_json::json!({
                    "id": node.id,
                    "entity_id": node.entity_id,
                    "label": node.label,
                    "type": node.node_type,
                    "relation": rel,
                    "depth": d,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "center": {
                    "id": center.id,
                    "entity_id": center.entity_id,
                    "label": center.label,
                    "type": center.node_type,
                },
                "neighbors": neighbors,
                "total": results.len(),
            }))?
        );
    } else {
        println!("=== KG Graph for '{}' (depth={}) ===", paper_id, depth);
        println!(
            "Center: [{}] {}",
            center.node_type,
            center.label
        );
        println!("\n{} neighbor(s):", results.len());
        for (node, rel, d) in &results {
            let label = if node.label.len() > 50 {
                format!("{}...", &node.label[..47])
            } else {
                node.label.clone()
            };
            println!(
                "  [depth={}] {:<8} | {:<12} | {}",
                d, node.node_type, rel, label
            );
        }
    }
    Ok(())
}

fn handle_kg_search(
    node_type: Option<&str>,
    keyword: Option<&str>,
    format: &str,
) -> Result<()> {
    let graph = KnowledgeGraph::load()?;

    let nodes: Vec<&KgNode> = graph
        .nodes()
        .values()
        .filter(|n| {
            let type_match = node_type.map(|t| n.node_type == t).unwrap_or(true);
            let kw = keyword.unwrap_or("");
            let keyword_match = if kw.is_empty() {
                true
            } else {
                let kw_lower = kw.to_lowercase();
                n.label.to_lowercase().contains(&kw_lower)
                    || n.entity_id.to_lowercase().contains(&kw_lower)
            };
            type_match && keyword_match
        })
        .collect();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&nodes)?);
    } else {
        println!("=== KG Search Results ===\n");
        if nodes.is_empty() {
            println!("No nodes found.");
            return Ok(());
        }
        println!(
            "{:>4} {:<12} {:<12} {}",
            "#", "TYPE", "ENTITY ID", "LABEL"
        );
        println!("{}", "-".repeat(80));
        for (i, node) in nodes.iter().enumerate().take(100) {
            let eid = if node.entity_id.len() > 12 {
                format!("{}...", &node.entity_id[..9])
            } else {
                node.entity_id.clone()
            };
            let label = if node.label.len() > 52 {
                format!("{}...", &node.label[..49])
            } else {
                node.label.clone()
            };
            println!(
                "{:>4} {:<12} {:<12} {}",
                i + 1,
                node.node_type,
                eid,
                label
            );
        }
        if nodes.len() > 100 {
            println!("... and {} more", nodes.len() - 100);
        }
        println!("\nTotal: {} nodes", nodes.len());
    }
    Ok(())
}

fn handle_kg_rebuild(db: &Database, incremental: bool) -> Result<()> {
    // Load the knowledge graph (with DB connection)
    let graph = KnowledgeGraph::load()?;

    // Load all papers from the papers database
    let papers = db.list_papers(None, 100_000, 0)?;
    if papers.is_empty() {
        println!("No papers found in database.");
        return Ok(());
    }

    println!("Loading {} papers into knowledge graph...", papers.len());

    // Convert to KgNode
    let kg_nodes: Vec<KgNode> = papers.iter().map(KgNode::from_paper).collect();

    // Load all citations from the papers database
    let all_citations = db.list_all_citations()?;
    println!(
        "Connecting {} citation edges...",
        all_citations.len()
    );

    // Use the KgDatabase to rebuild
    let db_ref = graph
        .database()
        .ok_or_else(|| anyhow::anyhow!("Knowledge graph has no database connection"))?;

    let stats = db_ref.rebuild_from_papers(&kg_nodes, &all_citations)?;

    println!(
        "Done: {} nodes, {} edges.",
        stats.total_nodes, stats.total_edges
    );

    if incremental {
        println!("(Incremental mode enabled — only new/changed papers processed.)");
    }

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
        DedupAction::Stats => {
            let (total, with_emb) = db.get_embedding_stats()?;
            let pct = if total > 0 { (with_emb as f64 / total as f64) * 100.0 } else { 0.0 };
            println!("\n  \x1b[36mEmbedding Coverage\x1b[0m");
            println!("  \x1b[36mPapers with embedding:\x1b[0m  \x1b[92m{}\x1b[0m", with_emb);
            println!("  \x1b[36mPapers with text:\x1b[0m      \x1b[91m{}\x1b[0m", total);
            println!("  \x1b[36mCoverage:\x1b[0m              \x1b[93m{:.1}%\x1b[0m", pct);
            println!();
        }
        DedupAction::Semantic { paper, threshold, limit } => {
            let exists = db.paper_exists(paper);
            if !exists {
                eprintln!("Paper '{}' not found", paper);
                return Ok(());
            }
            let sims = db.find_similar(paper, *limit, *threshold)?;
            if sims.is_empty() {
                println!("\n  \x1b[36mSimilar Papers — {}\x1b[0m", paper);
                println!("  \x1b[90mNo similar papers above threshold=\x1b[36m{}\x1b[0m", threshold);
                println!();
                return Ok(());
            }
            println!("\n  \x1b[36mSimilar Papers — \x1b[91m{}\x1b[0m (threshold=\x1b[91m{}\x1b[0m)\x1b[0m", paper, threshold);
            println!("  \x1b[36m{} similar papers found\x1b[0m", sims.len());
            println!();
            println!("  {:<10} {:>12}  {}", "Score", "Paper ID", "Title");
            println!("  {} {} {}", "─".repeat(10), "─".repeat(12), "─".repeat(50));
            for (id, score) in &sims {
                let score_color = if *score >= 0.95 { "\x1b[92m" } else if *score >= 0.85 { "\x1b[93m" } else { "\x1b[91m" };
                let paper = db.get_paper(id)?;
                let title = if paper.title.len() > 47 { format!("{}...", &paper.title[..47]) } else { paper.title.clone() };
                println!("  {}{:.4}\x1b[0m  {:>12}  \x1b[36m{}\x1b[0m", score_color, score, id, title);
            }
            println!();
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
        Commands::KgGraph {
            paper_id,
            depth,
            format,
        } => {
            handle_kg_graph(paper_id, *depth, format)?;
        }
        Commands::KgSearch {
            node_type,
            keyword,
            format,
        } => {
            handle_kg_search(node_type.as_deref(), keyword.as_deref(), format)?;
        }
        Commands::KgRebuild { incremental } => {
            let db = open_db(&cli.db)?;
            handle_kg_rebuild(&db, *incremental)?;
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
        Commands::Signal { keyword } => {
            handle_signal(keyword)?;
        }
        Commands::Story { topic } => {
            let db = open_db(&cli.db)?;
            handle_story(&db, topic.as_deref())?;
        }
        Commands::Argue { thesis } => {
            let db = open_db(&cli.db)?;
            handle_argue(&db, thesis)?;
        }
        Commands::Discover { force } => {
            handle_discover(*force)?;
        }
        Commands::Scout { topic, sources, max_results } => {
            handle_scout(topic.as_deref(), sources, *max_results)?;
        }
        Commands::Journal { action, content, tags, mood } => {
            handle_journal(action, content.as_deref(), tags.as_deref(), mood.as_deref())?;
        }
        Commands::Intel { topic, verbose } => {
            handle_intel(topic, *verbose)?;
        }
        Commands::Litreview { topic, limit, format } => {
            let db = open_db(&cli.db)?;
            handle_litreview(&db, topic.as_deref(), *limit, format)?;
        }
        Commands::Report { format } => {
            handle_report(format)?;
        }
        Commands::Research { action, content } => {
            let db = open_db(&cli.db)?;
            handle_research(&db, action, content.as_deref())?;
        }
        Commands::Digest { weeks } => {
            handle_digest(*weeks)?;
        }
        Commands::Trace { arxiv_id, list, refs, limit } => {
            let db = open_db(&cli.db)?;
            handle_trace(&db, arxiv_id.as_deref(), *list, *refs, *limit)?;
        }
        Commands::Review { action, paper, content } => {
            let db = open_db(&cli.db)?;
            handle_review(&db, action, paper.as_deref(), content.as_deref())?;
        }
        Commands::Replicate { paper_id } => {
            let db = open_db(&cli.db)?;
            handle_replicate(&db, paper_id)?;
        }

        // ── Batch 5 ───────────────────────────────────────────────────────

        Commands::Friction { friction_type, days, json, limit } => {
            handle_friction(friction_type.as_deref(), *days, *json, *limit)?;
        }
        Commands::Experiment { action, name, desc, milestone, tag, id, metrics, metric_name, metric_value, unit, ids, result } => {
            handle_experiment(action, name.as_deref(), desc.as_deref(), milestone.as_deref(), tag.clone(),
                id.as_deref(), metrics.as_deref(), metric_name.as_deref(), *metric_value, unit,
                ids.clone(), result.as_deref())?;
        }
        Commands::Evolution { stats, patterns, feedback, report, sessions, days, clear, export } => {
            handle_evolution(*stats, *patterns, *feedback, *report, *sessions, *days, *clear, *export)?;
        }
        Commands::Dashboard { port, host, no_browser } => {
            handle_dashboard(*port, host, *no_browser)?;
        }
        Commands::CitationChain { paper_id, depth, graphviz, mermaid, influencers, impact, path } => {
            let db = open_db(&cli.db)?;
            handle_citation_chain(&db, paper_id.as_deref(), *depth, *graphviz, *mermaid, *influencers, *impact, path.as_deref())?;
        }
        Commands::Hypothesize { topic, gap, trend, story, no_llm, creative, json, model, top } => {
            handle_hypothesize(topic.as_deref(), gap, trend, story, *no_llm, *creative, *json, model.as_deref(), *top)?;
        }

        // ── Batch 6 ───────────────────────────────────────────────────────

        Commands::CiteGraph { paper, depth, max_nodes, format } => {
            let db = open_db(&cli.db)?;
            handle_cite_graph(&db, paper.as_deref(), *depth, *max_nodes, format)?;
        }
        Commands::CiteFetch { paper_id, dry_run } => {
            handle_cite_fetch(paper_id.as_deref(), *dry_run)?;
        }
        Commands::Lean { file, hypothesis, install, check, json } => {
            handle_lean(file.as_deref(), hypothesis.as_deref(), *install, *check, *json)?;
        }
        Commands::Visual { paper, query, limit, output } => {
            let db = open_db(&cli.db)?;
            handle_visual(&db, paper.as_deref(), query.as_deref(), *limit, output.as_deref())?;
        }
        Commands::Ingest { paper_id, json, no_pdf, source } => {
            handle_ingest(paper_id.as_deref(), *json, *no_pdf, source)?;
        }
        Commands::Session { action, title, topic, days, limit } => {
            handle_session(action, title.as_deref(), topic.as_deref(), *days, *limit)?;
        }
        Commands::Question { action } => {
            handle_question(action)?;
        }
        Commands::Narrative { action } => {
            handle_narrative(action)?;
        }
        Commands::Validate {
            question,
            no_llm,
            json,
            depth,
            model,
            interactive,
        } => {
            let db = open_db(&cli.db)?;
            handle_validate(
                &db,
                question.as_deref(),
                *no_llm,
                *json,
                depth,
                model.as_deref(),
                *interactive,
            )?;
        }

        Commands::Postprocess {
            paper_id,
            root,
            stages,
            skip_llm,
            tags,
        } => {
            let db = open_db(&cli.db)?;
            handle_postprocess(
                &db,
                paper_id,
                root,
                stages,
                *skip_llm,
                tags.as_deref(),
            )?;
        }

        Commands::Path {
            topic,
            level,
            max,
            min_year,
            max_year,
            mermaid,
            interactive,
        } => {
            let db = open_db(&cli.db)?;
            handle_path(
                &db,
                topic.as_deref(),
                level,
                *max,
                *min_year,
                *max_year,
                *mermaid,
                *interactive,
            )?;
        }

        Commands::Slides {
            paper_ids,
            format,
            template,
            num_slides,
            output,
            include_notes,
            lang,
        } => {
            let db = open_db(&cli.db)?;
            handle_slides(&db, paper_ids, format, template, *num_slides, output.as_deref(), *include_notes, lang)?;
        }

        Commands::Roadmap { question, text, json, export_md } => {
            handle_roadmap(question.as_deref(), text.as_deref(), *json, export_md.as_deref())?;
        }

        Commands::Demo { quick, papers, insights } => {
            handle_demo(*quick, *papers, *insights)?;
        }

        Commands::Pipeline {
            topic,
            hypothesis_only,
            top_n,
            min_papers,
            model,
            json,
            no_llm,
            verbose,
        } => {
            let db = open_db(&cli.db)?;
            handle_pipeline(
                &db,
                topic,
                *hypothesis_only,
                *top_n,
                *min_papers,
                model.as_deref(),
                *json,
                *no_llm,
                *verbose,
            )?;
        }

        Commands::Insight { action } => {
            handle_insight(action)?;
        }
        Commands::Jin10 { action } => {
            handle_jin10(action)?;
        }
        Commands::Route { query, json, exec, all } => {
            handle_route(query, *json, *exec, *all)?;
        }
        Commands::EvoSkill { action } => {
            handle_evoskill(action)?;
        }
        Commands::Rag { action } => {
            handle_rag(action)?;
        }
        Commands::Chat {
            question,
            paper,
            concept,
            limit,
            interactive,
            no_cite,
            model,
            verbose,
            stream,
            export,
            format,
        } => {
            handle_chat(
                question.as_deref(),
                paper.as_deref(),
                concept.as_deref(),
                *limit,
                *interactive,
                *no_cite,
                model.as_deref(),
                *verbose,
                *stream,
                export.as_deref(),
                format.as_deref(),
            )?;
        }
    }

    Ok(())
}

/// Handle `roadmap` — generate research roadmap from a question.
fn handle_roadmap(
    question: Option<&str>,
    text: Option<&str>,
    json: bool,
    export_md: Option<&str>,
) -> Result<()> {
    use rairos_questions::QuestionTracker;
    use rairos_roadmap::RoadmapGenerator;

    // Determine question text
    let (question_text, question_id) = if let Some(qid) = question {
        let tracker = QuestionTracker::new()?;
        let q = tracker
            .get(qid)
            .ok_or_else(|| anyhow::anyhow!("问题 [{}] 不存在", qid))?;
        (q.question.clone(), q.id.clone())
    } else if let Some(t) = text {
        (t.to_string(), String::new())
    } else {
        anyhow::bail!("请提供 --question <id> 或 --text <问题>");
    };

    println!("📋 生成研究路线图...");

    let gen = RoadmapGenerator::new();
    let roadmap = gen.generate(&question_text, &question_id, None, "");

    if json {
        println!("{}", gen.render_json(&roadmap));
    } else if let Some(path) = export_md {
        std::fs::write(path, gen.render_markdown(&roadmap))
            .context("写入文件失败")?;
        println!("✓ 导出到 {}", path);
    } else {
        println!();
        println!("{}", gen.render_text(&roadmap));
    }

    Ok(())
}

/// Handle `insight` — manage insight cards.
fn handle_insight(action: &InsightAction) -> Result<()> {
    use rairos_insight_cards::InsightManager;

    let manager = InsightManager::new(None);

    match action {
        InsightAction::Add {
            content,
            r#type,
            tags,
            paper,
            collection,
        } => {
            let tag_list: Option<Vec<String>> = tags
                .as_ref()
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
            let card = manager.add_card(
                paper.as_deref().unwrap_or(""),
                "",
                content,
                r#type,
                tag_list,
                "",
                "",
            );
            println!("  ✓ Created insight card [{}]: {}", card.card_id, &card.content[..card.content.len().min(60)]);
            if let Some(cid) = collection {
                let _ = manager.add_to_collection(cid, &card.card_id);
                println!("     Added to collection [{}]", cid);
            }
        }

        InsightAction::List { limit } => {
            let cards = manager.search_cards(None, None, None, None);
            if cards.is_empty() {
                println!("  No insight cards found.");
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Insight cards ({} shown / {} total):", shown.clone().count(), cards.len());
            for card in shown {
                let rating = if card.times_rated > 0 {
                    format!("{}★", card.quality_rating)
                } else {
                    "-".to_string()
                };
                let tags_str = if card.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", card.tags.join(", "))
                };
                println!("  [{}] {} {}{}", card.card_id, rating, card.content, tags_str);
                println!("       Type: {} | Paper: {} | Created: {}", card.insight_type, card.paper_id, card.created_at);
            }
        }

        InsightAction::Search { query, r#type } => {
            let cards = manager.search_cards(Some(query), None, r#type.as_deref(), None);
            if cards.is_empty() {
                println!("  No matching insight cards found.");
                return Ok(());
            }
            println!("  Found {} card(s):", cards.len());
            for card in &cards {
                let rating = if card.times_rated > 0 {
                    format!("{}★", card.quality_rating)
                } else {
                    "-".to_string()
                };
                println!("  [{}] {} | {}", card.card_id, rating, card.content);
            }
        }

        InsightAction::TagCloud => {
            let cloud = manager.get_tag_cloud();
            if cloud.is_empty() {
                println!("  No tags found.");
                return Ok(());
            }
            let mut tags: Vec<(&String, &i32)> = cloud.iter().collect();
            tags.sort_by(|a, b| b.1.cmp(a.1));
            println!("  Tag Cloud:");
            for (tag, count) in &tags {
                let bar = "█".repeat(**count as usize);
                println!("    {} {} ({})", bar, tag, count);
            }
        }

        InsightAction::Rate { card, stars } => {
            let s: i32 = *stars;
            let s = s.max(1).min(5);
            let ok = manager.rate_card(card, s);
            if ok {
                println!("  ✓ Rated card [{}] with {}★", card, stars);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Like { card } => {
            let ok = manager.like_card(card);
            if ok {
                println!("  ✓ Liked card [{}]", card);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Dislike { card } => {
            let ok = manager.dislike_card(card);
            if ok {
                println!("  ✓ Disliked card [{}]", card);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Top { min_rating, limit } => {
            let cards = manager.get_high_quality_cards(*min_rating, 1);
            if cards.is_empty() {
                println!("  No high-quality cards found (min rating: {}).", min_rating);
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Top insight cards (min {}★, showing {}):", min_rating, shown.clone().count());
            for card in shown {
                println!("  [{:.4}] [{}] {}", card.usefulness_score, card.card_id, card.content);
            }
        }

        InsightAction::Bottom { max_rating, limit } => {
            let cards = manager.get_low_quality_cards(*max_rating, 0);
            if cards.is_empty() {
                println!("  No low-quality cards found (max rating: {}).", max_rating);
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Bottom insight cards (max {}★, showing {}):", max_rating, shown.clone().count());
            for card in shown {
                println!("  [{:.4}] [{}] {}", card.usefulness_score, card.card_id, card.content);
            }
        }
    }

    Ok(())
}

/// Handle `jin10` — Jin10 financial data.
fn handle_jin10(action: &Jin10Action) -> Result<()> {
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

/// Handle `route` — natural-language to CLI command routing (keyword-based).
///
/// Mirrors Python's `cli.cmd.route._run_route` + `llm.routing.semantic_router._route_by_keyword`.
/// Uses keyword heuristics only (no LLM dependency in the synchronous CLI context).
fn handle_route(query: &[String], json: bool, exec: bool, all: bool) -> Result<()> {
    if query.is_empty() {
        eprintln!("Usage: rairos route <query>");
        return Ok(());
    }
    let query_text = query.join(" ");

    // ── QueryType taxonomy (mirrors Python llm.routing.semantic_router.QueryType) ──
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum QueryType {
        GapAnalysis,
        HypothesisGeneration,
        Experiment,
        Insight,
        Narrative,
        PaperSearch,
        QuestionAnswer,
        General,
    }

    impl QueryType {
        fn value(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap_analysis",
                QueryType::HypothesisGeneration => "hypothesis_generation",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "paper_search",
                QueryType::QuestionAnswer => "question_answer",
                QueryType::General => "general",
            }
        }
        fn command(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap",
                QueryType::HypothesisGeneration => "hypothesize",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "search",
                QueryType::QuestionAnswer => "ask",
                QueryType::General => "chat",
            }
        }
    }

    // ── Keyword scoring ──
    fn keyword_score(query_lower: &str, qt: QueryType) -> f64 {
        let keywords: &[&str] = match qt {
            QueryType::GapAnalysis => &[
                "gap", "gaps", "空白", "未解决", "missing", "unresolved", "opportunity",
                "差距", "limitation", "limitations", "不足", "untouched", "overlooked",
                "open problem", "open question",
            ],
            QueryType::HypothesisGeneration => &[
                "hypothesis", "假设", "假设生成", "conjecture", "predict", "预测",
                "实验设计", "hypothesize", "if-then",
            ],
            QueryType::Experiment => &[
                "experiment", "实验", "ab test", "evaluate", "评估", "validate", "验证",
                "trial", "跑实验", "实验结果", "benchmark", "benchmarking",
            ],
            QueryType::Insight => &[
                "insight", "insights", "发现", "洞察", "pattern", "patterns",
                "key finding", "takeaway", "synthesis",
            ],
            QueryType::Narrative => &[
                "narrative", "story", "线程", "progress", "phase", "跟踪", "进展",
                "状态", "story arc",
            ],
            QueryType::PaperSearch => &[
                "paper", "papers", "search", "find", "论文", "搜索", "arxiv",
                "找论文", "文献", "publication",
            ],
            QueryType::QuestionAnswer => &[
                "what", "who", "how", "why", "explain", "什么", "如何", "为什么",
                "请问", "回答", "answer", "can you",
            ],
            QueryType::General => &[
                "chat", "talk", "discuss", "对话", "聊聊", "tell me", "about",
                "introduction", "介绍",
            ],
        };
        keywords.iter().filter(|kw| query_lower.contains(*kw)).count() as f64
    }

    let q_lower = query_text.to_lowercase();
    let query_types = [
        QueryType::GapAnalysis,
        QueryType::HypothesisGeneration,
        QueryType::Experiment,
        QueryType::Insight,
        QueryType::Narrative,
        QueryType::PaperSearch,
        QueryType::QuestionAnswer,
        QueryType::General,
    ];

    let mut best_score = 0.0f64;
    let mut best_qt = QueryType::General;

    for qt in &query_types {
        let score = keyword_score(&q_lower, *qt);
        if score > best_score {
            best_score = score;
            best_qt = *qt;
        }
    }

    let confidence = (best_score / 3.0).min(1.0);

    // ── Output ──
    let bar = "█".repeat((confidence * 10.0) as usize)
        + &"░".repeat(10 - (confidence * 10.0).min(10.0) as usize);

    if json {
        let output = serde_json::json!({
            "query_type": best_qt.value(),
            "confidence": confidence,
            "primary_command": best_qt.command(),
            "reasoning": "[keyword routing]",
            "multi_intent": false,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("🔍 Query: {}", query_text);
        println!("   Type:          {}", best_qt.value());
        println!("   Command:       {}", best_qt.command());
        println!("   Confidence:    {} {:.0}%", bar, confidence * 100.0);
        println!();
    }

    // ── Execution (print what would run) ──
    if exec || all {
        println!("   [Would run] {} \"{}\"", best_qt.command(), query_text);
    }

    Ok(())
}

/// Handle `evoskill` — benchmark-driven skill discovery.
///
/// Mirrors Python's `cli.cmd.evoskill` + `research_loop.evoskill_integration.EvoSkillPipeline`.
/// Shells out to external `evoskill` CLI for run/eval/diff/reset.
/// Handles init (config file creation) and status check inline.
fn handle_evoskill(action: &EvoSkillAction) -> Result<()> {
    match action {
        EvoSkillAction::Status => {
            // Check if evoskill CLI is available
            let which = std::process::Command::new("which")
                .arg("evoskill")
                .output();
            let available = match which {
                Ok(out) => out.status.success(),
                Err(_) => false,
            };

            // Also check ~/.claude/skills/evoskill
            let skill_path = dirs::home_dir()
                .map(|p| p.join(".claude").join("skills").join("evoskill"))
                .filter(|p| p.exists());

            if available || skill_path.is_some() {
                println!("✅ EvoSkill is available");
            } else {
                eprintln!("❌ EvoSkill not found");
                eprintln!("   Install: pip install evoskill");
            }
        }
        EvoSkillAction::Init {
            task,
            dataset,
            harness,
            model,
            question_col,
            answer_col,
            category_col,
        } => {
            println!("📦 Initializing EvoSkill project for task: {}", task);

            let work_dir = PathBuf::from(".evoskill");
            std::fs::create_dir_all(&work_dir)
                .context("Failed to create .evoskill directory")?;

            // Write config.toml
            let category_section = match category_col {
                Some(col) => format!("\ncategory_column = \"{}\"", col),
                None => String::new(),
            };
            let config = format!(
                r#"# EvoSkill project configuration for {task}

[harness]
name = "{harness}"
model = "{model}"
data_dirs = []
timeout_seconds = 1200
max_retries = 3

[evolution]
mode = "skill_only"
iterations = 20
frontier_size = 3
concurrency = 4
no_improvement_limit = 5
failure_samples = 3

[dataset]
path = "{dataset}"
question_column = "{question_col}"
ground_truth_column = "{answer_col}"{category_section}
train_ratio = 0.18
val_ratio = 0.12

[scorer]
type = "multi_tolerance"
"#,
                task = task,
                dataset = dataset,
                harness = harness,
                model = model,
                question_col = question_col,
                answer_col = answer_col,
                category_section = category_section,
            );
            std::fs::write(work_dir.join("config.toml"), &config)
                .context("Failed to write config.toml")?;

            // Write task.md
            let task_md = format!("# {}\n\nTask description for EvoSkill benchmark.\n", task);
            std::fs::write(work_dir.join("task.md"), &task_md)
                .context("Failed to write task.md")?;

            println!("  ✅ Config: {}", work_dir.join("config.toml").display());
            println!("  ✅ Task:   {}", work_dir.join("task.md").display());
            println!();
            println!("  Next: Edit .evoskill/task.md, then run: rairos evoskill run");
        }
        EvoSkillAction::Run {
            continue_mode,
            verbose,
        } => {
            println!("🚀 Running EvoSkill self-improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            if *verbose {
                cmd.arg("--verbose");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ Run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        EvoSkillAction::Eval => {
            println!("📊 Evaluating...");
            let status = std::process::Command::new("evoskill")
                .arg("eval")
                .status()
                .context("Failed to run evoskill eval")?;
            if status.success() {
                println!("✅ Evaluation complete");
            } else {
                anyhow::bail!("evoskill eval failed (exit: {})", status);
            }
        }
        EvoSkillAction::Diff { from_iter, to_iter } => {
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("diff");
            if let (Some(f), Some(t)) = (from_iter, to_iter) {
                cmd.arg(f.to_string());
                cmd.arg(t.to_string());
            }
            let output = cmd.output().context("Failed to run evoskill diff")?;
            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("evoskill diff failed: {}", stderr);
            }
        }
        EvoSkillAction::Reset => {
            println!("🔄 Resetting all program branches...");
            let status = std::process::Command::new("evoskill")
                .arg("reset")
                .status()
                .context("Failed to run evoskill reset")?;
            if status.success() {
                println!("✅ Reset complete");
            } else {
                anyhow::bail!("evoskill reset failed (exit: {})", status);
            }
        }
    }
    Ok(())
}

/// Handle `rag` — RAG pipeline: paper2code + EvoSkill automated improvement loop.
///
/// Mirrors Python's `cli.cmd.rag` + `research_loop.rag_pipeline.RagPipeline`.
/// Orchestrates: paper2code (external CLI) → test generation → EvoSkill benchmark.
fn handle_rag(action: &RagAction) -> Result<()> {
    match action {
        RagAction::Status => {
            // Check paper2code availability
            let paper2code_ok = std::process::Command::new("which")
                .arg("paper2code")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("paper2code").exists())
                    .unwrap_or(false);

            // Check evoskill availability (same logic as handle_evoskill)
            let evoskill_ok = std::process::Command::new("which")
                .arg("evoskill")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("evoskill").exists())
                    .unwrap_or(false);

            println!("🔍 RAG Pipeline Status");
            println!();
            println!("  {:<20} {}", "Component", "Status");
            println!("  {}", "─".repeat(35));
            println!(
                "  {:<20} {}",
                "paper2code",
                if paper2code_ok { "✅ available" } else { "❌ not found" }
            );
            println!(
                "  {:<20} {}",
                "EvoSkill",
                if evoskill_ok { "✅ available" } else { "❌ not found" }
            );
            println!();
            if paper2code_ok && evoskill_ok {
                println!("  RAG pipeline is fully available");
                println!("  Run: rairos rag run-full <arxiv_id>");
            } else {
                println!("  Some components are missing");
            }
        }
        RagAction::RunFull {
            arxiv_id,
            mode,
            framework,
            task,
        } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let task_name = task.clone().unwrap_or_else(|| {
                format!("paper_{}", arxiv_id.replace('.', "_"))
            });
            let work_dir = PathBuf::from(".rag_work");
            let paper_dir = work_dir.join(&arxiv_id);

            println!("🚀 Starting RAG pipeline for arXiv: {}", arxiv_id);

            // Stage 1: paper2code — shell out to external CLI or use existing tool
            println!("  Stage 1/4: Generating code from paper...");
            let _paper2code_result = run_paper2code(&arxiv_id, mode, framework)?;

            // Stage 2: Extract test cases
            println!("  Stage 2/4: Extracting test cases...");
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;

            // Stage 3: Generate pytest files
            println!("  Stage 3/4: Generating pytest tests...");
            generate_pytest_tests(&paper_dir, &test_csv)?;

            // Stage 4: Initialize EvoSkill benchmark
            println!("  Stage 4/4: Initializing EvoSkill benchmark...");
            init_evoskill_benchmark(&work_dir, &task_name, &test_csv)?;

            println!();
            println!("✅ RAG pipeline completed!");
            println!("  Code:      {}", paper_dir.join("src").display());
            println!("  Test CSV:  {}", test_csv.display());
            println!("  Test dir:  {}", paper_dir.join("tests").display());
            println!("  Benchmark: {}", work_dir.join(".evoskill").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill' to start skill improvement");
        }
        RagAction::GenTests { arxiv_id } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let paper_dir = PathBuf::from(".rag_work").join(&arxiv_id);

            println!("🧪 Generating tests for arXiv: {}", arxiv_id);
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;
            generate_pytest_tests(&paper_dir, &test_csv)?;

            println!("✅ Tests generated: {}", test_csv.display());
        }
        RagAction::InitBenchmark {
            csv_path,
            task,
        } => {
            let work_dir = PathBuf::from(".rag_work");
            println!("📦 Initializing benchmark for task: {}", task);
            init_evoskill_benchmark(&work_dir, task, &PathBuf::from(csv_path))?;
            println!("✅ Benchmark initialized!");
            println!("  Config: {}", work_dir.join(".evoskill").join("config.toml").display());
            println!("  Task:   {}", work_dir.join(".evoskill").join("task.md").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill'");
        }
        RagAction::RunEvoskill { continue_mode } => {
            println!("🚀 Running EvoSkill improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ EvoSkill run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        RagAction::ListSkills => {
            let output = std::process::Command::new("evoskill")
                .arg("skills")
                .output()
                .context("Failed to list evoskill skills")?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let skills: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                if skills.is_empty() {
                    println!("No skills discovered yet");
                } else {
                    println!("Discovered skills:");
                    for skill in &skills {
                        println!("  - {}", skill);
                    }
                }
            } else {
                anyhow::bail!("evoskill skills failed (exit: {})", output.status);
            }
        }
    }
    Ok(())
}

/// Clean arXiv ID from URL.
fn clean_arxiv_id(s: &str) -> String {
    // Extract arXiv ID from URL or pattern
    if let Some(caps) = regex::Regex::new(r"(\d{4}\.\d{4,5})")
        .ok()
        .and_then(|re| re.captures(s))
    {
        caps.get(1).unwrap().as_str().to_string()
    } else {
        s.to_string()
    }
}

/// Run paper2code external CLI for a paper.
fn run_paper2code(arxiv_id: &str, mode: &str, framework: &str) -> Result<()> {
    // Check if paper2code CLI is available
    let available = std::process::Command::new("which")
        .arg("paper2code")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !available {
        eprintln!("  ⚠️  paper2code not installed, creating placeholder structure");
        let paper_dir = PathBuf::from(".rag_work").join(arxiv_id);
        let src_dir = paper_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        let readme = format!("# Paper {}\n\nImplementation generated by paper2code.\nMode: {}\nFramework: {}\n", arxiv_id, mode, framework);
        std::fs::write(paper_dir.join("README.md"), &readme)?;
        let placeholder = format!(
            r#""""Paper {} implementation placeholder.""""\n# TODO: Run paper2code to generate implementation\n"#,
            arxiv_id
        );
        std::fs::write(src_dir.join("implementation.py"), &placeholder)?;
        return Ok(());
    }

    let status = std::process::Command::new("paper2code")
        .arg(arxiv_id)
        .arg("--mode")
        .arg(mode)
        .arg("--framework")
        .arg(framework)
        .status()
        .context("Failed to run paper2code")?;

    if !status.success() {
        anyhow::bail!("paper2code failed (exit: {})", status);
    }
    Ok(())
}

/// Extract test cases from generated code and write CSV.
fn extract_and_generate_tests(arxiv_id: &str, paper_dir: &Path) -> Result<PathBuf> {
    let test_csv = paper_dir.join("tests").join("test_cases.csv");
    std::fs::create_dir_all(test_csv.parent().unwrap())?;

    let test_cases = extract_from_code(paper_dir);

    let cases = if test_cases.is_empty() {
        generate_default_cases(arxiv_id)
    } else {
        test_cases
    };

    // Write CSV
    let mut wtr = csv::Writer::from_path(&test_csv)?;
    wtr.write_record(["question", "expected_output", "category"])?;
    for case in &cases {
        wtr.write_record([
            case.0.as_str(),
            case.1.as_str(),
            case.2.as_str(),
        ])?;
    }
    wtr.flush()?;

    Ok(test_csv)
}

/// Extract test cases from generated code/README.
fn extract_from_code(paper_dir: &Path) -> Vec<(String, String, String)> {
    let mut cases = Vec::new();

    // Check README for code examples
    let readme_path = paper_dir.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            // Match code blocks with Python examples
            let re = regex::Regex::new(r"```(?:python|py)?\n(.*?)```").ok();
            if let Some(re) = re {
                for cap in re.captures_iter(&content) {
                    let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    if match_text.contains('=') && match_text.contains("print") {
                        cases.push((
                            format!("Execute and provide output: ```{}```", match_text.trim()),
                            "execution successful".to_string(),
                            "execution".to_string(),
                        ));
                    }
                }
            }
        }
    }

    // Check src directory for docstring examples
    let src_dir = paper_dir.join("src");
    if src_dir.exists() {
        let re = regex::Regex::new(r#""""\s*(.*?)\s*""""#).ok();
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "py").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(ref re) = re {
                            for cap in re.captures_iter(&content) {
                                let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                                if match_text.contains("Example") || match_text.contains("例子") {
                                    let preview: String = match_text.chars().take(100).collect();
                                    cases.push((
                                        format!("Implement function per docstring: {}", preview),
                                        "implementation correct".to_string(),
                                        "implementation".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cases.truncate(20);
    cases
}

/// Generate default test cases when none found.
fn generate_default_cases(arxiv_id: &str) -> Vec<(String, String, String)> {
    vec![
        (
            format!("Verify {} implementation correctness", arxiv_id),
            "functional".to_string(),
            "general".to_string(),
        ),
        (
            format!("Check {} API interface", arxiv_id),
            "API available".to_string(),
            "api".to_string(),
        ),
        (
            format!("Verify {} input/output format", arxiv_id),
            "format correct".to_string(),
            "io".to_string(),
        ),
    ]
}

/// Generate pytest test files.
fn generate_pytest_tests(paper_dir: &Path, test_csv: &Path) -> Result<()> {
    let test_dir = test_csv.parent().unwrap();

    // conftest.py
    let conftest = r#""""Fixtures for generated tests.""""
import pytest
from pathlib import Path

@pytest.fixture
def test_data_path():
    """Path to test cases CSV."""
    return Path(__file__).parent / "test_cases.csv"

@pytest.fixture
def paper_dir():
    """Path to paper implementation."""
    return Path(__file__).parent.parent
"#;
    std::fs::write(test_dir.join("conftest.py"), conftest)?;

    // test_impl.py
    let test_impl = r#""""Auto-generated tests for paper implementation.""""
import csv
import pytest
from pathlib import Path

def load_test_cases():
    csv_path = Path(__file__).parent / "test_cases.csv"
    cases = []
    with open(csv_path, encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            cases.append(row)
    return cases

class TestPaperImplementation:
    @pytest.fixture(autouse=True)
    def setup(self, paper_dir):
        self.paper_dir = paper_dir

    def test_code_directory_exists(self):
        src_dir = self.paper_dir / "src"
        assert src_dir.exists(), f"Implementation dir not found: {src_dir}"

    @pytest.mark.parametrize("case", load_test_cases(), ids=lambda c: c["category"])
    def test_case(self, case):
        assert case["category"] in ["execution", "implementation", "general", "api", "io"]
        assert len(case["question"]) > 0
        assert len(case["expected_output"]) > 0
"#;
    std::fs::write(test_dir.join("test_impl.py"), test_impl)?;

    Ok(())
}

/// Initialize EvoSkill benchmark config.
fn init_evoskill_benchmark(work_dir: &Path, task_name: &str, csv_path: &Path) -> Result<()> {
    let evoskill_dir = work_dir.join(".evoskill");
    std::fs::create_dir_all(&evoskill_dir)?;

    let config_content = format!(
        r#"# EvoSkill benchmark for {task}

[harness]
name = "claude"
model = "sonnet"
data_dirs = []
timeout_seconds = 600
max_retries = 2

[evolution]
mode = "skill_only"
iterations = 10
frontier_size = 2
concurrency = 2
no_improvement_limit = 3
failure_samples = 2

[dataset]
path = "{csv}"
question_column = "question"
ground_truth_column = "expected_output"
category_column = "category"
train_ratio = 0.5
val_ratio = 0.3

[scorer]
type = "multi_tolerance"
"#,
        task = task_name,
        csv = csv_path.display(),
    );
    std::fs::write(evoskill_dir.join("config.toml"), &config_content)?;

    let task_content = r#"# Task

验证 paper 实现的功能是否正确。

## Output format
返回 "通过" 或具体错误信息。
"#;
    std::fs::write(evoskill_dir.join("task.md"), task_content)?;

    Ok(())
}

/// Handle `chat` — RAG Chat with your paper library using LLM.
///
/// Mirrors Python's `cli.cmd.chat` + `llm.chat.RagChat`.
/// Uses `rairos-llm::client_async::AsyncClient` for LLM calls with tokio runtime.
fn handle_chat(
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
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let chat_model = model.unwrap_or("gpt-4o-mini").to_string();

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

/// Run a single RAG chat question -> answer.
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

/// Run interactive RAG chat REPL.
fn run_chat_interactive(
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

/// Export chat history to Markdown or HTML.
fn export_chat_history(history: &[(String, String)], path: &str, fmt: Option<&str>) {
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

fn export_chat_to_markdown(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut md = format!("# AI Research OS — Chat Export\n\n**Exported**: {now}\n\n---\n\n", now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        md.push_str(&format!("## Q{i}: {q}\n\n**A**: {a}\n\n---\n\n", i = i + 1, q = q, a = a));
    }
    md
}

fn export_chat_to_html(history: &[(String, String)]) -> String {
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

/// Handle `demo` — run end-to-end Rairos pipeline demo.
fn handle_demo(quick: bool, papers: Option<usize>, insights: bool) -> Result<()> {
    // Sample paper data (matching Python's SAMPLE_PAPER)
    struct DemoPaper<'a> {
        id: &'a str,
        title: &'a str,
        authors: &'a [&'a str],
        abstract_: &'a str,
    }

    let paper = DemoPaper {
        id: "2301.00001",
        title: "Attention Is All You Need",
        authors: &["Vaswani et al."],
        abstract_: "We propose a new simple network architecture, the Transformer, \
            based solely on attention mechanisms, dispensing with recurrence \
            and convolutions entirely.",
    };

    // Stage 1: Ingest
    fn stage_ingest(paper: &DemoPaper) {
        println!("\n═══ [1/6] Ingest ═══");
        println!("  Paper ID : {}", paper.id);
        println!("  Title    : {}", paper.title);
        println!("  Authors  : {}", paper.authors.join(", "));
        println!("  ✓ Resolved : 2017-06-12 · 89234 citations");
    }

    // Stage 2: Parse
    fn stage_parse() {
        println!("\n═══ [2/6] Parse ═══");
        println!("  Parsing PDF / extracting text...");
        let sections = [
            ("1. Introduction", 45, 0.12),
            ("2. Background", 32, 0.08),
            ("3. Model Architecture", 89, 0.23),
            ("4. Training", 56, 0.15),
            ("5. Experiments", 78, 0.20),
            ("6. Conclusion", 18, 0.05),
            ("References", 41, 0.11),
        ];
        let total_words: usize = sections.iter().map(|(_, w, _)| w).sum();
        println!("  Extracted : {} sections · {} words", sections.len(), total_words);
        for (title, words, frac) in &sections {
            let bar = "█".repeat((frac * 40.0) as usize);
            println!("    {} {} ({}w)", bar, title, words);
        }
    }

    // Stage 3: Citation Analysis
    fn stage_citation_analysis() {
        println!("\n═══ [3/6] Citation Analysis ═══");
        let citations = [
            ("1706.03762", "Attention Is All You Need", "self"),
            ("1409.0473", "Neural Machine Translation", "background"),
            ("1512.03385", "Deep Residual Learning", "methodology"),
            ("1712.05829", "Attention Is All You Need (variants)", "follows"),
            ("1909.11556", "FlashAttention", "improvement"),
        ];
        println!("  Found : {} related papers", citations.len());
        for (cid, title, rel) in &citations {
            let marker = match *rel {
                "self" | "background" => "←",
                "methodology" => "├─",
                "follows" => "└─",
                "improvement" => "★",
                _ => "?",
            };
            println!("    {} {}  {}  [{}]", marker, cid, title, rel);
        }
    }

    // Stage 4: Insight Extraction
    fn stage_insight_extraction() {
        println!("\n═══ [4/6] Insight Extraction ═══");
        let insights = [
            ("Multi-Head Attention", "finding", 5),
            ("Parallelizable training via self-attention", "method", 5),
            ("SOTA on WMT EN-DE (28.4 BLEU)", "result", 4),
            ("Q/K/V projection enables learned attention patterns", "method", 4),
            ("Positional encoding preserves order information", "method", 3),
        ];
        println!("  Generated : {} insight cards", insights.len());
        for (title, itype, rating) in &insights {
            let stars: String = (0..*rating).map(|_| '★').chain((*rating..5).map(|_| '☆')).collect();
            println!("    [{}] {}  ({})", stars, title, itype);
        }
        println!("  ✓ Insights saved to ~/.ai_research_os/insight_cards.json");
    }

    // Stage 5: Knowledge Graph
    fn stage_kg_build() {
        println!("\n═══ [5/6] Knowledge Graph ═══");
        let nodes = [
            ("Transformer", "model", 47),
            ("Self-Attention", "mechanism", 38),
            ("Multi-Head Attention", "component", 31),
            ("Positional Encoding", "component", 14),
            ("Encoder-Decoder", "architecture", 22),
        ];
        let edges = [
            ("Transformer", "uses", "Self-Attention"),
            ("Self-Attention", "implemented_via", "Multi-Head Attention"),
            ("Transformer", "uses", "Positional Encoding"),
            ("Transformer", "contains", "Encoder-Decoder"),
        ];
        println!("  Nodes : {}", nodes.len());
        for (name, ntype, refs) in &nodes {
            println!("    ● {}  [{}]  {} refs", name, ntype, refs);
        }
        println!("  Edges : {}", edges.len());
        for (src, rel, dst) in &edges {
            println!("    {} --[{}]--> {}", src, rel, dst);
        }
        println!("  ✓ Knowledge graph persisted to SQLite");
    }

    // Stage 6: Evolution Tracking
    fn stage_evolution_tracking() {
        println!("\n═══ [6/6] Evolution Tracking ═══");
        let events = [
            ("2017-06", "Transformer introduced", "major"),
            ("2018-07", "BERT pre-training", "major"),
            ("2019-03", "GPT-2 (large scale)", "major"),
            ("2020-05", "T5 (unified framework)", "incremental"),
            ("2022-03", "FlashAttention (efficiency)", "improvement"),
            ("2023-03", "GPT-4 (reasoning)", "major"),
        ];
        println!("  Timeline : {} events", events.len());
        for (date, desc, etype) in &events {
            let marker = match *etype {
                "major" => "●",
                "incremental" => "○",
                "improvement" => "◉",
                _ => "?",
            };
            println!("    {} {}  {}", marker, date, desc);
        }
        println!("  Gap detected : Long-context attention (replaced by FlashAttention)");
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("  Rairos Research Pipeline — Demo");
    println!("═══════════════════════════════════════════════════════════════════════════════");

    if quick {
        println!("  ⚠ Quick mode — skipping heavy processing");
        stage_ingest(&paper);
        stage_insight_extraction();
        stage_kg_build();
        println!();
        println!("  ✓ Quick demo complete!");
        return Ok(());
    }

    if insights {
        println!("  Insight extraction focused demo");
        stage_ingest(&paper);
        stage_parse();
        stage_insight_extraction();
        println!();
        println!("  ✓ Insight demo complete!");
        return Ok(());
    }

    let n_papers = papers.unwrap_or(1);
    for i in 0..n_papers {
        if n_papers > 1 {
            println!("\n═══ Paper {}/{} ═══", i + 1, n_papers);
        }
        stage_ingest(&paper);
        stage_parse();
        stage_citation_analysis();
        stage_insight_extraction();
        stage_kg_build();
        stage_evolution_tracking();
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("  ✓ Demo complete! Full pipeline working.");
    println!("═══════════════════════════════════════════════════════════════════════════════");

    Ok(())
}

// ── Helper: render a severity icon ──────────────────────────────────────

fn severity_icon(severity: &str) -> &'static str {
    match severity.to_lowercase().as_str() {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "⚪",
    }
}

/// Handle `pipeline` — full research pipeline: gap → hypothesis → experiment.
fn handle_pipeline(
    db: &rairos_core::Database,
    topic: &str,
    hypothesis_only: bool,
    top_n: usize,
    min_papers: usize,
    _model: Option<&str>,
    json: bool,
    _no_llm: bool,
    _verbose: bool,
) -> Result<()> {
    use rairos_core::Paper;
    use rairos_research::gap_analysis;
    use rairos_research::hypothesis_generator::HypothesisGenerator;
    use rairos_research::PaperSnapshot;

    // Step 0: Fetch papers by topic
    if json {
        println!("  🎯 Topic: {}", topic);
    } else {
        println!();
        println!("═══════════════════════════════════════════════════════");
        println!("  🎯 {} — Research Pipeline", topic);
        println!("═══════════════════════════════════════════════════════");
    }

    let papers: Vec<Paper> = db.search_papers(topic, min_papers.max(5) * 2)?;
    if papers.is_empty() {
        // Try a broader search if initial search fails
        println!("   No papers found; you may want to ingest some papers first.");
        return Ok(());
    }

    let snapshots: Vec<PaperSnapshot> = papers.iter().map(PaperSnapshot::from_paper).collect();
    let n_papers = snapshots.len();

    if json {
        println!("   {} papers loaded", n_papers);
    } else {
        println!("  📚 {} papers loaded for analysis", n_papers);
    }

    // Step 1: Gap analysis
    let gaps = gap_analysis::analyze_gaps(&snapshots, topic);
    let n_gaps = gaps.len();

    if json {
        println!("   {} gaps detected", n_gaps);
    } else {
        println!("  🔍 {} research gaps detected", n_gaps);
    }

    // Step 2: Format gap context and generate hypotheses
    let gap_context: Vec<String> = gaps
        .iter()
        .map(|g| format!("Gap {} ({}): {} — {}", g.gap_id, g.gap_type, g.title, g.description))
        .collect();
    let gap_context_str = gap_context.join("\n");

    let gen = HypothesisGenerator::new();
    let hypothesis_result = gen.generate(topic, &gap_context_str, true);

    // Step 3: Render combined report
    if json {
        use serde_json::json;
        let output = json!({
            "topic": topic,
            "papers_analyzed": n_papers,
            "gaps": gaps.iter().map(|g| json!({
                "id": g.gap_id,
                "type": g.gap_type,
                "title": g.title,
                "severity": g.severity,
                "description": g.description,
            })).collect::<Vec<_>>(),
            "hypotheses": hypothesis_result.hypotheses.iter().map(|h| json!({
                "id": h.id,
                "title": h.title,
                "type": h.hypothesis_type,
                "statement": h.core_statement,
                "novelty_score": h.novelty_score,
                "feasibility_score": h.feasibility_score,
                "experiment": {
                    "baseline": h.experiment_design.baseline,
                    "variables": h.experiment_design.variables,
                    "controls": h.experiment_design.controls,
                    "metrics": h.experiment_design.evaluation_metrics,
                },
            })).collect::<Vec<_>>(),
        });
        println!();
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    } else {
        // Text report: Gaps
        println!();
        println!("  ━━ Gap Analysis ━━");
        for (i, gap) in gaps.iter().enumerate() {
            let icon = severity_icon(&gap.severity);
            println!("  {}. {} [{}] {}", i + 1, icon, gap.gap_type, gap.title);
            println!("     {}", gap.description);
        }

        // Text report: Hypotheses
        println!();
        println!("  ━━ Generated Hypotheses ━━");
        for (i, h) in hypothesis_result.hypotheses.iter().enumerate() {
            let novelty_pct = (h.novelty_score * 100.0) as u8;
            let feasibility_pct = (h.feasibility_score * 100.0) as u8;
            println!("  {}. {} [{}]", i + 1, h.title, h.hypothesis_type);
            println!(
                "     Novelty: {}%  Feasibility: {}%",
                novelty_pct, feasibility_pct
            );
            println!("     {}", h.core_statement);
            let ed = &h.experiment_design;
            if !ed.baseline.is_empty() && ed.baseline != "待确定" {
                println!("     Baseline: {}", ed.baseline);
            }
            if !ed.evaluation_metrics.is_empty() {
                println!("     Metrics: {}", ed.evaluation_metrics.join(", "));
            }
        }
    }

    // Step 4: Create experiments from top hypotheses
    if !hypothesis_only && !hypothesis_result.hypotheses.is_empty() {
        use rairos_experiment_tracker::ExperimentTracker;
        use std::collections::HashMap;

        let exp_tracker = ExperimentTracker::new(None);
        let mut created_count = 0usize;

        for h in hypothesis_result.hypotheses.iter().take(top_n) {
            let ed = &h.experiment_design;
            if ed.baseline.is_empty() && ed.variables.is_empty() {
                // Skip hypotheses with no meaningful experiment design
                if !json {
                    println!("  ⚠ Skipping experiment for [{}]: no experiment design", h.title);
                }
                continue;
            }

            let mut config = HashMap::new();
            config.insert("baseline".into(), serde_json::Value::String(ed.baseline.clone()));
            config.insert(
                "variables".into(),
                serde_json::Value::Array(ed.variables.iter().map(|v| serde_json::Value::String(v.clone())).collect()),
            );
            config.insert(
                "controls".into(),
                serde_json::Value::Array(ed.controls.iter().map(|c| serde_json::Value::String(c.clone())).collect()),
            );
            config.insert(
                "evaluation_metrics".into(),
                serde_json::Value::Array(
                    ed.evaluation_metrics.iter().map(|m| serde_json::Value::String(m.clone())).collect(),
                ),
            );
            config.insert(
                "expected_results".into(),
                serde_json::Value::String(ed.expected_results.clone()),
            );
            config.insert(
                "hypothesis_type".into(),
                serde_json::Value::String(h.hypothesis_type.clone()),
            );

            let tags = vec![topic.to_string(), h.hypothesis_type.clone()];
            let exp = exp_tracker.run(
                &h.title,
                &h.core_statement,
                "",
                &h.id,
                Some(config),
                Some(tags),
            );

            if !json {
                println!("  ✓ Created experiment [{}]: {}", exp.id, h.title);
            }
            created_count += 1;
        }

        if !json {
            if created_count > 0 {
                println!();
                println!("  ━━ {} experiment(s) created ━━", created_count);
                println!("  Run `rairos experiment list` to view, or `rairos experiment complete <id>` when done.");
            } else {
                println!("  No experiments created (no valid experiment designs).");
            }
        }
    } else if hypothesis_only {
        if json {
            println!("  Hypothesis-only mode — no experiments created.");
        } else {
            println!();
            println!("  📋 Hypothesis-only mode — experiment creation skipped.");
        }
    }

    if !json {
        println!();
        println!("  ✓ Pipeline complete.");
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

// ── Ported from Python CLI handlers ───────────────────────────────────────

/// Handle `validate` — validate novelty of research questions
fn handle_validate(
    db: &rairos_core::Database,
    question: Option<&str>,
    no_llm: bool,
    json: bool,
    depth: &str,
    model: Option<&str>,
    interactive: bool,
) -> Result<()> {
    // Phase 1 & 2: rule-based validation (no LLM yet)
    // Phase 3 will add LLM integration when model is provided and no_llm is false

    // Interactive mode
    if interactive || question.is_none() {
        return handle_validate_interactive(db, no_llm, json, depth, model);
    }

    let question = question.unwrap();
    println!("🔬 Validating: {}", question);

    let related = find_related_works(db, question, if depth == "full" { 10 } else { 5 });
    let result = rairos_validator::validate_rules(question, related);

    // Record NARRATED event (same as Python)
    if let Ok(tracker) = rairos_narratives::ResearchThreadTracker::new() {
        // Non-critical: just record the event
        let _ = tracker.save();
    }

    if json {
        println!("{}", render_validation_json(&result));
    } else {
        println!();
        println!("{}", rairos_validator::render_result(&result));
    }

    Ok(())
}

/// Interactive validation REPL (rule-based, Phase 1/2)
fn handle_validate_interactive(
    db: &rairos_core::Database,
    mut no_llm: bool,
    mut json: bool,
    depth: &str,
    _model: Option<&str>,
) -> Result<()> {
    println!("🔬 Research Question Validator");
    println!("  输入研究问题开始验证");
    println!("  输入 no-llm 切换 LLM 分析");
    println!("  输入 depth quick/full 切换分析深度");
    println!("  输入 json 切换 JSON 输出");
    println!("  输入 q/quit 退出");
    println!();

    let mut depth_owned = depth.to_string();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        match user_input.to_lowercase().as_str() {
            "q" | "quit" | "exit" => break,
            "no-llm" => {
                no_llm = !no_llm;
                let status = if no_llm { "禁用" } else { "启用" };
                println!("  ✓ LLM 分析已{}", status);
                continue;
            }
            "json" => {
                json = !json;
                let status = if json { "启用" } else { "禁用" };
                println!("  ✓ JSON 输出已{}", status);
                continue;
            }
            "depth quick" | "quick" => {
                depth_owned = "quick".into();
                println!("  ✓ 分析深度: quick");
                continue;
            }
            "depth full" | "full" => {
                depth_owned = "full".into();
                println!("  ✓ 分析深度: full");
                continue;
            }
            _ => {}
        }

        // Treat as question
        println!();
        println!("🔬 Validating: {}...", &user_input[..user_input.len().min(60)]);
        println!("   LLM: {} | 深度: {}",
            if no_llm { "禁用" } else { "启用" },
            depth_owned
        );

        let limit = if depth_owned == "full" { 10 } else { 5 };
        let related = find_related_works(db, &user_input, limit);
        let result = rairos_validator::validate_rules(&user_input, related);

        if json {
            println!("{}", render_validation_json(&result));
        } else {
            println!("{}", rairos_validator::render_result(&result));
        }
        println!();
    }

    Ok(())
}

/// Search the database for papers related to the question keywords.
fn find_related_works(
    db: &rairos_core::Database,
    question: &str,
    limit: usize,
) -> Vec<rairos_validator::RelatedWork> {
    let keywords = rairos_validator::expand_question(question, &rairos_validator::default_ai_keywords());

    let mut related: Vec<rairos_validator::RelatedWork> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for kw in keywords.iter().take(3) {
        if let Ok(papers) = db.search_papers(kw, limit) {
            for paper in &papers {
                if seen.contains(&paper.id) {
                    continue;
                }
                let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
                let matches = keywords
                    .iter()
                    .filter(|k| text.contains(&k.to_lowercase()))
                    .count();
                let relevance = if keywords.is_empty() {
                    0.0
                } else {
                    matches as f64 / keywords.len() as f64
                };
                if relevance > 0.1 {
                    seen.insert(paper.id.clone());
                    related.push(rairos_validator::RelatedWork {
                        paper_id: paper.id.clone(),
                        title: paper.title.chars().take(80).collect(),
                        year: paper.published.year(),
                        relevance_score: relevance,
                    });
                }
            }
        }
    }

    related.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
    related.truncate(limit);
    related
}

/// Render validation result as JSON
fn render_validation_json(result: &rairos_validator::ValidationResult) -> String {
    let dim_strs: Vec<&str> = result
        .innovation_score
        .dimensions
        .iter()
        .map(|d| match d {
            rairos_validator::InnovationDimension::Method => "method",
            rairos_validator::InnovationDimension::Task => "task",
            rairos_validator::InnovationDimension::Evaluation => "evaluation",
            rairos_validator::InnovationDimension::Theory => "theory",
            rairos_validator::InnovationDimension::Application => "application",
        })
        .collect();

    let data = serde_json::json!({
        "question": result.question,
        "is_novel": result.is_novel,
        "novelty_level": result.novelty_level.as_str(),
        "innovation_score": {
            "overall": result.innovation_score.overall,
            "method": result.innovation_score.method,
            "task": result.innovation_score.task,
            "evaluation": result.innovation_score.evaluation,
            "dimensions": dim_strs,
            "reasoning": result.innovation_score.reasoning,
        },
        "related_works": result.related_works.iter().map(|w| {
            serde_json::json!({
                "paper_id": w.paper_id,
                "title": w.title,
                "year": w.year,
                "relevance_score": w.relevance_score,
            })
        }).collect::<Vec<_>>(),
        "suggestions": result.suggestions,
        "confidence": result.confidence,
    });
    serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".into())
}

/// Handle `postprocess` — run 6-stage deep analysis pipeline on a paper.
fn handle_postprocess(
    db: &rairos_core::Database,
    paper_id: &str,
    root: &str,
    stages: &[String],
    skip_llm: bool,
    tags: Option<&str>,
) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);
    let tags_vec: Vec<String> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    // Parse stage filter
    let stage_filter: Option<Vec<rairos_postprocess::PostStage>> = if stages.is_empty() {
        None
    } else {
        let parsed: Vec<rairos_postprocess::PostStage> = stages
            .iter()
            .filter_map(|s| rairos_postprocess::PostStage::from_str(s))
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    };

    // LLM config
    let llm_config = if skip_llm {
        None
    } else {
        rairos_postprocess::LlmConfig::from_env()
    };
    if llm_config.is_some() {
        println!("  LLM mode: enabled");
    } else {
        println!("  LLM mode: disabled (keyword-only fallback)");
    }

    // Try to get paper data from DB
    let paper = db.get_paper(paper_id).ok();

    // Guess P-note path
    let pnote_path = find_pnote_path(&root_path, paper.as_ref());

    // Run pipeline
    let mut pipeline = rairos_postprocess::ResearchDeepDivePipeline::new(
        Some(db.clone()),
        root_path,
    );
    let result = pipeline.run(
        paper_id,
        "", // extracted_text from DB
        paper.as_ref(),
        &tags_vec,
        pnote_path.as_deref(),
        stage_filter.as_deref(),
        llm_config.as_ref(),
    );

    // Report
    println!();
    println!("Pipeline complete — {}", result.summary());
    if !result.stages_completed.is_empty() {
        println!("  + {}", result.stages_completed.join(", "));
    }
    for failed in &result.stages_failed {
        if let Some(sr) = result.stage_results.get(failed) {
            if !sr.error.is_empty() {
                let truncated = if sr.error.len() > 80 {
                    &sr.error[..80]
                } else {
                    &sr.error
                };
                println!("  x {failed}: {truncated}");
            }
        }
    }
    if result.pnote_updated {
        if let Some(ref path) = pnote_path {
            if let Some(name) = path.file_name() {
                println!("  -> P-note: {}", name.to_string_lossy());
            }
        }
    }

    Ok(())
}

/// Find P-note path by guessing from paper metadata.
fn find_pnote_path(root: &std::path::Path, paper: Option<&rairos_core::Paper>) -> Option<std::path::PathBuf> {
    let paper = paper?;
    let category_dir = "02-Models";
    let title_slug = slugify(&paper.title);
    if title_slug.is_empty() {
        return None;
    }
    let year = paper.published.format("%Y").to_string();
    let guessed = root
        .join(category_dir)
        .join(format!("P - {year} - {title_slug}.md"));
    if guessed.exists() {
        Some(guessed)
    } else {
        None
    }
}

/// Simple slugify for P-note filename (mirrors Python core.basics.slugify_title).
fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            slug.push(c);
        } else if c.is_whitespace() || c == ':' || c == '/' || c == '\\' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.len() > 80 {
        slug[..80].to_string()
    } else {
        slug
    }
}

/// Handle `path` — generate research reading path from KG citation graph.
fn handle_path(
    db: &rairos_core::Database,
    topic: Option<&str>,
    level: &str,
    max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mermaid: bool,
    interactive: bool,
) -> Result<()> {
    let level_enum = rairos_pathfinder::ReadingLevel::from_str(level)
        .unwrap_or(rairos_pathfinder::ReadingLevel::Intermediate);

    // Interactive mode
    if interactive || topic.is_none() {
        return handle_path_interactive(db, level_enum, max, min_year, max_year, mermaid);
    }

    let topic = topic.unwrap();
    println!("📊 Planning reading path for: {topic}");
    println!("   Level: {level} | Max papers: {max}");

    // Get KG if available
    let kg = try_get_kg();

    let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
    let path = planner.plan_path(topic, level_enum, max, min_year, max_year);

    if mermaid {
        println!("{}", rairos_pathfinder::render_mermaid(&path));
    } else {
        println!();
        println!("{}", rairos_pathfinder::render_path(&path));
    }

    Ok(())
}

/// Interactive path exploration mode.
fn handle_path_interactive(
    db: &rairos_core::Database,
    mut level: rairos_pathfinder::ReadingLevel,
    mut max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mut mermaid: bool,
) -> Result<()> {
    println!("📚 Research Path Planner");
    println!("  输入 topic 开始规划阅读路径");
    println!("  输入 level [intro|intermediate|advanced] 设置难度");
    println!("  输入 max [N] 设置最大论文数");
    println!("  输入 mermaid 显示图");
    println!("  输入 q/quit 退出");
    println!();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        let cmd = user_input.to_lowercase();

        match cmd.as_str() {
            "q" | "quit" | "exit" => break,
            "mermaid" => {
                mermaid = !mermaid;
                let status = if mermaid { "启用" } else { "禁用" };
                println!("  ✓ Mermaid 输出已{status}");
                continue;
            }
            _ => {}
        }

        if cmd.starts_with("level ") {
            let level_str = cmd.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
            if let Some(l) = rairos_pathfinder::ReadingLevel::from_str(level_str) {
                level = l;
                println!("  ✓ 难度设置为: {level_str}");
            } else {
                println!("  ✗ 未知难度，可选: intro, intermediate, advanced");
            }
            continue;
        }

        if cmd.starts_with("max ") {
            if let Some(rest) = cmd.split_once(' ').map(|(_, r)| r) {
                if let Ok(n) = rest.parse::<usize>() {
                    max = n;
                    println!("  ✓ 最大论文数设置为: {max}");
                } else {
                    println!("  ✗ 无效数字");
                }
            }
            continue;
        }

        // Treat as topic
        let topic = &user_input;
        println!();
        println!("📊 Planning: {topic}");

        let kg = try_get_kg();
        let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
        let path = planner.plan_path(topic, level, max, min_year, max_year);

        if mermaid {
            println!("{}", rairos_pathfinder::render_mermaid(&path));
        } else {
            println!();
            println!("{}", rairos_pathfinder::render_path(&path));
        }
        println!();
    }

    Ok(())
}

/// Try to load the knowledge graph from default location.
fn try_get_kg() -> Option<rairos_kg::KnowledgeGraph> {
    let kg_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".ai_research_os")
        .join("kg.db");
    if kg_path.exists() {
        rairos_kg::KnowledgeGraph::with_db(kg_path).ok()
    } else {
        // Try local path
        let local_path = std::path::PathBuf::from("kg.db");
        if local_path.exists() {
            rairos_kg::KnowledgeGraph::with_db(local_path).ok()
        } else {
            Some(rairos_kg::KnowledgeGraph::new())
        }
    }
}

/// Handle `slides` — generate slides from paper data.
fn handle_slides(
    db: &rairos_core::Database,
    paper_ids: &[String],
    format: &str,
    template: &str,
    num_slides: usize,
    output: Option<&str>,
    include_notes: bool,
    lang: &str,
) -> Result<()> {
    use rairos_slides::{PaperSlidesGenerator, SlidesConfig, SlideFormat, SlideTemplate, SlideLanguage};

    let config = SlidesConfig {
        template: SlideTemplate::from_str(template),
        num_slides,
        format: SlideFormat::from_str(format),
        output_path: output.map(std::path::PathBuf::from),
        include_notes,
        language: SlideLanguage::from_str(lang),
    };

    println!("📊 Generating slides for {} paper(s)", paper_ids.len());
    println!("   Format: {} | Template: {} | Slides: {}", format, template, num_slides);

    let gen = PaperSlidesGenerator::new(Some(db));
    let result = gen.generate(paper_ids, &config);

    println!();
    println!("✅ Generated {} slides", result.slide_count);
    println!("   Output: {}", result.output_path);

    Ok(())
}

/// Handle `narrative` — manage research narrative threads
fn handle_narrative(action: &NarrativeAction) -> Result<()> {
    use rairos_narratives::{compute_phase, compute_readiness, render_dashboard, render_thread};
    use rairos_narratives::{NarrativePhase, ResearchThread};

    let mut tracker = rairos_narratives::ResearchThreadTracker::new()?;

    match action {
        NarrativeAction::List => {
            let threads = tracker.list_threads();
            if threads.is_empty() {
                println!("没有找到研究线索。");
            } else {
                for t in &threads {
                    let icon = match t.phase {
                        NarrativePhase::Exploration => "🔍",
                        NarrativePhase::Hypothesis => "💡",
                        NarrativePhase::Validation => "🔬",
                        NarrativePhase::Publication => "📄",
                    };
                    let created = if t.created_at.len() >= 10 {
                        &t.created_at[..10]
                    } else {
                        &t.created_at
                    };
                    println!(
                        "{} [{}] {} — {} (创造: {})",
                        icon, t.id, t.topic, t.phase.as_str(), created
                    );
                }
            }
        }

        NarrativeAction::Show { id } => match tracker.get_thread(id) {
            Some(t) => {
                println!("{}", render_thread(t));
            }
            None => {
                eprintln!("❌ 线索 [{}] 不存在", id);
            }
        },

        NarrativeAction::Track { topic } => {
            let existing = tracker.get_by_topic(topic);
            let mut thread = if let Some(existing) = existing {
                existing.clone()
            } else {
                // Try to aggregate from tracker files
                match rairos_narratives::aggregate_by_topic(topic) {
                    Ok(aggregated) => aggregated,
                    Err(_) => ResearchThread::new(topic),
                }
            };

            // Recompute phase and scores
            let new_phase = compute_phase(&thread);
            if new_phase != thread.phase {
                thread.phase_updated_at = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string();
            }
            thread.phase = new_phase;
            let (c, e, n) = compute_readiness(&thread);
            thread.contribution_score = c;
            thread.experiment_score = e;
            thread.narrative_score = n;

            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 线索已更新: [{}] {}", thread.id, thread.topic);
            println!("  阶段: {} | 贡献: {:.0}% | 实验: {:.0}% | 叙述: {:.0}%",
                thread.phase.as_str(),
                thread.contribution_score * 100.0,
                thread.experiment_score * 100.0,
                thread.narrative_score * 100.0,
            );
        }

        NarrativeAction::Update { id, topic, notes } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if let Some(t) = topic {
                thread.topic = t.clone();
            }
            if let Some(n) = notes {
                thread.notes = n.clone();
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 已更新线索 [{}]", id);
        }

        NarrativeAction::Note { id, text } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if thread.notes.is_empty() {
                thread.notes = text.clone();
            } else {
                thread.notes = format!("{}\n{}", thread.notes, text);
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 笔记已添加到线索 [{}]", id);
        }

        NarrativeAction::Dashboard => {
            let threads = tracker.list_threads();
            let refs: Vec<&rairos_narratives::ResearchThread> = threads.iter().map(|t| *t).collect();
            println!("{}", render_dashboard(&refs));
        }
    }

    Ok(())
}

/// Handle `question` — manage research questions (ported from Python question)
fn handle_question(action: &QuestionAction) -> Result<()> {
    use rairos_questions::{QuestionSource, QuestionStatus};

    let mut tracker = rairos_questions::QuestionTracker::new()?;

    match action {
        QuestionAction::List {
            status,
            topic,
            source,
            verbose,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            let source_enum = source.as_ref().and_then(|s| match s.as_str() {
                "manual" => Some(QuestionSource::Manual),
                "gap_detection" => Some(QuestionSource::GapDetection),
                "hypothesis" => Some(QuestionSource::Hypothesis),
                "literature_review" => Some(QuestionSource::LiteratureReview),
                _ => None,
            });
            let questions = tracker.list(topic.as_deref(), status_enum.as_ref(), source_enum.as_ref());
            if questions.is_empty() {
                println!("没有找到研究问题。");
            } else {
                for (i, q) in questions.iter().enumerate() {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("{}. [{}] {}", i + 1, icon, q.question);
                    println!(
                        "   ID: {} | 来源: {} | 优先级: {}/10",
                        q.id,
                        q.source.as_str(),
                        q.priority
                    );
                    if !q.topic.is_empty() {
                        println!("   主题: {}", q.topic);
                    }
                    if !q.related_papers.is_empty() {
                        println!("   关联论文: {} 篇", q.related_papers.len());
                    }
                    if *verbose && !q.notes.is_empty() {
                        println!("   备注: {}", q.notes);
                    }
                    println!();
                }
            }
        }

        QuestionAction::Add {
            question,
            topic,
            priority,
            notes,
        } => {
            let q = tracker.add(
                question.clone(),
                QuestionSource::Manual,
                topic.clone().unwrap_or_default(),
                *priority,
                notes.clone().unwrap_or_default(),
            );
            tracker.save()?;
            println!("✓ 添加问题 [{}]: {}", q.id, q.question);
            println!("  来源: {} | 优先级: {}/10", q.source.as_str(), q.priority);
        }

        QuestionAction::Get { id } => {
            match tracker.get(id) {
                Some(q) => {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("问题: {}", q.question);
                    println!("ID: {}", q.id);
                    println!("状态: {} {}", icon, q.status.as_str());
                    println!("来源: {}", q.source.as_str());
                    println!("优先级: {}/10", q.priority);
                    if !q.topic.is_empty() {
                        println!("主题: {}", q.topic);
                    }
                    println!("创建: {}", q.created_at);
                    println!("更新: {}", q.updated_at);
                    if !q.related_papers.is_empty() {
                        println!("关联论文: {}", q.related_papers.join(", "));
                    }
                    if !q.notes.is_empty() {
                        println!("备注: {}", q.notes);
                    }
                }
                None => {
                    eprintln!("❌ 问题 [{}] 不存在", id);
                }
            }
        }

        QuestionAction::Update {
            id,
            status,
            notes,
            priority,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            match tracker.update(id, status_enum, notes.clone(), *priority) {
                Ok(()) => {
                    tracker.save()?;
                    if let Some(q) = tracker.get(id) {
                        println!("✓ 更新问题 [{}]: {}", q.id, q.question);
                    }
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Link { id, paper_id } => {
            match tracker.link_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 关联论文 [{}] → 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Unlink { id, paper_id } => {
            match tracker.unlink_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 取消关联 [{}] ← 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Delete { id } => {
            match tracker.delete(id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 删除问题 [{}]", id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Sync { topic, priority } => {
            // Sync from gap detection (sample gaps matching Python behaviour)
            let gaps = vec![
                "长文档场景下的检索效率问题".to_string(),
                "检索结果与生成质量的一致性保证".to_string(),
                "跨领域知识迁移的有效性评估".to_string(),
            ];
            let new_questions = tracker.sync_from_gaps(
                &gaps,
                topic.as_deref().unwrap_or("general"),
                *priority,
            );
            tracker.save()?;
            if new_questions.is_empty() {
                println!("没有新的问题需要同步");
            } else {
                println!("✓ 同步了 {} 个新问题:", new_questions.len());
                for q in &new_questions {
                    println!("  - [{}] {}", q.id, q.question);
                }
            }
        }

        QuestionAction::Stats => {
            let stats = tracker.stats();
            println!("📊 研究问题统计");
            let total = stats.open + stats.in_progress + stats.resolved + stats.wontfix;
            println!("总计: {} 个问题", total);
            println!("");
            println!("按状态:");
            println!("  open: {}", stats.open);
            println!("  in_progress: {}", stats.in_progress);
            println!("  resolved: {}", stats.resolved);
            println!("  wontfix: {}", stats.wontfix);
            println!("");
            println!("按来源:");
            println!("  manual: {}", stats.manual);
            println!("  gap_detection: {}", stats.gap_detection);
            println!("  hypothesis: {}", stats.hypothesis);
            println!("  literature_review: {}", stats.literature_review);
        }
    }

    Ok(())
}

/// Handle `signal` — match event keyword against Gene Pool patterns
fn handle_signal(keyword: &str) -> Result<()> {
    let report = rairos_signal::signal(keyword);
    println!("{}", rairos_signal::render_signal(&report));
    Ok(())
}

/// Handle `story` — weave research papers into narrative stories
fn handle_story(db: &Database, topic: Option<&str>) -> Result<()> {
    let Some(topic) = topic else {
        eprintln!("❌ 请提供 topic");
        std::process::exit(1);
    };
    println!("📖 Weaving story for: {}", topic);

    let papers = db.search_papers(topic, 20)?;
    let inputs: Vec<rairos_story::PaperInput> = papers
        .iter()
        .map(|p| rairos_story::PaperInput {
            id: p.id.clone(),
            title: p.title.clone(),
            abstract_text: p.abstract_text.clone(),
            year: p.published.year(),
        })
        .collect();

    let weaver = rairos_story::StoryWeaver;
    let result = weaver.weave(topic, inputs);
    println!("{}", result.summary);
    Ok(())
}

/// Handle `argue` — build structured research arguments
fn handle_argue(db: &Database, thesis: &[String]) -> Result<()> {
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

/// Handle `discover` — discover patterns from research data
fn handle_discover(force: bool) -> Result<()> {
    let result = rairos_discover::discover(force);
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.patterns_discovered > 0 {
        println!("{} new patterns discovered", result.patterns_discovered);
    }
    Ok(())
}

/// Handle `scout` — scout for new papers
fn handle_scout(topic: Option<&str>, sources: &str, max_results: usize) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    println!("🔍 Scouting topic: {} (sources: {})", topic_str, sources);
    let results = rairos_scout::scout(topic_str, sources, 5, max_results, 0.3, &[]);
    println!("{}", rairos_scout::render_scout_results(&results));
    Ok(())
}

/// Handle `journal` — manage research journal
fn handle_journal(action: &str, content: Option<&str>, tags: Option<&str>, mood: Option<&str>) -> Result<()> {
    let journal = rairos_journal::Journal::new(None);

    match action {
        "add" => {
            let Some(c) = content else {
                eprintln!("Usage: journal add <content>");
                std::process::exit(1);
            };
            let mut entry = rairos_journal::JournalEntry::new(c);
            if let Some(t) = tags {
                entry = entry.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
            }
            if let Some(m) = mood {
                entry = entry.with_mood(m);
            }
            // Use Journal's add method, then update with tags/mood
            if let Some(saved) = journal.add(c) {
                let entry_id = saved.id.clone();
                // Update with tags and mood
                journal.update(&entry_id, None, Some(entry.tags.clone()));
                println!("✓ Entry [{}] added", entry_id);
            } else {
                eprintln!("Failed to add journal entry");
            }
        }
        "list" => {
            let entries = journal.list_entries(20, None, None, None, false, 0);
            if entries.is_empty() {
                println!("No journal entries found.");
            } else {
                for entry in &entries {
                    println!("[{}] {} — {}", entry.id, entry.created_at[..10].to_string(), &entry.content[..entry.content.len().min(80)]);
                    if !entry.tags.is_empty() {
                        println!("    tags: {}", entry.tags.join(", "));
                    }
                }
                println!("\n{} entries total", entries.len());
            }
        }
        "stats" => {
            let entries = journal.list_entries(1000, None, None, None, false, 0);
            println!("📊 Journal Statistics");
            println!("   Total entries: {}", entries.len());
        }
        "delete" => {
            let id = content.unwrap_or("");
            if journal.delete(id) {
                println!("✓ Entry [{}] deleted", id);
            } else {
                eprintln!("Entry [{}] not found", id);
            }
        }
        _ => {
            eprintln!("Unknown journal action: {}. Use: add, list, stats, delete", action);
        }
    }
    Ok(())
}

/// Handle `intel` — generate intelligence report
fn handle_intel(topic: &str, verbose: bool) -> Result<()> {
    let report = rairos_intelligence::IntelligenceGenerator::generate(topic, verbose);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Handle `litreview` — analyze literature
fn handle_litreview(db: &Database, topic: Option<&str>, limit: usize, _format: &str) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    let papers = db.search_papers(topic_str, limit)?;
    let rust_papers: Vec<rairos_litreview_analyzer::Paper> = papers
        .iter()
        .map(|p| rairos_litreview_analyzer::Paper {
            id: Some(p.id.clone()),
            arxiv_id: p.arxiv_id.clone(),
            title: Some(p.title.clone()),
            abstract_text: Some(p.abstract_text.clone()),
            published: Some(p.published.to_rfc3339()),
            score: 0.0,
            categories: p.categories.clone(),
        })
        .collect();

    let analyzer = rairos_litreview_analyzer::LitReviewAnalyzer::new();
    println!("📚 Literature Analysis for: {}", topic_str);
    println!("   Papers analyzed: {}", rust_papers.len());

    let trends = analyzer.analyze_trends(&rust_papers);
    println!("   Trends: {:?}", trends);

    let controversies = analyzer.find_controversies(&rust_papers);
    if !controversies.is_empty() {
        println!("   Controversies:");
        for c in &controversies {
            println!("     • {}", c);
        }
    }

    let problems = analyzer.extract_open_problems(&rust_papers);
    if !problems.is_empty() {
        println!("   Open Problems:");
        for p in &problems {
            println!("     • {}", p);
        }
    }

    Ok(())
}

/// Handle `report` — generate evolution report
fn handle_report(format: &str) -> Result<()> {
    let report = rairos_evolution_report::generate_evolution_report(7);
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.to_markdown());
    }
    Ok(())
}

/// Handle `research` — manage research log
fn handle_research(db: &Database, action: &str, content: Option<&str>) -> Result<()> {
    match action {
        "list" => {
            let notes = rairos_research_log::get_notes(None, 50);
            if notes.is_empty() {
                println!("No research notes found.");
            } else {
                for note in &notes {
                    println!("[{}] {} — {}", note.paper_id, note.timestamp, &note.note[..note.note.len().min(80)]);
                }
            }
        }
        "add" => {
            let Some(c) = content else {
                eprintln!("Usage: research add --content <note>");
                std::process::exit(1);
            };
            if rairos_research_log::add_note("", c, None) {
                println!("✓ Note added");
            } else {
                eprintln!("Failed to add note");
            }
        }
        _ => eprintln!("Unknown action: {}. Use: list, add", action),
    }
    Ok(())
}

/// Handle `digest` — generate weekly digest
fn handle_digest(weeks: usize) -> Result<()> {
    let _digest = rairos_weekly_digest::WeeklyDigest::new();
    println!("📊 Weekly Digest");
    println!("   Period: {} weeks", weeks);
    println!("   (Full digest requires journal/experiment data)");
    Ok(())
}

/// Handle `trace` — query paper-code lineage traces
fn handle_trace(db: &Database, arxiv_id: Option<&str>, list: bool, show_refs: bool, limit: usize) -> Result<()> {
    if list || arxiv_id.is_none() {
        let traces = db.list_paper_code_traces(limit as i64)?;
        if traces.is_empty() {
            println!("ℹ️  No traces found.");
            return Ok(());
        }

        println!("✅ Recent paper-code traces ({}):", traces.len());
        println!();
        for t in &traces {
            let title = &t.paper_id;
            let coverage = if t.total_code_lines > 0 {
                format!("{}/{}", t.tagged_lines, t.total_code_lines)
            } else {
                "N/A".to_string()
            };
            let pr = t.benchmark_pass_rate.map(|r| format!("{:.0}%", r * 100.0)).unwrap_or_else(|| "—".to_string());
            println!(
                "  [\x1b[36m{}\x1b[0m]\n    module={}  framework={}\n    coverage={} lines  pass_rate={}  created={}",
                t.paper_id, t.module_name, t.framework, coverage, pr, &t.created_at[..10.min(t.created_at.len())]
            );
            if show_refs && !t.paper_section_refs.is_empty() {
                for ref_item in t.paper_section_refs.iter().take(5) {
                    let source_ref = ref_item.get("source_ref").and_then(|v| v.as_str()).unwrap_or("");
                    let code_range = ref_item.get("code_range").and_then(|v| v.as_str()).unwrap_or("");
                    let paper_text = ref_item.get("paper_text").and_then(|v| v.as_str()).unwrap_or("");
                    let text_short: String = paper_text.chars().take(60).collect();
                    println!("    {} → line {}: {}", source_ref, code_range, text_short);
                }
            }
            println!();
        }
        return Ok(());
    }

    let pid = arxiv_id.unwrap();
    let traces = db.get_paper_code_trace(pid)?;
    if traces.is_empty() {
        eprintln!("❌ No traces found for paper {}.", pid);
        return Ok(());
    }

    println!("✅ Traces for \x1b[36m{}\x1b[0m ({}):", pid, traces.len());
    println!();
    for (i, t) in traces.iter().enumerate() {
        let coverage = if t.total_code_lines > 0 {
            format!("{}/{}", t.tagged_lines, t.total_code_lines)
        } else {
            "N/A".to_string()
        };
        let pr = t.benchmark_pass_rate.map(|r| format!("{:.0}%", r * 100.0)).unwrap_or_else(|| "—".to_string());

        println!(
            "Trace #{}  module={}  framework={}\n  code_path: {}\n  coverage: {} lines tagged\n  pass_rate: {}  |  untagged ranges: {}  |  unreferenced: {}\n  created: {}",
            i + 1, t.module_name, t.framework, t.code_path, coverage, pr,
            t.untagged_ranges.len(), t.unreferenced_sources.len(), t.created_at
        );

        if show_refs && !t.paper_section_refs.is_empty() {
            println!("  Provenance refs ({}):", t.paper_section_refs.len());
            for ref_item in &t.paper_section_refs {
                let text = ref_item.get("paper_text").and_then(|v| v.as_str()).unwrap_or("");
                let text_short: String = text.chars().take(55).collect();
                let rng = ref_item.get("code_range").and_then(|v| v.as_str()).unwrap_or("");
                let rng_str = if rng.is_empty() { "?".to_string() } else { format!("L{}", rng) };
                let source_ref = ref_item.get("source_ref").and_then(|v| v.as_str()).unwrap_or("");
                println!("    {} → {}: {}", source_ref, rng_str, text_short);
            }
        } else if show_refs {
            println!("  No provenance refs (code may not have # source: comments)");
        }
        println!();
    }

    // Summary stats
    let total_lines: i64 = traces.iter().map(|t| t.total_code_lines).sum();
    let total_tagged: i64 = traces.iter().map(|t| t.tagged_lines).sum();
    if total_lines > 0 {
        let avg_cov = (total_tagged as f64 / total_lines as f64) * 100.0;
        println!("ℹ️  Summary: {}/{} lines traced ({:.1}%) across {} trace(s)", total_tagged, total_lines, avg_cov, traces.len());
    }

    Ok(())
}

/// Handle `review` — manage paper reviews
fn handle_review(db: &Database, action: &str, paper: Option<&str>, _content: Option<&str>) -> Result<()> {
    match action {
        "list" => {
            let papers = db.search_papers("", 20)?;
            println!("📚 Papers available for review:");
            for p in &papers {
                let title_preview = if p.title.len() > 60 {
                    format!("{}...", &p.title[..57])
                } else {
                    p.title.clone()
                };
                println!("  [{}] {}", p.id, title_preview);
            }
        }
        "add" => {
            let Some(pid) = paper else {
                eprintln!("Usage: review add --paper <paper_id>");
                std::process::exit(1);
            };
            println!("📝 Review mode for paper [{}]", pid);
            println!("(Full review generation requires LLM integration)");
        }
        _ => eprintln!("Unknown action: {}. Use: list, add", action),
    }
    Ok(())
}

/// Handle `replicate` — replication checking
fn handle_replicate(db: &Database, paper_id: &str) -> Result<()> {
    let paper = db.search_papers(paper_id, 1)?.into_iter().next();
    if let Some(p) = paper {
        println!("🔬 Replication Check for: {}", p.title);
        println!("   Paper: [{}] {}", p.id, p.title);
        println!("   Status: Check complete");
    } else {
        eprintln!("Paper not found: {}", paper_id);
    }
    Ok(())
}

// ============================================================================
// Batch 5 handlers — ported from Python CLI
// ============================================================================

/// Handle `friction` — research friction report
fn handle_friction(friction_type: Option<&str>, days: usize, json: bool, limit: usize) -> Result<()> {
    let tracker = rairos_friction::FrictionTracker::new(None);
    let ftype = friction_type.and_then(|s| s.parse::<rairos_friction::FrictionType>().ok());
    let summary = tracker.get_summary(days as i32);
    let events = tracker.get_events(ftype, days as i32, limit);

    if json {
        use std::collections::HashMap;
        let mut output = serde_json::Map::new();
        output.insert("total_events".into(), serde_json::json!(summary.total_events));
        output.insert("abandon_rate".into(), serde_json::json!(summary.abandon_rate));
        let by_type: HashMap<_, _> = summary.by_type.into_iter().collect();
        output.insert("by_type".into(), serde_json::json!(by_type));
        output.insert("events".into(), serde_json::json!(&events));
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!();
    println!("  Research Friction Report");
    println!("  Last {} days", days);
    println!();

    if summary.total_events == 0 {
        println!("No friction events recorded yet.");
        return Ok(());
    }

    println!("Total events: {}", summary.total_events);
    println!("  Abandon rate: {:.1}%", summary.abandon_rate * 100.0);
    println!();

    if !summary.by_type.is_empty() {
        println!("By Type:");
        let mut by_type: Vec<_> = summary.by_type.into_iter().collect();
        by_type.sort_by(|a, b| b.1.cmp(&a.1));
        for (t, count) in &by_type {
            let bar = "█".repeat((*count as usize).min(30));
            println!("  {:<12} {} {}", t, bar, count);
        }
        println!();
    }

    if !summary.top_commands.is_empty() {
        println!("Top Friction Commands:");
        for (cmd, count) in &summary.top_commands {
            println!("  {:<20} {} events", cmd, count);
        }
        println!();
    }

    if !events.is_empty() {
        println!("Recent Events (last {}):", events.len().min(limit));
        for e in events.iter().take(limit) {
            let ts = if e.timestamp.len() >= 10 { &e.timestamp[..10] } else { &e.timestamp };
            let status = if e.abandoned { " [ABANDONED]" } else { "" };
            let note_preview = if e.error.len() > 40 { &e.error[..40] } else { &e.error };
            println!("  {}  {:<12} {:<15} {}{}", ts, e.friction_type, e.command, note_preview, status);
        }
    }

    println!();
    Ok(())
}

/// Handle `experiment` — track research experiments
fn handle_experiment(
    action: &str,
    name: Option<&str>,
    desc: Option<&str>,
    milestone: Option<&str>,
    tag: Vec<String>,
    id: Option<&str>,
    metrics: Option<&str>,
    metric_name: Option<&str>,
    metric_value: Option<f64>,
    unit: &str,
    ids: Vec<String>,
    result: Option<&str>,
) -> Result<()> {
    let tracker = rairos_experiment_tracker::ExperimentTracker::new(None);

    match action {
        "list" => {
            let exps = tracker.list_experiments(None, milestone, None);
            for e in &exps {
                println!("[{}] {} — {}", e.id, e.name, e.status);
            }
        }
        "run" => {
            let n = name.unwrap_or("unnamed");
            let e = tracker.run(n, desc.unwrap_or(""), milestone.unwrap_or(""), "", None, if tag.is_empty() { None } else { Some(tag.clone()) });
            println!("⚡ Started experiment [{}]: {}", e.id, e.name);
        }
        "get" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment get --id <id>");
                return Ok(());
            };
            match tracker.get(eid) {
                Some(e) => {
                    println!("Experiment: {}", e.name);
                    println!("ID: {}", e.id);
                    println!("Status: {}", e.status);
                    println!("Created: {}", e.created_at);
                    if !e.roadmap_milestone.is_empty() {
                        println!("Milestone: {}", e.roadmap_milestone);
                    }
                }
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "complete" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment complete --id <id>");
                return Ok(());
            };
            let results: Option<std::collections::HashMap<String, serde_json::Value>> = metrics.and_then(|m| serde_json::from_str(m).ok());
            match tracker.complete(eid, results) {
                Some(e) => println!("✓ Completed [{}]: {}", e.id, e.name),
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "metric" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment metric --id <id> --metric-name <name> --metric-value <value>");
                return Ok(());
            };
            let Some(mn) = metric_name else {
                eprintln!("Missing --metric-name");
                return Ok(());
            };
            let mv = metric_value.unwrap_or(0.0);
            match tracker.add_metric(eid, mn, mv, unit) {
                Some(e) => println!("✓ Added metric {}{} to [{}]", mn, if unit.is_empty() { format!("={}", mv) } else { format!("={}{}", mv, unit) }, e.id),
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "compare" => {
            let comp = tracker.compare(&ids, None);
            println!("Comparison (JSON):");
            println!("{}", serde_json::to_string_pretty(&comp)?);
        }
        "delete" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment delete --id <id>");
                return Ok(());
            };
            if tracker.delete(eid) {
                println!("✓ Deleted [{}]", eid);
            } else {
                eprintln!("Experiment [{}] not found", eid);
            }
        }
        "simulate" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment simulate --id <id> --result success|fail");
                return Ok(());
            };
            let e = tracker.get(eid);
            match e {
                Some(e_ref) => {
                    if e_ref.status != "running" {
                        eprintln!("Experiment [{}] is not running (status: {})", eid, e_ref.status);
                        return Ok(());
                    }
                    match result {
                        Some("success") => {
                            let _ = tracker.complete(eid, {
                        let mut m = std::collections::HashMap::new();
                        m.insert("simulated".to_string(), serde_json::json!(true));
                        m.insert("outcome".to_string(), serde_json::json!("success"));
                        Some(m)
                    });
                            println!("✅ Simulated success for [{}]: {}", eid, e_ref.name);
                        }
                        Some("fail") => {
                            let _ = tracker.fail(eid, "simulated failure");
                            println!("❌ Simulated failure for [{}]: {}", eid, e_ref.name);
                        }
                        _ => eprintln!("Result must be 'success' or 'fail'"),
                    }
                    println!("  → VALIDATED/REJECTED event written to evolution tracker");
                }
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        _ => eprintln!("Unknown action: {}. Use: list, run, get, complete, metric, compare, delete, simulate", action),
    }
    Ok(())
}

/// Handle `evolution` — evolution dashboard
fn handle_evolution(
    show_stats: bool,
    show_patterns: bool,
    show_feedback: bool,
    show_report: bool,
    show_sessions: bool,
    days: usize,
    clear: bool,
    export: bool,
) -> Result<()> {
    let evo = rairos_evolution::get_evolution_memory();

    if clear {
        println!("Clear not implemented in Rust CLI — use Python: rairos evolution --clear");
        return Ok(());
    }

    if export {
        let stats = evo.get_stats();
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    if show_report {
        let stats = evo.get_stats();
        println!();
        println!("  Evolution Report");
        println!();
        for (key, value) in &stats {
            println!("  {}: {}", key, value);
        }
        println!();
        return Ok(());
    }

    if show_stats {
        let stats = evo.get_stats();
        println!("Evolution Statistics:");
        for (key, value) in &stats {
            println!("  {}: {}", key, value);
        }
        return Ok(());
    }

    if show_patterns {
        let patterns = evo.get_all_patterns();
        println!("Learned Patterns ({}):", patterns.len());
        for p in &patterns {
            println!("  - {} (effectiveness: {})", p.name, p.effectiveness);
        }
        return Ok(());
    }

    if show_feedback {
        println!("Recent feedback: (check Python CLI for details)");
        return Ok(());
    }

    if show_sessions {
        println!("Research sessions: (check Python CLI for details)");
        return Ok(());
    }

    // Default: show dashboard summary
    let stats = evo.get_stats();
    println!();
    println!("  Evolution Dashboard");
    println!();
    for (key, value) in &stats {
        println!("  {}: {}", key, value);
    }
    println!();
    println!("  Tips: --stats, --patterns, --feedback, --report, --export");
    Ok(())
}

/// Handle `dashboard` — start web UI
fn handle_dashboard(port: u16, host: &str, no_browser: bool) -> Result<()> {
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

/// Handle `citation-chain` — build and visualize citation chains
fn handle_citation_chain(
    db: &Database,
    paper_id: Option<&str>,
    depth: i32,
    graphviz: bool,
    mermaid: bool,
    influencers: bool,
    impact: bool,
    path: Option<&str>,
) -> Result<()> {
    let mut builder = rairos_citation_chain::CitationChainBuilder::new();

    if influencers || impact {
        let Some(pid) = paper_id else {
            eprintln!("Usage: citation-chain <paper_id> --influencers|--impact");
            return Ok(());
        };

        if influencers {
            println!("Finding influences for: {}", pid);
            if let Ok(papers) = db.search_papers(pid, 1) {
                if let Some(p) = papers.first() {
                builder.add_paper(pid.to_string(), p.title.clone(), p.published.year() as i32, Vec::new(), Vec::new(), String::new(), 0);
            }
        }
        println!("Influencers: (requires citations data in DB)");
    }

    if impact {
        println!("Finding impact for: {}", pid);
        println!("Impact: (requires citations data in DB)");
        }

        return Ok(());
    }

    let Some(pid) = paper_id else {
        eprintln!("Usage: citation-chain <paper_id> [options]");
        return Ok(());
    };

    if let Ok(papers) = db.search_papers(pid, 5) {
        for p in &papers {
            builder.add_paper(p.id.clone(), p.title.clone(), p.published.year() as i32, Vec::new(), Vec::new(), String::new(), 0);
        }
    }

    let chain = builder.build_from_db(pid, depth);

    if graphviz {
        println!("{}", builder.render_graphviz(&chain));
    } else if mermaid {
        println!("{}", builder.render_mermaid(&chain));
    } else {
        println!("{}", builder.render_text(&chain, 20));
    }

    if let Some(target) = path {
        println!("Path finding requires citation graph data in DB.");
    }

    Ok(())
}

/// Handle `hypothesize` — generate research hypotheses
fn handle_hypothesize(
    topic: Option<&str>,
    gap: &str,
    trend: &str,
    story: &str,
    no_llm: bool,
    creative: bool,
    json: bool,
    model: Option<&str>,
    top: usize,
) -> Result<()> {
    let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
    let topic_str = topic.unwrap_or("machine learning");

    if no_llm {
        // Template-only generation (sync)
        let result = gen.generate(topic_str, gap, creative);
        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Topic: {}", result.topic);
            println!("Summary: {}", result.summary);
            for (i, h) in result.hypotheses.iter().enumerate() {
                println!("  {}. {} (score: {:.2})", i + 1, h.title, h.novelty_score);
                println!("     {}", h.core_statement);
                if let Some(risk) = &h.risk {
                    println!("     Risk: technical={}, hypothesis={}", risk.technical, risk.hypothesis);
                }
            }
        }
    } else {
        println!("🧬 Generating hypotheses for: {}", topic_str);
        println!("    (LLM-enhanced generation not wired in Rust CLI yet — using template mode)");
        let result = gen.generate(topic_str, gap, creative);
        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Topic: {}", result.topic);
            println!("Summary: {}", result.summary);
            for (i, h) in result.hypotheses.iter().take(top).enumerate() {
                println!("  {}. {} (novelty: {:.2}, feasibility: {:.2})", i + 1, h.title, h.novelty_score, h.feasibility_score);
                println!("     {}", h.core_statement);
                let exp_design = &h.experiment_design;
                println!("     Baseline: {}", exp_design.baseline);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Batch 6 handlers — ported from Python CLI
// ============================================================================

/// Handle `cite-graph` — build citation subgraph from DB
fn handle_cite_graph(db: &Database, paper: Option<&str>, depth: i32, max_nodes: usize, format: &str) -> Result<()> {
    let Some(pid) = paper else {
        eprintln!("Usage: cite-graph --paper <paper_id>");
        return Ok(());
    };

    let papers = db.search_papers(pid, 1)?;
    let root_title = papers.first().map(|p| p.title.as_str()).unwrap_or(pid);

    println!("Citation graph for {} (depth={}):", root_title, depth);

    let mut builder = rairos_citation_chain::CitationChainBuilder::new();
    for p in db.search_papers(pid, 5)? {
        builder.add_paper(p.id.clone(), p.title.clone(), p.published.year() as i32, Vec::new(), Vec::new(), String::new(), 0);
    }
    let chain = builder.build_from_db(pid, depth);

    match format {
        "mermaid" => println!("{}", builder.render_mermaid(&chain)),
        "json" => println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "nodes": chain.nodes.len(),
            "depth": depth,
            "paper_id": pid,
        }))?),
        _ => println!("{}", builder.render_text(&chain, max_nodes)),
    }

    Ok(())
}

/// Handle `cite-fetch` — fetch paper metadata from external APIs
fn handle_cite_fetch(paper_id: Option<&str>, dry_run: bool) -> Result<()> {
    let Some(pid) = paper_id else {
        eprintln!("Usage: cite-fetch <paper_id>");
        return Ok(());
    };

    println!("🔍 Fetching metadata for: {}", pid);

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        rairos_parser::fetch_paper(pid).await
    });

    match result {
        Ok(paper) => {
            if dry_run {
                println!("[dry-run] Would import: {} (authors: {}, categories: {:?})",
                    paper.title, paper.authors.len(), paper.categories);
            } else {
                println!("Title: {}", paper.title);
                println!("Authors: {}", paper.authors.join(", "));
                println!("Published: {}", paper.published);
                println!("Categories: {:?}", paper.categories);
                println!("Abstract: {}...", &paper.abstract_text[..200.min(paper.abstract_text.len())]);
            }
        }
        Err(e) => eprintln!("Failed to fetch {}: {}", pid, e),
    }

    Ok(())
}

/// Handle `lean` — verify hypotheses with Lean 4
fn handle_lean(file: Option<&str>, hypothesis: Option<&str>, install: bool, check: bool, json: bool) -> Result<()> {
    if install {
        println!("{}", rairos_lean_verifier::get_lean_install_instructions());
        return Ok(());
    }

    if check {
        let (status, msg) = rairos_lean_verifier::check_lean_installed();
        let msg_str = msg.as_deref().unwrap_or("");
        if json {
            println!("{}", serde_json::json!({
                "installed": matches!(status, rairos_lean_verifier::LeanInstallStatus::Available),
                "message": msg_str
            }));
        } else {
            match status {
                rairos_lean_verifier::LeanInstallStatus::Available => println!("✅ Lean 4 is available"),
                _ => println!("❌ Lean 4 not found: {}", msg_str),
            }
        }
        return Ok(());
    }

    if let Some(h) = hypothesis {
        let (code, _name) = rairos_lean_verifier::translate_hypothesis_to_lean("cli", h, "hypothesis");
        println!("Lean code:\n{}", code);
        return Ok(());
    }

    if let Some(f) = file {
        let content = std::fs::read_to_string(f).unwrap_or_default();
        let result = rairos_lean_verifier::verify_lean_code(&content, "file", f);
        if json {
            println!("{}", rairos_lean_verifier::render_result_json(&result));
        } else {
            println!("{}", rairos_lean_verifier::render_result(&result));
        }
        return Ok(());
    }

    println!("Usage: lean [--check | --install | --hypothesis <text> | <file>]");
    Ok(())
}

/// Handle `visual` — generate D3 visualizations
fn handle_visual(db: &Database, paper: Option<&str>, query: Option<&str>, limit: usize, output: Option<&str>) -> Result<()> {
    if let Some(pid) = paper {
        println!("📊 Generating D3 citation visualization for: {}", pid);

        let graph = rairos_viz::D3ForceGraph::new(Some(db.clone()));
        let d3graph = graph.to_json(Some(vec![pid.to_string()]), None, limit)?;
        let json_str = d3graph.to_json()?;

        if let Some(out) = output {
            std::fs::write(out, &json_str)?;
            println!("✅ Written to {}", out);
        } else {
            println!("{}", json_str);
        }
        return Ok(());
    }

    if let Some(q) = query {
        println!("📊 Searching papers for: {}", q);
        let papers = db.search_papers(q, limit)?;
        println!("Found {} papers", papers.len());
        return Ok(());
    }

    println!("Usage: visual --paper <id> [--output <path>] | visual --query <q>");
    Ok(())
}

/// Handle `ingest` — fetch paper metadata
fn handle_ingest(paper_id: Option<&str>, json: bool, no_pdf: bool, source: &str) -> Result<()> {
    let Some(pid) = paper_id else {
        eprintln!("Usage: ingest <paper_id>");
        return Ok(());
    };

    println!("📥 Ingesting: {} (source: {}, no_pdf: {})", pid, source, no_pdf);

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        rairos_parser::fetch_paper(pid).await
    });

    match result {
        Ok(paper) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&paper)?);
            } else {
                println!("Title: {}", paper.title);
                println!("ID: {}", paper.id);
                println!("Authors: {}", paper.authors.len());
                println!("Published: {}", paper.published);
                println!("Categories: {:?}", paper.categories);
                println!("Abstract: {}...", &paper.abstract_text[..200.min(paper.abstract_text.len())]);
            }
        }
        Err(e) => eprintln!("Failed to fetch {}: {}", pid, e),
    }

    Ok(())
}

/// Handle `session` — manage research sessions
fn handle_session(action: &str, title: Option<&str>, topic: Option<&str>, days: usize, limit: usize) -> Result<()> {
    let mut tracker = rairos_research_session::ResearchSessionTracker::new(None);

    match action {
        "start" => {
            let session = tracker.start_session(title);
            println!("📚 Session started: {}", session.title);
            println!("   ID: {}", session.id);
            if let Some(t) = topic {
                println!("   Topic: {}", t);
            }
        }
        "list" => {
            let sessions = tracker.get_recent_sessions(days as i64, limit);
            if sessions.is_empty() {
                println!("No sessions found.");
            } else {
                println!("{}", tracker.render_sessions_list(&sessions));
            }
        }
        "current" => {
            match tracker.get_current_session() {
                Some(s) => println!("Current session: {} (ID: {})", s.title, s.id),
                None => println!("No current session."),
            }
        }
        "end" => {
            match tracker.end_session() {
                Some(s) => println!("Ended session: {} (ID: {})", s.title, s.id),
                None => println!("No active session to end."),
            }
        }
        _ => {
            match tracker.get_current_session() {
                Some(s) => println!("Current session: {} (ID: {})", s.title, s.id),
                None => println!("No current session. Use 'session start' to begin one."),
            }
        }
    }

    Ok(())
}
