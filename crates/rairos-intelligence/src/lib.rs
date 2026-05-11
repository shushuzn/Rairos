//! rairos-intelligence — Unified situation report from all data sources.
//!
//! Ported from `llm/intelligence.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashNewsItem {
    pub time: String,
    pub content: String,
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketQuote {
    pub code: String,
    pub name: String,
    pub price: String,
    pub change: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenePoolStats {
    pub total: usize,
    pub avg_score: f64,
    #[serde(default)]
    pub by_type: HashMap<String, usize>,
    pub high_credibility: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopCapsule {
    pub title: String,
    pub score: f64,
    pub badge: String,
    pub capsule_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchStatus {
    pub running: bool,
    pub last_check: String,
    pub events_monitored: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMatch {
    pub title: String,
    pub score: f64,
    pub capsule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceReport {
    pub generated_at: String,
    pub topic: String,
    #[serde(default)]
    pub flash_news: Vec<FlashNewsItem>,
    #[serde(default)]
    pub flash_error: Option<String>,
    #[serde(default)]
    pub markets: Vec<MarketQuote>,
    #[serde(default)]
    pub markets_error: Option<String>,
    #[serde(default)]
    pub gene_pool: GenePoolStats,
    #[serde(default)]
    pub top_capsules: Vec<TopCapsule>,
    #[serde(default)]
    pub watch: WatchStatus,
    #[serde(default)]
    pub papers: Vec<PaperMatch>,
}

impl Default for IntelligenceReport {
    fn default() -> Self {
        Self {
            generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string(),
            topic: "global".to_string(),
            flash_news: Vec::new(),
            flash_error: None,
            markets: Vec::new(),
            markets_error: None,
            gene_pool: GenePoolStats {
                total: 0,
                avg_score: 0.0,
                by_type: HashMap::new(),
                high_credibility: 0,
            },
            top_capsules: Vec::new(),
            watch: WatchStatus {
                running: false,
                last_check: String::new(),
                events_monitored: 0,
            },
            papers: Vec::new(),
        }
    }
}

impl IntelligenceReport {
    pub fn new(topic: &str) -> Self {
        Self {
            generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string(),
            topic: topic.to_string(),
            ..Default::default()
        }
    }

    pub fn with_flash_news(mut self, items: Vec<FlashNewsItem>) -> Self {
        self.flash_news = items;
        self
    }

    pub fn with_markets(mut self, quotes: Vec<MarketQuote>) -> Self {
        self.markets = quotes;
        self
    }

    pub fn with_gene_pool(mut self, stats: GenePoolStats, top_capsules: Vec<TopCapsule>) -> Self {
        self.gene_pool = stats;
        self.top_capsules = top_capsules;
        self
    }

    pub fn with_watch_status(mut self, status: WatchStatus) -> Self {
        self.watch = status;
        self
    }

    pub fn with_papers(mut self, papers: Vec<PaperMatch>) -> Self {
        self.papers = papers;
        self
    }

    pub fn with_flash_error(mut self, error: &str) -> Self {
        self.flash_error = Some(error.to_string());
        self
    }

    pub fn with_markets_error(mut self, error: &str) -> Self {
        self.markets_error = Some(error.to_string());
        self
    }
}

pub struct IntelligenceGenerator;

impl IntelligenceGenerator {
    pub fn generate(topic: &str, verbose: bool) -> IntelligenceReport {
        let mut report = IntelligenceReport::new(topic);

        if let Some((news, _error)) = Self::fetch_flash_news(topic) {
            report = report.with_flash_news(news);
        } else if verbose {
            report = report.with_flash_error("Failed to fetch flash news");
        }

        if let Some((quotes, _error)) = Self::fetch_market_quotes(topic) {
            report = report.with_markets(quotes);
        } else if verbose {
            report = report.with_markets_error("Failed to fetch market quotes");
        }

        if let Some((stats, top)) = Self::fetch_gene_pool_stats() {
            report = report.with_gene_pool(stats, top);
        }

        let watch_status = Self::fetch_watch_status();
        report = report.with_watch_status(watch_status);

        if let Some(papers) = Self::fetch_papers(topic) {
            report = report.with_papers(papers);
        }

        report
    }

    fn fetch_flash_news(_topic: &str) -> Option<(Vec<FlashNewsItem>, Option<String>)> {
        None
    }

    fn fetch_market_quotes(_topic: &str) -> Option<(Vec<MarketQuote>, Option<String>)> {
        None
    }

    fn fetch_gene_pool_stats() -> Option<(GenePoolStats, Vec<TopCapsule>)> {
        None
    }

    fn fetch_watch_status() -> WatchStatus {
        WatchStatus {
            running: false,
            last_check: String::new(),
            events_monitored: 0,
        }
    }

    fn fetch_papers(_topic: &str) -> Option<Vec<PaperMatch>> {
        None
    }

    pub fn render_report(report: &IntelligenceReport) -> String {
        let mut lines = Vec::new();

        lines.push("╔═══ Rairos Intelligence ═══╗".to_string());
        lines.push(format!(
            "  {}  |  Topic: {}",
            report.generated_at, report.topic
        ));
        lines.push(String::new());

        if !report.flash_news.is_empty() {
            lines.push("  Geopolitical Flash".to_string());
            for item in report.flash_news.iter().take(5) {
                let content = if item.content.len() > 80 {
                    format!("{}...", &item.content[..80])
                } else {
                    item.content.clone()
                };
                lines.push(format!("  [{}] {}", item.time, content));
            }
            if report.flash_news.len() > 5 {
                lines.push(format!("  ... and {} more", report.flash_news.len() - 5));
            }
            lines.push(String::new());
        }

        if !report.markets.is_empty() {
            lines.push("  Markets".to_string());
            for q in &report.markets {
                let change_str = &q.change;
                let color = if change_str.starts_with('-') {
                    "\x1b[32m"
                } else if !change_str.is_empty() && change_str != "?" {
                    "\x1b[31m"
                } else {
                    ""
                };
                let reset = if !color.is_empty() { "\x1b[0m" } else { "" };
                lines.push(format!(
                    "  {:<8} {:>8}  {}{}{}",
                    q.code, q.price, color, change_str, reset
                ));
            }
            lines.push(String::new());
        }

        if report.gene_pool.total > 0 {
            lines.push("  Gene Pool".to_string());
            lines.push(format!(
                "  {} capsules, avg score {}, {} high credibility",
                report.gene_pool.total,
                report.gene_pool.avg_score,
                report.gene_pool.high_credibility
            ));
            lines.push(format!("  Types: {:?}", report.gene_pool.by_type));
            lines.push(String::new());
        }

        if !report.top_capsules.is_empty() {
            lines.push("  Top Capsules".to_string());
            for c in &report.top_capsules {
                let badge = if !c.badge.is_empty() {
                    format!("[{}]", c.badge.to_uppercase())
                } else {
                    String::new()
                };
                let title = if c.title.len() > 60 {
                    format!("{}...", &c.title[..60])
                } else {
                    c.title.clone()
                };
                lines.push(format!("  {} {} (score={})", badge, title, c.score));
            }
            lines.push(String::new());
        }

        if report.watch.running || !report.watch.last_check.is_empty() {
            lines.push("  Watch Daemon".to_string());
            let status = if report.watch.running {
                "RUNNING"
            } else {
                "STOPPED"
            };
            lines.push(format!(
                "  {}  |  Last: {}  |  Events: {}",
                status, report.watch.last_check, report.watch.events_monitored
            ));
            lines.push(String::new());
        }

        if !report.papers.is_empty() {
            lines.push("  Related Papers".to_string());
            for p in &report.papers {
                let title = if p.title.len() > 60 {
                    format!("{}...", &p.title[..60])
                } else {
                    p.title.clone()
                };
                lines.push(format!("  {} (match={:.2})", title, p.score));
                if !p.capsule.is_empty() {
                    let capsule = if p.capsule.len() > 50 {
                        format!("{}...", &p.capsule[..50])
                    } else {
                        p.capsule.clone()
                    };
                    lines.push(format!("  → {}", capsule));
                }
            }
            lines.push(String::new());
        }

        lines.push("╚═══════════════════════════════════╝".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intelligence_report_default() {
        let report = IntelligenceReport::default();
        assert_eq!(report.topic, "global");
        assert!(report.flash_news.is_empty());
        assert!(report.markets.is_empty());
    }

    #[test]
    fn test_intelligence_report_new() {
        let report = IntelligenceReport::new("AI research");
        assert_eq!(report.topic, "AI research");
        assert!(!report.generated_at.is_empty());
    }

    #[test]
    fn test_intelligence_report_with_flash_news() {
        let news = vec![FlashNewsItem {
            time: "10:00".to_string(),
            content: "Test news".to_string(),
            topic: "test".to_string(),
        }];
        let report = IntelligenceReport::new("topic").with_flash_news(news.clone());
        assert_eq!(report.flash_news.len(), 1);
    }

    #[test]
    fn test_intelligence_report_with_markets() {
        let quotes = vec![MarketQuote {
            code: "XAUUSD".to_string(),
            name: "Gold".to_string(),
            price: "2000".to_string(),
            change: "+0.5%".to_string(),
        }];
        let report = IntelligenceReport::new("topic").with_markets(quotes);
        assert_eq!(report.markets.len(), 1);
    }

    #[test]
    fn test_intelligence_report_with_gene_pool() {
        let stats = GenePoolStats {
            total: 10,
            avg_score: 0.75,
            by_type: HashMap::new(),
            high_credibility: 3,
        };
        let top = vec![TopCapsule {
            title: "Test capsule".to_string(),
            score: 0.9,
            badge: "high".to_string(),
            capsule_type: "method".to_string(),
        }];
        let report = IntelligenceReport::new("topic").with_gene_pool(stats, top);
        assert_eq!(report.gene_pool.total, 10);
        assert_eq!(report.top_capsules.len(), 1);
    }

    #[test]
    fn test_intelligence_report_with_watch_status() {
        let status = WatchStatus {
            running: true,
            last_check: "2024-01-01T10:00".to_string(),
            events_monitored: 100,
        };
        let report = IntelligenceReport::new("topic").with_watch_status(status);
        assert!(report.watch.running);
        assert_eq!(report.watch.events_monitored, 100);
    }

    #[test]
    fn test_intelligence_report_with_papers() {
        let papers = vec![PaperMatch {
            title: "Test paper".to_string(),
            score: 0.85,
            capsule: "Test gap".to_string(),
        }];
        let report = IntelligenceReport::new("topic").with_papers(papers);
        assert_eq!(report.papers.len(), 1);
    }

    #[test]
    fn test_render_report() {
        let report = IntelligenceReport::new("test");
        let rendered = IntelligenceGenerator::render_report(&report);
        assert!(rendered.contains("Rairos Intelligence"));
        assert!(rendered.contains("test"));
    }

    #[test]
    fn test_render_report_with_all_sections() {
        let news = vec![FlashNewsItem {
            time: "10:00".to_string(),
            content: "Breaking news about markets".to_string(),
            topic: "markets".to_string(),
        }];
        let quotes = vec![MarketQuote {
            code: "XAUUSD".to_string(),
            name: "Gold".to_string(),
            price: "2000".to_string(),
            change: "-0.5%".to_string(),
        }];
        let stats = GenePoolStats {
            total: 5,
            avg_score: 0.7,
            by_type: HashMap::from([("method_limitation".to_string(), 3)]),
            high_credibility: 2,
        };
        let top = vec![TopCapsule {
            title: "Top capsule for testing purposes".to_string(),
            score: 0.9,
            badge: "high".to_string(),
            capsule_type: "method".to_string(),
        }];
        let watch = WatchStatus {
            running: true,
            last_check: "2024-01-01T10:00".to_string(),
            events_monitored: 50,
        };
        let papers = vec![PaperMatch {
            title: "A very important research paper title here".to_string(),
            score: 0.95,
            capsule: "Important gap that needs attention".to_string(),
        }];

        let report = IntelligenceReport::new("global")
            .with_flash_news(news)
            .with_markets(quotes)
            .with_gene_pool(stats, top)
            .with_watch_status(watch)
            .with_papers(papers);

        let rendered = IntelligenceGenerator::render_report(&report);
        assert!(rendered.contains("Geopolitical Flash"));
        assert!(rendered.contains("Markets"));
        assert!(rendered.contains("Gene Pool"));
        assert!(rendered.contains("Top Capsules"));
        assert!(rendered.contains("Watch Daemon"));
        assert!(rendered.contains("Related Papers"));
    }

    #[test]
    fn test_flash_news_item_serialization() {
        let item = FlashNewsItem {
            time: "12:00".to_string(),
            content: "Test content".to_string(),
            topic: "test".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("12:00"));
        assert!(json.contains("Test content"));
    }

    #[test]
    fn test_market_quote_serialization() {
        let quote = MarketQuote {
            code: "EURUSD".to_string(),
            name: "Euro".to_string(),
            price: "1.10".to_string(),
            change: "+0.1%".to_string(),
        };
        let json = serde_json::to_string(&quote).unwrap();
        assert!(json.contains("EURUSD"));
    }

    #[test]
    fn test_gene_pool_stats_serialization() {
        let stats = GenePoolStats {
            total: 100,
            avg_score: 0.75,
            by_type: HashMap::from([("type1".to_string(), 50)]),
            high_credibility: 25,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("0.75"));
    }
}
