//! Topic Discovery — intelligently suggest new arXiv subscription topics from gaps and papers.

//!
//! Given recent research output (gaps, papers), this module identifies research areas
//! that are active but not yet subscribed to, so the system can proactively expand
//! its monitoring boundary.
//!
//! # Discovery strategies
//! 1. Gap-cluster based: hot clusters with many high-novelty gaps → suggest subscription
//! 2. Gap-type trending: rising gap types (METHOD_LIMITATION growing) → suggest subscription
//! 3. Paper keyword extraction: frequent untracked keywords in recent papers
//! 4. Gap→subscription mapping: gap_type → topic keyword suggestions
//!
//! # Usage
//! ```ignore
//! let discoverer = TopicDiscoverer::new();
//! let suggestions = discoverer.suggest_new_topics(
//!     recent_gaps, recent_papers, gap_clusters, gap_trends, 5
//! ).await;
//! for s in &suggestions {
//!     println!("[{}] {} (confidence={:.2})", s.source, s.topic, s.confidence);
//!     println!("  reason: {}", s.reason);
//! }
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

// ─── Error type ─────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TopicDiscoveryError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

// ─── Dataclasses / Structs ───────────────────────────────────────────────────

/// A suggested new subscription topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSuggestion {
    /// e.g. "scaling laws for reasoning"
    pub topic: String,
    /// 'gap_cluster' | 'gap_type_trend' | 'paper_keyword' | 'gap_subscription_map'
    pub source: String,
    /// 0.0–1.0
    pub confidence: f64,
    /// human-readable explanation
    pub reason: String,
    /// associated gap type if applicable
    pub gap_type: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// if from gap cluster
    #[serde(default)]
    pub cluster_id: String,
    /// average novelty of source gaps
    pub novelty_score: f64,
}

impl TopicSuggestion {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        topic: String,
        source: &str,
        confidence: f64,
        reason: String,
        gap_type: String,
        keywords: Vec<String>,
        cluster_id: String,
        novelty_score: f64,
    ) -> Self {
        Self {
            topic,
            source: source.to_string(),
            confidence,
            reason,
            gap_type,
            keywords,
            cluster_id,
            novelty_score,
        }
    }
}

// ─── Gap type → topic keyword mapping ────────────────────────────────────────

fn gap_type_topic_map() -> HashMap<String, Vec<String>> {
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    m.insert(
        "method_limitation".into(),
        vec![
            "method limitation".into(),
            "inefficiency".into(),
            "scalability".into(),
        ],
    );
    m.insert(
        "unexplored_application".into(),
        vec!["application".into(), "domain".into(), "new setting".into()],
    );
    m.insert(
        "evaluation_gap".into(),
        vec![
            "benchmark".into(),
            "evaluation".into(),
            "measurement".into(),
        ],
    );
    m.insert(
        "scalability_issue".into(),
        vec!["scaling".into(), "large-scale".into(), "efficiency".into()],
    );
    m.insert(
        "theoretical_gap".into(),
        vec!["theory".into(), "analysis".into(), "foundation".into()],
    );
    m.insert(
        "dataset_gap".into(),
        vec!["dataset".into(), "data".into(), "corpus".into()],
    );
    m.insert(
        "generalization_gap".into(),
        vec![
            "generalization".into(),
            "out-of-distribution".into(),
            "robustness".into(),
        ],
    );
    m.insert(
        "contradiction".into(),
        vec![
            "contradiction".into(),
            "rebuttal".into(),
            "counter-example".into(),
        ],
    );
    m.insert(
        "capability_missing".into(),
        vec!["capability".into(), "ability".into(), "missing".into()],
    );
    m.insert(
        "unknown".into(),
        vec!["open problem".into(), "underexplored".into()],
    );
    // Aliases
    m.insert(
        "capability".into(),
        m.get("capability_missing").cloned().unwrap_or_default(),
    );
    m.insert(
        "quality".into(),
        m.get("method_limitation").cloned().unwrap_or_default(),
    );
    m.insert(
        "missing".into(),
        m.get("unexplored_application").cloned().unwrap_or_default(),
    );
    m
}

// ─── Keyword extraction ──────────────────────────────────────────────────────

static KEYWORD_REGEX: OnceLock<Regex> = OnceLock::new();

fn keyword_regex() -> &'static Regex {
    KEYWORD_REGEX.get_or_init(|| Regex::new(r"[a-z][a-z0-9-]*[a-z]").expect("valid regex"))
}

static GENERIC_TERMS: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn generic_terms() -> &'static HashSet<&'static str> {
    GENERIC_TERMS.get_or_init(|| {
        let mut s = HashSet::new();
        [
            "paper",
            "work",
            "method",
            "approach",
            "result",
            "experiment",
            "performance",
            "show",
            "propose",
            "state-of-the-art",
            "sota",
            "baseline",
            "existing",
            "current",
            "recent",
            "new",
            "novel",
            "task",
            "problem",
            "model",
            "data",
            "dataset",
            "training",
            "evaluation",
            "benchmark",
            "learning",
            "system",
            "framework",
            "the",
            "and",
            "for",
            "with",
            "from",
            "that",
            "this",
            "are",
        ]
        .iter()
        .for_each(|&v| {
            s.insert(v);
        });
        s
    })
}

/// Extract frequent meaningful phrases from a list of text strings.
///
/// Returns `[(keyword, frequency), ...]` sorted by frequency desc.
/// Filters out generic academic terms.
fn extract_keywords_from_text(texts: &[String], top_n: usize) -> Vec<(String, usize)> {
    let regex = keyword_regex();
    let generic = generic_terms();
    let mut token_counter: HashMap<String, usize> = HashMap::new();

    for text in texts {
        for token in regex.find_iter(&text.to_lowercase()) {
            let t = token.as_str().to_string();
            if t.len() >= 4 && !generic.contains(t.as_str()) {
                *token_counter.entry(t).or_insert(0) += 1;
            }
        }
    }

    let mut all: Vec<(String, usize)> = token_counter.into_iter().collect();
    all.sort_by_key(|x| std::cmp::Reverse(x.1));
    all.truncate(top_n * 2);
    all
}

/// Turn a list of (keyword, freq) into a coherent topic phrase.
fn phrase_suggestion_from_keywords(keywords: &[(String, usize)]) -> String {
    if keywords.is_empty() {
        return String::new();
    }
    let top: Vec<String> = keywords.iter().take(4).map(|(k, _)| k.clone()).collect();
    top.join(" + ")
}

// ─── Gap-cluster based discovery ─────────────────────────────────────────────

/// Trait for accessing gap/cluster objects dynamically (duck typing).
/// We use a generic approach via HashMap/serde Value.
pub trait GapClusterTrait {
    fn gaps(&self) -> &[serde_json::Value];
    fn novelty_score(&self) -> f64;
    fn gap_type(&self) -> String;
    fn cluster_id(&self) -> String;
    fn title(&self) -> String;
}

#[derive(Debug, Clone, Deserialize)]
pub struct GapCluster {
    pub gaps: Vec<serde_json::Value>,
    #[serde(default)]
    pub novelty_score: f64,
    #[serde(default)]
    pub gap_type: String,
    #[serde(default)]
    pub cluster_id: String,
}

impl GapClusterTrait for GapCluster {
    fn gaps(&self) -> &[serde_json::Value] {
        &self.gaps
    }
    fn novelty_score(&self) -> f64 {
        self.novelty_score
    }
    fn gap_type(&self) -> String {
        self.gap_type.clone()
    }
    fn cluster_id(&self) -> String {
        self.cluster_id.clone()
    }
    fn title(&self) -> String {
        String::new()
    }
}

fn from_gap_clusters(
    clusters: &[serde_json::Value],
    _all_gaps: &[serde_json::Value],
    threshold_novelty: f64,
) -> Vec<TopicSuggestion> {
    let mut suggestions = Vec::new();

    for cluster_val in clusters {
        // Try to deserialize as GapCluster
        let cluster = match serde_json::from_value::<GapCluster>(cluster_val.clone()) {
            Ok(c) => c,
            Err(_) => {
                // Fallback: try to access gaps field
                let gaps = cluster_val
                    .get("gaps")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let novelty_score = cluster_val
                    .get("novelty_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let gap_type = cluster_val
                    .get("gap_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let cluster_id = cluster_val
                    .get("cluster_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                GapCluster {
                    gaps,
                    novelty_score,
                    gap_type,
                    cluster_id,
                }
            }
        };

        if cluster.gaps.len() < 2 {
            continue;
        }

        // Compute avg novelty from gaps
        let mut total_novelty = 0.0;
        let mut novelty_count = 0;
        for g in &cluster.gaps {
            let ns = g
                .get("novelty_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            total_novelty += ns;
            novelty_count += 1;
        }
        let avg_novelty = if novelty_count > 0 {
            total_novelty / novelty_count as f64
        } else {
            0.0
        };

        if avg_novelty < threshold_novelty {
            continue;
        }

        let gap_type = if cluster.gap_type.is_empty() {
            "unknown".to_string()
        } else {
            cluster.gap_type.clone()
        };
        let cluster_id = cluster.cluster_id.clone();

        // Build topic from cluster keywords
        let titles: Vec<String> = cluster
            .gaps
            .iter()
            .filter_map(|g| {
                g.get("title")
                    .and_then(|v| v.as_str())
                    .or_else(|| g.get("gap_title").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
            })
            .collect();

        let keywords = extract_keywords_from_text(&titles, 5);
        let topic = if keywords.is_empty() {
            let first_title = titles
                .first()
                .map(|t| {
                    if t.len() > 60 {
                        t[..60].to_string()
                    } else {
                        t.clone()
                    }
                })
                .unwrap_or_default();
            format!("{}: {}", gap_type, first_title)
        } else {
            phrase_suggestion_from_keywords(&keywords)
        };

        let subscription_keywords: Vec<String> =
            keywords.iter().take(5).map(|(k, _)| k.clone()).collect();
        let confidence = (avg_novelty * 1.2).min(1.0);
        let first_title_short = titles
            .first()
            .map(|t| if t.len() > 60 { &t[..60] } else { t.as_str() })
            .unwrap_or("");
        let reason = format!(
            "Hot cluster: {} gaps (avg novelty={:.2}). Top: {}",
            cluster.gaps.len(),
            avg_novelty,
            first_title_short
        );

        suggestions.push(TopicSuggestion {
            topic,
            source: "gap_cluster".into(),
            confidence,
            reason,
            gap_type,
            keywords: subscription_keywords,
            cluster_id,
            novelty_score: avg_novelty,
        });
    }

    suggestions
}

// ─── Gap-type trend based discovery ──────────────────────────────────────────

fn from_gap_type_trends(
    trends: &HashMap<String, String>,
    gaps: &[serde_json::Value],
    threshold_count: usize,
) -> Vec<TopicSuggestion> {
    let mut suggestions = Vec::new();

    let rising_types: Vec<String> = trends
        .iter()
        .filter(|(_, v)| *v == "rising")
        .map(|(k, _)| k.clone())
        .collect();

    if rising_types.is_empty() {
        return suggestions;
    }

    for gap_type in rising_types {
        let type_gaps: Vec<&serde_json::Value> = gaps
            .iter()
            .filter(|g| {
                let gt = g.get("gap_type").and_then(|v| v.as_str()).unwrap_or("");
                gt == gap_type
            })
            .collect();

        if type_gaps.len() < threshold_count {
            continue;
        }

        let titles: Vec<String> = type_gaps
            .iter()
            .filter_map(|g| {
                g.get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        let keywords = extract_keywords_from_text(&titles, 5);
        let topic = if keywords.is_empty() {
            format!("rising {} research", gap_type)
        } else {
            phrase_suggestion_from_keywords(&keywords)
        };

        let novelty: f64 = type_gaps
            .iter()
            .map(|g| {
                g.get("novelty_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            })
            .sum::<f64>()
            / type_gaps.len() as f64;

        let kw: Vec<String> = keywords.iter().take(4).map(|(k, _)| k.clone()).collect();
        let reason = format!(
            "Gap type '{}' is trending ({} recent gaps)",
            gap_type,
            type_gaps.len()
        );

        suggestions.push(TopicSuggestion {
            topic,
            source: "gap_type_trend".into(),
            confidence: 0.6,
            reason,
            gap_type,
            keywords: kw,
            cluster_id: String::new(),
            novelty_score: novelty,
        });
    }

    suggestions
}

// ─── Paper keyword based discovery ──────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct PaperInput {
    pub title: Option<String>,
    pub abstract_: Option<String>,
}

impl PaperInput {
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }
    pub fn abstract_(&self) -> &str {
        self.abstract_.as_deref().unwrap_or("")
    }
}

fn from_paper_keywords(
    papers: &[serde_json::Value],
    existing_topics: &HashSet<String>,
    threshold_freq: usize,
) -> Vec<TopicSuggestion> {
    let mut suggestions = Vec::new();

    let mut texts = Vec::new();
    for p in papers {
        let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let abstract_trunc = p
            .get("abstract")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(300)
            .collect::<String>();
        texts.push(format!("{} {}", title, abstract_trunc));
    }

    let keywords = extract_keywords_from_text(&texts, 20);

    for (keyword, freq) in keywords {
        if freq < threshold_freq {
            break;
        }

        // Check if existing topics cover this keyword
        let keyword_lower = keyword.to_lowercase();
        let covered = existing_topics
            .iter()
            .any(|t| t.to_lowercase().contains(&keyword_lower));
        if covered {
            continue;
        }

        let confidence = (freq as f64 / 10.0 + 0.3).min(0.9);
        let reason = format!(
            "'{}' appears in {} recent papers but has no subscription",
            keyword, freq
        );

        suggestions.push(TopicSuggestion {
            topic: format!("{} research", keyword),
            source: "paper_keyword".into(),
            confidence,
            reason,
            gap_type: String::new(),
            keywords: vec![keyword],
            cluster_id: String::new(),
            novelty_score: 0.0,
        });
    }

    suggestions.truncate(5);
    suggestions
}

// ─── Gap→subscription mapping ────────────────────────────────────────────────

fn from_gap_subscription_map(
    gaps: &[serde_json::Value],
    existing_topics: &HashSet<String>,
    min_gaps_per_type: usize,
) -> Vec<TopicSuggestion> {
    let mut suggestions = Vec::new();
    let type_map = gap_type_topic_map();

    // Count gaps per gap type
    let mut type_counter: HashMap<String, usize> = HashMap::new();
    let mut type_gaps_map: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();

    for g in gaps {
        let gt_raw = g.get("gap_type");
        let gt = gt_raw.and_then(|v| v.as_str()).unwrap_or("unknown");
        *type_counter.entry(gt.to_string()).or_insert(0) += 1;
        type_gaps_map.entry(gt.to_string()).or_default().push(g);
    }

    for (gap_type, count) in type_counter {
        if count < min_gaps_per_type {
            continue;
        }

        let mapped_keywords = type_map.get(&gap_type).cloned().unwrap_or_default();
        if mapped_keywords.is_empty() {
            continue;
        }

        let topic = format!("{}: {}", gap_type.replace('_', " "), mapped_keywords[0]);

        // Check if this topic is already covered
        let gap_type_spaced = gap_type.replace('_', " ");
        let covered = existing_topics.iter().any(|t| {
            let t_lower = t.to_lowercase();
            t_lower.contains(&gap_type_spaced)
                || t_lower.contains(&mapped_keywords[0].to_lowercase())
        });
        if covered {
            continue;
        }

        let type_gaps = type_gaps_map.get(&gap_type);
        let avg_novelty = type_gaps
            .map(|gaps| {
                gaps.iter()
                    .map(|g| {
                        g.get("novelty_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0)
                    })
                    .sum::<f64>()
                    / gaps.len() as f64
            })
            .unwrap_or(0.0);

        let reason = format!(
            "Gap type '{}' has {} gaps but no subscription. Mapped from known gap-type patterns.",
            gap_type, count
        );

        suggestions.push(TopicSuggestion {
            topic,
            source: "gap_subscription_map".into(),
            confidence: (avg_novelty + 0.2).min(0.8),
            reason,
            gap_type,
            keywords: mapped_keywords.into_iter().take(3).collect(),
            cluster_id: String::new(),
            novelty_score: avg_novelty,
        });
    }

    suggestions
}

// ─── Main TopicDiscoverer ────────────────────────────────────────────────────

/// Suggest new arXiv subscription topics from recent gaps and papers.
#[derive(Debug, Clone)]
pub struct TopicDiscoverer {
    // db: Option<DB>, //暂时省略DB集成
}

impl TopicDiscoverer {
    pub fn new() -> Self {
        Self {}
    }

    /// Return topic suggestions ranked by confidence.
    ///
    /// - `recent_gaps`: List of gap objects as JSON values
    /// - `recent_papers`: List of paper dicts with 'title' and 'abstract'
    /// - `gap_clusters`: List of GapCluster objects as JSON values
    /// - `gap_trends`: Dict[gap_type -> 'rising'|'stable'|'declining']
    /// - `max_suggestions`: max suggestions to return
    pub fn suggest_new_topics(
        &self,
        recent_gaps: &[serde_json::Value],
        recent_papers: &[serde_json::Value],
        gap_clusters: &[serde_json::Value],
        gap_trends: &HashMap<String, String>,
        max_suggestions: usize,
    ) -> Vec<TopicSuggestion> {
        // Get existing subscriptions to avoid duplicates (empty for now without DB)
        let existing_topics: HashSet<String> = HashSet::new();

        let mut all_suggestions = Vec::new();

        // Strategy 1: gap clusters (highest priority)
        if !gap_clusters.is_empty() {
            all_suggestions.extend(from_gap_clusters(gap_clusters, recent_gaps, 0.4));
        }

        // Strategy 2: gap-type trends
        if !gap_trends.is_empty() {
            all_suggestions.extend(from_gap_type_trends(gap_trends, recent_gaps, 3));
        }

        // Strategy 3: gap→subscription mapping
        all_suggestions.extend(from_gap_subscription_map(recent_gaps, &existing_topics, 2));

        // Strategy 4: paper keywords
        if !recent_papers.is_empty() {
            all_suggestions.extend(from_paper_keywords(recent_papers, &existing_topics, 3));
        }

        // Deduplicate by topic (keep highest confidence)
        let mut seen_topics: HashMap<String, TopicSuggestion> = HashMap::new();
        for s in all_suggestions {
            let key = s.topic.to_lowercase();
            let entry = seen_topics.entry(key).or_insert(s.clone());
            if s.confidence > entry.confidence {
                *entry = s;
            }
        }

        // Sort by confidence desc
        let mut sorted: Vec<TopicSuggestion> = seen_topics.into_values().collect();
        sorted.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(max_suggestions);
        sorted
    }
}

impl Default for TopicDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let texts = vec![
            "Scaling laws for large language models".to_string(),
            "Scaling laws in neural network training".to_string(),
        ];
        let kw = extract_keywords_from_text(&texts, 5);
        assert!(!kw.is_empty());
        // "scaling" and "laws" should appear multiple times
        let scaling_count = kw
            .iter()
            .find(|(k, _)| k == "scaling")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert!(scaling_count >= 2);
    }

    #[test]
    fn test_phrase_suggestion() {
        let kw = vec![
            ("scaling".to_string(), 5),
            ("laws".to_string(), 4),
            ("language".to_string(), 3),
            ("models".to_string(), 2),
        ];
        let phrase = phrase_suggestion_from_keywords(&kw);
        assert_eq!(phrase, "scaling + laws + language + models");
    }

    #[test]
    fn test_gap_type_trends() {
        let mut trends = HashMap::new();
        trends.insert("method_limitation".to_string(), "rising".to_string());
        trends.insert("dataset_gap".to_string(), "stable".to_string());

        let gaps: Vec<serde_json::Value> = vec![
            serde_json::json!({"gap_type": "method_limitation", "title": "Method is slow"}),
            serde_json::json!({"gap_type": "method_limitation", "title": "Inefficient method"}),
            serde_json::json!({"gap_type": "method_limitation", "title": "Scaling issue"}),
        ];

        let suggestions = from_gap_type_trends(&trends, &gaps, 3);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].source, "gap_type_trend");
        assert_eq!(suggestions[0].gap_type, "method_limitation");
    }

    #[test]
    fn test_gap_subscription_map() {
        let gaps: Vec<serde_json::Value> = vec![
            serde_json::json!({"gap_type": "method_limitation", "title": "Slow method", "novelty_score": 0.5}),
            serde_json::json!({"gap_type": "method_limitation", "title": "Inefficient", "novelty_score": 0.6}),
        ];
        let existing_topics = HashSet::new();
        let suggestions = from_gap_subscription_map(&gaps, &existing_topics, 2);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].source, "gap_subscription_map");
    }

    #[test]
    fn test_paper_keywords() {
        let papers: Vec<serde_json::Value> = vec![
            serde_json::json!({"title": "Scaling laws for reasoning", "abstract": "We study scaling laws for reasoning capabilities."}),
            serde_json::json!({"title": "More scaling laws", "abstract": "Scaling laws for language models."}),
            serde_json::json!({"title": "Scaling in training", "abstract": "Training dynamics of scaling."}),
        ];
        let existing_topics = HashSet::new();
        let suggestions = from_paper_keywords(&papers, &existing_topics, 3);
        assert!(!suggestions.is_empty());
        // "scaling" should be suggested
        let has_scaling = suggestions.iter().any(|s| s.topic.contains("scaling"));
        assert!(has_scaling);
    }

    #[test]
    fn test_topic_discoverer_full() {
        let discoverer = TopicDiscoverer::new();

        let gaps: Vec<serde_json::Value> = vec![
            serde_json::json!({"gap_type": "method_limitation", "title": "Slow inference", "novelty_score": 0.7}),
            serde_json::json!({"gap_type": "method_limitation", "title": "High latency", "novelty_score": 0.8}),
        ];

        let papers: Vec<serde_json::Value> = vec![
            serde_json::json!({"title": "New scaling approach", "abstract": "A new approach to scaling."}),
            serde_json::json!({"title": "Scaling improvements", "abstract": "Improving scaling behavior."}),
            serde_json::json!({"title": "Scaling methods", "abstract": "Novel scaling methods."}),
        ];

        let mut trends = HashMap::new();
        trends.insert("method_limitation".to_string(), "rising".to_string());

        let suggestions = discoverer.suggest_new_topics(&gaps, &papers, &[], &trends, 5);
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_gap_cluster_suggestion() {
        let clusters: Vec<serde_json::Value> = vec![serde_json::json!({
            "gaps": [
                {"title": "Slow inference method", "novelty_score": 0.6},
                {"title": "High latency approach", "novelty_score": 0.7},
                {"title": "Inefficient algorithm", "novelty_score": 0.5},
            ],
            "gap_type": "method_limitation",
            "cluster_id": "cluster-1",
            "novelty_score": 0.6,
        })];
        let gaps: Vec<serde_json::Value> = vec![];

        let suggestions = from_gap_clusters(&clusters, &gaps, 0.4);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].source, "gap_cluster");
        assert_eq!(suggestions[0].cluster_id, "cluster-1");
    }
}
