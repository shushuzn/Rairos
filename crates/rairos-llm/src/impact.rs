//! Impact Scorer — citation-weighted composite scoring for papers.
//!
//! Mirrors llm/impact_scorer.py

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactScore {
    pub paper_id: String,
    pub title: String,
    pub citation_count: u32,
    pub recency_score: f64,    // 0.0-1.0
    pub venue_score: f64,      // 0.0-1.0
    pub composite: f64,        // weighted average
}

/// Score a paper's impact based on citations, recency, and venue.
pub fn score_paper(paper_id: &str, title: &str, citation_count: u32, year: i32, current_year: i32) -> ImpactScore {
    // Citation score: log-scaled, 0-1
    let citation_score = if citation_count > 0 {
        (citation_count as f64).ln() / (1000.0_f64).ln()
    } else {
        0.0
    }.min(1.0);

    // Recency score: newer = higher, linear decay over 10 years
    let age = (current_year - year).max(0) as f64;
    let recency_score = (1.0 - age / 10.0).max(0.0);

    // Composite: 60% citations + 40% recency
    let composite = citation_score * 0.6 + recency_score * 0.4;

    ImpactScore {
        paper_id: paper_id.to_string(),
        title: title.to_string(),
        citation_count,
        recency_score,
        venue_score: 0.5, // placeholder — no venue data
        composite,
    }
}

/// Rank a list of papers by impact score.
pub fn rank_papers(papers: &[(String, String, u32, i32)], current_year: i32, top_k: usize) -> Vec<ImpactScore> {
    let mut scores: Vec<ImpactScore> = papers.iter()
        .map(|(pid, title, cites, year)| score_paper(pid, title, *cites, *year, current_year))
        .collect();

    scores.sort_by(|a, b| b.composite.partial_cmp(&a.composite).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_paper_high_citations() {
        let score = score_paper("p1", "Important Paper", 500, 2023, 2025);
        assert!(score.composite > 0.5, "highly cited recent paper should score well");
    }

    #[test]
    fn test_score_paper_old_and_uncited() {
        let score = score_paper("p2", "Old Paper", 0, 2010, 2025);
        assert!(score.composite < 0.5, "old uncited paper should score poorly");
    }

    #[test]
    fn test_rank_papers_order() {
        let papers = vec![
            ("p1".to_string(), "New".to_string(), 100, 2024),
            ("p2".to_string(), "Old".to_string(), 5, 2015),
        ];
        let ranked = rank_papers(&papers, 2025, 5);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].composite > ranked[1].composite, "newer cited paper should rank higher");
    }
}
