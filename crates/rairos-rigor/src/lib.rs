//! Rairos Rigor — Research Rigor Scorer
//!
//! Rates papers by methodology transparency, reproducibility signals, and
//! dataset/code sharing indicators. Returns a RigorScore badge (A/B/C/D).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const CODE_SIGNALS: &[&str] = &[
    r"github\.com/[a-zA-Z0-9_\-]+/[a-zA-Z0-9_\-]+",
    r"https?://github\.com",
    r"code\s+(?:available|at|on)\s+github",
    r"implementation\s+(?:available|at|on|on\s+github)",
    r"open\s+source",
    r"repository\s+(?:available|on)",
    r"supplementary\s+code",
    r"bit\.ly/\w+",
];

const DATASET_SIGNALS: &[&str] = &[
    r"dataset\s+(?:available|at|from|from\s+the|from\s+authors?)",
    r"data\s+(?:available|at|from|upon\s+request)",
    r"benchmark\s+(?:dataset|data)",
    r"download\s+(?:dataset|data)",
    r"http[s]?://[^\s]*(?:dataset|data\.csv|data\.json|data\.zip)",
    r"zenodo",
    r"figshare",
    r"dryad",
    r"osf\.io",
    r"kaggle\.com",
    r"huggingface\.co/(?:datasets|spaces)",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigorScore {
    pub paper_id: String,
    pub overall: f64,
    pub has_code: bool,
    pub has_dataset: bool,
    pub methodology_clarity: String,
    pub reproducibility_signals: Vec<String>,
    pub badge: String,
}

pub struct RigorScorer {
    code_patterns: Vec<Regex>,
    dataset_patterns: Vec<Regex>,
}

impl Default for RigorScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl RigorScorer {
    pub fn new() -> Self {
        let code_patterns = CODE_SIGNALS
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();
        let dataset_patterns = DATASET_SIGNALS
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();
        Self {
            code_patterns,
            dataset_patterns,
        }
    }

    pub fn fast_scan(&self, text: &str) -> (bool, bool, Vec<String>) {
        let has_code = self.code_patterns.iter().any(|p| p.is_match(text));
        let has_dataset = self.dataset_patterns.iter().any(|p| p.is_match(text));

        let mut signals = Vec::new();
        if has_code {
            signals.push("Code/GitHub mentioned".to_string());
        }
        if has_dataset {
            signals.push("Dataset mentioned".to_string());
        }

        (has_code, has_dataset, signals)
    }

    fn compute_badge(has_code: bool, has_dataset: bool, clarity: &str) -> String {
        let mut score = 0;
        if has_code {
            score += 1;
        }
        if has_dataset {
            score += 1;
        }
        match clarity {
            "high" => score += 1,
            "low" => score -= 1,
            _ => {}
        }

        if score >= 3 {
            "A".to_string()
        } else if score == 2 {
            "B".to_string()
        } else if score == 1 {
            "C".to_string()
        } else {
            "D".to_string()
        }
    }

    pub fn score_paper(&self, paper_id: &str, abstract_text: &str, title: &str) -> RigorScore {
        let text = format!("{}\n\n{}", title, abstract_text);

        let (has_code, has_dataset, signals) = self.fast_scan(&text);

        let clarity = if has_code && has_dataset {
            "high"
        } else if has_code || has_dataset {
            "medium"
        } else {
            "low"
        };

        let mut overall: f64 = 0.0;
        if has_code {
            overall += 0.35;
        }
        if has_dataset {
            overall += 0.35;
        }
        match clarity {
            "high" => overall += 0.30,
            "medium" => overall += 0.15,
            _ => {}
        }

        let badge = Self::compute_badge(has_code, has_dataset, clarity);
        let overall = (overall * 100.0).round() / 100.0;

        RigorScore {
            paper_id: paper_id.to_string(),
            overall,
            has_code,
            has_dataset,
            methodology_clarity: clarity.to_string(),
            reproducibility_signals: signals,
            badge,
        }
    }

    pub fn render_badge_html(&self, score: &RigorScore) -> String {
        let colors: HashMap<&str, &str> = [
            ("A", "#7A9E7A"),
            ("B", "#6B8FB5"),
            ("C", "#D4A84B"),
            ("D", "#C4706A"),
        ]
        .into_iter()
        .collect();

        let color = colors.get(score.badge.as_str()).unwrap_or(&"#888");
        let clarity_labels: HashMap<&str, &str> =
            [("high", "High"), ("medium", "Medium"), ("low", "Low")]
                .into_iter()
                .collect();

        let signals_html = if score.reproducibility_signals.is_empty() {
            "No signals detected".to_string()
        } else {
            score
                .reproducibility_signals
                .iter()
                .map(|s| format!("• {}", s))
                .collect::<Vec<_>>()
                .join("<br>")
        };

        let clarity_label = clarity_labels
            .get(score.methodology_clarity.as_str())
            .unwrap_or(&"?");

        let overall_pct = (score.overall * 100.0).round() as i32;
        let code_icon = if score.has_code { "✓" } else { "✗" };
        let dataset_icon = if score.has_dataset { "✓" } else { "✗" };

        format!(
            r#"<span class="rigor-badge" style="
    display: inline-block;
    background: {0};
    color: white;
    font-family: 'Caveat', cursive;
    font-size: 1.1em;
    font-weight: 700;
    width: 2em;
    height: 2em;
    line-height: 2em;
    text-align: center;
    border-radius: 6px;
    cursor: help;
" title="code: {1} | dataset: {2} | clarity: {5}">
    {3}
</span>
<div class="rigor-tooltip" style="display:none; position: absolute; background:#2a2a2a; color:#e8e4de; padding:10px; border-radius:6px; font-size:13px; max-width:260px; z-index:100; font-family:Lora,serif;">
    <strong style="color:{0}">Rigor: {3}</strong>
    <hr style="border-color:#555; margin:6px 0">
    <div>Overall: {4}%</div>
    <div>Code shared: {1}</div>
    <div>Dataset shared: {2}</div>
    <div>Methodology: {5}</div>
    <hr style="border-color:#555; margin:6px 0">
    <div style="color:#aaa">Signals:</div>
    <div>{6}</div>
</div>"#,
            color, code_icon, dataset_icon, score.badge, overall_pct, clarity_label, signals_html,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_scan_with_code() {
        let scorer = RigorScorer::new();
        let text = "We release our code at https://github.com/author/repo";
        let (has_code, has_dataset, signals) = scorer.fast_scan(text);
        assert!(has_code);
        assert!(!has_dataset);
        assert!(signals.contains(&"Code/GitHub mentioned".to_string()));
    }

    #[test]
    fn test_fast_scan_with_dataset() {
        let scorer = RigorScorer::new();
        let text = "Dataset available at https://zenodo.org/record/123";
        let (has_code, has_dataset, signals) = scorer.fast_scan(text);
        assert!(!has_code);
        assert!(has_dataset);
        assert!(signals.contains(&"Dataset mentioned".to_string()));
    }

    #[test]
    fn test_fast_scan_none() {
        let scorer = RigorScorer::new();
        let text = "We propose a novel transformer architecture.";
        let (has_code, has_dataset, signals) = scorer.fast_scan(text);
        assert!(!has_code);
        assert!(!has_dataset);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_compute_badge() {
        assert_eq!(RigorScorer::compute_badge(true, true, "high"), "A");
        assert_eq!(RigorScorer::compute_badge(true, false, "high"), "B");
        assert_eq!(RigorScorer::compute_badge(true, true, "low"), "C");
        assert_eq!(RigorScorer::compute_badge(false, false, "medium"), "D");
        assert_eq!(RigorScorer::compute_badge(false, false, "low"), "D");
    }

    #[test]
    fn test_score_paper() {
        let scorer = RigorScorer::new();
        let score = scorer.score_paper(
            "p1",
            "We release code at github.com/test/repo and dataset at zenodo.",
            "Test Paper",
        );
        assert_eq!(score.paper_id, "p1");
        assert!(score.has_code);
        assert!(score.has_dataset);
        assert_eq!(score.badge, "A");
    }

    #[test]
    fn test_render_badge_html() {
        let scorer = RigorScorer::new();
        let score = scorer.score_paper("p1", "Code at github.com/test/repo", "Test");
        let html = scorer.render_badge_html(&score);
        assert!(html.contains("rigor-badge"));
        assert!(html.contains(&score.badge));
    }
}
