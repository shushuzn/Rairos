#![allow(dead_code)]
#![allow(clippy::manual_filter_map)]
//! rairos-evolution-report — Evolution Report Generator.
//!
//! Ported from `llm/evolution_report.py`.

use chrono::{Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use rairos_evolution::{get_evolution_memory, EvolutionMemory};

static STOPWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("the");
    s.insert("is");
    s.insert("are");
    s.insert("a");
    s.insert("an");
    s.insert("what");
    s.insert("how");
    s.insert("why");
    s.insert("this");
    s.insert("that");
    s.insert("and");
    s.insert("or");
    s.insert("的");
    s.insert("是");
    s.insert("如何");
    s.insert("什么");
    s.insert("怎么");
    s
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInsight {
    pub paper_id: String,
    pub title: String,
    pub positive_count: i32,
    pub negative_count: i32,
    pub avg_score: f64,
    #[serde(default)]
    pub related_queries: Vec<String>,
}

impl PaperInsight {
    pub fn boost_score(&self) -> f64 {
        let total = self.positive_count + self.negative_count;
        if total == 0 {
            return 0.0;
        }
        (self.positive_count as f64 - self.negative_count as f64 * 0.5) / total as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInsight {
    pub keywords: Vec<String>,
    pub avg_score: f64,
    pub success_rate: f64,
    #[serde(default)]
    pub related_papers: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningReport {
    pub period_start: String,
    pub period_end: String,
    pub total_queries: i32,
    pub positive_rate: f64,
    #[serde(default)]
    pub top_papers: Vec<PaperInsight>,
    #[serde(default)]
    pub top_keywords: Vec<String>,
    #[serde(default)]
    pub emerging_patterns: Vec<String>,
    #[serde(default)]
    pub predicted_interests: Vec<String>,
    #[serde(default)]
    pub questions_to_explore: Vec<String>,
    pub evolution_stage: String,
    pub progress_towards_next: String,
    #[serde(default)]
    pub user_journey: String,
    #[serde(default)]
    pub system_learned: String,
    #[serde(default)]
    pub highlight_moment: String,
}

impl LearningReport {
    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# AI Research OS 学习报告".to_string(),
            "".to_string(),
            format!(
                "*{} ~ {}*",
                &self.period_start[..10.min(self.period_start.len())],
                &self.period_end[..10.min(self.period_end.len())]
            ),
            "".to_string(),
        ];

        if !self.user_journey.is_empty() {
            lines.push(format!("> {}", self.user_journey));
            lines.push(String::new());
        }

        if !self.system_learned.is_empty() {
            lines.extend(vec![
                "## 系统学会了什么".to_string(),
                "".to_string(),
                format!("_{}_", self.system_learned),
                "".to_string(),
            ]);
        }

        lines.extend(vec![
            "## 回顾这一周".to_string(),
            "".to_string(),
            format!("你一共问了 **{}** 个问题，", self.total_queries),
            format!("其中 **{:.0}%** 让你感到满意。", self.positive_rate * 100.0),
            "".to_string(),
        ]);

        if !self.top_papers.is_empty() {
            lines.extend(vec!["### 你最常引用的论文".to_string(), "".to_string()]);
            let top = &self.top_papers[0];
            lines.push(format!("**{}** 是你的「老朋友」——", top.title));
            lines.push(format!(
                "你引用了 {} 次，每次都有收获。",
                top.positive_count
            ));
            if self.top_papers.len() > 1 {
                lines.push(String::new());
                lines.push("其他你关注的论文：".to_string());
                for p in &self.top_papers[1..3.min(self.top_papers.len())] {
                    lines.push(format!("- {}", p.title));
                }
            }
            lines.push(String::new());
        }

        if !self.top_keywords.is_empty() {
            let topics = self.top_keywords[..3.min(self.top_keywords.len())].join("、");
            lines.extend(vec![
                "### 你的研究焦点".to_string(),
                "".to_string(),
                format!("这周你主要探索了 **{}**。", topics),
                "".to_string(),
            ]);
        }

        if !self.questions_to_explore.is_empty() {
            lines.extend(vec![
                "### 你可能想问".to_string(),
                "".to_string(),
                "你问过类似的问题，也许可以深入一步：".to_string(),
                "".to_string(),
                format!("「{}」", self.questions_to_explore[0]),
            ]);
            lines.push(String::new());
        }

        if !self.predicted_interests.is_empty() {
            lines.extend(vec![
                "### 系统预测".to_string(),
                "".to_string(),
                format!(
                    "基于你的探索轨迹，我猜你接下来会感兴趣：**{}**。",
                    self.predicted_interests[0]
                ),
                "".to_string(),
            ]);
        }

        if !self.highlight_moment.is_empty() {
            lines.extend(vec![
                "### 高光时刻".to_string(),
                "".to_string(),
                format!("_{}_", self.highlight_moment),
                "".to_string(),
            ]);
        }

        lines.extend(vec![
            "---".to_string(),
            "".to_string(),
            format!("📍 {}", self.evolution_stage),
            "".to_string(),
            format!("**下一步**: {}", self.progress_towards_next),
            "".to_string(),
            "---".to_string(),
            "_由 AI Research OS 自进化系统生成_".to_string(),
        ]);

        lines.join("\n")
    }
}

pub struct EvolutionReporter {
    evo: EvolutionMemory,
}

impl EvolutionReporter {
    pub fn new(evolution_memory: Option<EvolutionMemory>) -> Self {
        Self {
            evo: evolution_memory.unwrap_or_else(get_evolution_memory),
        }
    }

    pub fn generate_report(&self, days: i32) -> LearningReport {
        let now = Utc::now();
        let start_time = (now - Duration::days(days as i64)).to_rfc3339();
        let feedbacks = self.collect_feedbacks_since(&start_time);

        if feedbacks.is_empty() {
            return self.empty_report(&start_time, &now.to_rfc3339());
        }

        let paper_insights = self.analyze_paper_insights(&feedbacks);
        let suggestions = self.generate_suggestions(&feedbacks, &paper_insights);
        let predicted = self.predict_interests(&feedbacks, &paper_insights);
        let stats = self.evo.get_stats();
        let (stage, progress) = self.get_evolution_status(&stats);

        let user_journey = self.generate_user_journey(&feedbacks, &paper_insights);
        let system_learned = self.generate_system_learned(&feedbacks, &paper_insights, &stats);
        let highlight = self.generate_highlight(&feedbacks, &paper_insights);

        let mut top_papers = paper_insights.clone();
        top_papers.sort_by(|a, b| {
            b.boost_score()
                .partial_cmp(&a.boost_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_papers.truncate(5);

        LearningReport {
            period_start: start_time,
            period_end: now.to_rfc3339(),
            total_queries: feedbacks.len() as i32,
            positive_rate: self.calc_positive_rate(&feedbacks),
            top_papers,
            top_keywords: self
                .extract_top_keywords(&feedbacks)
                .into_iter()
                .take(5)
                .collect(),
            emerging_patterns: self.find_emerging_patterns(&feedbacks),
            predicted_interests: predicted,
            questions_to_explore: suggestions,
            evolution_stage: stage,
            progress_towards_next: progress,
            user_journey,
            system_learned,
            highlight_moment: highlight,
        }
    }

    fn collect_feedbacks_since(&self, start_time: &str) -> Vec<HashMap<String, serde_json::Value>> {
        let mut feedbacks = Vec::new();
        if let Ok(text) = fs::read_to_string(self.evo.feedback_file()) {
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(data) = serde_json::from_str::<HashMap<String, serde_json::Value>>(line) {
                    if let Some(ts) = data.get("timestamp").and_then(|v| v.as_str()) {
                        if ts >= start_time {
                            feedbacks.push(data);
                        }
                    }
                }
            }
        }
        feedbacks
    }

    fn analyze_paper_insights(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
    ) -> Vec<PaperInsight> {
        let mut paper_data: HashMap<String, PaperInsightData> = HashMap::new();

        for fb in feedbacks {
            if fb.get("command").and_then(|v| v.as_str()) != Some("chat") {
                continue;
            }
            if let Some(paper_ids) = fb.get("paper_ids").and_then(|v| v.as_array()) {
                for pid in paper_ids {
                    if let Some(paper_id) = pid.as_str() {
                        let entry = paper_data.entry(paper_id.to_string()).or_default();
                        if fb.get("type").and_then(|v| v.as_str()) == Some("positive") {
                            entry.positive += 1;
                        } else {
                            entry.negative += 1;
                        }
                        if let Some(score) = fb.get("score").and_then(|v| v.as_f64()) {
                            entry.scores.push(score);
                        }
                        if let Some(query) = fb.get("query").and_then(|v| v.as_str()) {
                            entry.queries.push(query.chars().take(50).collect());
                        }
                    }
                }
            }
        }

        paper_data
            .into_iter()
            .map(|(paper_id, pdata)| {
                let avg_score = if !pdata.scores.is_empty() {
                    pdata.scores.iter().sum::<f64>() / pdata.scores.len() as f64
                } else {
                    0.0
                };
                PaperInsight {
                    paper_id: paper_id.clone(),
                    title: paper_id,
                    positive_count: pdata.positive,
                    negative_count: pdata.negative,
                    avg_score,
                    related_queries: pdata.queries.into_iter().take(3).collect(),
                }
            })
            .collect()
    }

    fn extract_top_keywords(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
    ) -> Vec<String> {
        let all_text: String = feedbacks
            .iter()
            .map(|fb| {
                let query = fb.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let papers: String = fb
                    .get("paper_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| p.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                format!("{} {}", query, papers)
            })
            .collect();

        let word_regex = Regex::new(r"[\u4e00-\u9fff]+|[a-zA-Z]{3,}").unwrap();
        let words: Vec<String> = word_regex
            .find_iter(&all_text.to_lowercase())
            .map(|m| m.as_str().to_string())
            .filter(|w| !STOPWORDS.contains(w.as_str()) && w.len() > 2)
            .collect();

        let mut word_counts: HashMap<&str, usize> = HashMap::new();
        for w in &words {
            *word_counts.entry(w).or_insert(0) += 1;
        }

        let mut sorted: Vec<(&str, usize)> = word_counts.into_iter().collect();
        sorted.sort_by_key(|a| a.1);
        sorted
            .into_iter()
            .take(10)
            .map(|(w, _)| w.to_string())
            .collect()
    }

    fn find_emerging_patterns(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
    ) -> Vec<String> {
        let mut patterns = Vec::new();
        let compare_kw = ["vs", "versus", "比较", "区别", "diff", "对比"];
        let compare_count: usize = feedbacks
            .iter()
            .filter(|fb| {
                if let Some(query) = fb.get("query").and_then(|v| v.as_str()) {
                    let q = query.to_lowercase();
                    compare_kw.iter().any(|kw| q.contains(kw))
                } else {
                    false
                }
            })
            .count();

        if compare_count > feedbacks.len() / 5 {
            patterns.push("你开始关注论文间的比较分析".to_string());
        }

        let long_queries: usize = feedbacks
            .iter()
            .filter(|fb| {
                fb.get("query")
                    .and_then(|v| v.as_str())
                    .map(|q| q.len() > 30)
                    .unwrap_or(false)
            })
            .count();

        if long_queries > feedbacks.len() / 2 {
            patterns.push("问题变得更加深入和具体".to_string());
        }

        patterns
    }

    fn generate_suggestions(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
        paper_insights: &[PaperInsight],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        if let Some(top_paper) = paper_insights.first() {
            suggestions.push(format!("深入探索 \"{}\" 的相关工作", top_paper.paper_id));
        }
        let keywords = self.extract_top_keywords(feedbacks);
        if let Some(first_kw) = keywords.first() {
            suggestions.push(format!("了解 {} 的最新研究进展", first_kw));
        }
        suggestions.extend(vec![
            "追踪你关注领域的最新论文".to_string(),
            "定期回顾已读论文的核心贡献".to_string(),
        ]);
        suggestions.truncate(5);
        suggestions
    }

    fn predict_interests(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
        _paper_insights: &[PaperInsight],
    ) -> Vec<String> {
        let mut predictions = Vec::new();
        let recent_queries: String = feedbacks
            .iter()
            .rev()
            .take(5)
            .filter_map(|fb| fb.get("query").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ");

        let recent_lower = recent_queries.to_lowercase();
        if recent_lower.contains("llm") || recent_lower.contains("language model") {
            predictions.push("LLM架构优化".to_string());
        }
        if recent_lower.contains("training") || recent_lower.contains("训练") {
            predictions.push("模型训练技巧".to_string());
        }
        if recent_lower.contains("efficient") || recent_lower.contains("高效") {
            predictions.push("效率优化方法".to_string());
        }
        predictions.truncate(3);
        predictions
    }

    fn calc_positive_rate(&self, feedbacks: &[HashMap<String, serde_json::Value>]) -> f64 {
        if feedbacks.is_empty() {
            return 0.0;
        }
        let positive = feedbacks
            .iter()
            .filter(|fb| fb.get("type").and_then(|v| v.as_str()) == Some("positive"))
            .count();
        positive as f64 / feedbacks.len() as f64
    }

    fn get_evolution_status(&self, stats: &HashMap<String, serde_json::Value>) -> (String, String) {
        let reliable = stats
            .get("reliable_patterns")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if reliable >= 5 {
            ("🚀 进化期".to_string(), "系统已具备自进化能力".to_string())
        } else if reliable >= 3 {
            (
                "🌲 成熟期".to_string(),
                "扩展模式库，覆盖更多场景".to_string(),
            )
        } else if reliable >= 1 {
            (
                "🌳 成长期".to_string(),
                "积累 10+ 反馈，强化现有模式".to_string(),
            )
        } else {
            (
                "🌱 种子期".to_string(),
                "继续使用，系统会持续学习".to_string(),
            )
        }
    }

    fn generate_user_journey(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
        _paper_insights: &[PaperInsight],
    ) -> String {
        let total = feedbacks.len();
        if total >= 20 {
            format!("这是充实的一周！你深入探索了 {} 个问题。", total)
        } else if total >= 10 {
            format!("你保持了良好的研究节奏，探讨了 {} 个有意义的问题。", total)
        } else if total >= 5 {
            format!("本周你提出了 {} 个问题，研究在稳步推进。", total)
        } else if total >= 1 {
            "你开始了新的探索旅程，提出了第一个问题。".to_string()
        } else {
            String::new()
        }
    }

    fn generate_system_learned(
        &self,
        feedbacks: &[HashMap<String, serde_json::Value>],
        _paper_insights: &[PaperInsight],
        stats: &HashMap<String, serde_json::Value>,
    ) -> String {
        let reliable = stats
            .get("reliable_patterns")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let total = feedbacks.len();

        if reliable >= 5 {
            format!(
                "我已经学会了 {} 个有效的回应模式，能够更好地帮助你理解论文。",
                reliable
            )
        } else if reliable >= 3 {
            let keywords = self.extract_top_keywords(feedbacks);
            let kw = keywords.first().map(|s| s.as_str()).unwrap_or("相关主题");
            format!("我注意到你对「{}」很感兴趣，学会了优先推荐这类内容。", kw)
        } else if total >= 10 {
            "通过你的反馈，我正在学习什么是最有帮助的回答方式。".to_string()
        } else if total >= 1 {
            "感谢你的第一个反馈！我正在学习如何更好地帮助你。".to_string()
        } else {
            "开始使用，让我了解你的研究风格。".to_string()
        }
    }

    fn generate_highlight(
        &self,
        _feedbacks: &[HashMap<String, serde_json::Value>],
        paper_insights: &[PaperInsight],
    ) -> String {
        if paper_insights.is_empty() {
            return String::new();
        }

        let top = &paper_insights[0];
        let pos_count = top.positive_count;

        if pos_count >= 5 {
            format!(
                "「{}」是你最信赖的参考资料，被引用了 {} 次！",
                top.title, pos_count
            )
        } else if pos_count >= 3 {
            format!("「{}」成为你的研究利器，帮你解答了多个问题。", top.title)
        } else if pos_count >= 1 {
            format!("「{}」开始进入你的研究视野。", top.title)
        } else {
            String::new()
        }
    }

    fn empty_report(&self, start: &str, end: &str) -> LearningReport {
        LearningReport {
            period_start: start.to_string(),
            period_end: end.to_string(),
            total_queries: 0,
            positive_rate: 0.0,
            top_papers: vec![],
            top_keywords: vec![],
            emerging_patterns: vec!["开始使用系统，开始你的研究之旅".to_string()],
            predicted_interests: vec![],
            questions_to_explore: vec![
                "尝试用 airos chat 问一个关于论文的问题".to_string(),
                "探索 airos search 发现新论文".to_string(),
                "用 airos slides 生成论文幻灯片".to_string(),
            ],
            evolution_stage: "🌱 种子期".to_string(),
            progress_towards_next: "开始使用，获得你的第一个满意回答".to_string(),
            user_journey: String::new(),
            system_learned: String::new(),
            highlight_moment: String::new(),
        }
    }
}

#[derive(Default)]
struct PaperInsightData {
    positive: i32,
    negative: i32,
    scores: Vec<f64>,
    queries: Vec<String>,
}

pub fn generate_evolution_report(days: i32) -> LearningReport {
    let reporter = EvolutionReporter::new(None);
    reporter.generate_report(days)
}

pub struct AdaptiveRetrieval {
    boost_data: HashMap<String, BoostEntry>,
    boost_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BoostEntry {
    #[serde(default)]
    positive_mentions: i32,
    #[serde(default)]
    negative_mentions: i32,
    #[serde(default)]
    queries: Vec<String>,
    #[serde(default)]
    boost_score: f64,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    last_update: String,
}

impl AdaptiveRetrieval {
    const CONFIDENCE_THRESHOLD: i32 = 5;
    const DIVERSITY_RATIO: f64 = 0.6;

    pub fn new() -> Self {
        let boost_file = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("memory")
            .join("evolution")
            .join("paper_boost.json");

        let boost_data = if let Ok(text) = fs::read_to_string(&boost_file) {
            serde_json::from_str(&text).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        Self {
            boost_data,
            boost_file,
        }
    }

    pub fn record_retrieval(&mut self, paper_id: &str, query: &str, was_useful: bool) {
        let entry = self
            .boost_data
            .entry(paper_id.to_string())
            .or_insert(BoostEntry {
                positive_mentions: 0,
                negative_mentions: 0,
                queries: vec![],
                boost_score: 0.0,
                confidence: 0.0,
                last_update: String::new(),
            });

        if was_useful {
            entry.positive_mentions += 1;
        } else {
            entry.negative_mentions += 1;
        }
        entry.queries.push(query.chars().take(100).collect());
        if entry.queries.len() > 20 {
            entry.queries = entry.queries.iter().rev().take(20).rev().cloned().collect();
        }

        let total = entry.positive_mentions + entry.negative_mentions;
        let (boost_score, confidence) = {
            let ws = Self::wilson_score(entry.positive_mentions, total, 0.95);
            let conf = (total as f64 / Self::CONFIDENCE_THRESHOLD as f64).min(1.0);
            (ws, conf)
        };
        entry.boost_score = boost_score;
        entry.confidence = confidence;
        entry.last_update = Utc::now().to_rfc3339();

        if let Ok(json) = serde_json::to_string_pretty(&self.boost_data) {
            let _ = fs::write(&self.boost_file, json);
        }
    }

    fn wilson_score(positives: i32, total: i32, _confidence: f64) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let p = positives as f64 / total as f64;
        let z = 1.645;
        let n = total as f64;
        let denom = 1.0 + z * z / n;
        let center = p + z * z / (2.0 * n);
        let margin = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
        let wilson_lower = (center - margin) / denom;
        wilson_lower * 2.0 - 0.5
    }

    pub fn get_boost(&self, paper_id: &str) -> (f64, f64) {
        self.boost_data
            .get(paper_id)
            .map(|e| (e.boost_score, e.confidence))
            .unwrap_or((0.0, 0.0))
    }
}

impl Default for AdaptiveRetrieval {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_adaptive_retrieval() -> AdaptiveRetrieval {
    AdaptiveRetrieval::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_insight_boost_score() {
        let insight = PaperInsight {
            paper_id: "paper1".to_string(),
            title: "Test Paper".to_string(),
            positive_count: 5,
            negative_count: 1,
            avg_score: 0.8,
            related_queries: vec![],
        };
        assert!((insight.boost_score() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_paper_insight_boost_score_zero() {
        let insight = PaperInsight {
            paper_id: "paper1".to_string(),
            title: "Test Paper".to_string(),
            positive_count: 0,
            negative_count: 0,
            avg_score: 0.0,
            related_queries: vec![],
        };
        assert_eq!(insight.boost_score(), 0.0);
    }

    #[test]
    fn test_learning_report_to_markdown() {
        let report = LearningReport {
            period_start: "2024-01-01T00:00:00Z".to_string(),
            period_end: "2024-01-07T00:00:00Z".to_string(),
            total_queries: 10,
            positive_rate: 0.7,
            top_papers: vec![],
            top_keywords: vec!["AI".to_string(), "ML".to_string()],
            emerging_patterns: vec![],
            predicted_interests: vec![],
            questions_to_explore: vec!["What is AI?".to_string()],
            evolution_stage: "🌳 成长期".to_string(),
            progress_towards_next: "Keep learning".to_string(),
            user_journey: "Great week!".to_string(),
            system_learned: "Learned patterns".to_string(),
            highlight_moment: "".to_string(),
        };
        let md = report.to_markdown();
        assert!(md.contains("AI Research OS 学习报告"));
        assert!(md.contains("10"));
        assert!(md.contains("70%"));
    }

    #[test]
    fn test_adaptive_retrieval_record_retrieval() {
        let mut retrieval = AdaptiveRetrieval::new();
        retrieval.record_retrieval("paper1", "test query", true);
        let (boost, conf) = retrieval.get_boost("paper1");
        assert!(boost > 0.0);
        assert!(conf > 0.0);
    }

    #[test]
    fn test_adaptive_retrieval_get_boost_unknown() {
        let retrieval = AdaptiveRetrieval::new();
        let (boost, conf) = retrieval.get_boost("unknown_paper");
        assert_eq!(boost, 0.0);
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn test_extract_top_keywords() {
        let reporter = EvolutionReporter::new(None);
        let feedbacks: Vec<HashMap<String, serde_json::Value>> = vec![serde_json::json!({
            "query": "What is machine learning?",
            "paper_ids": ["paper1", "paper2"]
        })]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        let keywords = reporter.extract_top_keywords(&feedbacks);
        assert!(
            keywords.contains(&"machine".to_string()) || keywords.contains(&"learning".to_string())
        );
    }
}
