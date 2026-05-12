//! rairos-code-trace — Code-paper traceability via provenance comments.
//!
//! Ported from `research_loop/code_trace.py`.
//!
//! Parses `# source:` provenance comments in generated code and builds
//! bidirectional traces to paper sections (equations, claims, algorithms).
//!
//! The main entry point is [`parse_source_comments`] which extracts provenance
//! from code. For full bidirectional tracing, see [`trace_code_to_paper`] which
//! requires structured paper source data (equation_sources, claim_sources,
//! algorithm_sources).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

// Global pre-compiled regexes (thread-safe, compiled once on first access)
static SOURCE_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\s*source:\s*((?:@(?:\w+)\[\d+\]\s*(?:,\s*)?)+)").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@(\w+)\[(\d+)\]").unwrap());

// ─── Data structures ────────────────────────────────────────────────────────────

/// A parsed `# source:` comment from generated code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSourceComment {
    pub line_number: usize,
    /// e.g. [("eq", 0), ("algo", 1)]
    pub tags: Vec<(String, usize)>,
    pub description: String,
}

/// A resolved paper source reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSourceRef {
    pub source_type: String,
    pub source_ref: String,
    pub paper_text: String,
    pub code_range_start: usize,
    pub code_range_end: usize,
    pub confidence: f64,
}

/// Line range (inclusive start, inclusive end).
pub type LineRange = (usize, usize);

// ─── Core parsing ─────────────────────────────────────────────────────────────

/// Extract all `# source:` comments from generated code.
///
/// Each comment may contain multiple tags like `@eq[0]`, `@claim[3]`.
pub fn parse_source_comments(code: &str) -> Vec<ParsedSourceComment> {
    let mut results = Vec::new();

    for (lineno, line) in code.lines().enumerate() {
        let lineno = lineno + 1; // 1-indexed

        let Some(caps) = SOURCE_COMMENT_RE.captures(line) else {
            continue;
        };

        let full_match = caps.get(1).map(|m| m.as_str()).unwrap_or("");

        let tags: Vec<(String, usize)> = TAG_RE
            .captures_iter(full_match)
            .filter_map(|cap| {
                let tag = cap.get(1).map(|m| m.as_str().to_string())?;
                let idx: usize = cap.get(2).and_then(|m| m.as_str().parse().ok())?;
                Some((tag, idx))
            })
            .collect();

        let description = match line.find('—') {
            Some(_) => line
                .chars()
                .skip_while(|&c| c != '—')
                .skip(1)
                .collect::<String>()
                .trim()
                .to_string(),
            None => String::new(),
        };

        results.push(ParsedSourceComment {
            line_number: lineno,
            tags,
            description,
        });
    }

    results
}

/// Parse a single source comment line, returning tags if it matches the pattern.
pub fn parse_source_tag_line(line: &str) -> Option<Vec<(String, usize)>> {
    let caps = SOURCE_COMMENT_RE.captures(line)?;
    let full_match = caps.get(1).map(|m| m.as_str())?;
    let tags: Vec<(String, usize)> = TAG_RE
        .captures_iter(full_match)
        .filter_map(|cap| {
            let tag = cap.get(1).map(|m| m.as_str().to_string())?;
            let idx: usize = cap.get(2).and_then(|m| m.as_str().parse().ok())?;
            Some((tag, idx))
        })
        .collect();
    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}

// ─── Line range utilities ──────────────────────────────────────────────────────

/// Coalesce a sorted list of line numbers into ranges of consecutive lines.
///
/// # Example
/// ```
/// use rairos_code_trace::coalesce_lines;
/// assert_eq!(coalesce_lines(&[1, 2, 3, 5, 6, 8]), vec![(1, 3), (5, 6), (8, 8)]);
/// assert_eq!(coalesce_lines(&[]), vec![]);
/// ```
pub fn coalesce_lines(sorted_lines: &[usize]) -> Vec<LineRange> {
    if sorted_lines.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = sorted_lines[0];
    let mut prev = start;

    for &cur in &sorted_lines[1..] {
        if cur == prev + 1 {
            prev = cur;
        } else {
            ranges.push((start, prev));
            start = cur;
            prev = cur;
        }
    }
    ranges.push((start, prev));
    ranges
}

/// Build a forward map: for each (tag_type, idx) → code ranges.
///
/// Takes parsed comments and returns a map from (type, idx) to line numbers.
pub fn build_tag_to_lines(comments: &[ParsedSourceComment]) -> Vec<((String, usize), Vec<usize>)> {
    let mut map: Vec<((String, usize), Vec<usize>)> = Vec::new();

    for c in comments {
        for (tag_type, idx) in &c.tags {
            map.push(((tag_type.clone(), *idx), vec![c.line_number]));
        }
    }

    // Merge entries for the same (tag_type, idx)
    let mut merged: std::collections::HashMap<(String, usize), Vec<usize>> =
        std::collections::HashMap::new();
    for ((tag_type, idx), lines) in map {
        merged.entry((tag_type, idx)).or_default().extend(lines);
    }

    let mut result: Vec<_> = merged
        .into_iter()
        .map(|((t, i), lines)| {
            let mut sorted = lines.clone();
            sorted.sort_unstable();
            ((t, i), sorted)
        })
        .collect();

    result.sort_by_key(|((t, i), _)| (t.clone(), *i));
    result
}

// ─── Paper source resolution ───────────────────────────────────────────────────

/// Structured source item from a paper (equation, claim, or algorithm).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSourceItem {
    pub index: usize,
    pub text: String,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub section: String,
    pub page: usize,
}

/// Paper content with named source arrays.
///
/// Provide this to [`trace_code_to_paper`] to enable full bidirectional tracing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaperContent {
    #[serde(default)]
    pub equation_sources: Vec<PaperSourceItem>,
    #[serde(default)]
    pub claim_sources: Vec<PaperSourceItem>,
    #[serde(default)]
    pub algorithm_sources: Vec<PaperSourceItem>,
}

impl PaperContent {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set equation sources from a JSON-like list of `{index, text, location}`.
    pub fn with_equations(mut self, eqs: Vec<PaperSourceItem>) -> Self {
        self.equation_sources = eqs;
        self
    }

    /// Set claim sources.
    pub fn with_claims(mut self, claims: Vec<PaperSourceItem>) -> Self {
        self.claim_sources = claims;
        self
    }

    /// Set algorithm sources.
    pub fn with_algorithms(mut self, algos: Vec<PaperSourceItem>) -> Self {
        self.algorithm_sources = algos;
        self
    }

    fn resolve(&self, tag_type: &str, idx: usize) -> String {
        let sources = match tag_type {
            "eq" => &self.equation_sources,
            "claim" => &self.claim_sources,
            "algo" => &self.algorithm_sources,
            _ => return format!("[unknown {tag_type}[{idx}]]"),
        };

        for s in sources {
            if s.index == idx {
                return s.text.chars().take(80).collect::<String>();
            }
        }
        format!("[unknown {tag_type}[{idx}]]")
    }

    fn location_info(&self, tag_type: &str, idx: usize) -> String {
        let sources = match tag_type {
            "eq" => &self.equation_sources,
            "claim" => &self.claim_sources,
            "algo" => &self.algorithm_sources,
            _ => return String::new(),
        };
        for s in sources {
            if s.index == idx {
                if let Some(ref loc) = s.location {
                    return format!("§{} p{}", loc.section, loc.page);
                }
            }
        }
        String::new()
    }
}

/// Full bidirectional trace result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePaperTrace {
    pub forward: Vec<ForwardRef>,
    pub untagged_ranges: Vec<LineRange>,
    pub unreferenced_sources: Vec<(String, usize, String)>,
    pub total_tagged_lines: usize,
    pub total_code_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRef {
    pub source_type: String,
    pub source_ref: String,
    pub paper_text: String,
    pub code_ranges: Vec<LineRange>,
    pub location: String,
}

/// Build bidirectional trace between generated code and paper sources.
///
/// Returns which lines of code came from which paper sections,
/// which code lines have no provenance, and which paper sources are unreferenced.
pub fn trace_code_to_paper(code: &str, paper: &PaperContent) -> CodePaperTrace {
    let comments = parse_source_comments(code);
    let lines: Vec<&str> = code.lines().collect();
    let total_lines = lines.len();

    let tag_map = build_tag_to_lines(&comments);

    // Build forward map
    let mut forward = Vec::new();
    for ((tag_type, idx), sorted_lines) in &tag_map {
        let ranges = coalesce_lines(sorted_lines);
        let paper_text = paper.resolve(tag_type, *idx);
        let location = paper.location_info(tag_type, *idx);
        let source_ref = format!("@{tag_type}[{idx}]");

        forward.push(ForwardRef {
            source_type: tag_type.clone(),
            source_ref,
            paper_text,
            code_ranges: ranges,
            location,
        });
    }

    // Find untagged line ranges
    let mut all_tagged: HashSet<usize> = HashSet::new();
    for (_, lines) in &tag_map {
        for &ln in lines {
            all_tagged.insert(ln);
        }
    }

    let untagged_ranges: Vec<LineRange> = if all_tagged.is_empty() {
        if total_lines > 0 {
            vec![(1, total_lines)]
        } else {
            vec![]
        }
    } else {
        let mut ranges = Vec::new();
        let mut sorted: Vec<usize> = all_tagged.into_iter().collect();
        sorted.sort_unstable();

        if sorted[0] > 1 {
            ranges.push((1, sorted[0] - 1));
        }
        for window in sorted.windows(2) {
            let gap_s = window[0] + 1;
            let gap_e = window[1] - 1;
            if gap_s <= gap_e {
                ranges.push((gap_s, gap_e));
            }
        }
        if sorted[sorted.len() - 1] < total_lines {
            ranges.push((sorted[sorted.len() - 1] + 1, total_lines));
        }
        ranges
    };

    // Find unreferenced sources
    let mut unreferenced = Vec::new();
    for s in &paper.equation_sources {
        let key = ("eq".to_string(), s.index);
        if !tag_map.iter().any(|((t, i), _)| t == &key.0 && i == &key.1) {
            unreferenced.push(("eq".to_string(), s.index, s.text.chars().take(60).collect()));
        }
    }
    for s in &paper.claim_sources {
        let key = ("claim".to_string(), s.index);
        if !tag_map.iter().any(|((t, i), _)| t == &key.0 && i == &key.1) {
            unreferenced.push((
                "claim".to_string(),
                s.index,
                s.text.chars().take(60).collect(),
            ));
        }
    }
    for s in &paper.algorithm_sources {
        let key = ("algo".to_string(), s.index);
        if !tag_map.iter().any(|((t, i), _)| t == &key.0 && i == &key.1) {
            unreferenced.push((
                "algo".to_string(),
                s.index,
                s.text.chars().take(60).collect(),
            ));
        }
    }

    let total_tagged_lines = forward
        .iter()
        .flat_map(|f| f.code_ranges.iter())
        .map(|(s, e)| *s..=*e)
        .flatten()
        .collect::<HashSet<_>>()
        .len();

    CodePaperTrace {
        forward,
        untagged_ranges,
        unreferenced_sources: unreferenced,
        total_tagged_lines,
        total_code_lines: total_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_comments_basic() {
        let code = r#"
# source: @eq[0] — Attention from §3.2
x = F.multi_head_attention(query, key, value)
# source: @algo[1]
def transformer_layer(x):
"#;
        let comments = parse_source_comments(code);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].line_number, 2);
        assert_eq!(comments[0].tags, &[("eq".to_string(), 0)]);
        assert_eq!(comments[0].description, "Attention from §3.2");
        assert_eq!(comments[1].line_number, 4);
        assert_eq!(comments[1].tags, &[("algo".to_string(), 1)]);
    }

    #[test]
    fn test_parse_source_comments_multiple_tags() {
        let code = "# source: @eq[0], @claim[1], @algo[2]\n";
        let comments = parse_source_comments(code);
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments[0].tags,
            &[
                ("eq".to_string(), 0),
                ("claim".to_string(), 1),
                ("algo".to_string(), 2)
            ]
        );
    }

    #[test]
    fn test_parse_source_comments_empty() {
        let code = "def foo():\n    x = 1\n";
        let comments = parse_source_comments(code);
        assert!(comments.is_empty());
    }

    #[test]
    fn test_coalesce_lines_basic() {
        assert_eq!(
            coalesce_lines(&[1, 2, 3, 5, 6, 8]),
            vec![(1, 3), (5, 6), (8, 8)]
        );
        assert_eq!(coalesce_lines(&[1, 2, 3]), vec![(1, 3)]);
        assert_eq!(coalesce_lines(&[5]), vec![(5, 5)]);
        assert_eq!(coalesce_lines(&[]), vec![]);
    }

    #[test]
    fn test_coalesce_lines_disconnected() {
        assert_eq!(
            coalesce_lines(&[1, 3, 5, 7]),
            vec![(1, 1), (3, 3), (5, 5), (7, 7)]
        );
    }

    #[test]
    fn test_build_tag_to_lines() {
        let comments = vec![
            ParsedSourceComment {
                line_number: 2,
                tags: vec![("eq".to_string(), 0)],
                description: "".to_string(),
            },
            ParsedSourceComment {
                line_number: 4,
                tags: vec![("eq".to_string(), 0), ("claim".to_string(), 1)],
                description: "".to_string(),
            },
        ];
        let result = build_tag_to_lines(&comments);
        assert_eq!(result.len(), 2);
        // eq[0] appears on lines 2 and 4
        assert_eq!(
            result
                .iter()
                .find(|((t, i), _)| t == "eq" && *i == 0)
                .map(|(_, l)| l.as_slice()),
            Some(&vec![2, 4][..])
        );
    }

    #[test]
    fn test_trace_code_to_paper_forward() {
        let code = "# source: @eq[0]\nx = attention(q, k, v)\n";
        let paper = PaperContent::new().with_equations(vec![PaperSourceItem {
            index: 0,
            text: "Attention from Vaswani et al.".to_string(),
            location: None,
        }]);

        let trace = trace_code_to_paper(code, &paper);
        assert_eq!(trace.total_code_lines, 2);
        assert_eq!(trace.forward.len(), 1);
        assert_eq!(trace.forward[0].source_ref, "@eq[0]");
        assert_eq!(trace.forward[0].paper_text, "Attention from Vaswani et al.");
    }

    #[test]
    fn test_trace_code_to_paper_untagged() {
        let code = "# source: @eq[0]\nx = 1\ny = 2\n";
        let paper = PaperContent::new().with_equations(vec![PaperSourceItem {
            index: 0,
            text: "Eq 0".to_string(),
            location: None,
        }]);

        let trace = trace_code_to_paper(code, &paper);
        // Line 1 is tagged, lines 2-3 are untagged
        assert!(trace.untagged_ranges.contains(&(2, 3)));
    }

    #[test]
    fn test_trace_code_to_paper_unreferenced() {
        let code = "# source: @eq[0]\nx = 1\n";
        let paper = PaperContent::new()
            .with_equations(vec![
                PaperSourceItem {
                    index: 0,
                    text: "Used".to_string(),
                    location: None,
                },
                PaperSourceItem {
                    index: 1,
                    text: "Not used".to_string(),
                    location: None,
                },
            ])
            .with_claims(vec![PaperSourceItem {
                index: 0,
                text: "Also not used".to_string(),
                location: None,
            }]);

        let trace = trace_code_to_paper(code, &paper);
        // eq[1] and claim[0] are unreferenced
        assert!(trace
            .unreferenced_sources
            .iter()
            .any(|(t, i, _)| t == "eq" && *i == 1));
        assert!(trace
            .unreferenced_sources
            .iter()
            .any(|(t, i, _)| t == "claim" && *i == 0));
    }
}
