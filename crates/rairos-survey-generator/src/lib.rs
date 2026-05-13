use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredGap {
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub gap_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub novelty_score: f64,
    #[serde(default)]
    pub gene_pool_score: f64,
    #[serde(default)]
    pub preference_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GapHistoryStats {
    #[serde(default)]
    pub new: usize,
}

fn severity_rank(s: &str) -> u8 {
    match s {
        "HIGH" => 0,
        "MEDIUM" => 1,
        _ => 2,
    }
}

fn severity_badge(s: &str) -> &str {
    match s {
        "HIGH" => "🔴 HIGH",
        "MEDIUM" => "🟡 MEDIUM",
        "LOW" => "🟢 LOW",
        _ => s,
    }
}

fn build_gap_list(gaps: &[ScoredGap]) -> String {
    let mut sorted = gaps.to_vec();
    sorted.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| {
                b.novelty_score
                    .partial_cmp(&a.novelty_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let lines: Vec<String> = sorted
        .iter()
        .take(15)
        .map(|g| {
            format!(
                "- [{}] ({}) novelty={:.2} gp_score={:.2} | {}",
                g.severity,
                g.gap_type,
                g.novelty_score,
                g.gene_pool_score,
                &g.title[..g.title.len().min(80)]
            )
        })
        .collect();
    if lines.is_empty() {
        "No gaps found.".to_string()
    } else {
        lines.join("\n")
    }
}

fn build_gap_table(gaps: &[ScoredGap]) -> String {
    let mut sorted = gaps.to_vec();
    sorted.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| {
                b.novelty_score
                    .partial_cmp(&a.novelty_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let rows: Vec<String> = sorted
        .iter()
        .take(20)
        .map(|g| {
            let pref = if g.preference_boost { "✓" } else { "" };
            format!(
                "| {} | {} | {:.2} | {:.2} | {} | {} |",
                severity_badge(&g.severity),
                g.gap_type,
                g.novelty_score,
                g.gene_pool_score,
                pref,
                &g.title[..g.title.len().min(70)]
            )
        })
        .collect();
    rows.join("\n")
}

pub fn generate_survey(
    topic: &str,
    scored_gaps: &[ScoredGap],
    papers_analyzed: usize,
    session_id: &str,
    iterations: usize,
    gap_history_stats: Option<&GapHistoryStats>,
    output_dir: Option<&str>,
) -> String {
    let gap_count = scored_gaps.len();
    let new_count = gap_history_stats.map(|s| s.new).unwrap_or(0);

    let mut sev_counts: HashMap<String, usize> = HashMap::new();
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for g in scored_gaps {
        *sev_counts.entry(g.severity.clone()).or_default() += 1;
        *type_counts.entry(g.gap_type.clone()).or_default() += 1;
    }

    let _gap_list_text = build_gap_list(scored_gaps);
    let gap_table = build_gap_table(scored_gaps);

    let type_lines: Vec<String> = {
        let mut v: Vec<_> = type_counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.into_iter()
            .map(|(t, c)| format!("- **{t}**: {c}"))
            .collect()
    };
    let type_dist = if type_lines.is_empty() {
        "—".to_string()
    } else {
        type_lines.join("\n")
    };

    let mut sev_lines = Vec::new();
    for (s, label) in [("HIGH", "🔴 HIGH"), ("MEDIUM", "🟡 MEDIUM"), ("LOW", "🟢 LOW")] {
        if let Some(&c) = sev_counts.get(s) {
            sev_lines.push(format!("- **{label}**: {c}"));
        }
    }
    let sev_dist = if sev_lines.is_empty() {
        "—".to_string()
    } else {
        sev_lines.join("\n")
    };

    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let session_short = if session_id.len() > 8 {
        &session_id[..8]
    } else {
        session_id
    };
    let top_n = scored_gaps.len().min(20);

    let markdown = format!(
        r#"# Research Survey: {topic}

**Generated:** {now}
**Session:** `{session_short}`
**Papers Analyzed:** {papers_analyzed} | **Iterations:** {iterations} | **Gaps Found:** {gap_count} ({new_count} new)

---

## Gap Distribution

### By Severity
{sev_dist}

### By Type
{type_dist}

---

## Gap Details (Top {top_n} by Priority)

| Severity | Type | Novelty | GenePool | Pref | Title |
|----------|------|---------|----------|------|-------|
{gap_table}

---

## Next Steps

1. Review 🔴 HIGH severity gaps — these are the most impactful research opportunities
2. Check GenePool for existing code implementations matching top gaps
3. Run `airos paper trace <topic>` to see paper→code lineage
4. Consider running a focused DeepResearch iteration on the top gap

> _This survey was auto-generated by Rairos AI Research OS_
"#
    );

    let out_dir = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output/surveys"));
    fs::create_dir_all(&out_dir).ok();

    let safe_topic: String = topic
        .chars()
        .map(|c| match c {
            ' ' => '_',
            '/' => '-',
            _ => c,
        })
        .take(50)
        .collect();
    let ts = Local::now().format("%Y%m%d_%H%M").to_string();
    let filename = format!("survey_{safe_topic}_{ts}_{session_short}.md");
    let file_path = out_dir.join(&filename);
    fs::write(&file_path, &markdown).ok();
    file_path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gaps() -> Vec<ScoredGap> {
        vec![
            ScoredGap {
                severity: "HIGH".to_string(),
                gap_type: "capability".to_string(),
                title: "LLMs cannot handle long context efficiently".to_string(),
                novelty_score: 0.85,
                gene_pool_score: 0.72,
                preference_boost: true,
            },
            ScoredGap {
                severity: "MEDIUM".to_string(),
                gap_type: "improvement".to_string(),
                title: "Fine-tuning requires too much data".to_string(),
                novelty_score: 0.45,
                gene_pool_score: 0.30,
                preference_boost: false,
            },
        ]
    }

    #[test]
    fn test_generate_survey_basic() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("surveys");
        let path = generate_survey(
            "Test Topic",
            &sample_gaps(),
            5,
            "sess123",
            2,
            None,
            Some(out.to_str().unwrap()),
        );
        assert!(path.contains("survey_"));
        let content = fs::read_to_string(&path).unwrap_or_default();
        assert!(content.contains("Research Survey"));
        assert!(content.contains("🔴 HIGH"));
        assert!(content.contains("LLMs cannot handle"));
    }

    #[test]
    fn test_build_gap_list() {
        let list = build_gap_list(&sample_gaps());
        assert!(list.contains("[HIGH]"));
        assert!(list.contains("[MEDIUM]"));
    }

    #[test]
    fn test_severity_badge() {
        assert_eq!(severity_badge("HIGH"), "🔴 HIGH");
        assert_eq!(severity_badge("MEDIUM"), "🟡 MEDIUM");
        assert_eq!(severity_badge("LOW"), "🟢 LOW");
    }

    #[test]
    fn test_empty_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("surveys");
        let path = generate_survey("Empty", &[], 0, "", 0, None, Some(out.to_str().unwrap()));
        let content = fs::read_to_string(&path).unwrap_or_default();
        // With empty session_id, the format string produces "Gaps Found: 0 (0 new)"
        assert!(content.contains("0 new"), "content: {content}");
    }
}
