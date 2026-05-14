//! Gap Analysis — rule-based research gap detection from paper summaries.
//!
//! Mirrors GapDetector._detect_gaps_rules() from llm/research/gap_detector.py

use crate::{GapSnapshot, PaperSnapshot};
use std::collections::HashMap;

// ─── Gap type patterns (keyword/substring based) ───────────────────────────────

struct GapPattern {
    gap_type: &'static str,
    label: &'static str,
    patterns: &'static [&'static str],
}

const GAP_PATTERNS: &[GapPattern] = &[
    GapPattern {
        gap_type: "method_limitation",
        label: "Method Limitation",
        patterns: &[
            "limitation", "drawback", "however", "but ", "not suitable",
            "not efficient", "poor performance", "high latency", "high cost",
            "low accuracy", "not scalable", "bottleneck",
        ],
    },
    GapPattern {
        gap_type: "unexplored_application",
        label: "Unexplored Application",
        patterns: &[
            "future work", "open question", "not explore", "remains unexplored",
            "beyond the scope", "left for future", "not covered", "out of scope",
            "limited to", "only consider",
        ],
    },
    GapPattern {
        gap_type: "contradiction",
        label: "Contradiction",
        patterns: &[
            "inconsistent", "contradict", "debate", "disagree",
            "conflicting", "opposing", "mixed results", "not conclusive",
            "unclear whether", "discrepancy",
        ],
    },
    GapPattern {
        gap_type: "evaluation_gap",
        label: "Evaluation Gap",
        patterns: &[
            "no benchmark", "lack evaluation", "not compare", "no standard",
            "not evaluated", "no metric", "hard to evaluate", "lack of benchmark",
            "no ground truth", "not validated",
        ],
    },
    GapPattern {
        gap_type: "scalability_issue",
        label: "Scalability Issue",
        patterns: &[
            "scalab", "large scale", "computational cost", "memory footprint",
            "not efficient", "complexity", "expensive", "resource intensive",
            "not practical", "real world deployment",
        ],
    },
    GapPattern {
        gap_type: "theoretical_gap",
        label: "Theoretical Gap",
        patterns: &[
            "theoretical", "foundation", "lack formal", "not well understood",
            "lacks theoretical", "no proof", "heuristic", "empirical only",
            "no guarantee", "not rigorous",
        ],
    },
    GapPattern {
        gap_type: "dataset_gap",
        label: "Dataset Gap",
        patterns: &[
            "dataset lack", "no data", "limited data", "small dataset",
            "not enough data", "data scarcity", "lack of dataset",
            "no available", "proprietary data", "synthetic only",
        ],
    },
    GapPattern {
        gap_type: "generalization_gap",
        label: "Generalization Gap",
        patterns: &[
            "generaliz", "transfer", "domain shift", "out of distribution",
            "not generalize", "overfit", "domain adaptation",
            "cross domain", "unseen data",
        ],
    },
];

// ─── Raw detection result ──────────────────────────────────────────────────────

struct RawGap {
    gap_type: String,
    title: String,
    description: String,
    matched_papers: Vec<String>,
    severity: String,
}

// ─── Detect gaps from paper summaries ──────────────────────────────────────────

pub fn analyze_gaps(
    snapshots: &[PaperSnapshot],
    topic: &str,
) -> Vec<GapSnapshot> {
    let raw = detect_gaps_raw(snapshots);
    let mut enriched = enrich_gaps(raw, snapshots, topic);
    enriched.truncate(5);
    enriched
}

fn detect_gaps_raw(snapshots: &[PaperSnapshot]) -> Vec<RawGap> {
    // Collect all paper text
    let papers_text: Vec<(&str, &str)> = snapshots
        .iter()
        .map(|s| {
            let text = s
                .extracted_text
                .as_deref()
                .unwrap_or(&s.abstract_text);
            (s.paper_id.as_str(), text)
        })
        .collect();

    let mut results: Vec<RawGap> = Vec::new();
    let mut seen_types: std::collections::HashSet<String> = std::collections::HashSet::new();

    for pattern in GAP_PATTERNS {
        let mut matched_papers: Vec<String> = Vec::new();
        let mut evidence: Vec<String> = Vec::new();

        for (paper_id, text) in &papers_text {
            let lower = text.to_lowercase();
            for pat in pattern.patterns {
                if lower.contains(pat) {
                    if !matched_papers.iter().any(|p| p == paper_id) {
                        matched_papers.push((*paper_id).to_string());
                    }
                    evidence.push(format!("{} ({})", pat, paper_id));
                    break; // one pattern match per paper is enough
                }
            }
        }

        if matched_papers.is_empty() {
            continue;
        }

        // Avoid duplicate gap types
        let key = pattern.gap_type.to_string();
        if seen_types.contains(&key) {
            continue;
        }
        seen_types.insert(key.clone());

        let evidence_text = if evidence.len() > 3 {
            format!("{} and {} more", evidence[..3].join("; "), evidence.len() - 3)
        } else {
            evidence.join("; ")
        };

        let severity = if matched_papers.len() >= 3 {
            "high".to_string()
        } else if matched_papers.len() >= 2 {
            "medium".to_string()
        } else {
            "low".to_string()
        };

        results.push(RawGap {
            gap_type: key.clone(),
            title: pattern.label.to_string(),
            description: format!(
                "Detected {} in {} papers: {}",
                pattern.label.to_lowercase(),
                matched_papers.len(),
                evidence_text,
            ),
            matched_papers,
            severity,
        });
    }

    results
}

fn enrich_gaps(
    raw: Vec<RawGap>,
    snapshots: &[PaperSnapshot],
    topic: &str,
) -> Vec<GapSnapshot> {
    let pool = crate::gene_pool::GenePool::new();
    let mut results: Vec<GapSnapshot> = Vec::new();

    for r in raw {
        // Get Gene Pool match score
        let (_hint, gp_score) = pool.find_capsule(topic, &r.gap_type, None, 0.0);

        let mut gap_id = String::new();
        for ch in r.gap_type.chars() {
            if ch.is_alphanumeric() {
                gap_id.push(ch);
            }
        }

        results.push(GapSnapshot {
            gap_id,
            gap_type: r.gap_type,
            title: r.title,
            description: r.description,
            severity: r.severity,
            novelty_score: gp_score,
            related_paper_ids: r.matched_papers,
            archetype_match: gp_score,
            accepted: false,
        });
    }

    // Sort: high severity first, then by novelty_score
    results.sort_by(|a, b| {
        let sev_a = match a.severity.as_str() {
            "high" => 0,
            "medium" => 1,
            _ => 2,
        };
        let sev_b = match b.severity.as_str() {
            "high" => 0,
            "medium" => 1,
            _ => 2,
        };
        sev_a.cmp(&sev_b).then_with(|| {
            b.novelty_score
                .partial_cmp(&a.novelty_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    results
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(id: &str, text: &str) -> PaperSnapshot {
        PaperSnapshot {
            paper_id: id.to_string(),
            arxiv_id: Some(id.to_string()),
            title: String::new(),
            abstract_text: text.to_string(),
            published: String::new(),
            citations: Vec::new(),
            extracted_text: Some(text.to_string()),
        }
    }

    #[test]
    fn test_detect_method_limitation() {
        let snapshots = vec![make_paper(
            "2401.00001",
            "This method has a major limitation: it does not scale well.",
        )];
        let gaps = analyze_gaps(&snapshots, "test topic");
        assert!(!gaps.is_empty());
        assert_eq!(gaps[0].gap_type, "method_limitation");
    }

    #[test]
    fn test_detect_future_work() {
        let snapshots = vec![make_paper(
            "2401.00002",
            "This approach is left for future work to explore.",
        )];
        let gaps = analyze_gaps(&snapshots, "test topic");
        assert!(
            gaps.iter().any(|g| g.gap_type == "unexplored_application"),
            "should detect unexplored_application from future work, got: {:?}",
            gaps.iter().map(|g| &g.gap_type).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_gaps_for_clean_paper() {
        let snapshots = vec![make_paper(
            "2401.00003",
            "We propose a novel method. Our approach achieves SOTA results. The method is efficient.",
        )];
        let gaps = analyze_gaps(&snapshots, "test topic");
        // May detect something from heuristics, but should be few
        assert!(gaps.len() <= 2, "clean papers should have few gaps, got {}", gaps.len());
    }

    #[test]
    fn test_detect_dataset_gap() {
        let snapshots = vec![make_paper(
            "2401.00004",
            "There is limited data available for this task. No benchmark exists.",
        )];
        let gaps = analyze_gaps(&snapshots, "test topic");
        assert!(gaps.iter().any(|g| g.gap_type == "dataset_gap"));
    }

    #[test]
    fn test_multiple_papers_increase_severity() {
        let snapshots = vec![
            make_paper("2401.00001", "limitation of current method"),
            make_paper("2401.00002", "another limitation found"),
            make_paper("2401.00003", "this also has a limitation"),
        ];
        let gaps = analyze_gaps(&snapshots, "test topic");
        let ml = gaps.iter().find(|g| g.gap_type == "method_limitation");
        assert!(ml.is_some(), "should detect method_limitation");
        if let Some(g) = ml {
            assert_eq!(g.severity, "high", "3+ papers should be high severity");
        }
    }

    #[test]
    fn test_max_five_gaps() {
        // Use papers that trigger many gap types
        let snapshots = vec![make_paper(
            "2401.00001",
            "limitation however not scalable no benchmark not generalize future work",
        )];
        let gaps = analyze_gaps(&snapshots, "test topic");
        assert!(gaps.len() <= 5, "should return at most 5 gaps, got {}", gaps.len());
    }
}
