//! rairos-trend-analyzer — Research Trend Analyzer
//!
//! Analyzes research trends over time: yearly distribution, keyword trends,
//! citation velocity, and trend classification (rising/falling/emerging/stable).
//!
//! Ported from `llm/trend_analyzer.py`.

#![allow(clippy::unnecessary_unwrap)]

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::Datelike;

pub use rairos_constants::AI_RESEARCH_KEYWORDS;

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum TrendAnalyzerError {
    #[error("Insufficient papers: {0}")]
    InsufficientPapers(String),
    #[error("Invalid year range: {0}")]
    InvalidYearRange(String),
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    #[default]
    Unknown,
    Rising,
    Falling,
    Emerging,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyStats {
    pub year: i32,
    pub paper_count: usize,
    pub total_citations: usize,
    pub avg_citations: f64,
    pub keywords: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendKeyword {
    pub keyword: String,
    pub direction: TrendDirection,
    pub yearly_counts: HashMap<i32, usize>,
    pub growth_rate: f64,
    pub peak_year: i32,
    pub current_year_count: usize,
    pub velocity: f64,
    pub momentum: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysisResult {
    pub topic: String,
    pub year_range: (i32, i32),
    pub total_papers: usize,
    #[serde(default)]
    pub yearly_distribution: Vec<YearlyStats>,
    #[serde(default)]
    pub rising_trends: Vec<TrendKeyword>,
    #[serde(default)]
    pub falling_trends: Vec<TrendKeyword>,
    #[serde(default)]
    pub emerging_trends: Vec<TrendKeyword>,
    #[serde(default)]
    pub stable_trends: Vec<TrendKeyword>,
    #[serde(default)]
    pub hot_keywords: Vec<String>,
    #[serde(default)]
    pub declining_keywords: Vec<String>,
    #[serde(default)]
    pub emerging_keywords: Vec<String>,
    pub growth_rate: f64,
}

impl Default for TrendAnalysisResult {
    fn default() -> Self {
        Self {
            topic: String::new(),
            year_range: (0, 0),
            total_papers: 0,
            yearly_distribution: Vec::new(),
            rising_trends: Vec::new(),
            falling_trends: Vec::new(),
            emerging_trends: Vec::new(),
            stable_trends: Vec::new(),
            hot_keywords: Vec::new(),
            declining_keywords: Vec::new(),
            emerging_keywords: Vec::new(),
            growth_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
    pub year: i32,
    pub citations: usize,
    pub reference_count: usize,
    pub authors: String,
}

pub struct TrendAnalyzer {
    tech_keywords: HashSet<&'static str>,
}

impl Default for TrendAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TrendAnalyzer {
    pub fn new() -> Self {
        Self {
            tech_keywords: AI_RESEARCH_KEYWORDS.clone(),
        }
    }

    pub fn with_keywords(keywords: HashSet<&'static str>) -> Self {
        Self { tech_keywords: keywords }
    }

    pub fn analyze(
        &self,
        topic: &str,
        year_range: Option<(i32, i32)>,
        min_papers: usize,
        papers: &[Paper],
    ) -> TrendAnalysisResult {
        let current_year = chrono::Utc::now().year();
        let year_range = year_range.unwrap_or((current_year - 5, current_year));

        if year_range.0 > year_range.1 {
            return self.empty_result(topic, year_range);
        }

        if papers.len() < min_papers {
            return self.empty_result(topic, year_range);
        }

        let yearly_stats = self.compute_yearly_stats(papers, year_range);
        let trends = self.detect_keyword_trends(papers, year_range);
        let growth = self.compute_growth_rate(&yearly_stats);

        let rising: Vec<_> = trends.iter().filter(|t| t.direction == TrendDirection::Rising).cloned().collect();
        let falling: Vec<_> = trends.iter().filter(|t| t.direction == TrendDirection::Falling).cloned().collect();
        let emerging: Vec<_> = trends.iter().filter(|t| t.direction == TrendDirection::Emerging).cloned().collect();
        let stable: Vec<_> = trends.iter().filter(|t| t.direction == TrendDirection::Stable).cloned().collect();

        TrendAnalysisResult {
            topic: topic.to_string(),
            year_range,
            total_papers: papers.len(),
            yearly_distribution: yearly_stats,
            rising_trends: rising.clone(),
            falling_trends: falling.clone(),
            emerging_trends: emerging.clone(),
            stable_trends: stable,
            hot_keywords: rising.iter().take(5).map(|t| t.keyword.clone()).collect(),
            declining_keywords: falling.iter().take(5).map(|t| t.keyword.clone()).collect(),
            emerging_keywords: emerging.iter().take(5).map(|t| t.keyword.clone()).collect(),
            growth_rate: growth,
        }
    }

    fn compute_yearly_stats(
        &self,
        papers: &[Paper],
        year_range: (i32, i32),
    ) -> Vec<YearlyStats> {
        let mut yearly_data: HashMap<i32, YearlyData> = HashMap::new();

        for paper in papers {
            if year_range.0 <= paper.year && paper.year <= year_range.1 {
                let entry = yearly_data.entry(paper.year).or_default();
                entry.count += 1;
                entry.citations += paper.citations;

                let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
                for kw in &self.tech_keywords {
                    if text.contains(&(**kw).to_lowercase()) {
                        *entry.keywords.entry((*kw).to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut stats = Vec::new();
        for year in year_range.0..=year_range.1 {
            let data = yearly_data.get(&year).cloned().unwrap_or_default();
            stats.push(YearlyStats {
                year,
                paper_count: data.count,
                total_citations: data.citations,
                avg_citations: if data.count > 0 {
                    data.citations as f64 / data.count as f64
                } else {
                    0.0
                },
                keywords: data.keywords,
            });
        }
        stats
    }

    fn detect_keyword_trends(
        &self,
        papers: &[Paper],
        year_range: (i32, i32),
    ) -> Vec<TrendKeyword> {
        let mut keyword_yearly: HashMap<String, HashMap<i32, usize>> = HashMap::new();

        for paper in papers {
            if year_range.0 <= paper.year && paper.year <= year_range.1 {
                let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
                for kw in &self.tech_keywords {
                    if text.contains(&(**kw).to_lowercase()) {
                        keyword_yearly
                            .entry((*kw).to_string())
                            .or_default()
                            .entry(paper.year)
                            .or_insert(0);
                        *keyword_yearly.get_mut(*kw).unwrap().get_mut(&paper.year).unwrap() += 1;
                    }
                }
            }
        }

        let mut trends = Vec::new();
        for (keyword, yearly_counts) in keyword_yearly {
            if yearly_counts.values().sum::<usize>() < 3 {
                continue;
            }
            if let Some(trend) = self.compute_trend(&keyword, &yearly_counts, year_range) {
                trends.push(trend);
            }
        }

        trends.sort_by(|a, b| b.growth_rate.partial_cmp(&a.growth_rate).unwrap_or(std::cmp::Ordering::Equal));
        trends.truncate(20);
        trends
    }

    fn compute_trend(
        &self,
        keyword: &str,
        yearly_counts: &HashMap<i32, usize>,
        year_range: (i32, i32),
    ) -> Option<TrendKeyword> {
        let years: Vec<i32> = (year_range.0..=year_range.1).collect();
        let counts: Vec<usize> = years.iter().map(|y| yearly_counts.get(y).copied().unwrap_or(0)).collect();

        let first_nonzero = counts.iter().copied().find(|&c| c > 0).unwrap_or(0);
        let last_count = *counts.last().unwrap_or(&0);

        if first_nonzero == 0 {
            return None;
        }

        let growth_rate = ((last_count as f64 - first_nonzero as f64) / first_nonzero as f64) * 100.0;

        let direction = if last_count as f64 > first_nonzero as f64 * 1.5 {
            if counts.len() >= 2 && counts[counts.len() - 1] > counts[counts.len() - 2] && counts[counts.len() - 2] > 0 {
                TrendDirection::Emerging
            } else {
                TrendDirection::Rising
            }
        } else if (last_count as f64) < first_nonzero as f64 * 0.7 {
            TrendDirection::Falling
        } else if growth_rate.abs() < 20.0 {
            TrendDirection::Stable
        } else {
            TrendDirection::Unknown
        };

        let peak_year = yearly_counts
            .iter()
            .max_by_key(|(_, v)| *v)
            .map(|(y, _)| *y)
            .unwrap_or(year_range.1);

        let momentum = if counts.len() >= 3 {
            let recent_change = counts[counts.len() - 1] as f64 - counts[counts.len() - 2] as f64;
            let prev_change = counts[counts.len() - 2] as f64 - counts[counts.len() - 3] as f64;
            let prev_change = if prev_change.abs() < 0.001 { 1.0 } else { prev_change };
            (recent_change - prev_change) / prev_change.abs()
        } else {
            0.0
        };

        let velocity = if counts.len() >= 3 {
            counts[counts.len() - 3..].iter().sum::<usize>() as f64
        } else {
            counts.iter().sum::<usize>() as f64
        };

        Some(TrendKeyword {
            keyword: keyword.to_string(),
            direction,
            yearly_counts: yearly_counts.clone(),
            growth_rate,
            peak_year,
            current_year_count: last_count,
            velocity,
            momentum,
        })
    }

    fn compute_growth_rate(&self, yearly_stats: &[YearlyStats]) -> f64 {
        if yearly_stats.len() < 2 {
            return 0.0;
        }
        let first = yearly_stats[0].paper_count;
        let last = yearly_stats[yearly_stats.len() - 1].paper_count;
        if first == 0 {
            return 0.0;
        }
        ((last as f64 - first as f64) / first as f64) * 100.0
    }

    fn empty_result(&self, topic: &str, year_range: (i32, i32)) -> TrendAnalysisResult {
        TrendAnalysisResult {
            topic: topic.to_string(),
            year_range,
            total_papers: 0,
            ..Default::default()
        }
    }

    pub fn render_result(&self, result: &TrendAnalysisResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "📈 《{}》研究趋势分析 ({}-{})",
            result.topic, result.year_range.0, result.year_range.1
        ));
        lines.push(format!(
            "   总论文数: {} | 整体增长率: {:+.1}",
            result.total_papers, result.growth_rate
        ));
        lines.push(String::new());

        if !result.yearly_distribution.is_empty() {
            lines.push("📊 年度分布:".to_string());
            for stats in &result.yearly_distribution {
                let bar_count = std::cmp::min(stats.paper_count, 20);
                let bar = "█".repeat(bar_count);
                lines.push(format!("   {}: {:3} {}", stats.year, stats.paper_count, bar));
            }
            lines.push(String::new());
        }

        if !result.rising_trends.is_empty() {
            lines.push("🔥 上升趋势:".to_string());
            for trend in result.rising_trends.iter().take(5) {
                let growth_str = if trend.growth_rate >= 0.0 {
                    format!("+{:.0}%", trend.growth_rate)
                } else {
                    format!("{:.0}%", trend.growth_rate)
                };
                lines.push(format!(
                    "   ↑ {}: {} ({}篇)",
                    trend.keyword, growth_str, trend.current_year_count
                ));
            }
            lines.push(String::new());
        }

        if !result.emerging_trends.is_empty() {
            lines.push("🆕 新兴方向:".to_string());
            for trend in result.emerging_trends.iter().take(5) {
                lines.push(format!(
                    "   ✨ {}: +{:.0}% 加速中",
                    trend.keyword, trend.growth_rate
                ));
            }
            lines.push(String::new());
        }

        if !result.falling_trends.is_empty() {
            lines.push("📉 下降趋势:".to_string());
            for trend in result.falling_trends.iter().take(5) {
                lines.push(format!(
                    "   ↓ {}: {:.0}% ({}篇)",
                    trend.keyword, trend.growth_rate, trend.current_year_count
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    pub fn render_mermaid_timeline(&self, result: &TrendAnalysisResult) -> String {
        let mut lines = vec![
            "gantt".to_string(),
            format!("    title Research Trends - {}", result.topic),
            "    dateFormat YYYY".to_string(),
            "    section Keywords".to_string(),
        ];

        for trend in result.emerging_trends.iter().take(3).chain(result.rising_trends.iter().take(3)) {
            if let (Some(&start_year), Some(&end_year)) = (
                trend.yearly_counts.keys().min(),
                trend.yearly_counts.keys().max(),
            ) {
                let status = if trend.direction == TrendDirection::Emerging {
                    "active"
                } else {
                    "done"
                };
                lines.push(format!(
                    "    {} ({}) :t{}, {}y",
                    trend.keyword,
                    status,
                    start_year,
                    end_year - start_year + 1
                ));
            }
        }

        lines.join("\n")
    }

    pub fn render_mermaid_timeline_v2(&self, result: &TrendAnalysisResult) -> String {
        let all_trends: Vec<_> = result
            .emerging_trends
            .iter()
            .take(2)
            .chain(result.rising_trends.iter().take(2))
            .collect();

        if all_trends.is_empty() {
            return String::new();
        }

        let all_years: HashSet<i32> = all_trends
            .iter()
            .flat_map(|t| t.yearly_counts.keys())
            .copied()
            .collect();

        if all_years.is_empty() {
            return String::new();
        }

        let year_range: Vec<i32> = (all_years.iter().min().copied().unwrap_or(0)
            ..=all_years.iter().max().copied().unwrap_or(0))
            .collect();

        let mut lines = vec![
            "%%{ init: { 'theme': 'base', 'themeVariables': { 'primaryColor': '#ff9900' } } }%%".to_string(),
            "```mermaid".to_string(),
            "xychart-beta".to_string(),
            format!(r#"    title "{} - Keyword Trends""#, result.topic),
            format!(
                "    x-axis [{}]",
                year_range.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", ")
            ),
            "    y-axis \"Papers\" 0 --> 50".to_string(),
            String::new(),
            "    bar".to_string(),
        ];

        for trend in all_trends.iter().take(4) {
            let counts: Vec<String> = year_range
                .iter()
                .map(|y| trend.yearly_counts.get(y).unwrap_or(&0).to_string())
                .collect();
            lines.push(format!(r#"        "{}" : {}"#, trend.keyword, counts.join(", ")));
        }

        lines.push("```".to_string());
        lines.join("\n")
    }
}

#[derive(Default, Clone)]
struct YearlyData {
    count: usize,
    citations: usize,
    keywords: HashMap<String, usize>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_papers() -> Vec<Paper> {
        vec![
            Paper {
                id: "1".to_string(),
                title: "Attention is all you need".to_string(),
                abstract_text: "We propose a new simple network architecture based on attention mechanisms.".to_string(),
                year: 2020,
                citations: 100,
                reference_count: 10,
                authors: "Vaswani et al".to_string(),
            },
            Paper {
                id: "2".to_string(),
                title: "BERT: Pre-training of Deep Bidirectional Transformers".to_string(),
                abstract_text: "We introduce a new language representation model called BERT.".to_string(),
                year: 2021,
                citations: 80,
                reference_count: 8,
                authors: "Devlin et al".to_string(),
            },
            Paper {
                id: "3".to_string(),
                title: "GPT-3: Language Models are Few-Shot Learners".to_string(),
                abstract_text: "We train a large language model with 175 billion parameters.".to_string(),
                year: 2021,
                citations: 120,
                reference_count: 12,
                authors: "Brown et al".to_string(),
            },
            Paper {
                id: "4".to_string(),
                title: "Diffusion Models Beat GANs on Image Synthesis".to_string(),
                abstract_text: "We show that diffusion models can achieve better image synthesis than GANs.".to_string(),
                year: 2022,
                citations: 60,
                reference_count: 6,
                authors: "Dhariwal et al".to_string(),
            },
            Paper {
                id: "5".to_string(),
                title: "RLHF: Learning to summarize with human feedback".to_string(),
                abstract_text: "We use reinforcement learning from human feedback to train a summarization model.".to_string(),
                year: 2022,
                citations: 50,
                reference_count: 5,
                authors: "Stiennon et al".to_string(),
            },
            Paper {
                id: "6".to_string(),
                title: "Chain-of-Thought Prompting Elicits Reasoning".to_string(),
                abstract_text: "We show that chain-of-thought prompting improves reasoning in large language models.".to_string(),
                year: 2023,
                citations: 40,
                reference_count: 4,
                authors: "Wei et al".to_string(),
            },
            Paper {
                id: "7".to_string(),
                title: "LLaMA: Open Foundation Models".to_string(),
                abstract_text: "We open-source a set of foundation language models ranging from 7B to 65B parameters.".to_string(),
                year: 2023,
                citations: 70,
                reference_count: 7,
                authors: "Touvron et al".to_string(),
            },
            Paper {
                id: "8".to_string(),
                title: "Mistral 7B: Efficient Language Models".to_string(),
                abstract_text: "We introduce Mistral 7B, a efficient language model that outperforms existing open-source models.".to_string(),
                year: 2024,
                citations: 30,
                reference_count: 3,
                authors: "Jiang et al".to_string(),
            },
            Paper {
                id: "9".to_string(),
                title: "Qwen Technical Report".to_string(),
                abstract_text: "We introduce Qwen, a large language model trained on massive text data.".to_string(),
                year: 2024,
                citations: 20,
                reference_count: 2,
                authors: "Bai et al".to_string(),
            },
            Paper {
                id: "10".to_string(),
                title: "Retrieval-Augmented Generation for Knowledge-Intensive NLP".to_string(),
                abstract_text: "We propose retrieval-augmented generation to improve factual accuracy.".to_string(),
                year: 2021,
                citations: 55,
                reference_count: 5,
                authors: "Lewis et al".to_string(),
            },
            Paper {
                id: "11".to_string(),
                title: "Llama 2: Open Foundation Models".to_string(),
                abstract_text: "We open the weights of Llama 2, a collection of foundation language models.".to_string(),
                year: 2023,
                citations: 90,
                reference_count: 9,
                authors: "Touvron et al".to_string(),
            },
            Paper {
                id: "12".to_string(),
                title: "Constitutional AI".to_string(),
                abstract_text: "We propose constitutional AI, a method for training AI systems to be helpful and harmless.".to_string(),
                year: 2022,
                citations: 45,
                reference_count: 4,
                authors: "Bai et al".to_string(),
            },
        ]
    }

    #[test]
    fn test_analyze_with_minimal_data() {
        let analyzer = TrendAnalyzer::new();
        let papers = vec![Paper {
            id: "1".to_string(),
            title: "Attention in deep learning".to_string(),
            abstract_text: "Attention mechanism improves neural networks.".to_string(),
            year: 2023,
            citations: 10,
            reference_count: 1,
            authors: "Author".to_string(),
        }];
        let result = analyzer.analyze("deep learning", Some((2020, 2024)), 10, &papers);
        assert_eq!(result.total_papers, 0);
        assert_eq!(result.topic, "deep learning");
    }

    #[test]
    fn test_analyze_rising_trend() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let result = analyzer.analyze("llm research", Some((2020, 2024)), 3, &papers);
        assert!(result.total_papers >= 3);
        assert_eq!(result.topic, "llm research");
        assert!(!result.yearly_distribution.is_empty());
    }

    #[test]
    fn test_yearly_stats_computation() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let stats = analyzer.compute_yearly_stats(&papers, (2020, 2024));
        assert_eq!(stats.len(), 5);
        assert!(stats.iter().all(|s| s.year >= 2020 && s.year <= 2024));
        let total: usize = stats.iter().map(|s| s.paper_count).sum();
        assert_eq!(total, papers.len());
    }

    #[test]
    fn test_keyword_detection() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let trends = analyzer.detect_keyword_trends(&papers, (2020, 2024));
        let has_attention = trends.iter().any(|t| t.keyword.to_lowercase().contains("attention"));
        assert!(has_attention || !trends.is_empty());
    }

    #[test]
    fn test_compute_trend_rising() {
        let analyzer = TrendAnalyzer::new();
        let mut yearly_counts = HashMap::new();
        yearly_counts.insert(2020, 5);
        yearly_counts.insert(2021, 10);
        yearly_counts.insert(2022, 20);
        let trend = analyzer.compute_trend("transformer", &yearly_counts, (2020, 2022));
        assert!(trend.is_some());
        let trend = trend.unwrap();
        assert!([TrendDirection::Rising, TrendDirection::Emerging].contains(&trend.direction));
        assert!(trend.growth_rate > 0.0);
    }

    #[test]
    fn test_compute_trend_falling() {
        let analyzer = TrendAnalyzer::new();
        let mut yearly_counts = HashMap::new();
        yearly_counts.insert(2020, 100);
        yearly_counts.insert(2021, 50);
        yearly_counts.insert(2022, 25);
        let trend = analyzer.compute_trend("legacy-method", &yearly_counts, (2020, 2022));
        assert!(trend.is_some());
        let trend = trend.unwrap();
        assert_eq!(trend.direction, TrendDirection::Falling);
        assert!(trend.growth_rate < 0.0);
    }

    #[test]
    fn test_compute_trend_stable() {
        let analyzer = TrendAnalyzer::new();
        let mut yearly_counts = HashMap::new();
        yearly_counts.insert(2020, 100);
        yearly_counts.insert(2021, 105);
        yearly_counts.insert(2022, 102);
        let trend = analyzer.compute_trend("stable-method", &yearly_counts, (2020, 2022));
        assert!(trend.is_some());
        let trend = trend.unwrap();
        assert_eq!(trend.direction, TrendDirection::Stable);
    }

    #[test]
    fn test_growth_rate() {
        let analyzer = TrendAnalyzer::new();
        let stats = vec![
            YearlyStats { year: 2020, paper_count: 100, total_citations: 500, avg_citations: 5.0, keywords: HashMap::new() },
            YearlyStats { year: 2021, paper_count: 150, total_citations: 750, avg_citations: 5.0, keywords: HashMap::new() },
        ];
        let growth = analyzer.compute_growth_rate(&stats);
        assert!((growth - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_render_result() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let result = analyzer.analyze("AI research", Some((2020, 2024)), 3, &papers);
        let output = analyzer.render_result(&result);
        assert!(output.contains("AI research"));
        assert!(output.contains("研究趋势分析"));
    }

    #[test]
    fn test_render_mermaid_timeline() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let result = analyzer.analyze("AI research", Some((2020, 2024)), 3, &papers);
        let mermaid = analyzer.render_mermaid_timeline(&result);
        assert!(mermaid.contains("gantt"));
        assert!(mermaid.contains("Research Trends"));
    }

    #[test]
    fn test_render_mermaid_timeline_v2() {
        let analyzer = TrendAnalyzer::new();
        let papers = make_papers();
        let result = analyzer.analyze("AI research", Some((2020, 2024)), 3, &papers);
        let mermaid = analyzer.render_mermaid_timeline_v2(&result);
        if !result.rising_trends.is_empty() || !result.emerging_trends.is_empty() {
            assert!(mermaid.contains("xychart-beta"));
        }
    }

    #[test]
    fn test_empty_papers_returns_empty_result() {
        let analyzer = TrendAnalyzer::new();
        let result = analyzer.empty_result("test", (2020, 2024));
        assert_eq!(result.topic, "test");
        assert_eq!(result.total_papers, 0);
    }

    #[test]
    fn test_peak_year_detection() {
        let analyzer = TrendAnalyzer::new();
        let mut yearly_counts = HashMap::new();
        yearly_counts.insert(2020, 10);
        yearly_counts.insert(2021, 50);
        yearly_counts.insert(2022, 30);
        yearly_counts.insert(2023, 5);
        let trend = analyzer.compute_trend("peak-test", &yearly_counts, (2020, 2023));
        assert!(trend.is_some());
        assert_eq!(trend.unwrap().peak_year, 2021);
    }

    #[test]
    fn test_momentum_calculation() {
        let analyzer = TrendAnalyzer::new();
        let mut yearly_counts = HashMap::new();
        yearly_counts.insert(2020, 10);
        yearly_counts.insert(2021, 20);
        yearly_counts.insert(2022, 35);
        let trend = analyzer.compute_trend("momentum-test", &yearly_counts, (2020, 2022));
        assert!(trend.is_some());
        let t = trend.unwrap();
        assert!(t.velocity > 0.0);
    }
}
