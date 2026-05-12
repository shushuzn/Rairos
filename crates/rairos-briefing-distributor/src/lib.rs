use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const BRIEFINGS_DIR_NAME: &str = "briefings";
const LINKS_FILE_NAME: &str = "briefing_links.json";
const SHORTCODE_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn briefings_dir() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join(BRIEFINGS_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(BRIEFINGS_DIR_NAME))
}

fn links_file() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join(LINKS_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(LINKS_FILE_NAME))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BriefingLink {
    arxiv_id: String,
    title: String,
    audience: String,
    created_at: String,
    clicks: i32,
}

fn load_links() -> HashMap<String, BriefingLink> {
    let path = links_file();
    if !path.exists() {
        return HashMap::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

fn save_links(links: &HashMap<String, BriefingLink>) -> Result<(), std::io::Error> {
    let path = links_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(links).unwrap();
    std::fs::write(&path, json)
}

pub fn make_short_id(title: &str, arxiv_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let raw = format!("{}:{}", arxiv_id, &title[..title.len().min(30)]);
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    let hash_bytes = hasher.finish().to_le_bytes();

    let chars: Vec<char> = SHORTCODE_CHARS.chars().collect();
    let char_count = chars.len();

    (0..6)
        .map(|i| {
            let idx = (hash_bytes[i] as usize) % char_count;
            chars[idx]
        })
        .collect()
}

pub fn create_share_link(arxiv_id: &str, title: &str, audience: &str) -> String {
    let mut links = load_links();
    let short_id = make_short_id(title, arxiv_id);

    links.insert(
        short_id.clone(),
        BriefingLink {
            arxiv_id: arxiv_id.to_string(),
            title: title.to_string(),
            audience: audience.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            clicks: 0,
        },
    );

    let _ = save_links(&links);
    short_id
}

pub fn get_latest_briefing_markdown(arxiv_id: &str) -> Option<String> {
    let dir = briefings_dir();
    if !dir.exists() {
        return None;
    }

    let _pattern = format!("*{}*briefing*", arxiv_id);
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(arxiv_id) && n.contains("briefing"))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return None;
    }

    let latest = entries
        .into_iter()
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))?;

    std::fs::read_to_string(latest.path()).ok()
}

fn parse_markdown_sections(md: &str) -> HashMap<String, String> {
    let mut sections: HashMap<String, String> = HashMap::new();
    let mut current = "header".to_string();
    let mut body_lines: Vec<String> = Vec::new();

    for line in md.lines() {
        let line = line.trim();
        if line.starts_with("## ") {
            if !body_lines.is_empty() {
                sections.insert(current.clone(), body_lines.join("\n").trim().to_string());
                body_lines.clear();
            }
            current = line
                .strip_prefix("## ")
                .unwrap_or(line)
                .trim()
                .to_lowercase()
                .replace(' ', "_");
        } else if line.starts_with("# ") {
            sections.insert("_title".to_string(), line[2..].trim().to_string());
        } else {
            body_lines.push(line.to_string());
        }
    }

    if !body_lines.is_empty() && current != "header" {
        sections.insert(current, body_lines.join("\n").trim().to_string());
    } else if current == "header" {
        sections.insert(
            "_body".to_string(),
            body_lines.join("\n").trim().to_string(),
        );
    }

    sections
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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

fn render_phd_advisor(sections: &HashMap<String, String>, raw: &str) -> String {
    let extract = |key: &str| {
        sections
            .get(key)
            .map(|s| &s[..s.len().min(300)])
            .unwrap_or(&raw[raw.len().min(200)..raw.len().min(500)])
    };

    section_html(
        "📚 Paper Summary",
        sections
            .get("_body")
            .or_else(|| sections.get("summary"))
            .map(|s| &s[..s.len().min(400)])
            .unwrap_or(&raw[..raw.len().min(400)]),
    ) + &section_html("🔬 Methodology Assessment", extract("methodology"))
        + &section_html("❓ Open Questions for Student", extract("research_gaps"))
}

fn render_industry_engineer(sections: &HashMap<String, String>, raw: &str) -> String {
    let extract = |key: &str| {
        sections
            .get(key)
            .map(|s| &s[..s.len().min(300)])
            .unwrap_or(&raw[raw.len().min(200)..raw.len().min(500)])
    };

    section_html(
        "⚡ What It Does",
        sections
            .get("_body")
            .or_else(|| sections.get("summary"))
            .map(|s| &s[..s.len().min(200)])
            .unwrap_or(&raw[..raw.len().min(200)]),
    ) + &section_html("🛠️ Implementation Signals", extract("methodology"))
        + &section_html("📊 Benchmark / Compute", extract("experiments"))
}

fn render_policy_maker(sections: &HashMap<String, String>, raw: &str) -> String {
    let extract = |key: &str| {
        sections
            .get(key)
            .map(|s| &s[..s.len().min(300)])
            .unwrap_or(&raw[raw.len().min(300)..raw.len().min(600)])
    };

    section_html(
        "🏛️ What This Means",
        sections
            .get("_body")
            .or_else(|| sections.get("summary"))
            .map(|s| &s[..s.len().min(300)])
            .unwrap_or(&raw[..raw.len().min(300)]),
    ) + &section_html("⚠️ Risks & Concerns", extract("limitations"))
        + &section_html(
            "📅 Deployment Timeline",
            &raw[raw.len().min(200)..raw.len().min(500)],
        )
}

fn render_researcher(sections: &HashMap<String, String>, raw: &str) -> String {
    let v = sections
        .get("verdict")
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "neutral".to_string());
    let (badge_text, badge_cls) = match v.as_str() {
        "validates" => ("✅ Validates", "verdict-validates"),
        "contradicts" => ("❌ Contradicts", "verdict-contradicts"),
        _ => ("➖ Neutral", "verdict-neutral"),
    };

    let mut lines = vec![format!(
        "<span class='verdict-badge {}'>{}</span>",
        badge_cls, badge_text
    )];
    lines.push(format!(
        "<p style='margin-top:8px'>{}</p>",
        sections
            .get("_body")
            .or_else(|| sections.get("summary"))
            .map(|s| &s[..s.len().min(400)])
            .unwrap_or(&raw[..raw.len().min(400)])
    ));

    if let Some(gaps) = sections
        .get("research_gaps")
        .or_else(|| sections.get("gaps"))
    {
        lines.push(section_html(
            "🎯 Research Gaps",
            &gaps[..gaps.len().min(300)],
        ));
    }

    lines.join("\n")
}

pub fn render_distributed_briefing(
    arxiv_id: &str,
    title: &str,
    markdown: &str,
    audience: &str,
) -> String {
    let audience_labels: HashMap<&str, &str> = [
        ("phd_advisor", "🎓 PhD Advisor Digest"),
        ("industry_engineer", "⚙️ Industry Engineer Digest"),
        ("policy_maker", "🏛️ Policy Maker Digest"),
        ("researcher", "🔬 Researcher Digest"),
    ]
    .into_iter()
    .collect();

    let label = audience_labels.get(audience).copied().unwrap_or(audience);
    let short_id = create_share_link(arxiv_id, title, audience);
    let sections = parse_markdown_sections(markdown);

    let body_content = match audience {
        "phd_advisor" => render_phd_advisor(&sections, markdown),
        "industry_engineer" => render_industry_engineer(&sections, markdown),
        "policy_maker" => render_policy_maker(&sections, markdown),
        _ => render_researcher(&sections, markdown),
    };

    let mut lines = Vec::new();
    lines.push("<div class=\"briefing-dist\">".to_string());
    lines.push(format!("<h2 style='margin:0 0 8px 0'>{}</h2>", title));
    lines.push("<div style='display:flex;justify-content:space-between;align-items:center;margin-bottom:16px'>".to_string());
    lines.push(format!("<h3 style='margin:0'>{}</h3>", label));
    lines.push(format!(
        "<span style='font-size:11px;color:#A89E8C;background:#f5f0e8;padding:3px 10px;border-radius:12px'>\
Share: <code style='font-size:11px'>rairos.app/b/{}</code></span>",
        short_id
    ));
    lines.push("</div>".to_string());
    lines.push(format!("<div class='digest-body'>{}</div>", body_content));
    lines.push("<details style='margin-top:20px'>".to_string());
    lines.push(
        "<summary style='cursor:pointer;font-size:12px;color:#A89E8C'>View Raw Briefing</summary>"
            .to_string(),
    );
    lines.push(format!(
        "<pre style='font-size:11px;background:#f8f4ef;padding:12px;border-radius:4px;overflow:auto'>{}</pre>",
        escape_html(&markdown[..markdown.len().min(2000)])
    ));
    lines.push("</details>".to_string());
    lines.push("<style>".to_string());
    lines.push(".briefing-dist { font-family: Georgia, serif; max-width: 800px; }".to_string());
    lines.push(".digest-section { margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid #e8e4dc; }".to_string());
    lines.push(".digest-section h4 { font-size: 13px; font-weight: 700; color: #2a4a6a; margin-bottom: 6px; }".to_string());
    lines.push(
        ".digest-section p { font-size: 13px; color: #444; line-height: 1.6; margin: 0; }"
            .to_string(),
    );
    lines.push(".verdict-badge { display: inline-block; padding: 2px 10px; border-radius: 12px; font-size: 11px; font-weight: 600; }".to_string());
    lines.push(
        ".verdict-validates { background: rgba(107,191,138,0.15); color: #4a8a5a; }".to_string(),
    );
    lines.push(
        ".verdict-contradicts { background: rgba(196,112,106,0.15); color: #C4706A; }".to_string(),
    );
    lines.push(
        ".verdict-neutral { background: rgba(168,158,140,0.15); color: #7a7570; }".to_string(),
    );
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

pub fn render_distributor_panel(arxiv_id: &str, title: &str) -> String {
    let _ = create_share_link(arxiv_id, title, "researcher");

    let mut lines = Vec::new();
    lines.push("<div class=\"dist-panel\">".to_string());
    lines.push("<h3>📬 Briefing Distributor</h3>".to_string());
    lines.push("<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>Render this briefing for different audiences, or share a public link.</p>".to_string());

    let audiences = [
        (
            "researcher",
            "🔬 Researcher",
            "Concise technical summary with gap analysis",
        ),
        (
            "phd_advisor",
            "🎓 PhD Advisor",
            "Methodology critique and open questions",
        ),
        (
            "industry_engineer",
            "⚙️ Industry Engineer",
            "Practical applicability and benchmarks",
        ),
        (
            "policy_maker",
            "🏛️ Policy Maker",
            "Societal impact and regulatory implications",
        ),
    ];

    for (aud_id, aud_name, aud_desc) in &audiences {
        let s = create_share_link(arxiv_id, title, aud_id);
        lines.push(
            "<div style='margin-bottom:14px;padding:12px;background:#f8f4ef;border-radius:6px'>"
                .to_string(),
        );
        lines.push(format!(
            "<div style='font-weight:700;font-size:13px;margin-bottom:2px'>{}</div>",
            aud_name
        ));
        lines.push(format!(
            "<div style='font-size:12px;color:#A89E8C;margin-bottom:6px'>{}</div>",
            aud_desc
        ));
        lines.push(format!(
            "<button id='btn-{}' style='background:#6B8FB5;color:white;border:none;border-radius:4px;\
padding:5px 12px;cursor:pointer;font-size:12px'>Preview</button> ",
            aud_id
        ));
        lines.push(format!(
            "<button onclick=\"copyShareLink('{}')\" \
style='background:transparent;color:#6B8FB5;border:1px solid #6B8FB5;\
border-radius:4px;padding:5px 12px;cursor:pointer;font-size:12px;margin-left:6px'>\
Copy Link</button>",
            s
        ));
        lines.push("</div>".to_string());
    }

    lines.push("<div id='audience-preview' style='margin-top:16px'></div>".to_string());

    lines.push(format!(
        r#"<script>
document.querySelectorAll('button[id^="btn-"]').forEach(function(btn) {{
    var aud = btn.id.replace('btn-', '');
    btn.addEventListener('click', function() {{
        var preview = document.getElementById('audience-preview');
        preview.innerText = 'Loading...';
        fetch('/briefing/distribute/{}?audience=' + aud)
          .then(function(r) {{ return r.text(); }})
          .then(function(html) {{
              var tmp = document.createElement('div');
              tmp.innerHTML = html;
              preview.innerHTML = tmp.querySelector('.briefing-dist') ? tmp.querySelector('.briefing-dist').innerHTML : html;
          }});
    }});
}});
function copyShareLink(shortId) {{
    navigator.clipboard.writeText(window.location.origin + '/b/' + shortId)
      .then(function() {{ alert('Link copied!'); }});
}}
</script>"#,
        arxiv_id
    ));

    lines.push("<style>.dist-panel { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_short_id() {
        let id1 = make_short_id("Test Title", "1234.56789");
        let id2 = make_short_id("Test Title", "1234.56789");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 6);
    }

    #[test]
    fn test_make_short_id_different_inputs() {
        let id1 = make_short_id("Title A", "1234.56789");
        let id2 = make_short_id("Title B", "1234.56789");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_parse_markdown_sections() {
        let md = "# Title\n\n## Summary\n\nContent here\n\n## Method\n\nMethod content";
        let sections = parse_markdown_sections(md);
        assert!(sections.contains_key("_title"));
        assert!(sections.contains_key("summary"));
        assert!(sections.contains_key("method"));
    }

    #[test]
    fn test_escape_html() {
        let input = "<script>alert('xss')</script>";
        let output = escape_html(input);
        assert!(!output.contains("<script>"));
        assert!(output.contains("&lt;"));
    }

    #[test]
    fn test_create_share_link() {
        let short_id = create_share_link("1234.56789", "Test Paper", "researcher");
        assert_eq!(short_id.len(), 6);
    }

    #[test]
    fn test_render_distributed_briefing() {
        let html = render_distributed_briefing(
            "1234.56789",
            "Test Paper",
            "# Test\n\n## Summary\n\nTest content",
            "researcher",
        );
        assert!(html.contains("briefing-dist"));
        assert!(html.contains("Test Paper"));
    }

    #[test]
    fn test_render_distributor_panel() {
        let html = render_distributor_panel("1234.56789", "Test Paper");
        assert!(html.contains("dist-panel"));
        assert!(html.contains("Briefing Distributor"));
    }
}
