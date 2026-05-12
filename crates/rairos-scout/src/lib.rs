//! rairos-scout — Paper scout that proactively finds papers matching Gene Pool interests.
//!
//! Scans ArXiv and news RSS feeds for papers, scores them against Gene Pool capsules,
//! and returns ranked results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const CACHE_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutResult {
    pub arxiv_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub published: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub match_score: f64,
    #[serde(default)]
    pub matched_capsule_id: String,
    #[serde(default)]
    pub matched_gap_title: String,
    #[serde(default)]
    pub matched_gap_type: String,
    #[serde(default)]
    pub credibility_of_match: f64,
    #[serde(default)]
    pub rank: i32,
    #[serde(default)]
    pub reason: String,
}

fn default_source() -> String {
    "arxiv".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsFeed {
    pub name: String,
    pub url: String,
}

pub const NEWS_FEEDS: &[(&str, &str)] = &[
    ("Reuters World", "https://www.reuters.com/world/rss"),
    ("Reuters Business", "https://www.reuters.com/business/rss"),
    ("BBC World", "https://feeds.bbci.co.uk/news/world/rss.xml"),
    ("BBC Technology", "https://feeds.bbci.co.uk/news/technology/rss.xml"),
    (
        "Google News Top",
        "https://news.google.com/rss?hl=en-US&gl=US&ceid=US:en",
    ),
    (
        "Google News Science",
        "https://news.google.com/rss/topics/CAAqJggKIiBDQkFTRWdvSUwyMHZNRFp0Y1RjU0FtVnVHZ0pWVXlnQVAB",
    ),
    ("Hacker News", "https://hnrss.org/frontpage"),
];

#[derive(Debug, Clone)]
struct CacheEntry {
    timestamp: Instant,
    results: Vec<CachedPaper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPaper {
    pub arxiv_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub published: String,
    pub source: String,
}

struct SearchCache {
    entries: HashMap<String, CacheEntry>,
}

impl SearchCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<Vec<CachedPaper>> {
        self.entries.get(key).and_then(|entry| {
            if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                Some(entry.results.clone())
            } else {
                None
            }
        })
    }

    fn insert(&mut self, key: String, results: Vec<CachedPaper>) {
        self.entries.insert(
            key,
            CacheEntry {
                timestamp: Instant::now(),
                results,
            },
        );
    }

    #[allow(dead_code)]
    fn clear_expired(&mut self) {
        self.entries
            .retain(|_, entry| entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECS));
    }
}

static SEARCH_CACHE: std::sync::LazyLock<std::sync::Mutex<SearchCache>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(SearchCache::new()));

#[derive(Debug, Deserialize)]
struct RssEntry {
    title: Option<String>,
    link: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    published: Option<String>,
    #[serde(default)]
    authors: Vec<RssAuthor>,
}

#[derive(Debug, Deserialize)]
struct RssAuthor {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RssFeed {
    entries: Option<Vec<RssEntry>>,
}

pub async fn fetch_rss_feed(feed_url: &str, feed_name: &str, max_items: usize) -> Vec<CachedPaper> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let resp = match client.get(feed_url).send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let feed: RssFeed = match serde_xml_rs::de::from_str(&text) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let entries = feed.entries.unwrap_or_default();
    entries
        .into_iter()
        .take(max_items)
        .map(|entry| {
            let authors: Vec<String> = entry
                .authors
                .iter()
                .filter_map(|a| a.name.clone())
                .collect();
            let summary = entry.summary.or(entry.description).unwrap_or_default();
            let published = entry.published.unwrap_or_default();
            let link_hash = entry
                .link
                .as_ref()
                .map(|l| {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    l.hash(&mut hasher);
                    hasher.finish() % 10_000_000
                })
                .unwrap_or(0);
            CachedPaper {
                arxiv_id: format!("news_{}_{:07}", feed_name, link_hash),
                title: entry.title.unwrap_or_default(),
                authors,
                abstract_text: summary.chars().take(500).collect(),
                categories: vec![feed_name.to_string()],
                published: published.chars().take(10).collect(),
                source: feed_name.to_string(),
            }
        })
        .collect()
}

pub fn search_arxiv_cached(_query: &str, _max_results: usize) -> Vec<CachedPaper> {
    let cache_key = format!("{}:{}", _query, _max_results);
    if let Ok(cache) = SEARCH_CACHE.lock() {
        if let Some(results) = cache.get(&cache_key) {
            return results;
        }
    }
    vec![]
}

pub fn cache_arxiv_results(query: &str, max_results: usize, results: Vec<CachedPaper>) {
    let cache_key = format!("{}:{}", query, max_results);
    if let Ok(mut cache) = SEARCH_CACHE.lock() {
        cache.insert(cache_key, results);
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapsuleStub {
    pub capsule_id: String,
    pub trigger_keywords: Vec<String>,
    pub trigger_topic: String,
    pub trigger_gap_type: String,
    pub action_gap_title: String,
    pub credibility_score: f64,
    pub status: String,
}

impl CapsuleStub {
    pub fn trigger_match(&self, _topic: &str, _gap_type: &str, _keywords: &[String]) -> f64 {
        0.0
    }
}

pub fn score_article(
    paper: &CachedPaper,
    capsules: &[CapsuleStub],
    topic: &str,
) -> Option<ScoutResult> {
    let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();

    let mut best_score = 0.0;
    let mut best_capsule: Option<&CapsuleStub> = None;
    let mut best_reason = String::new();

    for capsule in capsules {
        let kw_overlap = capsule
            .trigger_keywords
            .iter()
            .filter(|kw| text.contains(&kw.to_lowercase()))
            .count() as f64;

        let mut match_score =
            capsule.trigger_match(topic, &capsule.trigger_gap_type, &capsule.trigger_keywords);
        if kw_overlap > 0.0 {
            match_score = match_score.max(0.3 + kw_overlap * 0.15).min(0.8);
        }
        let cred = capsule.credibility_score;
        let weighted = match_score * (0.5 + 0.5 * cred);

        if weighted > best_score {
            best_score = weighted;
            best_capsule = Some(capsule);
            let mut reasons = Vec::new();
            if match_score > 0.0 {
                reasons.push(format!("trigger_match={:.2}", match_score));
            }
            if kw_overlap > 0.0 {
                reasons.push(format!("keyword overlap={}", kw_overlap as i32));
            }
            if cred > 0.0 {
                reasons.push(format!("capsule credibility={:.2}", cred));
            }
            best_reason = if reasons.is_empty() {
                "topic match".to_string()
            } else {
                reasons.join("; ")
            };
        }
    }

    if best_score > 0.0 {
        let capsule = best_capsule?;
        return Some(ScoutResult {
            arxiv_id: paper.arxiv_id.clone(),
            title: paper.title.chars().take(200).collect(),
            authors: paper.authors.iter().take(5).cloned().collect(),
            abstract_text: paper.abstract_text.chars().take(500).collect(),
            categories: paper.categories.clone(),
            published: paper.published.chars().take(10).collect(),
            source: paper.source.clone(),
            match_score: (best_score * 1000.0).round() / 1000.0,
            matched_capsule_id: capsule.capsule_id.clone(),
            matched_gap_title: capsule.action_gap_title.chars().take(100).collect(),
            matched_gap_type: capsule.trigger_gap_type.clone(),
            credibility_of_match: (capsule.credibility_score * 1000.0).round() / 1000.0,
            rank: 0,
            reason: best_reason,
        });
    }
    None
}

pub fn scout(
    topic: &str,
    sources: &str,
    max_papers_per_query: usize,
    max_results: usize,
    min_match_score: f64,
    capsules: &[CapsuleStub],
) -> Vec<ScoutResult> {
    let mut topics = if topic.is_empty() {
        vec![]
    } else {
        vec![topic.to_string()]
    };

    if topics.is_empty() {
        topics = get_topics_from_capsules(capsules);
    }

    if topics.is_empty() {
        topics = vec!["machine learning".to_string()];
    }

    let mut seen = std::collections::HashSet::new();
    let mut all_papers: Vec<ScoutResult> = Vec::new();

    if sources == "news" || sources == "all" {
        for (feed_name, feed_url) in NEWS_FEEDS {
            let articles = futures::executor::block_on(fetch_rss_feed(feed_url, feed_name, 5));
            for p in articles {
                let pid = p.arxiv_id.clone();
                if seen.contains(&pid) {
                    continue;
                }
                seen.insert(pid);
                if let Some(result) = score_article(&p, capsules, topic) {
                    all_papers.push(result);
                }
            }
        }
    }

    if sources == "arxiv" || sources == "all" {
        for t in topics.iter().take(5) {
            let papers = search_arxiv_cached(t, max_papers_per_query);
            for p in papers {
                let pid = p.arxiv_id.clone();
                if seen.contains(&pid) {
                    continue;
                }
                seen.insert(pid);
                if let Some(result) = score_article(&p, capsules, t) {
                    if result.match_score >= min_match_score {
                        all_papers.push(result);
                    }
                }
            }
        }
    }

    all_papers.sort_by(|a, b| {
        b.match_score
            .partial_cmp(&a.match_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (i, sr) in all_papers.iter_mut().take(max_results).enumerate() {
        sr.rank = (i + 1) as i32;
    }

    all_papers.into_iter().take(max_results).collect()
}

pub fn get_topics_from_capsules(capsules: &[CapsuleStub]) -> Vec<String> {
    use std::collections::HashMap;

    let mut all_keywords = Vec::new();
    for c in capsules {
        all_keywords.extend(c.trigger_keywords.clone());
    }

    let mut kw_counts: HashMap<String, usize> = HashMap::new();
    for kw in &all_keywords {
        if kw.len() > 2 {
            *kw_counts.entry(kw.to_lowercase()).or_insert(0) += 1;
        }
    }

    let mut top_kws: Vec<String> = kw_counts
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    top_kws.sort_by_key(|k| std::cmp::Reverse(k.len()));
    top_kws.truncate(10);

    let mut topics = std::collections::HashSet::new();

    for c in capsules {
        let words: Vec<&str> = c.trigger_topic.split_whitespace().collect();
        if !words.is_empty() {
            let short = words.iter().take(4).copied().collect::<Vec<_>>().join(" ");
            if short.len() > 10 {
                topics.insert(short);
            }
        }
    }

    for pair in top_kws.chunks(2) {
        if pair.len() == 2 {
            topics.insert(format!("{} {}", pair[0], pair[1]));
        }
    }

    let mut topics_vec: Vec<String> = topics.into_iter().collect();
    topics_vec.sort_by_key(|s| s.len());
    topics_vec.truncate(10);
    topics_vec
}

pub fn render_scout_results(results: &[ScoutResult]) -> String {
    if results.is_empty() {
        return "  No matching papers found.".to_string();
    }

    let mut lines = vec![format!(
        "\n  Found {} items matching Gene Pool interests:\n",
        results.len()
    )];
    for r in results {
        let sev = r.match_score;
        let icon = if sev >= 0.5 {
            "🟢"
        } else if sev >= 0.3 {
            "🟡"
        } else {
            "⚪"
        };
        let authors_str = r
            .authors
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let source_tag = if r.source != "arxiv" {
            format!("[{}]", r.source)
        } else {
            String::new()
        };
        let title = if r.title.len() > 70 {
            format!("{}...", &r.title[..70])
        } else {
            r.title.clone()
        };
        lines.push(format!("  {} {} [{}] {}", icon, source_tag, r.rank, title));
        lines.push(format!("       {} · {}", r.published, authors_str));
        lines.push(format!(
            "       Match: {:.2} ← {}",
            r.match_score, r.matched_gap_type
        ));
        let gap_title = if r.matched_gap_title.len() > 50 {
            format!("{}...", &r.matched_gap_title[..50])
        } else {
            r.matched_gap_title.clone()
        };
        lines.push(format!("       Capsule: {}", gap_title));
        if !r.reason.is_empty() {
            lines.push(format!("       Why: {}", r.reason));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_capsule() -> CapsuleStub {
        CapsuleStub {
            capsule_id: "cap-001".to_string(),
            trigger_keywords: vec![
                "transformer".to_string(),
                "attention".to_string(),
                "language model".to_string(),
            ],
            trigger_topic: "deep learning neural networks".to_string(),
            trigger_gap_type: "accuracy".to_string(),
            action_gap_title: "Improve language understanding".to_string(),
            credibility_score: 0.8,
            status: "active".to_string(),
        }
    }

    fn create_test_paper() -> CachedPaper {
        CachedPaper {
            arxiv_id: "2301.00001".to_string(),
            title: "Attention Is All You Need".to_string(),
            authors: vec!["Vaswani".to_string(), "Shazeer".to_string()],
            abstract_text: "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks. We propose a new simple network architecture, the Transformer, based solely on attention mechanisms.".to_string(),
            categories: vec!["cs.CL".to_string()],
            published: "2023-06-01".to_string(),
            source: "arxiv".to_string(),
        }
    }

    #[test]
    fn test_scout_result_serialization() {
        let result = ScoutResult {
            arxiv_id: "2301.00001".to_string(),
            title: "Test Paper".to_string(),
            authors: vec!["Author1".to_string()],
            abstract_text: "Abstract text".to_string(),
            categories: vec!["cs.AI".to_string()],
            published: "2023-01-01".to_string(),
            source: "arxiv".to_string(),
            match_score: 0.75,
            matched_capsule_id: "cap-001".to_string(),
            matched_gap_title: "Test Gap".to_string(),
            matched_gap_type: "accuracy".to_string(),
            credibility_of_match: 0.8,
            rank: 1,
            reason: "trigger_match=0.5".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ScoutResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.arxiv_id, "2301.00001");
        assert_eq!(deserialized.match_score, 0.75);
    }

    #[test]
    fn test_cached_paper_serialization() {
        let paper = create_test_paper();
        let json = serde_json::to_string(&paper).unwrap();
        let deserialized: CachedPaper = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.arxiv_id, "2301.00001");
        assert_eq!(deserialized.title, "Attention Is All You Need");
    }

    #[test]
    fn test_news_feeds_not_empty() {
        assert!(!NEWS_FEEDS.is_empty());
        assert!(NEWS_FEEDS.len() >= 5);
    }

    #[test]
    fn test_score_article_no_match() {
        let paper = CachedPaper {
            arxiv_id: "0000.00000".to_string(),
            title: "Unrelated Topic".to_string(),
            authors: vec![],
            abstract_text: "This paper is about cooking".to_string(),
            categories: vec![],
            published: "2023-01-01".to_string(),
            source: "arxiv".to_string(),
        };
        let capsules = vec![create_test_capsule()];
        let result = score_article(&paper, &capsules, "");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_topics_from_capsules() {
        let capsules = vec![create_test_capsule()];
        let topics = get_topics_from_capsules(&capsules);
        assert!(!topics.is_empty());
    }

    #[test]
    fn test_render_empty_results() {
        let results: Vec<ScoutResult> = vec![];
        let output = render_scout_results(&results);
        assert_eq!(output, "  No matching papers found.");
    }

    #[test]
    fn test_render_single_result() {
        let result = ScoutResult {
            arxiv_id: "2301.00001".to_string(),
            title: "Attention Is All You Need".to_string(),
            authors: vec!["Vaswani".to_string(), "Shazeer".to_string()],
            abstract_text: "The dominant sequence transduction models.".to_string(),
            categories: vec!["cs.CL".to_string()],
            published: "2023-06-01".to_string(),
            source: "arxiv".to_string(),
            match_score: 0.75,
            matched_capsule_id: "cap-001".to_string(),
            matched_gap_title: "Improve language understanding".to_string(),
            matched_gap_type: "accuracy".to_string(),
            credibility_of_match: 0.8,
            rank: 1,
            reason: "trigger_match=0.5; keyword overlap=2".to_string(),
        };
        let output = render_scout_results(&[result]);
        assert!(output.contains("Found 1 items"));
        assert!(output.contains("Attention Is All You Need"));
    }

    #[test]
    fn test_scout_with_empty_capsules() {
        let capsules: Vec<CapsuleStub> = vec![];
        let results = scout("machine learning", "arxiv", 10, 20, 0.15, &capsules);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_cache_basic() {
        let results = search_arxiv_cached("test query", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_capsule_stub_default() {
        let capsule = CapsuleStub::default();
        assert_eq!(capsule.status, "");
        assert_eq!(capsule.credibility_score, 0.0);
    }
}
