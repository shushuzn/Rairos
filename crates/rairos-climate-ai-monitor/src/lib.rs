use std::collections::HashSet;

pub const PAPERS_DIR: &str = ".ai_research_os";
pub const PAPERS_DB: &str = ".ai_research_os/papers.json";
pub const CLIMATE_WATCH_FILE: &str = ".ai_research_os/climate_watch.json";

pub static CLIMATE_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "climate change",
        "global warming",
        "carbon",
        "emissions",
        "greenhouse gas",
        "renewable energy",
        "solar",
        "wind power",
        "energy efficiency",
        "sustainable",
        "sustainability",
        "fossil fuel",
        "net-zero",
        "carbon neutral",
        "climate model",
        "weather prediction",
        "earth system",
        "carbon capture",
        "data center",
        "water consumption",
        "e-waste",
        "environmental impact",
        "green AI",
        "energy-aware",
        "low-carbon",
        "carbon footprint",
        "FLOPs per watt",
        "compute efficiency",
        "model efficiency",
    ]
});

pub static CLIMATE_CATS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("cs.AI");
    s.insert("cs.LG");
    s.insert("cs.ET");
    s.insert("physics.ao-ph");
    s.insert("atm.ph");
    s
});

use once_cell::sync::Lazy;

#[derive(Debug, Clone, Default)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub published: String,
}

pub fn is_climate_related(title: &str, abstract_text: &str, categories: &[String]) -> bool {
    let text = format!("{} {}", title.to_lowercase(), abstract_text.to_lowercase());

    let cats_set: HashSet<&str> = categories.iter().map(|s| s.as_str()).collect();
    if CLIMATE_CATS.iter().any(|c| cats_set.contains(c)) {
        return true;
    }

    CLIMATE_KEYWORDS.iter().any(|kw| text.contains(&kw.to_lowercase()))
}

pub fn filter_climate_papers(papers: &[Paper]) -> Vec<&Paper> {
    papers.iter().filter(|p| is_climate_related(&p.title, &p.abstract_text, &p.categories)).collect()
}

#[derive(Debug, Clone, Default)]
pub struct WatchStats {
    pub total_climate_papers: usize,
    pub watched_count: usize,
    pub recent_count: usize,
    pub last_scan: String,
}

pub fn get_watch_stats(papers: &[Paper], watched_ids: &[String], last_scan: &str) -> WatchStats {
    let climate_papers = filter_climate_papers(papers);
    let watched_ids_set: HashSet<&str> = watched_ids.iter().map(|s| s.as_str()).collect();

    let recent_count = climate_papers.iter().filter(|p| {
        p.published.len() >= 4 && &p.published[..4] >= "2025"
    }).count();

    WatchStats {
        total_climate_papers: climate_papers.len(),
        watched_count: climate_papers.iter().filter(|p| watched_ids_set.contains(p.id.as_str())).count(),
        recent_count,
        last_scan: last_scan.to_string(),
    }
}

pub fn render_climate_monitor_html(stats: Option<&WatchStats>, papers: &[Paper], watched_ids: &[String]) -> String {
    let stats = match stats {
        Some(s) => s.clone(),
        None => get_watch_stats(papers, watched_ids, "never"),
    };

    let climate_papers = filter_climate_papers(papers);
    let watched_ids_set: HashSet<&str> = watched_ids.iter().map(|s| s.as_str()).collect();

    let mut lines: Vec<String> = Vec::new();
    lines.push("<div class=\"climate-monitor\">".to_string());
    lines.push("<h3>Climate AI Monitor</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>Papers at the intersection of climate science and AI. High priority in gap watch matching.</p>".to_string()
    );

    lines.push("<div style='display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;margin-bottom:20px'>".to_string());
    let stats_cells = [
        ("Total Climate Papers", stats.total_climate_papers, "#6B8FB5"),
        ("In Your Watch List", stats.watched_count, "#6BBF8A"),
        ("Published 2025+", stats.recent_count, "#D4A055"),
    ];
    for (label, val, color) in stats_cells {
        lines.push(format!(
            "<div style='background:#f8f4ef;border-radius:6px;padding:12px;text-align:center'><div style='font-size:22px;font-weight:700;color:{}'>{}</div><div style='font-size:11px;color:#A89E8C;margin-top:2px'>{}</div></div>",
            color, val, label
        ));
    }
    lines.push("</div>".to_string());

    if climate_papers.is_empty() {
        lines.push("<p style='color:#A89E8C;font-size:13px'>No climate-related papers in your library yet.</p>".to_string());
    } else {
        for p in climate_papers.iter().take(15) {
            let is_watched = watched_ids_set.contains(p.id.as_str());
            let cats = p.categories.iter().take(2).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            let title = p.title.chars().take(70).collect::<String>();
            let published = p.published.chars().take(4).collect::<String>();
            let _kw_matches: Vec<&str> = CLIMATE_KEYWORDS.iter()
                .filter(|kw| {
                    let text_lower = format!("{} {}", p.title.to_lowercase(), p.abstract_text.to_lowercase());
                    text_lower.contains(&kw.to_lowercase())
                })
                .take(3)
                .copied()
                .collect();

            lines.push(format!(
                "<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 12px; margin-bottom: 10px'><div style='display: flex; justify-content: space-between; align-items: flex-start'><div style='flex:1'><div style='font-size: 12px; color: #6B8FB5; font-weight: 600'>{}</div><div style='font-size: 11px; color: #A89E8C; margin-top: 2px'>{} . {}</div></div><button onclick=\"toggleWatch('{}', this)\" style='font-size: 10px; padding: 3px 8px; cursor: pointer; border-radius: 3px; border: 1px solid #ccc; background: transparent; color: {}'>{}</button></div></div>",
                title,
                cats,
                published,
                p.id,
                if is_watched { "#6BBF8A" } else { "#A89E8C" },
                if is_watched { "Watched" } else { "+ Watch" }
            ));
        }
    }

    lines.push("<script>function toggleWatch(paperId, btn) { fetch('/climate-monitor/toggle-watch', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({paper_id: paperId}) }).then(function(r) { return r.json(); }).then(function(d) { if (d.success) { var isWatched = btn.textContent.trim() === 'Watched'; btn.textContent = isWatched ? '+ Watch' : 'Watched'; btn.style.color = isWatched ? '#A89E8C' : '#6BBF8A'; } }); }</script>".to_string());
    lines.push("<style>.climate-monitor { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climate_keywords_not_empty() {
        assert!(!CLIMATE_KEYWORDS.is_empty());
        assert!(CLIMATE_KEYWORDS.contains(&"carbon"));
        assert!(CLIMATE_KEYWORDS.contains(&"climate change"));
    }

    #[test]
    fn test_climate_cats_not_empty() {
        assert!(!CLIMATE_CATS.is_empty());
        assert!(CLIMATE_CATS.contains("cs.AI"));
    }

    #[test]
    fn test_is_climate_related_by_category() {
        let paper = Paper {
            id: "1".to_string(),
            title: "Test".to_string(),
            abstract_text: "Some abstract".to_string(),
            categories: vec!["cs.AI".to_string()],
            published: "2024".to_string(),
        };
        assert!(is_climate_related(&paper.title, &paper.abstract_text, &paper.categories));
    }

    #[test]
    fn test_is_climate_related_by_keyword() {
        let paper = Paper {
            id: "1".to_string(),
            title: "Climate Change and AI".to_string(),
            abstract_text: "Global warming impacts".to_string(),
            categories: vec!["cs.CV".to_string()],
            published: "2024".to_string(),
        };
        assert!(is_climate_related(&paper.title, &paper.abstract_text, &paper.categories));
    }

    #[test]
    fn test_is_not_climate_related() {
        let paper = Paper {
            id: "1".to_string(),
            title: "Object Detection".to_string(),
            abstract_text: "Convolutional neural networks".to_string(),
            categories: vec!["cs.CV".to_string()],
            published: "2024".to_string(),
        };
        assert!(!is_climate_related(&paper.title, &paper.abstract_text, &paper.categories));
    }

    #[test]
    fn test_filter_climate_papers() {
        let papers = vec![
            Paper { id: "1".to_string(), title: "Climate AI".to_string(), abstract_text: "Test".to_string(), categories: vec!["cs.AI".to_string()], published: "2024".to_string() },
            Paper { id: "2".to_string(), title: "Object Detection".to_string(), abstract_text: "CNN".to_string(), categories: vec!["cs.CV".to_string()], published: "2024".to_string() },
        ];
        let filtered = filter_climate_papers(&papers);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn test_get_watch_stats() {
        let papers = vec![
            Paper { id: "1".to_string(), title: "Climate AI".to_string(), abstract_text: "Test".to_string(), categories: vec!["cs.AI".to_string()], published: "2026".to_string() },
            Paper { id: "2".to_string(), title: "Carbon Footprint".to_string(), abstract_text: "Energy".to_string(), categories: vec![], published: "2026".to_string() },
        ];
        let watched = vec!["1".to_string()];
        let stats = get_watch_stats(&papers, &watched, "2026-01-01");
        assert_eq!(stats.total_climate_papers, 2);
        assert_eq!(stats.watched_count, 1);
        assert!(stats.recent_count >= 2);
    }

    #[test]
    fn test_render_climate_monitor_html() {
        let papers = vec![];
        let watched = vec![];
        let html = render_climate_monitor_html(None, &papers, &watched);
        assert!(html.contains("climate-monitor"));
        assert!(html.contains("Climate AI Monitor"));
    }
}
