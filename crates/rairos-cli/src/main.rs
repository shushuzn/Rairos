//! Rairos CLI — Rust command-line interface
//!
//! Architecture: commands defined via clap derive, handlers in separate module.

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

mod handlers;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rairos_core::{Database, ParseStatus};
use std::path::PathBuf;


// Re-export handler symbols for dispatch
use handlers::*;

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

    /// Run the setup wizard
    Setup {
        /// Show quick start guide only
        #[arg(short, long)]
        guide: bool,
    },

    /// Show research radar heat tracking
    Radar {
        /// Action: show, update
        #[arg(short, long, default_value = "show")]
        action: String,

        /// Tags for update (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Date note for update
        #[arg(short, long)]
        note_date: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show research timeline
    Timeline {
        /// Action: show, update
        #[arg(short, long, default_value = "show")]
        action: String,

        /// Year for timeline entry
        #[arg(short, long)]
        year: Option<String>,

        /// Paper note path for timeline entry
        #[arg(short, long)]
        pnote: Option<String>,

        /// Title for timeline entry
        #[arg(short, long)]
        title: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
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

    /// Full-screen TUI chat with paper context sidebar
    ChatTui,
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

// All handler implementations moved to handlers.rs
#[cfg(test)]
mod tests;

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
        Commands::Setup { guide } => {
            if *guide {
                let wizard = rairos_setup::SetupWizard::new();
                println!("{}", wizard.quick_start_guide());
            } else {
                let mut wizard = rairos_setup::SetupWizard::new();
                let results = wizard.run();
                println!("Setup complete: {}/{} steps done",
                    results.iter().filter(|(_, done)| *done).count(),
                    results.len()
                );
                for (name, done) in &results {
                    println!("  {}: {}", if *done { "✓" } else { "✗" }, name);
                }
            }
        }
        Commands::Radar { action, tags, note_date, format } => {
            let root = dirs::home_dir().map(|h| h.join(".ai_research_os")).unwrap_or_default();
            if action == "show" {
                match rairos_updaters::read_radar(&root) {
                    Ok(state) => println!("{:#?}", state),
                    Err(e) => eprintln!("Failed to read radar: {}", e),
                }
            } else if action == "update" {
                let tag_list: Vec<String> = tags.as_deref()
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let date = note_date.as_deref().unwrap_or("today");
                match rairos_updaters::update_radar(&root, &tag_list, date) {
                    Ok(_) => println!("Radar updated"),
                    Err(e) => eprintln!("Failed to update radar: {}", e),
                }
            }
        }
        Commands::Timeline { action, year, pnote, title, format } => {
            let root = dirs::home_dir().map(|h| h.join(".ai_research_os")).unwrap_or_default();
            if action == "show" {
                match rairos_updaters::read_timeline(&root) {
                    Ok(state) => {
                        let rendered = rairos_updaters::render_timeline(&state);
                        println!("{}", rendered);
                    }
                    Err(e) => eprintln!("Failed to read timeline: {}", e),
                }
            } else if action == "update" {
                let y = year.as_deref().unwrap_or("2026");
                let p = pnote.as_deref().unwrap_or("");
                let t = title.as_deref().unwrap_or("");
                match rairos_updaters::update_timeline(&root, y, p, t) {
                    Ok(_) => println!("Timeline updated"),
                    Err(e) => eprintln!("Failed to update timeline: {}", e),
                }
            }
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
        Commands::ChatTui => {
            handle_chat_tui()?;
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


