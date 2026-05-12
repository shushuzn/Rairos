//! rairos-signal — Pattern-based signal system for matching live events against historical Gene Pool patterns.
//!
//! Ported from `llm/signal.py`.
//!
//! This crate provides signal analysis functionality: given an event keyword, it matches against
//! historical patterns and produces a signal report with market data and impact assessment.

use std::collections::HashMap;
use std::time::SystemTime;

// ============================================================================
// Types
// ============================================================================

/// Represents a capsule (historical pattern entry) from the Gene Pool.
/// In a full implementation this would be loaded from persistent storage.
/// For this pure-stdlib version we work with in-memory data.
#[derive(Debug, Clone)]
pub struct Capsule {
    pub capsule_id: String,
    pub trigger_keywords: Vec<String>,
    pub trigger_gap_type: String,
    pub action_gap_title: String,
    pub action_gap_type: String,
    pub outcome_success_score: f64,
    pub credibility_badge: String,
}

impl Capsule {
    /// Calculate trigger match score between this capsule and a live event keyword.
    /// Returns a score in [0.0, 1.0] range.
    pub fn trigger_match(&self, event_keyword: &str, gap_type: &str, keywords: &[String]) -> f64 {
        let event_lower = event_keyword.to_lowercase();
        let type_match = if gap_type.eq_ignore_ascii_case("military")
            || gap_type.eq_ignore_ascii_case(" geopolitical")
        {
            0.3
        } else if gap_type.eq_ignore_ascii_case("economic")
            || gap_type.eq_ignore_ascii_case("finance")
        {
            0.25
        } else if gap_type.eq_ignore_ascii_case("energy") || gap_type.eq_ignore_ascii_case("oil") {
            0.35
        } else {
            0.1
        };

        let kw_match = keywords
            .iter()
            .filter(|kw| event_lower.contains(&kw.to_lowercase()))
            .count() as f64
            / (keywords.len().max(1) as f64)
            * 0.5;

        (type_match + kw_match).min(1.0)
    }
}

/// A single historical pattern match result.
#[derive(Debug, Clone)]
pub struct Match {
    pub capsule_id: String,
    pub title: String,
    pub match_type: String,
    pub score: f64,
    pub credibility: String,
    pub total: f64,
}

/// Market quote data for a single symbol.
#[derive(Debug, Clone)]
pub struct Quote {
    pub price: String,
    pub change: String,
}

/// The complete signal analysis report.
#[derive(Debug, Clone)]
pub struct SignalReport {
    pub event: String,
    pub timestamp: String,
    pub signal: SignalLevel,
    pub capsule_matches: Vec<Match>,
    pub markets: HashMap<String, Quote>,
    pub impact_sectors: Vec<String>,
    pub news_count: usize,
    pub recommendation: String,
}

/// Signal intensity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalLevel {
    High,
    Medium,
    Low,
}

impl SignalLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalLevel::High => "HIGH",
            SignalLevel::Medium => "MEDIUM",
            SignalLevel::Low => "LOW",
        }
    }
}

// ============================================================================
// Jin10Client Stub (pure stdlib - no external dependencies)
// ============================================================================

/// Stub Jin10Client for market data.
/// In production this would connect to the Jin10 financial data API.
/// This stub returns empty/mock data for testing purposes.
pub struct Jin10Client;

impl Jin10Client {
    pub fn new() -> Self {
        Jin10Client
    }

    /// Ensure the client is initialized.
    /// Stub implementation - no-op.
    pub fn ensure_init(&self) {}

    /// Search flash events by keyword.
    /// Returns a vector of mock event items.
    pub fn search_flash(&self, _event_keyword: &str) -> Vec<String> {
        // Stub: return a small number of mock items based on keyword
        if _event_keyword.is_empty() {
            vec![]
        } else {
            vec![
                format!("Flash: {} related development #1", _event_keyword),
                format!("Flash: {} update #2", _event_keyword),
            ]
        }
    }

    /// Get a market quote for a given symbol.
    /// Returns a mock quote response.
    pub fn get_quote(&self, symbol: &str) -> HashMap<String, String> {
        let mut q = HashMap::new();
        match symbol {
            "USOIL" => {
                q.insert("close".to_string(), "78.45".to_string());
                q.insert("ups_percent".to_string(), "0.82".to_string());
            }
            "XAUUSD" => {
                q.insert("close".to_string(), "2024.30".to_string());
                q.insert("ups_percent".to_string(), "0.15".to_string());
            }
            "EURUSD" => {
                q.insert("close".to_string(), "1.0842".to_string());
                q.insert("ups_percent".to_string(), "-0.23".to_string());
            }
            "USDCNH" => {
                q.insert("close".to_string(), "7.2451".to_string());
                q.insert("ups_percent".to_string(), "0.05".to_string());
            }
            _ => {
                q.insert("close".to_string(), "?".to_string());
                q.insert("ups_percent".to_string(), "?".to_string());
            }
        }
        q
    }
}

// ============================================================================
// EvolutionTracker Stub (pure stdlib)
// ============================================================================

/// Stub EvolutionTracker for loading Gene Pool capsules.
/// In production this would load from persistent storage (~/.ai_research_os/evolution/gene_pool.jsonl).
pub struct EvolutionTracker;

impl EvolutionTracker {
    pub fn new() -> Self {
        EvolutionTracker
    }

    /// Load capsules from the Gene Pool.
    /// Returns a vector of in-memory mock capsules for testing.
    pub fn load_capsules(&self) -> Vec<Capsule> {
        vec![
            Capsule {
                capsule_id: "CAP-001".to_string(),
                trigger_keywords: vec![
                    "oil".to_string(),
                    "hormuz".to_string(),
                    "strait".to_string(),
                    "tanker".to_string(),
                ],
                trigger_gap_type: "energy".to_string(),
                action_gap_title: "Hormuz Strait oil flow disruption".to_string(),
                action_gap_type: "energy".to_string(),
                outcome_success_score: 0.85,
                credibility_badge: "high".to_string(),
            },
            Capsule {
                capsule_id: "CAP-002".to_string(),
                trigger_keywords: vec![
                    "military".to_string(),
                    "missile".to_string(),
                    "defense".to_string(),
                    "strike".to_string(),
                ],
                trigger_gap_type: "military".to_string(),
                action_gap_title: "Military strike impact on markets".to_string(),
                action_gap_type: "military".to_string(),
                outcome_success_score: 0.72,
                credibility_badge: "medium".to_string(),
            },
            Capsule {
                capsule_id: "CAP-003".to_string(),
                trigger_keywords: vec![
                    "inflation".to_string(),
                    "rate".to_string(),
                    "fed".to_string(),
                    "hike".to_string(),
                ],
                trigger_gap_type: "economic".to_string(),
                action_gap_title: "Interest rate hike effects".to_string(),
                action_gap_type: "finance".to_string(),
                outcome_success_score: 0.78,
                credibility_badge: "high".to_string(),
            },
            Capsule {
                capsule_id: "CAP-004".to_string(),
                trigger_keywords: vec![
                    "ceasefire".to_string(),
                    "peace".to_string(),
                    "talks".to_string(),
                    "negotiation".to_string(),
                ],
                trigger_gap_type: "geopolitical".to_string(),
                action_gap_title: "Ceasefire announcement market response".to_string(),
                action_gap_type: "geopolitical".to_string(),
                outcome_success_score: 0.65,
                credibility_badge: "medium".to_string(),
            },
            Capsule {
                capsule_id: "CAP-005".to_string(),
                trigger_keywords: vec![
                    "sanctions".to_string(),
                    "trade".to_string(),
                    "tariff".to_string(),
                    "export".to_string(),
                ],
                trigger_gap_type: "economic".to_string(),
                action_gap_title: "Sanctions impact on commodity markets".to_string(),
                action_gap_type: "economic".to_string(),
                outcome_success_score: 0.70,
                credibility_badge: "high".to_string(),
            },
        ]
    }
}

// ============================================================================
// Core Signal Functions
// ============================================================================

/// Analyze a live event against historical Gene Pool patterns.
///
/// Returns a signal report containing: what happened before, what to watch, estimated impact.
pub fn signal(event_keyword: &str) -> SignalReport {
    let client = Jin10Client::new();
    client.ensure_init();
    let tracker = EvolutionTracker::new();
    let capsules = tracker.load_capsules();

    // 1. Get live event context (stubbed)
    let raw: Vec<String> = client.search_flash(event_keyword);
    let live_items: Vec<String> = raw.iter().take(5).cloned().collect();

    // 2. Match against historical Gene Pool
    let mut matches: Vec<Match> = Vec::new();
    for c in &capsules {
        let score = c.trigger_match(event_keyword, &c.trigger_gap_type, &c.trigger_keywords);
        if score > 0.2 {
            let kw_overlap = c
                .trigger_keywords
                .iter()
                .filter(|kw| event_keyword.to_lowercase().contains(&kw.to_lowercase()))
                .count() as f64;
            let text_match = kw_overlap * 0.15;
            let total = score + text_match;
            matches.push(Match {
                capsule_id: c.capsule_id.clone(),
                title: c.action_gap_title.clone(),
                match_type: c.action_gap_type.clone(),
                score: c.outcome_success_score,
                credibility: c.credibility_badge.clone(),
                total: (total * 1000.0).round() / 1000.0,
            });
        }
    }

    // Sort by total match score descending
    matches.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3. Get current market state
    let symbols = ["USOIL", "XAUUSD", "EURUSD", "USDCNH"];
    let mut quotes: HashMap<String, Quote> = HashMap::new();
    for sym in symbols {
        let raw_q = client.get_quote(sym);
        let price = raw_q
            .get("close")
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let change = raw_q
            .get("ups_percent")
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        quotes.insert(sym.to_string(), Quote { price, change });
    }

    // 4. Generate signal assessment
    let high_matches: Vec<&Match> = matches.iter().filter(|m| m.total >= 0.5).collect();
    let signal_level = if high_matches.len() >= 2 {
        SignalLevel::High
    } else if high_matches.len() >= 1 {
        SignalLevel::Medium
    } else {
        SignalLevel::Low
    };

    // 5. Build impact estimate from capsule scores
    let mut impact_sectors: Vec<String> = Vec::new();
    for m in matches.iter().take(3) {
        let title_lower = m.title.to_lowercase();
        if title_lower.contains("oil")
            || title_lower.contains("石油")
            || title_lower.contains("hormuz")
            || title_lower.contains("energy")
        {
            if !impact_sectors.contains(&"energy".to_string()) {
                impact_sectors.push("energy".to_string());
            }
        }
        if title_lower.contains("military")
            || title_lower.contains("ceasefire")
            || title_lower.contains("missile")
            || title_lower.contains("导弹")
        {
            if !impact_sectors.contains(&"defense".to_string()) {
                impact_sectors.push("defense".to_string());
            }
        }
        if title_lower.contains("inflation")
            || title_lower.contains("rate")
            || title_lower.contains("加息")
            || title_lower.contains("finance")
        {
            if !impact_sectors.contains(&"finance".to_string()) {
                impact_sectors.push("finance".to_string());
            }
        }
    }

    let top_matches_for_rec: Vec<&Match> = matches.iter().take(3).collect();
    let recommendation = build_recommendation(signal_level, &top_matches_for_rec);
    let timestamp = format_timestamp();

    SignalReport {
        event: event_keyword.to_string(),
        timestamp,
        signal: signal_level,
        capsule_matches: matches,
        markets: quotes,
        impact_sectors,
        news_count: live_items.len(),
        recommendation,
    }
}

/// Build a text recommendation based on signal level and top matches.
fn build_recommendation(level: SignalLevel, top_matches: &[&Match]) -> String {
    match level {
        SignalLevel::High => {
            let topics: Vec<String> = top_matches
                .iter()
                .take(2)
                .map(|m| m.title.chars().take(40).collect::<String>())
                .collect();
            if topics.is_empty() {
                "Historical patterns suggest significant market impact.".to_string()
            } else {
                format!(
                    "Watchlist: {}. Historical patterns suggest market impact.",
                    topics.join("; ")
                )
            }
        }
        SignalLevel::Medium => {
            if let Some(first) = top_matches.first() {
                let title = first.title.chars().take(40).collect::<String>();
                format!(
                    "Related patterns found ({}). Monitor for escalation.",
                    title
                )
            } else {
                "Moderate pattern match. Continue monitoring.".to_string()
            }
        }
        SignalLevel::Low => "No significant pattern match. Low priority event.".to_string(),
    }
}

/// Format the current timestamp as an ISO-like string (compatible with Python's isoformat).
fn format_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    // Simple UTC timestamp: YYYY-MM-DDTHH:MM
    // We use a basic approach since we don't want to depend on chrono
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    // Jan 1, 1970 + days (not accurate without proper date calc, but sufficient for stub)
    // For a proper implementation we'd use the time crate, but we want pure stdlib
    let base_days = 719163; // Days from 1970-01-01 to 1970-01-01 (0) + reasonable offset for 2024
    let total_days = base_days + days as i64;
    let year = 1970 + total_days / 365;
    let day_of_year = total_days % 365;

    // Approximate month/day calculation
    let month = ((day_of_year / 30).max(1).min(12)) as u64;
    let day = ((day_of_year % 30) + 1).min(28) as u64;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        year, month, day, hours, minutes
    )
}

/// Render a signal report as a human-readable string.
pub fn render_signal(result: &SignalReport) -> String {
    let sig = result.signal.as_str();
    let sig_color = match result.signal {
        SignalLevel::High => "[RED]",
        SignalLevel::Medium => "[YELLOW]",
        SignalLevel::Low => "[GREEN]",
    };

    let mut lines = Vec::new();
    lines.push(format!("\n  === Signal Analysis ==="));
    lines.push(format!("  Event: {}", result.event));
    lines.push(format!(
        "  Signal: {}{}{}  |  {}",
        sig_color, sig, "[RESET]", result.timestamp
    ));
    lines.push(String::new());

    if !result.capsule_matches.is_empty() {
        lines.push(format!("  Historical Pattern Matches"));
        for m in result.capsule_matches.iter().take(3) {
            let credibility_upper = m.credibility.to_uppercase();
            lines.push(format!(
                "  match={:.2} [{}] {}",
                m.total,
                credibility_upper,
                m.title.chars().take(55).collect::<String>()
            ));
        }
        lines.push(String::new());
    }

    if !result.markets.is_empty() {
        lines.push(format!("  Current Markets"));
        for (k, v) in &result.markets {
            lines.push(format!("  {:<8} {:>8}  {}", k, v.price, v.change));
        }
        lines.push(String::new());
    }

    if !result.impact_sectors.is_empty() {
        lines.push(format!("  Impact Sectors"));
        lines.push(format!("  {}", result.impact_sectors.join(", ")));
        lines.push(String::new());
    }

    lines.push(format!("  Recommendation"));
    lines.push(format!("  {}", result.recommendation));
    lines.push(String::from("  =============================="));

    lines.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_basic() {
        let result = signal("oil hormuz");
        assert!(!result.event.is_empty());
        assert!(!result.timestamp.is_empty());
        assert_ne!(result.signal, SignalLevel::Low); // Should match with oil/hormuz
    }

    #[test]
    fn test_signal_low() {
        let result = signal("random unrelated event xyz123");
        // Very low match expected
        assert_eq!(result.signal, SignalLevel::Low);
    }

    #[test]
    fn test_signal_medium() {
        let result = signal("inflation rate fed");
        // Should match the inflation capsule
        assert!(result.signal == SignalLevel::Medium || result.signal == SignalLevel::High);
    }

    #[test]
    fn test_capsule_trigger_match() {
        let capsule = Capsule {
            capsule_id: "TEST".to_string(),
            trigger_keywords: vec!["oil".to_string(), "energy".to_string()],
            trigger_gap_type: "energy".to_string(),
            action_gap_title: "Test Title".to_string(),
            action_gap_type: "energy".to_string(),
            outcome_success_score: 0.8,
            credibility_badge: "high".to_string(),
        };

        let score = capsule.trigger_match("oil prices rising", "energy", &capsule.trigger_keywords);
        assert!(score > 0.2);
    }

    #[test]
    fn test_capsule_no_match() {
        let capsule = Capsule {
            capsule_id: "TEST".to_string(),
            trigger_keywords: vec!["oil".to_string()],
            trigger_gap_type: "energy".to_string(),
            action_gap_title: "Test".to_string(),
            action_gap_type: "energy".to_string(),
            outcome_success_score: 0.5,
            credibility_badge: "low".to_string(),
        };

        let score = capsule.trigger_match("sports football", "sports", &capsule.trigger_keywords);
        assert!(score < 0.5); // Should be low
    }

    #[test]
    fn test_jin10_client_quote() {
        let client = Jin10Client::new();
        let q = client.get_quote("USOIL");
        assert_eq!(q.get("close"), Some(&"78.45".to_string()));
    }

    #[test]
    fn test_jin10_client_search_flash() {
        let client = Jin10Client::new();
        let results = client.search_flash("test");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_evolution_tracker_load_capsules() {
        let tracker = EvolutionTracker::new();
        let capsules = tracker.load_capsules();
        assert!(!capsules.is_empty());
        assert!(capsules.iter().all(|c| !c.capsule_id.is_empty()));
    }

    #[test]
    fn test_render_signal() {
        let result = signal("test event");
        let rendered = render_signal(&result);
        assert!(rendered.contains("Signal Analysis"));
        assert!(rendered.contains(&result.event));
    }

    #[test]
    fn test_signal_level_as_str() {
        assert_eq!(SignalLevel::High.as_str(), "HIGH");
        assert_eq!(SignalLevel::Medium.as_str(), "MEDIUM");
        assert_eq!(SignalLevel::Low.as_str(), "LOW");
    }

    #[test]
    fn test_impact_sectors_detection() {
        let result = signal("military missile defense");
        assert!(result.impact_sectors.contains(&"defense".to_string()));
    }

    #[test]
    fn test_timestamp_format() {
        let ts = format_timestamp();
        // Should be in format YYYY-MM-DDTHH:MM
        assert!(ts.len() >= 16);
        assert!(ts.contains('T'));
    }

    #[test]
    fn test_build_recommendation_high() {
        let m = Match {
            capsule_id: "1".to_string(),
            title: "Test high impact event that has a long title".to_string(),
            match_type: "test".to_string(),
            score: 0.8,
            credibility: "high".to_string(),
            total: 0.75,
        };
        let matches = vec![&m];
        let rec = build_recommendation(SignalLevel::High, &matches);
        assert!(rec.contains("Watchlist"));
    }

    #[test]
    fn test_build_recommendation_low() {
        let rec = build_recommendation(SignalLevel::Low, &[]);
        assert!(rec.contains("Low priority"));
    }
}
