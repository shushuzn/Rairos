//! rairos-distributor — Briefing Distributor
//!
//! Audience-specific research digests and shareable short links.
//! Supports: phd_advisor, industry_engineer, policy_maker, researcher
//!
//! Ported from `llm/briefing_distributor.py`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const SHORTCODE_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn briefings_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("briefings")
}

fn links_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("briefing_links.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingLink {
    pub arxiv_id: String,
    pub title: String,
    pub audience: String,
    pub created_at: String,
    pub clicks: usize,
}

fn load_links() -> HashMap<String, BriefingLink> {
    let path = links_file();
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

fn save_links(links: &HashMap<String, BriefingLink>) {
    let path = links_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(links) {
        let _ = fs::write(&path, json);
    }
}

pub fn make_short_id(title: &str, arxiv_id: &str) -> String {
    let raw = format!("{}:{}", arxiv_id, &title[..title.len().min(30)]);
    let hash = Sha256::digest(raw.as_bytes());
    (0..6)
        .map(|i| {
            let idx = hash[i] as usize % SHORTCODE_CHARS.len();
            SHORTCODE_CHARS[idx] as char
        })
        .collect()
}

pub fn create_share_link(arxiv_id: &str, title: &str, audience: &str) -> String {
    let mut links = load_links();
    let short_id = make_short_id(title, arxiv_id);
    let now = chrono::Utc::now().to_rfc3339();
    links.insert(
        short_id.clone(),
        BriefingLink {
            arxiv_id: arxiv_id.to_string(),
            title: title.to_string(),
            audience: audience.to_string(),
            created_at: now,
            clicks: 0,
        },
    );
    save_links(&links);
    short_id
}

pub fn get_latest_briefing_markdown(arxiv_id: &str) -> Option<String> {
    let dir = briefings_dir();
    if !dir.exists() {
        return None;
    }
    let prefix = arxiv_id;
    let entries: Vec<_> = fs::read_dir(&dir).ok()?.collect();
    let mut matches: Vec<PathBuf> = entries
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().contains(prefix))
        .filter(|p| p.to_string_lossy().contains("briefing"))
        .collect();
    matches.sort_by_key(|p| std::cmp::Reverse(p.clone()));
    matches
        .into_iter()
        .next()
        .and_then(|p| fs::read_to_string(&p).ok())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSections {
    #[serde(flatten)]
    pub sections: HashMap<String, String>,
    pub title: Option<String>,
    pub body: Option<String>,
}

pub fn parse_markdown_sections(md: &str) -> ParsedSections {
    let mut sections: HashMap<String, String> = HashMap::new();
    let mut current = "header".to_string();
    let mut body_lines: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    let mut body: Option<String> = None;

    for line in md.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("## ") {
            if !body_lines.is_empty() || current != "header" {
                let key = current.to_lowercase().replace(' ', "_");
                sections.insert(key, body_lines.join("\n").trim().to_string());
                body_lines = Vec::new();
            }
            current = stripped.trim().to_string();
        } else if let Some(stripped) = line.strip_prefix("# ") {
            title = Some(stripped.trim().to_string());
        } else {
            body_lines.push(line.to_string());
        }
    }

    if !body_lines.is_empty() {
        let key = current.to_lowercase().replace(' ', "_");
        if key == "header" {
            body = Some(body_lines.join("\n").trim().to_string());
        } else {
            sections.insert(key, body_lines.join("\n").trim().to_string());
        }
    }

    ParsedSections {
        sections,
        title,
        body,
    }
}

fn escape_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

fn section_html(heading: &str, content: &str) -> String {
    let truncated = if content.len() > 300 {
        &content[..300]
    } else {
        content
    };
    format!(
        "<div class='digest-section'><h4>{}</h4><p>{}</p></div>",
        heading, truncated
    )
}

fn render_phd_advisor(sections: &ParsedSections, raw: &str) -> String {
    let summary = sections
        .sections
        .get("summary")
        .map(|s| s.as_str())
        .unwrap_or(&raw[..400.min(raw.len())]);
    let methodology = sections
        .sections
        .get("methodology")
        .map(|s| s.as_str())
        .unwrap_or("");
    let gaps = sections
        .sections
        .get("research_gaps")
        .map(|s| s.as_str())
        .unwrap_or("");

    format!(
        "{}{}{}",
        section_html("📚 Paper Summary", summary),
        section_html("🔬 Methodology Assessment", methodology),
        section_html("❓ Open Questions for Student", gaps)
    )
}

fn render_industry_engineer(sections: &ParsedSections, raw: &str) -> String {
    let summary = sections
        .sections
        .get("summary")
        .map(|s| s.as_str())
        .unwrap_or(&raw[..200.min(raw.len())]);
    let methodology = sections
        .sections
        .get("methodology")
        .map(|s| s.as_str())
        .unwrap_or("");
    let experiments = sections
        .sections
        .get("experiments")
        .map(|s| s.as_str())
        .unwrap_or("");

    format!(
        "{}{}{}",
        section_html("⚡ What It Does", summary),
        section_html("🛠️ Implementation Signals", methodology),
        section_html("📊 Benchmark / Compute", experiments)
    )
}

fn render_policy_maker(sections: &ParsedSections, raw: &str) -> String {
    let summary = sections
        .sections
        .get("summary")
        .map(|s| s.as_str())
        .unwrap_or(&raw[..300.min(raw.len())]);
    let limitations = sections
        .sections
        .get("limitations")
        .map(|s| s.as_str())
        .unwrap_or("");

    format!(
        "{}{}",
        section_html("🏛️ What This Means", summary),
        section_html("⚠️ Risks & Concerns", limitations)
    )
}

fn render_researcher(sections: &ParsedSections, raw: &str) -> String {
    let v = sections
        .sections
        .get("verdict")
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "neutral".to_string());
    let (badge_text, badge_cls) = match v.as_str() {
        "validates" => ("✅ Validates", "verdict-validates"),
        "contradicts" => ("❌ Contradicts", "verdict-contradicts"),
        _ => ("➖ Neutral", "verdict-neutral"),
    };

    let body = sections.body.as_deref().unwrap_or(raw);
    let summary = sections
        .sections
        .get("summary")
        .map(|s| s.as_str())
        .unwrap_or(&body[..400.min(body.len())]);
    let gaps = sections
        .sections
        .get("research_gaps")
        .or(sections.sections.get("gaps"))
        .map(|s| s.as_str())
        .unwrap_or("");

    let mut result = format!(
        "<span class='verdict-badge {}'>{}</span><p style='margin-top:8px'>{}</p>",
        badge_cls, badge_text, summary
    );

    if !gaps.is_empty() {
        result.push_str(&section_html("🎯 Research Gaps", gaps));
    }
    result
}

pub fn render_distributed_briefing(
    arxiv_id: &str,
    title: &str,
    markdown: &str,
    audience: &str,
) -> String {
    let label = match audience {
        "phd_advisor" => "🎓 PhD Advisor Digest",
        "industry_engineer" => "⚙️ Industry Engineer Digest",
        "policy_maker" => "🏛️ Policy Maker Digest",
        _ => "🔬 Researcher Digest",
    };

    let short_id = create_share_link(arxiv_id, title, audience);
    let sections = parse_markdown_sections(markdown);

    let body_content = match audience {
        "phd_advisor" => render_phd_advisor(&sections, markdown),
        "industry_engineer" => render_industry_engineer(&sections, markdown),
        "policy_maker" => render_policy_maker(&sections, markdown),
        _ => render_researcher(&sections, markdown),
    };

    let escaped_markdown = escape_html(&markdown[..markdown.len().min(2000)]);

    let style = ".briefing-dist { font-family: Georgia, serif; max-width: 800px; } \
                 .digest-section { margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid #e8e4dc; } \
                 .digest-section h4 { font-size: 13px; font-weight: 700; color: #2a4a6a; margin-bottom: 6px; } \
                 .digest-section p { font-size: 13px; color: #444; line-height: 1.6; margin: 0; } \
                 .verdict-badge { display: inline-block; padding: 2px 10px; border-radius: 12px; font-size: 11px; font-weight: 600; } \
                 .verdict-validates { background: rgba(107,191,138,0.15); color: #4a8a5a; } \
                 .verdict-contradicts { background: rgba(196,112,106,0.15); color: #C4706A; } \
                 .verdict-neutral { background: rgba(168,158,140,0.15); color: #7a7570; }";

    format!(
        "<div class=\"briefing-dist\">\
           <div style='display:flex;justify-content:space-between;align-items:center;margin-bottom:16px'>\
             <h3 style='margin:0'>{}</h3>\
             <span style='font-size:11px;color:#A89E8C;background:#f5f0e8;padding:3px 10px;border-radius:12px'>\
               Share: <code style='font-size:11px'>rairos.app/b/{}</code></span>\
           </div>\
           <div class='digest-body'>{}</div>\
           <details style='margin-top:20px'>\
             <summary style='cursor:pointer;font-size:12px;color:#A89E8C'>View Raw Briefing</summary>\
             <pre style='font-size:11px;background:#f8f4ef;padding:12px;border-radius:4px;overflow:auto'>{}</pre>\
           </details>\
           <style>{}</style>\
         </div>",
        label, short_id, body_content, escaped_markdown, style
    )
}

pub fn render_distributor_panel(arxiv_id: &str, title: &str) -> String {
    let _short_id = create_share_link(arxiv_id, title, "researcher");

    let buttons = [
        ("researcher", "🔬 Researcher", "peer review format"),
        (
            "phd_advisor",
            "🎓 PhD Advisor",
            "methodology and open questions",
        ),
        (
            "industry_engineer",
            "⚙️ Industry Engineer",
            "implementation and deployment",
        ),
        (
            "policy_maker",
            "🏛️ Policy Maker",
            "societal impact and risks",
        ),
    ];

    let buttons_html: String = buttons
        .iter()
        .map(|(id, label, desc)| {
            format!(
                "<button onclick=\"renderBriefing('{}','{}')\" \
                 style='margin:4px;padding:6px 12px;cursor:pointer;border:1px solid #ccc;border-radius:4px;background:transparent;font-size:12px'>\
                 {} <span style='color:#888;font-size:10px'>— {}</span></button>",
                arxiv_id, id, label, desc
            )
        })
        .collect();

    format!(
        "<div class=\"dist-panel\">\
           <h3>📬 Briefing Distributor</h3>\
           <p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>\
             Render this briefing for different audiences, or share a public link.</p>\
           <div style='margin-bottom:16px'>{}</div>\
           <div id='briefing-output'></div>\
           <style>.dist-panel {{ font-family: Georgia, serif; }}</style>\
           <script>\
           function renderBriefing(arxivId, audience) {{\
             fetch('/briefing/' + arxivId + '/' + audience)\
               .then(r => r.text())\
               .then(d => {{ document.getElementById('briefing-output').innerHTML = d; }});\
           }}\
           </script>\
         </div>",
        buttons_html
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_short_id() {
        let id = make_short_id("Test Paper Title", "2301.12345");
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_make_short_id_deterministic() {
        let id1 = make_short_id("Test Paper Title", "2301.12345");
        let id2 = make_short_id("Test Paper Title", "2301.12345");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_make_short_id_different_titles() {
        let id1 = make_short_id("Paper A", "2301.12345");
        let id2 = make_short_id("Paper B", "2301.12345");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_escape_html() {
        let result = escape_html("<div>&&\"test\"</div>");
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
        assert!(result.contains("&amp;"));
    }

    #[test]
    fn test_parse_markdown_sections() {
        let md = "# Title\n\n## Summary\nThis is the summary.\n\n## Methodology\nMethodology here.";
        let sections = parse_markdown_sections(md);
        assert_eq!(sections.title, Some("Title".to_string()));
        assert!(sections.sections.contains_key("summary"));
        assert!(sections.sections.contains_key("methodology"));
    }

    #[test]
    fn test_parse_markdown_sections_no_headers() {
        let md = "Just some text without headers.";
        let sections = parse_markdown_sections(md);
        assert!(sections.title.is_none());
        assert!(sections.body.is_some());
    }

    #[test]
    fn test_render_distributor_panel() {
        let html = render_distributor_panel("2301.12345", "Test Paper");
        assert!(html.contains("Briefing Distributor"));
        assert!(html.contains("researcher"));
        assert!(html.contains("phd_advisor"));
    }

    #[test]
    fn test_render_distributed_briefing() {
        let html = render_distributed_briefing(
            "2301.12345",
            "Test Paper",
            "# Title\n## Summary\nTest content",
            "researcher",
        );
        assert!(html.contains("Researcher Digest"));
        assert!(html.contains("briefing-dist"));
    }

    #[test]
    fn test_create_share_link() {
        let short_id = create_share_link("2301.12345", "Test Paper", "researcher");
        assert_eq!(short_id.len(), 6);
    }
}
