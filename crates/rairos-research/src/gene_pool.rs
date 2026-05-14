//! Gene Pool — capsule-based research guidance from historical feedback.
//!
//! Mirrors llm/insight/gene.py + storage.py for Rust-side search and logging.
//! Reads/writes ~/.ai_research_os/evolution/gene_pool.jsonl (JSON Lines).

use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;

// ─── Stopwords for keyword/keyword extraction (mirrors llm/text_utils.py) ──────

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "and", "the", "a", "an", "of", "in", "on", "to",
    "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
    "does", "did", "will", "would", "could", "should", "may", "might", "shall",
    "can", "not", "or", "but", "if", "so", "as", "at", "by", "from", "about",
    "into", "through", "during", "before", "after", "above", "below", "between",
    "more", "less", "very", "just", "also", "well", "too", "only", "however",
    "method", "approach", "gap", "issue", "problem", "limitation", "paper",
    "research", "study", "work", "novel", "propose", "show", "based", "using",
    "proposed", "our", "we", "their", "this", "that", "these", "those",
];

const KEYWORD_MIN_LEN: usize = 3;

// ─── CapsuleGene sub-schema ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct CapsuleGene {
    #[serde(default)]
    pub capsule_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub trigger_topic: String,
    #[serde(default)]
    pub trigger_gap_type: String,
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
    #[serde(default)]
    pub action_gap_type: String,
    #[serde(default)]
    pub action_gap_title: String,
    #[serde(default)]
    pub outcome_success_score: f64,
    #[serde(default)]
    pub feedback_count: i32,
    #[serde(default)]
    pub status: String,
}

// ─── Category grouping ─────────────────────────────────────────────────────────

fn gap_category(gap_type: &str) -> &str {
    match gap_type {
        "improvement" | "method_gap" | "method_limitation" => "method",
        "application_gap" | "exploration_gap" | "capability" => "content",
        _ => gap_type,
    }
}

// ─── Keyword extraction ────────────────────────────────────────────────────────

pub fn extract_keywords(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            let lower = current.to_lowercase();
            if lower.len() >= KEYWORD_MIN_LEN && !stop.contains(lower.as_str()) {
                result.push(lower);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        let lower = current.to_lowercase();
        if lower.len() >= KEYWORD_MIN_LEN && !stop.contains(lower.as_str()) {
            result.push(lower);
        }
    }
    result
}

// ─── trigger_match — 5-signal scoring ──────────────────────────────────────────

impl CapsuleGene {
    fn trigger_match(&self, topic: &str, gap_type: &str, keywords: &[String]) -> f64 {
        let mut score = 0.0;

        // Signal 1: action_gap_title substring (max +0.5)
        let title_lower = self.action_gap_title.to_lowercase();
        let topic_lower = topic.to_lowercase();
        if topic_lower.contains(&title_lower) && !title_lower.is_empty() {
            score += 0.5;
        } else if title_lower.contains(&topic_lower) && !topic_lower.is_empty() {
            score += 0.3;
        }

        // Signal 2: trigger_topic substring (max +0.3)
        let trig_lower = self.trigger_topic.to_lowercase();
        if topic_lower.contains(&trig_lower) && !trig_lower.is_empty() {
            score += 0.3;
        } else if trig_lower.contains(&topic_lower) && !topic_lower.is_empty() {
            score += 0.2;
        }

        // Signal 3: gap_type exact + category match (max +0.3)
        let gap_lower = gap_type.to_lowercase();
        let trig_gap_lower = self.trigger_gap_type.to_lowercase();
        if !gap_lower.is_empty() && !trig_gap_lower.is_empty() {
            if gap_lower == trig_gap_lower {
                score += 0.3;
            } else if gap_category(&gap_lower) == gap_category(&trig_gap_lower) {
                score += 0.1;
            }
        }

        // Signal 4: keyword overlap (max +0.15)
        if !keywords.is_empty() && !self.trigger_keywords.is_empty() {
            let kw_set: HashSet<&str> = keywords.iter().map(|s| s.as_str()).collect();
            let trig_set: HashSet<&str> = self.trigger_keywords.iter().map(|s| s.as_str()).collect();
            let overlap = kw_set.intersection(&trig_set).count();
            let denom = keywords.len().max(self.trigger_keywords.len());
            if denom > 0 {
                score += 0.15 * (overlap as f64 / denom as f64);
            }
        }

        // Signal 5: token Jaccard (max +0.25)
        let topic_tokens: HashSet<&str> = topic_lower
            .split_whitespace()
            .filter(|w| w.len() >= KEYWORD_MIN_LEN && !STOPWORDS.contains(w))
            .collect();
        let title_tokens: HashSet<&str> = title_lower
            .split_whitespace()
            .filter(|w| w.len() >= KEYWORD_MIN_LEN && !STOPWORDS.contains(w))
            .collect();
        let intersection = topic_tokens.intersection(&title_tokens).count();
        let union = topic_tokens.union(&title_tokens).count();
        if union > 0 {
            score += 0.25 * (intersection as f64 / union as f64);
        }

        score.min(1.0)
    }
}

// ─── GenePool Manager ──────────────────────────────────────────────────────────

pub struct GenePool {
    jsonl_path: PathBuf,
    events_path: PathBuf,
}

impl Default for GenePool {
    fn default() -> Self {
        let base = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("evolution");
        Self {
            jsonl_path: base.join("gene_pool.jsonl"),
            events_path: base.join("events.jsonl"),
        }
    }
}

impl GenePool {
    pub fn new() -> Self {
        Self::default()
    }

    fn load_capsules(&self) -> Vec<CapsuleGene> {
        let content = match std::fs::read_to_string(&self.jsonl_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                serde_json::from_str::<CapsuleGene>(trimmed).ok()
            })
            .collect()
    }

    pub fn find_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: Option<&[String]>,
        min_score: f64,
    ) -> (Option<String>, f64) {
        let capsules = self.load_capsules();
        let keywords = keywords.unwrap_or(&[]);
        let kw_vec: Vec<String> = if keywords.is_empty() {
            extract_keywords(topic)
        } else {
            keywords.to_vec()
        };

        let mut best_score = 0.0_f64;
        let mut best_hint: Option<String> = None;

        for cap in &capsules {
            if cap.status == "archived" {
                continue;
            }
            let score = cap.trigger_match(topic, gap_type, &kw_vec);
            if score >= min_score && score > best_score {
                best_score = score;
                best_hint = Some(cap.action_gap_title.clone());
            }
        }

        (best_hint, best_score)
    }

    pub fn record_gap_accept(
        &self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        gap_description: &str,
    ) -> Result<(), String> {
        let evt = serde_json::json!({
            "timestamp": chrono_now(),
            "topic": topic,
            "action": "accepted",
            "gap_type": gap_type,
            "gap_title": gap_title,
            "gap_description": gap_description,
            "hypothesis_id": "",
            "question_id": "",
            "paper_ids": [],
            "duration_seconds": 0,
            "notes": "",
            "insight_card_id": "",
        });

        // Ensure parent directory exists
        if let Some(parent) = self.events_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .map_err(|e| format!("Failed to open events.jsonl: {}", e))?;

        use std::io::Write;
        writeln!(file, "{}", evt).map_err(|e| format!("Failed to write event: {}", e))?;

        Ok(())
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.f").to_string()
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_capsule() -> CapsuleGene {
        CapsuleGene {
            capsule_id: "test001".to_string(),
            created_at: "2024-01-01".to_string(),
            trigger_topic: "reinforcement learning".to_string(),
            trigger_gap_type: "method_limitation".to_string(),
            trigger_keywords: vec!["rl".to_string(), "reward".to_string()],
            action_gap_type: "method_limitation".to_string(),
            action_gap_title: "RL sample efficiency".to_string(),
            outcome_success_score: 0.8,
            feedback_count: 5,
            status: "active".to_string(),
        }
    }

    #[test]
    fn test_trigger_match_exact_title() {
        let cap = sample_capsule();
        // topic contains the capsule's action_gap_title
        let score = cap.trigger_match("RL sample efficiency issues", "method_limitation", &[]);
        assert!(score > 0.5, "score was {}", score);
    }

    #[test]
    fn test_trigger_match_gap_type_boost() {
        let cap = sample_capsule();
        let score_same = cap.trigger_match("some topic", "method_limitation", &[]);
        let score_diff = cap.trigger_match("some topic", "application_gap", &[]);
        assert!(score_same >= score_diff, "same type should score >= different");
    }

    #[test]
    fn test_trigger_match_no_match() {
        let cap = sample_capsule();
        let score = cap.trigger_match("quantum physics", "theoretical_gap", &[]);
        assert!(score < 0.5, "unrelated topic should score low, got {}", score);
    }

    #[test]
    fn test_extract_keywords() {
        let result = extract_keywords("novel RL sample efficiency method");
        assert!(result.contains(&"efficiency".to_string()));
        assert!(!result.contains(&"method".to_string())); // stopword
        assert!(!result.contains(&"rl".to_string())); // too short (len=2)
    }

    #[test]
    fn test_find_capsule_empty_gene_pool() {
        let dir = std::env::temp_dir().join("rairos_gene_pool_empty_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pool = GenePool {
            jsonl_path: dir.join("gene_pool.jsonl"),
            events_path: dir.join("events.jsonl"),
        };
        let (hint, score) = pool.find_capsule("test", "method", None, 0.0);
        assert!(hint.is_none(), "expected no hint, got {:?}", hint);
        assert_eq!(score, 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_gap_category_groups() {
        assert_eq!(gap_category("improvement"), "method");
        assert_eq!(gap_category("method_limitation"), "method");
        assert_eq!(gap_category("application_gap"), "content");
        assert_eq!(gap_category("exploration_gap"), "content");
        assert_eq!(gap_category("theoretical_gap"), "theoretical_gap");
    }

    #[test]
    fn test_record_gap_accept() {
        let dir = std::env::temp_dir().join("rairos_gene_pool_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pool = GenePool {
            jsonl_path: dir.join("gene_pool.jsonl"),
            events_path: dir.join("events.jsonl"),
        };
        let r = pool.record_gap_accept("test topic", "method_gap", "test title", "test desc");
        assert!(r.is_ok());
        let content = std::fs::read_to_string(&pool.events_path).unwrap_or_default();
        assert!(content.contains("test topic"));
        assert!(content.contains("accepted"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
