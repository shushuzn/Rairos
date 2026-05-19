//! CLI command definitions — enums for clap argument parsing.
//!
//! All command enums are defined here so main.rs stays focused on dispatch logic.

use clap::Subcommand;
use std::path::PathBuf;

// ============================================================================
// Action Enums (defined before Commands so Commands can reference them)
// ============================================================================

#[derive(Subcommand)]
pub enum RagAction {
    RunFull {
        arxiv_id: String,
        #[arg(short, long, default_value = "minimal")]
        mode: String,
        #[arg(short, long, default_value = "pytorch")]
        framework: String,
        #[arg(short, long)]
        task: Option<String>,
    },
    GenTests {
        arxiv_id: String,
    },
    InitBenchmark {
        csv_path: String,
        #[arg(short, long)]
        task: String,
    },
    RunEvoskill {
        #[arg(long)]
        continue_mode: bool,
    },
    ListSkills,
    Status,
}

#[derive(Subcommand)]
pub enum EvoSkillAction {
    Init {
        #[arg(short, long)]
        task: String,
        #[arg(short, long)]
        dataset: String,
        #[arg(short = 'H', long, default_value = "claude")]
        harness: String,
        #[arg(short, long, default_value = "sonnet")]
        model: String,
        #[arg(long, default_value = "question")]
        question_col: String,
        #[arg(long, default_value = "answer")]
        answer_col: String,
        #[arg(long)]
        category_col: Option<String>,
    },
    Run {
        #[arg(long)]
        continue_mode: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    Eval,
    Diff {
        from_iter: Option<i32>,
        to_iter: Option<i32>,
    },
    Reset,
    Status,
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    Snapshot {
        path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum Jin10Action {
    Quote {
        code: String,
    },
    Kline {
        code: String,
        #[arg(short, long, default_value = "1")]
        time: i32,
        #[arg(short = 'n', long, default_value = "10")]
        count: i32,
    },
    Flash {
        #[arg(long)]
        cursor: Option<String>,
    },
    SearchFlash {
        keyword: String,
    },
    News {
        #[arg(long)]
        cursor: Option<String>,
    },
    SearchNews {
        keyword: String,
        #[arg(long)]
        cursor: Option<String>,
    },
    NewsDetail {
        id: String,
    },
    Calendar,
    Symbols,
}

#[derive(Subcommand)]
pub enum CacheAction {
    Stats,
    Clear,
    ClearApi,
    ClearParsed,
    List {
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum DedupAction {
    Find {
        #[arg(short, long, default_value = "0.85")]
        threshold: f32,
    },
    Remove {
        #[arg(short, long)]
        papers: String,
    },
    Groups,
    Stats,
    Semantic {
        paper: String,
        #[arg(short, long, default_value = "0.85")]
        threshold: f32,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum QuestionAction {
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },
    Add {
        question: String,
        #[arg(short, long)]
        topic: Option<String>,
        #[arg(short, long, default_value = "5")]
        priority: u8,
        #[arg(short, long)]
        notes: Option<String>,
    },
    Get {
        id: String,
    },
    Update {
        id: String,
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        notes: Option<String>,
        #[arg(short, long)]
        priority: Option<u8>,
    },
    Link {
        id: String,
        paper_id: String,
    },
    Unlink {
        id: String,
        paper_id: String,
    },
    Delete {
        id: String,
    },
    Sync {
        #[arg(short, long)]
        topic: Option<String>,
        #[arg(short, long, default_value = "7")]
        priority: u8,
    },
    Stats,
}

#[derive(Subcommand)]
pub enum NarrativeAction {
    List,
    Show {
        #[arg(short, long)]
        id: String,
    },
    Track {
        topic: String,
    },
    Update {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        topic: Option<String>,
        #[arg(short, long)]
        notes: Option<String>,
    },
    Note {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        text: String,
    },
    Dashboard,
}

#[derive(Subcommand)]
pub enum InsightAction {
    Add {
        #[arg(long)]
        content: String,
        #[arg(short = 't', long, default_value = "finding")]
        r#type: String,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long)]
        paper: Option<String>,
        #[arg(short = 'c', long)]
        collection: Option<String>,
    },
    List {
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    Search {
        #[arg(short = 'q', long)]
        query: String,
        #[arg(short = 't', long)]
        r#type: Option<String>,
    },
    TagCloud,
    Rate {
        #[arg(long)]
        card: String,
        #[arg(long)]
        stars: i32,
    },
    Like {
        #[arg(long)]
        card: String,
    },
    Dislike {
        #[arg(long)]
        card: String,
    },
    Top {
        #[arg(long, default_value = "3")]
        min_rating: i32,
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
    Bottom {
        #[arg(long, default_value = "2")]
        max_rating: i32,
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
}

// ============================================================================
// Commands Enum
// ============================================================================

#[derive(Subcommand)]
pub enum Commands {
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

    /// Suggest code optimizations for a research gap
    GapSuggestCode {
        /// Gap ID to generate code optimizations for
        #[arg(short, long)]
        gap_id: String,

        /// Target crate to optimize
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Detect code optimization opportunities from papers
    Optimize {
        /// Research/technology topic to analyze
        #[arg(short, long)]
        topic: String,

        /// Target crate to optimize (e.g., rairos-core, rairos-llm)
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Maximum number of optimization suggestions
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format (table/json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// List code optimization genes
    CodeGeneList {
        /// Filter by target crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Maximum number to show
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Run evolution on code optimization genes
    CodeEvolve {
        /// Filter by target crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Maximum number of crossovers to suggest
        #[arg(short, long, default_value = "10")]
        max_crossovers: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show workflow statistics (gaps ↔ code genes)
    WorkflowStats,

    /// Show gap to code gene linkage details
    GapCodeLink {
        /// Gap ID to show links for
        #[arg(short, long)]
        gap_id: Option<String>,
    },

    /// Full pipeline: detect gap → generate code optimizations → evolve
    OptimizePipeline {
        /// Research topic
        #[arg(short, long)]
        topic: String,

        /// Target crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Number of optimizations to generate
        #[arg(short, long, default_value = "5")]
        optimizations: usize,

        /// Number of evolutions to run
        #[arg(short, long, default_value = "3")]
        evolutions: usize,
    },

    /// Record feedback for a code gene
    CodeGeneFeedback {
        /// Code gene ID
        #[arg(short, long)]
        id: String,

        /// Positive or negative feedback
        #[arg(short, long)]
        positive: bool,
    },

    /// Export code genes to file
    CodeGeneExport {
        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Filter by crate
        #[arg(short, long)]
        crate_name: Option<String>,
    },

    /// Clean low-quality code genes
    CodeGeneClean {
        /// Minimum score threshold (0.0-1.0)
        #[arg(short = 's', long, default_value = "0.3")]
        min_score: f64,

        /// Also remove genes with feedback_count < min_feedback
        #[arg(long, default_value = "0")]
        min_feedback: i32,

        /// Also remove genes with code_snippet.len() < min_code_length
        #[arg(long, default_value = "100")]
        min_code_length: usize,

        /// Actually delete (dry-run if not set)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Sync code genes to GitHub Issues
    CodeGeneSyncToIssue {
        /// Gene IDs to sync (comma-separated, or "all")
        #[arg(short, long, default_value = "all")]
        ids: String,

        /// Filter by crate
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Minimum score to include
        #[arg(short = 's', long, default_value = "0.0")]
        min_score: f64,
    },

    /// Sync code genes from GitHub Issues
    CodeGeneSyncFromIssue {
        /// Issue numbers to import (comma-separated, or "all")
        #[arg(short, long, default_value = "all")]
        issues: String,

        /// GitHub repo (owner/repo format)
        #[arg(short, long, default_value = "shushuzn/Rairos")]
        repo: String,
    },

    /// Add a code gene to the pool from code snippet
    CodeGeneAdd {
        /// Target crate (e.g., rairos-rankers-base)
        #[arg(short = 'c', long)]
        crate_name: String,

        /// Gap type (performance, memory, concurrency, architecture, evaluation)
        #[arg(short = 'g', long)]
        gap_type: String,

        /// Code snippet (multiline string)
        #[arg(long)]
        code: String,

        /// Optimization description
        #[arg(short = 'o', long)]
        optimization: String,

        /// Keywords for discovery (comma-separated)
        #[arg(short = 'k', long, default_value = "")]
        keywords: String,
    },

    /// Implement code gene from GitHub Issue with workflow
    /// Workflow: 1) Search existing code 2) Post plan to issue 3) Confirm 4) Implement
    CodeGeneImplement {
        /// Issue number to implement
        #[arg(short, long)]
        issue: usize,

        /// GitHub repo (owner/repo format)
        #[arg(short, long, default_value = "shushuzn/Rairos")]
        repo: String,

        /// Actually implement (dry-run if not set)
        #[arg(short, long)]
        execute: bool,
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

    /// Show achievements and gamification progress
    Achievements {
        /// Action: list, report, stats, or unlock
        #[arg(short, long, default_value = "list")]
        action: String,

        /// Achievement ID (for unlock action)
        achievement_id: Option<String>,
    },

    /// Show badges and research game mode progress
    Badges {
        /// Action: list or award
        #[arg(short, long, default_value = "list")]
        action: String,

        /// Badge ID (for award action)
        badge_id: Option<String>,
    },

    /// Detect contradictions in research gaps
    Contradictions {
        /// Action: list (show contradictions) or render (heatmap)
        #[arg(short, long, default_value = "list")]
        action: String,

        /// Max papers to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Analyze research trends for a topic
    Trends {
        /// Topic to analyze
        topic: String,

        /// Number of years to look back
        #[arg(short = 'y', long)]
        years: Option<i32>,

        /// Output format: text or mermaid
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Score research rigor of a paper
    Rigor {
        /// Paper ID or arXiv ID to score
        paper_id: String,
    },

    /// Score paper impact
    Impact {
        /// Action: leaderboard or score
        #[arg(short, long, default_value = "leaderboard")]
        action: String,

        /// Paper ID (for score action)
        paper_id: Option<String>,

        /// Max papers to show (for leaderboard action)
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Generate research briefing for a paper
    Briefing {
        /// arXiv ID of the paper
        arxiv_id: String,

        /// List existing briefings
        #[arg(short, long, default_value = "false")]
        list: bool,

        /// Max briefings to list
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Detect paradigm shifts in a research topic
    Paradigm {
        /// Research topic to analyze
        topic: String,

        /// List detected paradigm shifts
        #[arg(short, long, default_value = "false")]
        list: bool,

        /// Max shifts to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Analyze cross-references between papers
    Crossref {
        /// Paper ID to analyze
        paper_id: String,

        /// List cross-reference reports
        #[arg(short, long, default_value = "false")]
        list: bool,

        /// Max reports to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Analyze scoring momentum for research tags
    Momentum {
        /// Tag to score
        #[arg(short, long, default_value = "")]
        tag: String,

        /// Show leaderboard
        #[arg(short, long, default_value = "false")]
        leaderboard: bool,
    },

    /// Run genetic crossover on research capsules
    Crossover {
        /// List top crossover candidates
        #[arg(short, long, default_value = "false")]
        list: bool,
    },

    /// Analyze gene pool decay and resurrection
    Decay {
        /// Capsule ID to check (optional)
        #[arg(short, long, default_value = "")]
        capsule_id: String,

        /// Show decay statistics
        #[arg(short, long, default_value = "false")]
        stats: bool,
    },

    /// Scan for at-risk capsules needing attention
    AtRisk {
        /// Risk score threshold (default 50)
        #[arg(short = 't', long, default_value = "50")]
        threshold: u32,

        /// Capsule ID to keep active
        #[arg(short, long, default_value = "")]
        keep: String,
    },

    /// Score capsule credibility and detect trendslop
    Credibility {
        /// Show trend-slop capsules
        #[arg(short, long, default_value = "false")]
        trendslop: bool,
    },

    /// Analyze claim graphs and find contradictions
    ClaimGraph {
        /// Show claim graph statistics
        #[arg(short, long, default_value = "false")]
        stats: bool,

        /// Find contradictions in claims
        #[arg(short, long, default_value = "false")]
        contradictions: bool,
    },

    /// List high-risk/high-reward bold capsules
    Bold,

    /// Show performance profiler report
    Profiler {
        /// Show profiler statistics as JSON
        #[arg(short, long, default_value = "false")]
        stats: bool,
    },

    /// CodeGraph knowledge graph operations
    CodeGraph {
        /// Show codegraph statistics
        #[arg(short, long, default_value = "false")]
        stats: bool,

        /// List indexed files
        #[arg(short = 'f', long, default_value = "false")]
        files: bool,

        /// Search for a symbol by name
        #[arg(short = 's', long)]
        search: Option<String>,

        /// Show a specific node by ID
        #[arg(short = 'n', long)]
        node: Option<i64>,

        /// Show callers of a node
        #[arg(long)]
        callers: Option<i64>,

        /// Show callees of a node
        #[arg(long)]
        callees: Option<i64>,

        /// Traversal depth for callers/callees
        #[arg(long, default_value = "1")]
        depth: usize,
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

    // ── Utility commands ────────────────────────────────────────────

    /// Run LSP diagnostics (ruff/pyright) on a file or directory
    Diagnostics {
        /// Run ruff check
        #[arg(long)]
        ruff: bool,

        /// Run pyright check
        #[arg(long)]
        pyright: bool,

        /// Path to file or directory to check
        path: PathBuf,
    },

    /// Manage workspace snapshots
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Show system information (CPU, memory, disk)
    Sysinfo,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (bash, zsh, fish)
        #[arg(value_parser = ["bash", "zsh", "fish"])]
        shell: String,
    },
}
