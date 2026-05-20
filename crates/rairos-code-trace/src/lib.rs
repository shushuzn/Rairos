//! rairos-code-trace — Code-Paper Traceability.
//!
//! Ported from `research_loop/code_trace.py` (199 LOC).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A parsed `# source:` comment from generated code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSourceComment {
    pub line_number: u32,
    pub tags: Vec<(String, u32)>,  // e.g. [("eq".to_string(), 0), ("algo".to_string(), 1)]
    pub description: String,
}

/// Extract all `# source:` comments from generated code.
pub fn parse_source_comments(code: &str) -> Vec<ParsedSourceComment> {
    let source_re = Regex::new(r"#\s*source:\s*((?:@(?:\w+)\[\d+\]\s*(?:,\s*)?)+)").expect("valid regex");
    let tag_re = Regex::new(r"@(\w+)\[(\d+)\]").expect("valid regex");

    let mut results = vec![];
    for (lineno, line) in code.lines().enumerate() {
        let lineno = (lineno + 1) as u32;
        if let Some(m) = source_re.captures(line) {
            let caps = m.get(1).unwrap().as_str();
            let tags: Vec<_> = tag_re
                .captures_iter(caps)
                .map(|c| {
                    (
                        c.get(1).unwrap().as_str().to_string(),
                        c.get(2).unwrap().as_str().parse().unwrap_or(0),
                    )
                })
                .collect();

            let desc = if let Some(dash_idx) = line.find('\u{2014}') {
                line[line.char_indices().nth(dash_idx + 1).map(|(p, _)| p).unwrap_or(line.len())..].trim().to_string()
            } else {
                String::new()
            };

            results.push(ParsedSourceComment {
                line_number: lineno,
                tags,
                description: desc,
            });
        }
    }
    results
}

fn resolve_tag(
    tag_type: &str,
    idx: u32,
    sources: &[serde_json::Value],
) -> String {
    for s in sources {
        if s.get("index").and_then(|v| v.as_u64()) == Some(idx as u64) {
            let text = match tag_type {
                "eq" => s.get("equation").or(s.get("text")).map(|v| v.as_str().unwrap_or("")),
                "claim" => s.get("claim").map(|v| v.as_str().unwrap_or("")),
                "algo" => s.get("description").map(|v| v.as_str().unwrap_or("")),
                _ => None,
            };
            if let Some(t) = text {
                let truncated = if t.chars().count() > 80 {
                    t.chars().take(80).collect::<String>()
                } else {
                    t.to_string()
                };
                return truncated;
            }
        }
    }
    format!("[unknown {}[{}]]", tag_type, idx)
}

/// Build paper_section_refs list for CapsuleGene archetype.
/// paper_content is a JSON Value with optional equation_sources, claim_sources, algorithm_sources.
pub fn build_paper_section_refs(
    paper_content: &serde_json::Value,
    parsed_comments: &[ParsedSourceComment],
) -> Vec<serde_json::Value> {
    let equation_sources = paper_content
        .get("equation_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let claim_sources = paper_content
        .get("claim_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let algorithm_sources = paper_content
        .get("algorithm_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let mut refs = vec![];
    for comment in parsed_comments {
        for (tag_type, idx) in &comment.tags {
            let sources = match tag_type.as_str() {
                "eq" => equation_sources,
                "claim" => claim_sources,
                "algo" => algorithm_sources,
                _ => &[],
            };
            refs.push(serde_json::json!({
                "type": tag_type,
                "source_ref": format!("@{} [{}]", tag_type, idx),
                "paper_text": resolve_tag(tag_type, *idx, sources),
                "code_range": [comment.line_number, comment.line_number],
                "confidence": 1.0,
            }));
        }
    }
    refs
}

/// Bidirectional trace between generated code and paper sources.
pub fn code_to_paper_trace(
    code_str: &str,
    paper_content: &serde_json::Value,
) -> serde_json::Value {
    let comments = parse_source_comments(code_str);
    let total_lines = code_str.lines().count() as u32;

    // Map (tag_type, idx) -> list of line numbers
    let mut tag_to_lines: std::collections::HashMap<(String, u32), Vec<u32>> =
        std::collections::HashMap::new();
    for c in &comments {
        for (tag_type, idx) in &c.tags {
            tag_to_lines
                .entry((tag_type.clone(), *idx))
                .or_default()
                .push(c.line_number);
        }
    }

    // Coalesce consecutive line numbers into ranges
    fn coalesce(sorted_lines: &[u32]) -> Vec<(u32, u32)> {
        if sorted_lines.is_empty() {
            return vec![];
        }
        let mut ranges = vec![];
        let mut start = sorted_lines[0];
        let mut prev = sorted_lines[0];
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

    let equation_sources = paper_content
        .get("equation_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let claim_sources = paper_content
        .get("claim_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let algorithm_sources = paper_content
        .get("algorithm_sources")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    // Build forward map
    let mut forward = vec![];
    let mut all_tagged: HashSet<u32> = HashSet::new();

    for ((tag_type, idx), line_nums) in &tag_to_lines {
        let mut sorted_nums = line_nums.clone();
        sorted_nums.sort_unstable();
        let ranges = coalesce(&sorted_nums);
        all_tagged.extend(line_nums);

        let sources = match tag_type.as_str() {
            "eq" => equation_sources,
            "claim" => claim_sources,
            "algo" => algorithm_sources,
            _ => &[],
        };

        let mut paper_text = String::new();
        let mut location_info = String::new();
        for s in sources {
            if s.get("index").and_then(|v| v.as_u64()) == Some(*idx as u64) {
                paper_text = match tag_type.as_str() {
                    "eq" => s.get("equation").or(s.get("text")).map(|v| v.as_str().unwrap_or("")).unwrap_or("").to_string(),
                    "claim" => s.get("claim").map(|v| v.as_str().unwrap_or("")).unwrap_or("").to_string(),
                    "algo" => s.get("description").map(|v| v.as_str().unwrap_or("")).unwrap_or("").to_string(),
                    _ => String::new(),
                };
                if paper_text.chars().count() > 80 {
                    paper_text = paper_text.chars().take(80).collect();
                }
                if let (Some(section), Some(page)) = (
                    s.get("location").and_then(|l| l.get("section")).and_then(|v| v.as_str()),
                    s.get("location").and_then(|l| l.get("page")).and_then(|v| v.as_u64()),
                ) {
                    location_info = format!("\u{a7}{} p{}", section, page);
                }
                break;
            }
        }

        forward.push(serde_json::json!({
            "source_ref": format!("@{} [{}]", tag_type, idx),
            "code_ranges": ranges,
            "paper_text": paper_text,
            "location": location_info,
        }));
    }

    // Find untagged line ranges
    let mut untagged = vec![];
    if !all_tagged.is_empty() {
        let mut sorted_tagged: Vec<_> = all_tagged.iter().copied().collect();
        sorted_tagged.sort_unstable();
        if sorted_tagged[0] > 1 {
            untagged.push(vec![1, sorted_tagged[0] - 1]);
        }
        for i in 0..sorted_tagged.len().saturating_sub(1) {
            let gap_s = sorted_tagged[i] + 1;
            let gap_e = sorted_tagged[i + 1].saturating_sub(1);
            if gap_s <= gap_e {
                untagged.push(vec![gap_s, gap_e]);
            }
        }
        if sorted_tagged.last().copied().unwrap_or(0) < total_lines {
            untagged.push(vec![sorted_tagged.last().copied().unwrap_or(0) + 1, total_lines]);
        }
    } else if total_lines > 0 {
        untagged.push(vec![1, total_lines]);
    }

    // Find unreferenced paper sources
    let mut unreferenced = vec![];
    for s in equation_sources {
        if let (Some(idx_val), Some(text_val)) = (s.get("index"), s.get("equation")) {
            let idx = idx_val.as_u64().unwrap_or(0) as u32;
            let text = text_val.as_str().unwrap_or("").chars().take(60).collect::<String>();
            if !tag_to_lines.contains_key(&("eq".to_string(), idx)) {
                unreferenced.push(serde_json::json!(["eq", idx, text]));
            }
        }
    }
    for s in claim_sources {
        if let (Some(idx_val), Some(text_val)) = (s.get("index"), s.get("claim")) {
            let idx = idx_val.as_u64().unwrap_or(0) as u32;
            let text = text_val.as_str().unwrap_or("").chars().take(60).collect::<String>();
            if !tag_to_lines.contains_key(&("claim".to_string(), idx)) {
                unreferenced.push(serde_json::json!(["claim", idx, text]));
            }
        }
    }
    for s in algorithm_sources {
        if let (Some(idx_val), Some(text_val)) = (s.get("index"), s.get("description")) {
            let idx = idx_val.as_u64().unwrap_or(0) as u32;
            let text = text_val.as_str().unwrap_or("").chars().take(60).collect::<String>();
                if !tag_to_lines.contains_key(&("algo".to_string(), idx)) {
                    unreferenced.push(serde_json::json!(["algo", idx, text]));
            }
        }
    }

    serde_json::json!({
        "forward": forward,
        "untagged_ranges": untagged,
        "unreferenced_sources": unreferenced,
        "total_tagged_lines": all_tagged.len(),
        "total_code_lines": total_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_comments_simple() {
        let code = r#"
# source: @eq[0] — Attention mechanism
x = self.attention(x)
# source: @algo[1], @claim[2]
y = self.compute(x)
"#;
        let results = parse_source_comments(code);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 2);
        assert_eq!(results[0].tags, vec![("eq".to_string(), 0)]);
        assert_eq!(results[0].description, "Attention mechanism");
        assert_eq!(results[1].line_number, 4);
        assert_eq!(results[1].tags, vec![("algo".to_string(), 1), ("claim".to_string(), 2)]);
    }

    #[test]
    fn test_parse_source_comments_no_tags() {
        let code = "x = self.attention(x)\ny = self.compute(x)";
        let results = parse_source_comments(code);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_code_to_paper_trace_basic() {
        let code = r#"# source: @eq[0] — Attention
x = self.attention(x)
"#;
        let paper_content = serde_json::json!({
            "equation_sources": [
                {"index": 0, "equation": "E = mc^2", "location": {"section": "1", "page": 1}}
            ],
            "claim_sources": [],
            "algorithm_sources": []
        });
        let result = code_to_paper_trace(code, &paper_content);
        assert_eq!(result["total_tagged_lines"], 1);
        assert_eq!(result["total_code_lines"], 2);
        assert_eq!(result["forward"][0]["source_ref"], "@eq [0]");
    }

    #[test]
    fn test_build_paper_section_refs() {
        let comments = vec![ParsedSourceComment {
            line_number: 5,
            tags: vec![("eq".to_string(), 0), ("claim".to_string(), 1)],
            description: "Test".to_string(),
        }];
        let paper_content = serde_json::json!({
            "equation_sources": [{"index": 0, "equation": "E = mc^2"}],
            "claim_sources": [{"index": 1, "claim": "Test claim"}],
            "algorithm_sources": []
        });
        let refs = build_paper_section_refs(&paper_content, &comments);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0]["type"], "eq");
        assert_eq!(refs[1]["type"], "claim");
        assert_eq!(refs[0]["confidence"], 1.0);
    }
}
