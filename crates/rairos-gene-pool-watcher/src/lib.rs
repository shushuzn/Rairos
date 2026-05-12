//! Gene Pool Watcher — auto-discover and fill diversity gaps.
//!
//! Monitors underrepresented algorithm families in the Gene Pool and automatically
//! creates ArXiv subscriptions to fill those gaps. Closes the self-evolution loop:
//! Gene Pool gap → auto-subscribe → paper2code → Gene Pool encode → diversity re评估.
//!
//! Python original: `llm/gene_pool_watcher.py`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Gap subscription ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSubscription {
    pub family: String,
    pub keywords: Vec<String>,
    pub arxiv_category: String,
    pub enabled: bool,
    pub last_checked: String,
}

impl Default for GapSubscription {
    fn default() -> Self {
        Self {
            family: "other".to_string(),
            keywords: vec!["neural network".to_string()],
            arxiv_category: "cs.LG".to_string(),
            enabled: true,
            last_checked: String::new(),
        }
    }
}

// ─── Watcher state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherState {
    pub gap_subscriptions: Vec<GapSubscription>,
    pub last_diversity_check: String,
    pub underrepresented_families: Vec<String>,
    pub diversity_score: f64,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            gap_subscriptions: Vec::new(),
            last_diversity_check: String::new(),
            underrepresented_families: Vec::new(),
            diversity_score: 0.0,
        }
    }
}

// ─── Family → ArXiv config ───────────────────────────────────────────────────

pub const FAMILY_ARXIV_CONFIG: &[(&str, &[&str], &str)] = &[
    (
        "attention",
        &[
            "transformer",
            "self-attention",
            "multi-head attention",
            "vision transformer",
            "ViT",
        ],
        "cs.CL",
    ),
    (
        "reinforcement",
        &[
            "reinforcement learning",
            "policy gradient",
            "DQN",
            "PPO",
            "A3C",
            "reward",
        ],
        "cs.LG",
    ),
    (
        "language_model",
        &[
            "language model",
            "LLM",
            "GPT",
            "BERT",
            "decoder",
            "autoregressive",
            " Transformer",
        ],
        "cs.CL",
    ),
    (
        "vision",
        &[
            "CNN",
            "image classification",
            "object detection",
            "segmentation",
            "ViT",
            "vision transformer",
        ],
        "cs.CV",
    ),
    (
        "optimization",
        &[
            "optimizer",
            "Adam",
            "SGD",
            "gradient descent",
            "loss landscape",
            "training dynamics",
        ],
        "cs.LG",
    ),
    (
        "graph",
        &[
            "graph neural network",
            "GNN",
            "message passing",
            "node classification",
            "graph convolution",
        ],
        "cs.SD",
    ),
    (
        "reasoning",
        &[
            "chain-of-thought",
            "reasoning",
            "logical inference",
            "planning",
            "theorem proving",
        ],
        "cs.AI",
    ),
    (
        "embodied",
        &[
            "robotics",
            "embodied",
            "navigation",
            "control",
            "motor",
            "reinforcement learning robot",
        ],
        "cs.RO",
    ),
    (
        "other",
        &[
            "neural network",
            "deep learning",
            "training",
            "representation learning",
        ],
        "cs.LG",
    ),
];

// ─── GP_DIR ───────────────────────────────────────────────────────────────────

const GP_DIR_NAME: &str = ".ai_research_os/evolution";

fn get_gp_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|p| p.join(GP_DIR_NAME))
        .unwrap_or_else(|| std::path::PathBuf::from(GP_DIR_NAME))
}

// ─── State persistence ───────────────────────────────────────────────────────

fn gap_subscriptions_path() -> std::path::PathBuf {
    get_gp_dir().join("gap_subscriptions.json")
}

/// Load watcher state from disk, or return empty state.
pub fn load_watcher_state() -> WatcherState {
    let mut state = WatcherState::default();

    // Load diversity from gene-pool-io
    let diversity = rairos_gene_pool_io::get_gene_pool_diversity();
    state.underrepresented_families = diversity
        .get("underrepresented_families")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    state.diversity_score = diversity
        .get("diversity_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    state.last_diversity_check = now_iso();

    // Load gap subscriptions from file
    let path = gap_subscriptions_path();
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(subs) = data.get("gap_subscriptions").and_then(|v| v.as_array()) {
                    state.gap_subscriptions = subs
                        .iter()
                        .filter_map(|s| {
                            Some(GapSubscription {
                                family: s.get("family")?.as_str()?.to_string(),
                                keywords: s
                                    .get("keywords")?
                                    .as_array()?
                                    .iter()
                                    .filter_map(|k| k.as_str().map(String::from))
                                    .collect(),
                                arxiv_category: s
                                    .get("arxiv_category")?
                                    .as_str()?
                                    .to_string(),
                                enabled: s.get("enabled")?.as_bool().unwrap_or(true),
                                last_checked: s
                                    .get("last_checked")?
                                    .as_str()?
                                    .to_string(),
                            })
                        })
                        .collect();
                }
            }
        }
    }

    state
}

/// Persist watcher state to disk.
pub fn save_watcher_state(state: &WatcherState) -> std::io::Result<()> {
    let gp_dir = get_gp_dir();
    std::fs::create_dir_all(&gp_dir)?;

    let data = serde_json::json!({
        "gap_subscriptions": state
            .gap_subscriptions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "family": s.family,
                    "keywords": s.keywords,
                    "arxiv_category": s.arxiv_category,
                    "enabled": s.enabled,
                    "last_checked": s.last_checked,
                })
            })
            .collect::<Vec<_>>(),
        "last_diversity_check": state.last_diversity_check,
        "underrepresented_families": state.underrepresented_families,
        "diversity_score": state.diversity_score,
    });

    std::fs::write(gap_subscriptions_path(), serde_json::to_string_pretty(&data).unwrap())?;
    Ok(())
}

// ─── Gap subscription builders ───────────────────────────────────────────────

/// Build a GapSubscription for a given family from FAMILY_ARXIV_CONFIG.
pub fn build_gap_subscription(family: &str) -> GapSubscription {
    for (fam, keywords, category) in FAMILY_ARXIV_CONFIG {
        if *fam == family {
            return GapSubscription {
                family: fam.to_string(),
                keywords: keywords.iter().map(|s| s.to_string()).collect(),
                arxiv_category: category.to_string(),
                enabled: true,
                last_checked: now_iso(),
            };
        }
    }
    GapSubscription::default()
}

/// Build gap subscriptions for all underrepresented families.
pub fn build_gap_subscriptions_for_families(underrep: &[String]) -> Vec<GapSubscription> {
    underrep
        .iter()
        .map(|fam| build_gap_subscription(fam))
        .collect()
}

/// Diff existing vs new gap subscriptions.
/// Returns (to_add, families_to_remove).
pub fn diff_subscriptions(
    existing: &[GapSubscription],
    new: &[GapSubscription],
) -> (Vec<GapSubscription>, Vec<String>) {
    let existing_families: std::collections::HashSet<_> =
        existing.iter().map(|s| s.family.clone()).collect();
    let new_families: std::collections::HashSet<_> =
        new.iter().map(|s| s.family.clone()).collect();

    let to_add: Vec<GapSubscription> = new
        .iter()
        .filter(|s| !existing_families.contains(&s.family))
        .cloned()
        .collect();
    let to_remove: Vec<String> = existing_families
        .iter()
        .filter(|f| !new_families.contains(*f))
        .cloned()
        .collect();

    (to_add, to_remove)
}

// ─── Diversity pressure result ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityPressureResult {
    pub triggered: bool,
    pub pressure_level: f64,
    pub overrepresented_families: Vec<String>,
    pub underrepresented_families: Vec<String>,
    #[serde(default)]
    pub eviction_candidates: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub archived_capsule_ids: Vec<String>,
    pub diversity_score: f64,
    pub saturation: f64,
}

const SATURATION_THRESHOLD: f64 = 0.80;
const DIVERSITY_THRESHOLD: f64 = 40.0;
const EVICTION_RATE: f64 = 0.20;
const MIN_CAPSULES_TO_EVICT: usize = 1;

const FAMILY_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "attention",
        &[
            "attention",
            "transformer",
            "multi-head",
            "self-attention",
            "cross-attention",
        ],
    ),
    (
        "reinforcement",
        &[
            "rl",
            "reinforcement",
            "policy",
            "reward",
            "agent",
            "DQN",
            "PPO",
            "A3C",
        ],
    ),
    (
        "language_model",
        &[
            "LM",
            "language model",
            "decoder",
            "autoregressive",
            "LLM",
            "GPT",
            "BERT",
        ],
    ),
    (
        "vision",
        &[
            "CNN",
            "convolution",
            "resnet",
            "image",
            "vision",
            "ViT",
            "classification",
        ],
    ),
    (
        "optimization",
        &["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
    ),
    ("graph", &["GNN", "graph", "node", "edge", "message passing"]),
    (
        "reasoning",
        &[
            "reasoning",
            "chain-of-thought",
            "logical",
            "inference",
            "planning",
        ],
    ),
    ("embodied", &["embodied", "robotics", "navigation", "control", "motor"]),
];

// ─── Diversity pressure evaluator ────────────────────────────────────────────

pub struct DiversityPressureEvaluator {
    capacity: usize,
}

impl DiversityPressureEvaluator {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    fn family_of_keywords(&self, keywords: &[String]) -> String {
        let kw_set: std::collections::HashSet<String> =
            keywords.iter().map(|k| k.to_lowercase()).collect();

        for (fam, fam_kws) in FAMILY_KEYWORDS {
            if fam_kws
                .iter()
                .any(|fk| kw_set.contains(&fk.to_lowercase()))
            {
                return fam.to_string();
            }
        }
        "other".to_string()
    }

    fn compute_saturation(&self, total: usize) -> f64 {
        (total as f64 / self.capacity as f64).min(1.0)
    }

    /// Evaluate diversity pressure and return eviction candidates.
    pub fn evaluate(
        &self,
        capsules: &[HashMap<String, serde_json::Value>],
        _gap_subscriptions: Option<&[GapSubscription]>,
    ) -> DiversityPressureResult {
        let total = capsules.len();
        let saturation = self.compute_saturation(total);

        // Load diversity metrics from gene-pool-io
    let diversity = rairos_gene_pool_io::get_gene_pool_diversity();
    let diversity_score = diversity
        .get("diversity_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let overrep: Vec<String> = diversity
        .get("overrepresented_families")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let underrep: Vec<String> = diversity
        .get("underrepresented_families")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

        // Compute pressure level
        let sat_pressure = ((saturation - SATURATION_THRESHOLD) / (1.0 - SATURATION_THRESHOLD))
            .max(0.0);
        let div_pressure = ((DIVERSITY_THRESHOLD - diversity_score) / DIVERSITY_THRESHOLD).max(0.0);
        let pressure_level = sat_pressure * 0.5 + div_pressure * 0.5;

        let triggered =
            saturation >= SATURATION_THRESHOLD && diversity_score < DIVERSITY_THRESHOLD;

        if !triggered {
            return DiversityPressureResult {
                triggered: false,
                pressure_level: round(pressure_level, 3),
                overrepresented_families: overrep,
                underrepresented_families: underrep,
                eviction_candidates: vec![],
                archived_capsule_ids: vec![],
                diversity_score,
                saturation,
            };
        }

        if underrep.is_empty() || overrep.is_empty() {
            return DiversityPressureResult {
                triggered: false,
                pressure_level: round(pressure_level, 3),
                overrepresented_families: overrep,
                underrepresented_families: underrep,
                eviction_candidates: vec![],
                archived_capsule_ids: vec![],
                diversity_score,
                saturation,
            };
        }

        let mut eviction_candidates: Vec<HashMap<String, serde_json::Value>> = vec![];

        for fam in &overrep {
            let fam_caps: Vec<_> = capsules
                .iter()
                .filter(|c| {
                    let keywords: Vec<String> = c
                        .get("trigger_keywords")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    self.family_of_keywords(&keywords) == *fam
                })
                .collect();

            if fam_caps.is_empty() {
                continue;
            }

            let mut scored: Vec<_> = fam_caps
                .iter()
                .map(|c| {
                    let score = c
                        .get("outcome_success_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    (c, score)
                })
                .collect();
            scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let n_evict = ((MIN_CAPSULES_TO_EVICT as f64)
                .max(scored.len() as f64 * EVICTION_RATE)) as usize;

            for (cap, score) in scored.into_iter().take(n_evict) {
                let mut candidate = HashMap::new();
                candidate.insert(
                    "capsule_id".to_string(),
                    cap.get("capsule_id")
                        .cloned()
                        .unwrap_or(serde_json::Value::String(String::new())),
                );
                candidate.insert("family".to_string(), serde_json::json!(fam));
                candidate.insert("score".to_string(), serde_json::json!(round(score, 4)));
                candidate.insert(
                    "reason".to_string(),
                    serde_json::json!(format!(
                        "diversity_pressure: {} is over-represented (pressure={:.2})",
                        fam,
                        pressure_level
                    )),
                );
                eviction_candidates.push(candidate);
            }
        }

        DiversityPressureResult {
            triggered: true,
            pressure_level: round(pressure_level, 3),
            overrepresented_families: overrep,
            underrepresented_families: underrep,
            eviction_candidates,
            archived_capsule_ids: vec![],
            diversity_score,
            saturation,
        }
    }

    /// Execute evictions: archive eviction candidates from the GenePool.
    /// Returns updated result with archived_capsule_ids filled in.
    pub fn execute_evictions(&self, result: &mut DiversityPressureResult) {
        if result.eviction_candidates.is_empty() {
            return;
        }

        let mut archived: Vec<String> = vec![];
        let capsules = rairos_gene_pool_io::load_capsules(None, Some("active"), None);

        for candidate in &result.eviction_candidates {
            let cid = candidate
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(cap) = capsules.iter().find(|c| {
                c.get("capsule_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s == cid)
                    .unwrap_or(false)
            }) {
                // Mark as archived by updating status
                let mut cap = cap.clone();
                cap.insert(
                    "status".to_string(),
                    serde_json::Value::String("archived".to_string()),
                );
                // Write back via gene-pool-io (append to jsonl with new status)
                // Mark capsule as archived by updating status in JSONL
                let jsonl_path = get_gp_dir().join("gene_pool.jsonl");
                if let Ok(text) = std::fs::read_to_string(&jsonl_path) {
                    let mut lines: Vec<String> = text
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(String::from)
                        .collect();
                    let mut found = false;
                    for line in &mut lines {
                        if let Ok(mut cap) = serde_json::from_str::<serde_json::Value>(line) {
                            if cap.get("capsule_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s == cid)
                                .unwrap_or(false)
                            {
                                cap["status"] = serde_json::Value::String("archived".to_string());
                                *line = serde_json::to_string(&cap).unwrap();
                                found = true;
                            }
                        }
                    }
                    if found {
                        let new_text = lines.join("\n") + "\n";
                        let _ = std::fs::write(&jsonl_path, new_text);
                        archived.push(cid.to_string());
                    }
                }
            }
        }

        result.archived_capsule_ids = archived;
    }
}

// ─── GenePool Watcher ────────────────────────────────────────────────────────

/// Check and update result summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherCheckResult {
    pub diversity_score: f64,
    pub total_capsules: usize,
    pub underrepresented_families: Vec<String>,
    pub gap_subscriptions_added: Vec<String>,
    pub gap_subscriptions_removed: Vec<String>,
    pub triggered: bool,
    #[serde(default)]
    pub diversity_pressure_triggered: bool,
    #[serde(default)]
    pub pressure_level: f64,
    #[serde(default)]
    pub archived_by_pressure: Vec<String>,
    #[serde(default)]
    pub eviction_candidates: Vec<String>,
}

impl Default for WatcherCheckResult {
    fn default() -> Self {
        Self {
            diversity_score: 0.0,
            total_capsules: 0,
            underrepresented_families: Vec::new(),
            gap_subscriptions_added: Vec::new(),
            gap_subscriptions_removed: Vec::new(),
            triggered: false,
            diversity_pressure_triggered: false,
            pressure_level: 0.0,
            archived_by_pressure: Vec::new(),
            eviction_candidates: Vec::new(),
        }
    }
}

/// GenePoolWatcher periodically checks Gene Pool diversity and auto-creates gap subscriptions.
pub struct GenePoolWatcher {
    interval_seconds: u64,
    min_diversity_score: f64,
    enabled: bool,
    state: WatcherState,
}

impl GenePoolWatcher {
    /// Create a new watcher.
    ///
    /// - `interval_minutes`: how often to check diversity (default 60 min)
    /// - `min_diversity_score`: trigger gap-filling only if diversity_score falls below this
    /// - `enabled`: if false, watcher only monitors without acting
    pub fn new(interval_minutes: u64, min_diversity_score: f64, enabled: bool) -> Self {
        let state = load_watcher_state();
        Self {
            interval_seconds: interval_minutes * 60,
            min_diversity_score,
            enabled,
            state,
        }
    }

    /// Returns the current watcher state.
    pub fn get_state(&self) -> &WatcherState {
        &self.state
    }

    /// Returns the current gap subscriptions.
    pub fn get_subscriptions(&self) -> &[GapSubscription] {
        &self.state.gap_subscriptions
    }

    /// Returns currently underrepresented families.
    pub fn get_underrepresented_families(&self) -> &[String] {
        &self.state.underrepresented_families
    }

    /// Manually trigger a diversity check and subscription update.
    /// Returns a summary of what was done.
    pub fn watch(&mut self) -> WatcherCheckResult {
        let diversity = rairos_gene_pool_io::get_gene_pool_diversity();

        let underrep: Vec<String> = diversity
            .get("underrepresented_families")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let diversity_score = diversity
            .get("diversity_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_capsules = diversity
            .get("capsule_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;

        self.state.underrepresented_families = underrep.clone();
        self.state.diversity_score = diversity_score;
        self.state.last_diversity_check = now_iso();

        let mut result = WatcherCheckResult {
            diversity_score,
            total_capsules,
            underrepresented_families: underrep.clone(),
            triggered: diversity_score < self.min_diversity_score,
            ..Default::default()
        };

        if !self.enabled {
            let _ = save_watcher_state(&self.state);
            return result;
        }

        // Build target gap subscriptions for current underrepresented families
        let target_subs = build_gap_subscriptions_for_families(&underrep);

        // Diff against existing gap subscriptions
        let (to_add, to_remove) = diff_subscriptions(&self.state.gap_subscriptions, &target_subs);

        // Add new subscriptions
        for sub in &to_add {
            let sub_id = register_gap_subscription(sub);
            if sub_id.is_some() {
                self.state.gap_subscriptions.push(sub.clone());
                result.gap_subscriptions_added.push(sub.family.clone());
            }
        }

        // Disable subscriptions for families no longer underrepresented
        for fam in &to_remove {
            for gs in &mut self.state.gap_subscriptions {
                if gs.family == *fam {
                    gs.enabled = false;
                    let _ = disable_subscription(&gs);
                    result.gap_subscriptions_removed.push(fam.clone());
                }
            }
        }

        // Diversity Pressure Eviction
        let ev = DiversityPressureEvaluator::new(50);
        let capsules = rairos_gene_pool_io::load_capsules(None, Some("active"), None);
        let mut pres = ev.evaluate(&capsules, Some(&self.state.gap_subscriptions));

        if pres.triggered && !pres.eviction_candidates.is_empty() {
            ev.execute_evictions(&mut pres);
            result.diversity_pressure_triggered = true;
            result.pressure_level = pres.pressure_level;
            result.archived_by_pressure = pres.archived_capsule_ids.clone();
            result.eviction_candidates = pres
                .eviction_candidates
                .iter()
                .filter_map(|c| c.get("capsule_id").and_then(|v| v.as_str()).map(String::from))
                .collect();
        }

        let _ = save_watcher_state(&self.state);
        result
    }

    /// Auto-create subscriptions for underrepresented families.
    /// Returns list of families for which subscriptions were created.
    pub fn auto_create_subscriptions(&mut self) -> Vec<String> {
        let diversity = rairos_gene_pool_io::get_gene_pool_diversity();
        let underrep: Vec<String> = diversity
            .get("underrepresented_families")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let target_subs = build_gap_subscriptions_for_families(&underrep);
        let (to_add, _) = diff_subscriptions(&self.state.gap_subscriptions, &target_subs);

        let mut created = vec![];
        for sub in &to_add {
            let sub_id = register_gap_subscription(sub);
            if sub_id.is_some() {
                self.state.gap_subscriptions.push(sub.clone());
                created.push(sub.family.clone());
            }
        }

        let _ = save_watcher_state(&self.state);
        created
    }
}

// ─── Subscription DB helpers ─────────────────────────────────────────────────

/// Register a gap subscription (file-based, no DB).
/// Returns subscription_id if successful, None otherwise.
fn register_gap_subscription(sub: &GapSubscription) -> Option<String> {
    let mut state = load_watcher_state();
    let id = format!("gap_{}_{}", sub.family, chrono::Utc::now().timestamp_millis());
    let new_sub = GapSubscription {
        family: sub.family.clone(),
        keywords: sub.keywords.clone(),
        arxiv_category: sub.arxiv_category.clone(),
        enabled: true,
        last_checked: chrono::Utc::now().to_rfc3339(),
    };
    state.gap_subscriptions.push(new_sub);
    save_watcher_state(&state).ok()?;
    Some(id)
}

/// Disable a subscription by family (file-based, no DB).
/// Returns true if successful.
fn disable_subscription(sub: &GapSubscription) -> bool {
    let mut state = load_watcher_state();
    for s in &mut state.gap_subscriptions {
        if s.family == sub.family {
            s.enabled = false;
            s.last_checked = chrono::Utc::now().to_rfc3339();
        }
    }
    save_watcher_state(&state).is_ok()
}

// ─── HTML rendering ──────────────────────────────────────────────────────────

/// Render watcher status as HTML for web UI.
pub fn render_watcher_status_html(opt_state: Option<&WatcherState>) -> String {
    let owned = load_watcher_state();
    let state: &WatcherState = match opt_state {
        Some(s) => s,
        None => &owned,
    };

    let mut lines = vec!["<div class=\"watcher-panel\">".to_string()];
    lines.push("<h3>🧬 Gene Pool Gap Watcher</h3>".to_string());

    if state.underrepresented_families.is_empty() {
        lines.push(format!(
            "<p style='font-size:13px;color:#A89E8C;margin-bottom:12px'>\
             Gene Pool is well-diversified. Diversity score: <b>{}</b></p>",
            state.diversity_score
        ));
    } else {
        lines.push(format!(
            "<p style='font-size:13px;color:#A89E8C;margin-bottom:12px'>\
             Underrepresented families detected: <b>{}</b><br>\
             Diversity score: <b>{}</b></p>",
            state.underrepresented_families.join(", "),
            state.diversity_score
        ));
    }

    if state.gap_subscriptions.is_empty() {
        lines.push(
            "<p style='font-size:13px;color:#A89E8C'>No gap subscriptions active.</p>".to_string(),
        );
    } else {
        lines.push("<h4>Auto-Gap Subscriptions</h4>".to_string());
        lines.push("<ul style='font-size:13px'>".to_string());
        for gs in &state.gap_subscriptions {
            let status = if gs.enabled { "✓" } else { "✗" };
            let keywords_preview = gs.keywords.iter().take(2).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            lines.push(format!(
                "<li>{} <b>{}</b> — {}</li>",
                status, gs.family, keywords_preview
            ));
        }
        lines.push("</ul>".to_string());
    }

    lines.push(format!(
        "<p style='font-size:12px;color:#888;margin-top:12px'>\
         Last checked: {}</p>",
        state.last_diversity_check.is_empty()
            .then(|| "never".to_string())
            .unwrap_or_else(|| state.last_diversity_check.clone())
    ));
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ─── Utilities ───────────────────────────────────────────────────────────────

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_subscription_default() {
        let sub = GapSubscription::default();
        assert_eq!(sub.family, "other");
        assert!(sub.enabled);
    }

    #[test]
    fn test_build_gap_subscription_attention() {
        let sub = build_gap_subscription("attention");
        assert_eq!(sub.family, "attention");
        assert!(sub
            .keywords
            .contains(&"transformer".to_string()));
        assert_eq!(sub.arxiv_category, "cs.CL");
    }

    #[test]
    fn test_build_gap_subscriptions_for_families() {
        let subs = build_gap_subscriptions_for_families(&["attention".to_string(), "vision".to_string()]);
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().any(|s| s.family == "attention"));
        assert!(subs.iter().any(|s| s.family == "vision"));
    }

    #[test]
    fn test_diff_subscriptions_add() {
        let existing = vec![GapSubscription {
            family: "attention".to_string(),
            keywords: vec!["transformer".to_string()],
            arxiv_category: "cs.CL".to_string(),
            enabled: true,
            last_checked: "".to_string(),
        }];
        let new = vec![
            GapSubscription {
                family: "attention".to_string(),
                keywords: vec!["transformer".to_string()],
                arxiv_category: "cs.CL".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
            GapSubscription {
                family: "vision".to_string(),
                keywords: vec!["CNN".to_string()],
                arxiv_category: "cs.CV".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
        ];
        let (to_add, to_remove) = diff_subscriptions(&existing, &new);
        assert_eq!(to_add.len(), 1);
        assert_eq!(to_add[0].family, "vision");
        assert!(to_remove.is_empty());
    }

    #[test]
    fn test_diff_subscriptions_remove() {
        let existing = vec![GapSubscription {
            family: "attention".to_string(),
            keywords: vec!["transformer".to_string()],
            arxiv_category: "cs.CL".to_string(),
            enabled: true,
            last_checked: "".to_string(),
        }];
        let new = vec![];
        let (to_add, to_remove) = diff_subscriptions(&existing, &new);
        assert!(to_add.is_empty());
        assert_eq!(to_remove, vec!["attention"]);
    }

    #[test]
    fn test_diversity_pressure_evaluator_not_triggered() {
        let evaluator = DiversityPressureEvaluator::new(50);
        let capsules: Vec<HashMap<String, serde_json::Value>> = vec![];
        let result = evaluator.evaluate(&capsules, None);
        assert!(!result.triggered);
    }

    #[test]
    fn test_diversity_pressure_evaluator_empty() {
        let evaluator = DiversityPressureEvaluator::new(50);
        let result = evaluator.evaluate(&[], None);
        assert!(!result.triggered);
        assert_eq!(result.saturation, 0.0);
    }

    #[test]
    fn test_round() {
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(1.235, 2), 1.24);
        assert_eq!(round(1.999, 2), 2.0);
    }

    #[test]
    fn test_watcher_state_default() {
        let state = WatcherState::default();
        assert!(state.gap_subscriptions.is_empty());
        assert_eq!(state.diversity_score, 0.0);
    }

    #[test]
    fn test_render_watcher_status_html_empty() {
        let html = render_watcher_status_html(None);
        assert!(html.contains("Gene Pool Gap Watcher"));
        assert!(html.contains("well-diversified"));
    }

    #[test]
    fn test_gene_pool_watcher_new() {
        let watcher = GenePoolWatcher::new(60, 50.0, true);
        assert_eq!(watcher.interval_seconds, 3600);
        assert_eq!(watcher.min_diversity_score, 50.0);
        assert!(watcher.enabled);
    }

    #[test]
    fn test_build_gap_subscription_unknown_family() {
        let sub = build_gap_subscription("unknown_family");
        assert_eq!(sub.family, "other");
        assert_eq!(sub.arxiv_category, "cs.LG");
    }
}
