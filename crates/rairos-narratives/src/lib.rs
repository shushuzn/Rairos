//! rairos-narratives — Research narrative tracker.
//!
//! Ported from `llm/research/research_narrative_tracker.py` + `cli/cmd/narrative.py`.
//! JSON file format is backward-compatible with the Python version:
//! `~/.ai_research_os/narrative/threads.json`
//!
//! This crate provides:
//! - Data types: `ResearchThread`, `NarrativePhase`
//! - Persistence: `ResearchThreadTracker` (CRUD + atomic JSON write)
//! - Scoring: phase computation + publication readiness (3-axis)
//! - Next-step recommendations
//! - Rendering: single-thread view + dashboard table
//! - Aggregator: read from tracker JSON files to auto-populate threads

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

/// Phase of a research narrative thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativePhase {
    Exploration,
    Hypothesis,
    Validation,
    Publication,
}

impl NarrativePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            NarrativePhase::Exploration => "exploration",
            NarrativePhase::Hypothesis => "hypothesis",
            NarrativePhase::Validation => "validation",
            NarrativePhase::Publication => "publication",
        }
    }
}

/// A research topic trajectory — unified view across all trackers.
///
/// Fields match Python's `ResearchThread` dataclass exactly for JSON compat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchThread {
    pub id: String,
    pub topic: String,
    pub phase: NarrativePhase,
    #[serde(default)]
    pub phase_updated_at: String,

    // Aggregated IDs from existing trackers
    #[serde(default)]
    pub paper_ids: Vec<String>,
    #[serde(default)]
    pub question_ids: Vec<String>,
    #[serde(default)]
    pub hypothesis_ids: Vec<String>,
    #[serde(default)]
    pub experiment_ids: Vec<String>,
    #[serde(default)]
    pub insight_card_ids: Vec<String>,

    // Counts
    #[serde(default)]
    pub gap_count: u32,
    #[serde(default)]
    pub hypothesis_count: u32,
    #[serde(default)]
    pub validated_count: u32,
    #[serde(default)]
    pub rejected_count: u32,
    #[serde(default)]
    pub running_count: u32,

    // Computed readiness scores (0.0–1.0)
    #[serde(default)]
    pub contribution_score: f64,
    #[serde(default)]
    pub experiment_score: f64,
    #[serde(default)]
    pub narrative_score: f64,

    // User-facing narrative
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub created_at: String,
    pub updated_at: String,
}

impl ResearchThread {
    /// Create a new thread with auto-generated timestamps and ID.
    pub fn new(topic: &str) -> Self {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        ResearchThread {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            topic: topic.to_string(),
            phase: NarrativePhase::Exploration,
            phase_updated_at: now.clone(),
            paper_ids: Vec::new(),
            question_ids: Vec::new(),
            hypothesis_ids: Vec::new(),
            experiment_ids: Vec::new(),
            insight_card_ids: Vec::new(),
            gap_count: 0,
            hypothesis_count: 0,
            validated_count: 0,
            rejected_count: 0,
            running_count: 0,
            contribution_score: 0.0,
            experiment_score: 0.0,
            narrative_score: 0.0,
            notes: String::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// A recommended next step.
#[derive(Debug, Clone, Serialize)]
pub struct NextStep {
    pub action: String,
    pub reason: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Tracker (CRUD + persistence)
// ═══════════════════════════════════════════════════════════════════════════

/// Persistent research thread tracker.
///
/// In-memory: `HashMap` for O(1) lookup.
/// On disk: JSON array for Python file-format compatibility.
pub struct ResearchThreadTracker {
    file_path: PathBuf,
    threads: HashMap<String, ResearchThread>,
}

impl ResearchThreadTracker {
    /// Create or load from `~/.ai_research_os/narrative/threads.json`.
    pub fn new() -> Result<Self> {
        let data_dir = Self::default_data_dir();
        fs::create_dir_all(&data_dir)?;
        Self::open(data_dir.join("threads.json"))
    }

    /// Open from a specific file path (useful for testing).
    pub fn open(file_path: PathBuf) -> Result<Self> {
        let threads = if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            if content.trim().is_empty() {
                HashMap::new()
            } else {
                let list: Vec<ResearchThread> = serde_json::from_str(&content).context(
                    format!("Failed to parse threads.json at {}", file_path.display()),
                )?;
                list.into_iter().map(|t| (t.id.clone(), t)).collect()
            }
        } else {
            HashMap::new()
        };
        Ok(Self { file_path, threads })
    }

    fn default_data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("narrative")
    }

    // ── CRUD ────────────────────────────────────────────────────────────

    /// List all threads.
    pub fn list_threads(&self) -> Vec<&ResearchThread> {
        let mut list: Vec<&ResearchThread> = self.threads.values().collect();
        list.sort_by(|a, b| a.updated_at.cmp(&b.updated_at).reverse());
        list
    }

    /// Get a thread by ID.
    pub fn get_thread(&self, id: &str) -> Option<&ResearchThread> {
        self.threads.get(id)
    }

    /// Get a thread by topic (case-insensitive).
    pub fn get_by_topic(&self, topic: &str) -> Option<&ResearchThread> {
        let topic_lower = topic.to_lowercase();
        self.threads
            .values()
            .find(|t| t.topic.to_lowercase() == topic_lower)
    }

    /// Insert or update a thread. Sets `updated_at`; preserves `created_at`.
    pub fn upsert(&mut self, thread: &mut ResearchThread) {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        if let Some(existing) = self.threads.get(&thread.id) {
            thread.created_at = existing.created_at.clone();
        } else if thread.created_at.is_empty() {
            thread.created_at = now.clone();
        }

        thread.updated_at = now.clone();
        self.threads
            .insert(thread.id.clone(), thread.clone());
    }

    /// Delete a thread by ID. Returns `true` if it existed.
    pub fn delete(&mut self, id: &str) -> bool {
        self.threads.remove(id).is_some()
    }

    // ── Persistence ─────────────────────────────────────────────────────

    /// Atomic write to JSON file (Python-compatible array format).
    pub fn save(&self) -> Result<()> {
        let mut list: Vec<&ResearchThread> = self.threads.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        let json = serde_json::to_string_pretty(&list)?;

        let tmp_path = self.file_path.with_extension("tmp");
        let mut tmp = fs::File::create(&tmp_path)?;
        tmp.write_all(json.as_bytes())?;
        tmp.sync_all()?;
        fs::rename(&tmp_path, &self.file_path)?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase & readiness computation (stateless pure functions)
// ═══════════════════════════════════════════════════════════════════════════

/// Determine the current narrative phase based on the thread's collected data.
///
/// Matches Python's `ResearchNarrativeService._compute_phase()`.
pub fn compute_phase(thread: &ResearchThread) -> NarrativePhase {
    if thread.experiment_score >= 0.8 && thread.contribution_score >= 0.7 {
        return NarrativePhase::Publication;
    }
    if !thread.hypothesis_ids.is_empty() {
        return NarrativePhase::Validation;
    }
    if !thread.question_ids.is_empty() {
        return NarrativePhase::Hypothesis;
    }
    NarrativePhase::Exploration
}

/// Compute the three publication readiness scores (0.0–1.0).
///
/// Matches Python's `ResearchNarrativeService._compute_readiness()`.
pub fn compute_readiness(thread: &ResearchThread) -> (f64, f64, f64) {
    let contrib = contribution_score(thread);
    let exp = experiment_score(thread);
    let narr = narrative_score(thread, contrib, exp);
    (contrib, exp, narr)
}

fn contribution_score(thread: &ResearchThread) -> f64 {
    let mut score: f64 = 0.0;
    if thread.gap_count > 0 && thread.hypothesis_count > 0 {
        score += 0.3;
    }
    if thread.hypothesis_count > 1 {
        score += 0.2;
    }
    if !thread.insight_card_ids.is_empty() {
        score += 0.2;
    }
    if thread.paper_ids.len() >= 3 {
        score += 0.2;
    }
    if !thread.question_ids.is_empty() {
        score += 0.1;
    }
    score.min(1.0)
}

fn experiment_score(thread: &ResearchThread) -> f64 {
    let total = thread.validated_count + thread.rejected_count + thread.running_count;
    if total == 0 {
        return 0.0;
    }
    let mut score: f64 = 0.0;
    if thread.validated_count >= 3 {
        score += 0.4;
    } else if thread.validated_count >= 1 {
        score += 0.2;
    }
    if thread.validated_count >= 1 && thread.rejected_count >= 1 {
        score += 0.2;
    }
    if thread.running_count >= 1 {
        score += 0.1;
    }
    if thread.hypothesis_count >= 2 {
        score += 0.1;
    }
    if !thread.paper_ids.is_empty() {
        score += 0.2;
    }
    score.min(1.0)
}

fn narrative_score(thread: &ResearchThread, contrib: f64, _exp: f64) -> f64 {
    let mut score: f64 = 0.5;
    if !thread.question_ids.is_empty() && !thread.hypothesis_ids.is_empty() {
        score += 0.15;
    }
    if !thread.hypothesis_ids.is_empty() && !thread.experiment_ids.is_empty() {
        score += 0.15;
    }
    if thread.validated_count >= 1 && contrib >= 0.4 {
        score += 0.1;
    }
    if thread.insight_card_ids.len() >= 3 {
        score += 0.1;
    }
    score.min(1.0)
}

// ═══════════════════════════════════════════════════════════════════════════
// Next-step recommendations
// ═══════════════════════════════════════════════════════════════════════════

/// Generate concrete next-step recommendations based on current state.
///
/// Matches Python's `ResearchNarrativeService.generate_next_steps()`.
pub fn generate_next_steps(thread: &ResearchThread) -> Vec<NextStep> {
    let mut steps = Vec::new();

    match thread.phase {
        NarrativePhase::Exploration => {
            if thread.gap_count == 0 {
                steps.push(NextStep {
                    action: format!("Run gap analysis on '{}'", thread.topic),
                    reason: "No gaps identified yet — start with landscape analysis".into(),
                });
            } else if thread.question_ids.is_empty() {
                steps.push(NextStep {
                    action: "Generate research questions from gaps".into(),
                    reason: format!(
                        "{} gaps found — convert to actionable questions",
                        thread.gap_count
                    ),
                });
            } else {
                steps.push(NextStep {
                    action: "Generate hypotheses from questions".into(),
                    reason: format!(
                        "{} questions ready — move to hypothesis phase",
                        thread.question_ids.len()
                    ),
                });
            }
        }
        NarrativePhase::Hypothesis => {
            steps.push(NextStep {
                action: format!("Run hypothesize '{}' with gap context", thread.topic),
                reason: format!(
                    "{} hypotheses generated — design experiments",
                    thread.hypothesis_count
                ),
            });
            if thread.insight_card_ids.is_empty() {
                steps.push(NextStep {
                    action: "Extract insight cards from related papers".into(),
                    reason: "No insight cards linked — build supporting evidence".into(),
                });
            }
        }
        NarrativePhase::Validation => {
            if thread.validated_count == 0 {
                steps.push(NextStep {
                    action: format!(
                        "Design first experiment for hypothesis {}",
                        thread.hypothesis_ids.first().map(|s| s.as_str()).unwrap_or("")
                    ),
                    reason: "Hypotheses exist but no experiments completed yet".into(),
                });
            }
            if thread.running_count == 0 {
                steps.push(NextStep {
                    action: "Run experiment —hypothesis-id <id>".into(),
                    reason: "No experiments in progress — start validation".into(),
                });
            }
            if thread.experiment_score < 0.5 {
                steps.push(NextStep {
                    action: "Expand benchmark coverage (need ≥3 benchmarks)".into(),
                    reason:                    format!(
                        "Experiment score {:.0} — add more benchmarks",
                        thread.experiment_score * 100.0
                    ),
                });
            }
            if thread.contribution_score < 0.5 {
                steps.push(NextStep {
                    action: "Collect more insight cards to strengthen narrative".into(),
                    reason: format!(
                        "Contribution score {:.0} — build supporting evidence",
                        thread.contribution_score * 100.0
                    ),
                });
            }
        }
        NarrativePhase::Publication => {
            steps.push(NextStep {
                action: format!("Draft paper structure using story '{}'", thread.topic),
                reason: "Research is publication-ready — start writing".into(),
            });
            if thread.narrative_score < 0.8 {
                steps.push(NextStep {
                    action: "Strengthen narrative coherence (add more insight cards)".into(),
                    reason: format!(
                        "Narrative score {:.0} — polish story",
                        thread.narrative_score * 100.0
                    ),
                });
            }
        }
    }

    steps
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderers
// ═══════════════════════════════════════════════════════════════════════════

fn phase_icon(phase: &NarrativePhase) -> &'static str {
    match phase {
        NarrativePhase::Exploration => "🔍 EXPLORATION",
        NarrativePhase::Hypothesis => "💡 HYPOTHESIS",
        NarrativePhase::Validation => "🔬 VALIDATION",
        NarrativePhase::Publication => "📄 PUBLICATION",
    }
}

fn score_bar(score: f64, width: usize) -> String {
    let filled = (score * width as f64).round() as usize;
    "█".repeat(filled) + &"░".repeat(width.saturating_sub(filled))
}

/// Render a single thread in detail. Matches Python's `render_thread()`.
pub fn render_thread(thread: &ResearchThread) -> String {
    let phase_updated_at = if thread.phase_updated_at.len() >= 10 {
        &thread.phase_updated_at[..10]
    } else {
        &thread.phase_updated_at
    };
    let created_at = if thread.created_at.len() >= 10 {
        &thread.created_at[..10]
    } else {
        &thread.created_at
    };

    let mut lines = vec![
        "═".repeat(60),
        format!("📊 Research Narrative: {}", thread.topic),
        "═".repeat(60),
        format!(
            "Phase: {}   Updated: {}",
            phase_icon(&thread.phase),
            phase_updated_at
        ),
        String::new(),
    ];

    // Gap summary
    if thread.gap_count > 0 {
        lines.push(format!("GAP ANALYSIS ({} gaps identified)", thread.gap_count));
        lines.push(format!(
            "  Insight cards: {} linked",
            thread.insight_card_ids.len()
        ));
    } else {
        lines.push("GAP ANALYSIS — No gaps identified yet. Run gap <topic> first.".into());
    }

    // Question summary
    if !thread.question_ids.is_empty() {
        lines.push(format!("QUESTIONS: {} tracked", thread.question_ids.len()));
    } else {
        lines.push("QUESTIONS: None tracked yet".into());
    }
    lines.push(String::new());

    // Hypothesis status
    if !thread.hypothesis_ids.is_empty() {
        lines.push(format!(
            "HYPOTHESIS STATUS ({} generated)",
            thread.hypothesis_count
        ));
        for hid in thread.hypothesis_ids.iter().take(5) {
            lines.push(format!("  • [{}]", hid));
        }
        if thread.hypothesis_ids.len() > 5 {
            lines.push(format!(
                "  ... +{} more",
                thread.hypothesis_ids.len() - 5
            ));
        }
    } else {
        lines.push("HYPOTHESIS STATUS — None generated yet".into());
    }
    lines.push(String::new());

    // Experiment summary
    if !thread.experiment_ids.is_empty() {
        lines.push(format!("EXPERIMENTS: {} total", thread.experiment_ids.len()));
        lines.push(format!(
            "  ✅ {} validated  ❌ {} rejected  ⚡ {} running",
            thread.validated_count, thread.rejected_count, thread.running_count
        ));
    } else {
        lines.push("EXPERIMENTS: None yet".into());
    }
    lines.push(String::new());

    // Publication readiness
    lines.push("PUBLICATION READINESS".into());
    lines.push(format!(
        "├─ Theoretical contribution  {}  {:.0}%",
        score_bar(thread.contribution_score, 8),
        thread.contribution_score * 100.0
    ));
    lines.push(format!(
        "├─ Experimental support      {}  {:.0}%",
        score_bar(thread.experiment_score, 8),
        thread.experiment_score * 100.0
    ));
    lines.push(format!(
        "└─ Narrative coherence       {}  {:.0}%",
        score_bar(thread.narrative_score, 8),
        thread.narrative_score * 100.0
    ));

    // Next steps
    let steps = generate_next_steps(thread);
    if !steps.is_empty() {
        lines.push(String::new());
        lines.push("NEXT RECOMMENDED STEPS".into());
        for (i, s) in steps.iter().enumerate().take(5) {
            lines.push(format!("  {}. {}", i + 1, s.action));
            lines.push(format!("     理由: {}", s.reason));
        }
    }

    // User notes
    if !thread.notes.is_empty() {
        lines.push(String::new());
        lines.push("NARRATIVE NOTES".into());
        lines.push(format!("  {}", thread.notes));
    }

    lines.push(String::new());
    lines.push("═".repeat(60));
    lines.push(format!(
        "Thread ID: {}  |  Created: {}",
        thread.id, created_at
    ));

    lines.join("\n")
}

/// Render a table overview of all threads. Matches Python's `render_dashboard()`.
pub fn render_dashboard(threads: &[&ResearchThread]) -> String {
    if threads.is_empty() {
        return "No research threads yet. Run `narrative track <topic>` to start.".into();
    }

    let phase_col = |p: &NarrativePhase| -> &'static str {
        match p {
            NarrativePhase::Exploration => "🔍 EXPL",
            NarrativePhase::Hypothesis => "💡 HYP",
            NarrativePhase::Validation => "🔬 VAL",
            NarrativePhase::Publication => "📄 PUB",
        }
    };

    let mut lines = vec![
        "═".repeat(70),
        "📊 Research Narrative Dashboard".into(),
        "═".repeat(70),
        format!(
            "  {:22} {:10} {:>4} {:>4} {:>4} {:>4} {:>7} {:>8} {:>5}",
            "Topic", "Phase", "Gaps", "Hyps", "Exp", "Val", "Contrib", "ExpScore", "Narr"
        ),
        format!(
            "  {:22} {:10} {:>4} {:>4} {:>4} {:>4} {:>7} {:>8} {:>5}",
            "-".repeat(22),
            "-".repeat(10),
            "-".repeat(4),
            "-".repeat(4),
            "-".repeat(4),
            "-".repeat(4),
            "-".repeat(7),
            "-".repeat(8),
            "-".repeat(5)
        ),
    ];

    let mut sorted: Vec<&&ResearchThread> = threads.iter().collect();
    sorted.sort_by(|a, b| a.phase.as_str().cmp(b.phase.as_str()));

    for t in sorted {
        let topic_short = if t.topic.len() > 22 {
            format!("{}...", &t.topic[..19])
        } else {
            format!("{:22}", t.topic)
        };
        lines.push(format!(
            "  {} {:10} {:>4} {:>4} {:>4} {:>4} {:>7.0}% {:>8.0}% {:>5.0}%",
            topic_short,
            phase_col(&t.phase),
            t.gap_count,
            t.hypothesis_count,
            t.experiment_ids.len(),
            t.validated_count,
            t.contribution_score * 100.0,
            t.experiment_score * 100.0,
            t.narrative_score * 100.0,
        ));
    }

    lines.push("═".repeat(70));
    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════════
// Aggregator — read from tracker JSON files to populate thread data
// ═══════════════════════════════════════════════════════════════════════════

/// Aggregate data from all available tracker JSON files for a topic.
///
/// Reads:
/// - `~/.ai_research_os/questions/questions.json`
/// - `~/.ai_research_os/evolution/events.json`
/// - `~/.ai_research_os/experiments/experiments.json`
/// - `~/.ai_research_os/insights/cards.json`
///
/// Missing files are silently skipped (no data contributed).
/// After aggregation, phase and readiness scores are recomputed.
pub fn aggregate_by_topic(topic: &str) -> Result<ResearchThread> {
    let mut thread = ResearchThread::new(topic);
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let base = home.join(".ai_research_os");

    // 1. Questions: ~/.ai_research_os/questions/questions.json
    let q_path = base.join("questions").join("questions.json");
    if let Ok(content) = fs::read_to_string(&q_path) {
        if !content.trim().is_empty() {
            if let Ok(questions) =
                serde_json::from_str::<Vec<serde_json::Value>>(&content)
            {
                let matching: Vec<_> = questions
                    .iter()
                    .filter(|q| {
                        q["topic"]
                            .as_str()
                            .is_some_and(|t| {
                                t.to_lowercase().contains(&topic.to_lowercase())
                            })
                    })
                    .collect();
                thread.question_ids = matching
                    .iter()
                    .filter_map(|q| q["id"].as_str().map(String::from))
                    .collect();
                thread.gap_count = matching.len() as u32;
            }
        }
    }

    // 2. Evolution events: ~/.ai_research_os/evolution/events.json
    let e_path = base.join("evolution").join("events.json");
    if let Ok(content) = fs::read_to_string(&e_path) {
        if !content.trim().is_empty() {
            if let Ok(events) =
                serde_json::from_str::<Vec<serde_json::Value>>(&content)
            {
                let topic_lower = topic.to_lowercase();
                let mut hids: Vec<String> = Vec::new();
                for ev in &events {
                    let topic_match = ev["topic"].as_str().is_some_and(|t| {
                        t.to_lowercase().contains(&topic_lower)
                    });
                    let gap_match = ev["gap_title"].as_str().is_some_and(|g| {
                        g.to_lowercase().contains(&topic_lower)
                    });
                    if topic_match || gap_match {
                        if let Some(hid) = ev["hypothesis_id"].as_str() {
                            if !hid.is_empty() && !hids.iter().any(|h| h == hid) {
                                hids.push(hid.to_string());
                            }
                        }
                    }
                }
                thread.hypothesis_ids = hids;
                thread.hypothesis_count = thread.hypothesis_ids.len() as u32;
            }
        }
    }

    // 3. Experiments: ~/.ai_research_os/experiments/experiments.json
    let x_path = base.join("experiments").join("experiments.json");
    if let Ok(content) = fs::read_to_string(&x_path) {
        if !content.trim().is_empty() {
            if let Ok(experiments) =
                serde_json::from_str::<Vec<serde_json::Value>>(&content)
            {
                let linked: Vec<_> = experiments
                    .iter()
                    .filter(|e| {
                        thread.hypothesis_ids.contains(
                            &e["hypothesis_id"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                        )
                    })
                    .collect();
                thread.experiment_ids = linked
                    .iter()
                    .filter_map(|e| e["id"].as_str().map(String::from))
                    .collect();
                thread.validated_count = linked
                    .iter()
                    .filter(|e| {
                        matches!(
                            e["status"].as_str(),
                            Some("completed" | "validated")
                        )
                    })
                    .count() as u32;
                thread.rejected_count = linked
                    .iter()
                    .filter(|e| matches!(e["status"].as_str(), Some("failed" | "rejected")))
                    .count() as u32;
                thread.running_count = linked
                    .iter()
                    .filter(|e| matches!(e["status"].as_str(), Some("running")))
                    .count() as u32;
            }
        }
    }

    // 4. Insight cards: ~/.ai_research_os/insights/cards.json
    let c_path = base.join("insights").join("cards.json");
    if let Ok(content) = fs::read_to_string(&c_path) {
        if !content.trim().is_empty() {
            if let Ok(cards) =
                serde_json::from_str::<Vec<serde_json::Value>>(&content)
            {
                let matching: Vec<_> = cards
                    .iter()
                    .filter(|c| {
                        let title = c["title"].as_str().unwrap_or("");
                        let desc = c["description"].as_str().unwrap_or("");
                        let content_txt = c["content"].as_str().unwrap_or("");
                        let haystack =
                            format!("{} {} {}", title, desc, content_txt)
                                .to_lowercase();
                        haystack.contains(&topic.to_lowercase())
                    })
                    .collect();
                thread.insight_card_ids = matching
                    .iter()
                    .filter_map(|c| {
                        c["card_id"]
                            .as_str()
                            .or_else(|| c["id"].as_str())
                            .map(String::from)
                    })
                    .collect();
            }
        }
    }

    // Recompute phase & scores
    thread.phase = compute_phase(&thread);
    let (c, e, n) = compute_readiness(&thread);
    thread.contribution_score = c;
    thread.experiment_score = e;
    thread.narrative_score = n;

    Ok(thread)
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_tracker() -> (ResearchThreadTracker, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("threads.json");
        let tracker = ResearchThreadTracker::open(file_path).unwrap();
        (tracker, dir)
    }

    #[test]
    fn test_create_thread() {
        let t = ResearchThread::new("LLM Reasoning");
        assert_eq!(t.id.len(), 8);
        assert_eq!(t.topic, "LLM Reasoning");
        assert_eq!(t.phase, NarrativePhase::Exploration);
        assert!(!t.created_at.is_empty());
        assert!(!t.updated_at.is_empty());
    }

    #[test]
    fn test_upsert_and_get() {
        let (mut tracker, _dir) = setup_tracker();
        let mut t = ResearchThread::new("Transformers");
        let id = t.id.clone();
        tracker.upsert(&mut t);
        assert!(tracker.get_thread(&id).is_some());
        assert_eq!(tracker.get_thread(&id).unwrap().topic, "Transformers");
    }

    #[test]
    fn test_get_by_topic() {
        let (mut tracker, _dir) = setup_tracker();
        let mut t = ResearchThread::new("Attention");
        tracker.upsert(&mut t);
        assert!(tracker.get_by_topic("attention").is_some());
        assert!(tracker.get_by_topic("Attention").is_some());
        assert!(tracker.get_by_topic("nonexistent").is_none());
    }

    #[test]
    fn test_list_threads() {
        let (mut tracker, _dir) = setup_tracker();
        let mut t1 = ResearchThread::new("A");
        let mut t2 = ResearchThread::new("B");
        tracker.upsert(&mut t1);
        tracker.upsert(&mut t2);
        assert_eq!(tracker.list_threads().len(), 2);
    }

    #[test]
    fn test_delete() {
        let (mut tracker, _dir) = setup_tracker();
        let mut t = ResearchThread::new("Delete me");
        let id = t.id.clone();
        tracker.upsert(&mut t);
        assert!(tracker.delete(&id));
        assert!(!tracker.delete(&id));
    }

    #[test]
    fn test_save_and_reload_python_compat() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("threads.json");

        // Write Python-format JSON array
        let python_json = serde_json::json!([
            {
                "id": "narr001",
                "topic": "Python Compat",
                "phase": "exploration",
                "phase_updated_at": "2026-01-01T12:00:00",
                "paper_ids": ["p1"],
                "question_ids": [],
                "hypothesis_ids": [],
                "experiment_ids": [],
                "insight_card_ids": [],
                "gap_count": 3,
                "hypothesis_count": 0,
                "validated_count": 0,
                "rejected_count": 0,
                "running_count": 0,
                "contribution_score": 0.0,
                "experiment_score": 0.0,
                "narrative_score": 0.0,
                "notes": "",
                "created_at": "2026-01-01T12:00:00",
                "updated_at": "2026-01-01T12:00:00"
            }
        ]);
        fs::write(&file_path, serde_json::to_string_pretty(&python_json).unwrap()).unwrap();

        let tracker = ResearchThreadTracker::open(file_path).unwrap();
        assert_eq!(tracker.threads.len(), 1);
        let t = tracker.threads.get("narr001").unwrap();
        assert_eq!(t.topic, "Python Compat");
        assert_eq!(t.phase, NarrativePhase::Exploration);
        assert_eq!(t.gap_count, 3);
    }

    #[test]
    fn test_roundtrip_save() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("threads.json");

        let mut tracker = ResearchThreadTracker::open(file_path.clone()).unwrap();
        let mut t = ResearchThread::new("Roundtrip");
        tracker.upsert(&mut t);
        tracker.save().unwrap();

        let loaded = ResearchThreadTracker::open(file_path).unwrap();
        assert_eq!(loaded.list_threads().len(), 1);
    }

    // ── Phase computation ──────────────────────────────────────────────

    #[test]
    fn test_compute_phase_exploration() {
        let t = ResearchThread::new("New topic");
        assert_eq!(compute_phase(&t), NarrativePhase::Exploration);
    }

    #[test]
    fn test_compute_phase_hypothesis() {
        let mut t = ResearchThread::new("Has questions");
        t.question_ids.push("q1".into());
        assert_eq!(compute_phase(&t), NarrativePhase::Hypothesis);
    }

    #[test]
    fn test_compute_phase_validation() {
        let mut t = ResearchThread::new("Has hypotheses");
        t.hypothesis_ids.push("h1".into());
        assert_eq!(compute_phase(&t), NarrativePhase::Validation);
    }

    #[test]
    fn test_compute_phase_publication() {
        let mut t = ResearchThread::new("Publish ready");
        t.experiment_score = 0.85;
        t.contribution_score = 0.75;
        assert_eq!(compute_phase(&t), NarrativePhase::Publication);
    }

    // ── Readiness scoring ───────────────────────────────────────────────

    #[test]
    fn test_contribution_score_basic() {
        let t = ResearchThread::new("Empty");
        let (c, _e, _n) = compute_readiness(&t);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn test_contribution_score_with_data() {
        let mut t = ResearchThread::new("Loaded");
        t.gap_count = 2;
        t.hypothesis_count = 2;
        t.insight_card_ids.push("ic1".into());
        t.paper_ids = vec!["p1".into(), "p2".into(), "p3".into()];
        t.question_ids.push("q1".into());
        let (c, _e, _n) = compute_readiness(&t);
        assert!(c > 0.5);
        assert!(c <= 1.0);
    }

    #[test]
    fn test_experiment_score_empty() {
        let t = ResearchThread::new("Empty");
        let (_, e, _) = compute_readiness(&t);
        assert_eq!(e, 0.0);
    }

    #[test]
    fn test_experiment_score_validated() {
        let mut t = ResearchThread::new("Tested");
        t.validated_count = 3;
        t.hypothesis_count = 2;
        t.paper_ids.push("p1".into());
        let (_, e, _) = compute_readiness(&t);
        assert_eq!(e, 0.7);
    }

    #[test]
    fn test_narrative_score() {
        let mut t = ResearchThread::new("Narrative");
        t.gap_count = 2;
        t.hypothesis_count = 2;
        t.question_ids.push("q1".into());
        t.hypothesis_ids.push("h1".into());
        t.experiment_ids.push("e1".into());
        t.validated_count = 1;
        t.insight_card_ids = vec!["i1".into(), "i2".into(), "i3".into()];
        t.paper_ids = vec!["p1".into(), "p2".into(), "p3".into()];
        let (c, _e, n) = compute_readiness(&t);
        // contrib: gap+hypo(0.3) + hyp>1(0.2) + insight(0.2) + papers>=3(0.2) + questions(0.1) = 1.0
        assert!((c - 1.0).abs() < 0.01);
        // narrative: baseline 0.5 + q+h 0.15 + h+e 0.15 + validated+contrib>=0.4 0.1 + insight>=3 0.1 = 1.0
        assert!((n - 1.0).abs() < 0.01);
    }

    // ── Next steps ──────────────────────────────────────────────────────

    #[test]
    fn test_next_steps_exploration_no_gaps() {
        let t = ResearchThread::new("Fresh");
        let steps = generate_next_steps(&t);
        assert!(!steps.is_empty());
        assert!(steps[0].action.contains("gap analysis"));
    }

    #[test]
    fn test_next_steps_publication() {
        let mut t = ResearchThread::new("Publish");
        t.experiment_score = 0.9;
        t.contribution_score = 0.8;
        t.phase = compute_phase(&t);
        assert_eq!(t.phase, NarrativePhase::Publication);
        let steps = generate_next_steps(&t);
        assert!(!steps.is_empty());
        assert!(steps[0].action.contains("paper structure"));
    }
}
