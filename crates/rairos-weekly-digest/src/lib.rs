//! rairos-weekly-digest — Generate weekly research summaries.

//!
//! Ported from `llm/weekly_digest.py`.
//!
//! Pure stdlib implementation.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Data for a week.
#[derive(Debug, Clone, Default)]
pub struct WeekData {
    /// Start date string (YYYY-MM-DD).
    pub start_date: String,
    /// End date string (YYYY-MM-DD).
    pub end_date: String,
    /// Number of journal entries.
    pub journal_entries: usize,
    /// Number of experiments started.
    pub experiments_started: usize,
    /// Number of experiments completed.
    pub experiments_completed: usize,
    /// Number of new questions.
    pub questions_new: usize,
    /// Number of questions resolved.
    pub questions_resolved: usize,
    /// Number of papers added.
    pub papers_added: usize,
    /// Mood breakdown (mood -> count).
    pub mood_breakdown: HashMap<String, usize>,
    /// Top tags as (tag, count) pairs.
    pub top_tags: Vec<(String, usize)>,
    /// Highlight strings.
    pub highlights: Vec<String>,
}

/// A journal entry summary (minimal set of fields needed by WeeklyDigest).
#[derive(Debug, Clone, Default)]
pub struct JournalEntry {
    pub mood: Option<String>,
    pub tags: Vec<String>,
}

/// A question tracker entry.
#[derive(Debug, Clone, Default)]
pub struct Question {
    pub created_at: String,
    pub status: String,
    pub updated_at: Option<String>,
}

/// Trait for providing journal entries.
pub trait JournalProvider {
    fn list_entries(&self, days: u32) -> Vec<JournalEntry>;
}

/// Trait for providing experiment data.
pub trait ExperimentProvider {
    fn list_experiments(&self) -> Vec<Experiment>;
}

/// An experiment record.
#[derive(Debug, Clone)]
pub struct Experiment {
    pub created_at: String,
    pub status: String,
    pub completed_at: Option<String>,
}

/// Trait for providing question data.
pub trait QuestionProvider {
    fn list_questions(&self) -> Vec<Question>;
}

// ============================================================================
// Date Utilities (stdlib only)
// ============================================================================

/// Get current Unix timestamp seconds.
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Get timestamp from an ISO-8601 / RFC-3339-ish date string (YYYY-MM-DD).
/// Returns Unix timestamp seconds for midnight UTC of that day.
fn parse_date_to_timestamp(date_str: &str) -> Option<u64> {
    // Accept YYYY-MM-DD format
    let parts: Vec<u32> = date_str
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);

    // Simple day count from Unix epoch (rough, ignoring leap seconds)
    // days = 365*year + year/4 - year/100 + year/400 + (153*month+2)/5 + day-719469
    let days = days_since_epoch(year, month, day);
    Some(days * 86400)
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Compute days elapsed since Unix epoch for the given Gregorian date.
fn days_since_epoch(year: u32, month: u32, day: u32) -> u64 {
    // Days from 1970-01-01 to year-01-01
    let mut total_days: i64 = 0;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    // Days in months before this month
    let month_days = if is_leap(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for m in 0..month.saturating_sub(1) {
        total_days += month_days[m as usize] as i64;
    }
    (total_days + (day as i64 - 1)) as u64
}

/// Format a Unix timestamp (seconds) as YYYY-MM-DD in UTC.
fn format_timestamp(ts_secs: u64) -> String {
    let days = ts_secs / 86400;
    let mut year: u32 = 1970;
    let mut days_left = days as i64;

    loop {
        let days_in_year: i64 = if is_leap(year) { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month: u32 = 1;
    for (i, &md) in month_days.iter().enumerate() {
        if days_left < md as i64 {
            break;
        }
        days_left -= md as i64;
        month = i as u32 + 2;
    }
    let day = (days_left + 1) as u32;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

// ============================================================================
// Weekly Digest
// ============================================================================

/// Generate weekly research summaries.
pub struct WeeklyDigest;

impl WeeklyDigest {
    pub fn new() -> Self {
        Self
    }

    /// Collect data for the past N days.
    pub fn collect_week_data(
        &self,
        days: u32,
        journal: &dyn JournalProvider,
        experiments: &dyn ExperimentProvider,
        questions: &dyn QuestionProvider,
    ) -> WeekData {
        let now_ts = now_seconds();
        let start_ts = now_ts.saturating_sub((days as u64) * 86400);
        let start_str = format_timestamp(start_ts);
        let end_str = format_timestamp(now_ts);

        let mut week_data = WeekData {
            start_date: start_str,
            end_date: end_str,
            ..Default::default()
        };

        // Journal entries
        let entries = journal.list_entries(days);
        week_data.journal_entries = entries.len();

        // Mood breakdown and tag counts
        let mut mood_count: HashMap<String, usize> = HashMap::new();
        let mut tag_count: HashMap<String, usize> = HashMap::new();
        for e in &entries {
            if let Some(ref mood) = e.mood {
                *mood_count.entry(mood.clone()).or_insert(0) += 1;
            }
            for tag in &e.tags {
                *tag_count.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        week_data.mood_breakdown = mood_count;

        // Top tags sorted by count, top 5
        let mut tags_vec: Vec<(String, usize)> = tag_count.into_iter().collect();
        tags_vec.sort_by_key(|x| std::cmp::Reverse(x.1));
        week_data.top_tags = tags_vec.into_iter().take(5).collect();

        // Experiments
        let exps = experiments.list_experiments();
        for e in &exps {
            if let Some(ts) = parse_date_to_timestamp(&e.created_at) {
                if ts >= start_ts {
                    week_data.experiments_started += 1;
                }
            }
            if e.status == "completed" {
                if let Some(ref ca) = e.completed_at {
                    if let Some(ts) = parse_date_to_timestamp(ca) {
                        if ts >= start_ts {
                            week_data.experiments_completed += 1;
                        }
                    }
                }
            }
        }

        // Questions
        let qs = questions.list_questions();
        for q in &qs {
            if let Some(ts) = parse_date_to_timestamp(&q.created_at) {
                if ts >= start_ts {
                    week_data.questions_new += 1;
                }
            }
            if q.status == "resolved" {
                if let Some(ref ua) = q.updated_at {
                    if let Some(ts) = parse_date_to_timestamp(ua) {
                        if ts >= start_ts {
                            week_data.questions_resolved += 1;
                        }
                    }
                }
            }
        }

        week_data
    }

    /// Generate a text summary.
    pub fn generate_summary(&self, data: &WeekData) -> String {
        let mut lines = Vec::new();

        lines.push("=".repeat(60));
        lines.push("\u{1F4CA} Weekly Research Digest".to_string());
        lines.push(format!("   {} ~ {}", data.start_date, data.end_date));
        lines.push("=".repeat(60));
        lines.push(String::new());

        // Activity stats
        lines.push("## \u{1F4C8} Activity".to_string());
        lines.push(format!("  Journal entries: {}", data.journal_entries));
        lines.push(format!("  Experiments started: {}", data.experiments_started));
        lines.push(format!("  Experiments completed: {}", data.experiments_completed));
        lines.push(format!("  New questions: {}", data.questions_new));
        lines.push(format!("  Questions resolved: {}", data.questions_resolved));
        lines.push(String::new());

        // Mood
        if !data.mood_breakdown.is_empty() {
            lines.push("## \u{1F4AD} Mood".to_string());
            let mood_icons: HashMap<&str, &str> = [
                ("productive", "\u{26A1}"),
                ("stuck", "\u{1F613}"),
                ("excited", "\u{1F389}"),
                ("neutral", "\u{1F4DD}"),
            ]
            .into_iter()
            .collect();
            for (mood, count) in &data.mood_breakdown {
                let icon = mood_icons.get(mood.as_str()).copied().unwrap_or("\u{1F4DD}");
                lines.push(format!("  {} {}: {}", icon, mood, count));
            }
            lines.push(String::new());
        }

        // Tags
        if !data.top_tags.is_empty() {
            lines.push("## \u{1F3F7} Top Topics".to_string());
            for (tag, count) in data.top_tags.iter().take(5) {
                lines.push(format!("  {}: {}", tag, count));
            }
            lines.push(String::new());
        }

        // Highlights
        if !data.highlights.is_empty() {
            lines.push("## \u{2B50} Highlights".to_string());
            for h in &data.highlights {
                lines.push(format!("  \u{2022} {}", h));
            }
            lines.push(String::new());
        }

        // Productivity score
        let score = self.calculate_productivity_score(data);
        lines.push(format!("## \u{1F4C5} Productivity Score: {}/100", score));
        lines.push(String::new());
        lines.push("=".repeat(60));

        lines.join("\n")
    }

    /// Calculate a simple productivity score.
    pub fn calculate_productivity_score(&self, data: &WeekData) -> usize {
        let mut score: usize = 0;
        score += std::cmp::min(data.journal_entries * 5, 25); // Max 25 pts
        score += std::cmp::min(data.experiments_completed * 20, 40); // Max 40 pts
        score += std::cmp::min(data.questions_resolved * 15, 30); // Max 30 pts
        if data.mood_breakdown.get("excited").copied().unwrap_or(0) > 0 {
            score += 5; // Bonus for excitement
        }
        std::cmp::min(score, 100)
    }

    /// Render as Markdown.
    pub fn render_markdown(&self, data: &WeekData) -> String {
        let mut lines = Vec::new();

        lines.push("# Weekly Research Digest".to_string());
        lines.push(format!("**Period**: {} ~ {}", data.start_date, data.end_date));
        lines.push(String::new());

        // Stats table
        lines.push("## Stats".to_string());
        lines.push("| Metric | Value |".to_string());
        lines.push("|--------|-------|".to_string());
        lines.push(format!("| Journal entries | {} |", data.journal_entries));
        lines.push(format!("| Experiments started | {} |", data.experiments_started));
        lines.push(format!("| Experiments completed | {} |", data.experiments_completed));
        lines.push(format!("| New questions | {} |", data.questions_new));
        lines.push(format!("| Questions resolved | {} |", data.questions_resolved));
        lines.push(String::new());

        // Mood
        if !data.mood_breakdown.is_empty() {
            lines.push("## Mood Distribution".to_string());
            for (mood, count) in &data.mood_breakdown {
                lines.push(format!("- {}: {}", mood, count));
            }
            lines.push(String::new());
        }

        // Tags
        if !data.top_tags.is_empty() {
            lines.push("## Top Topics".to_string());
            for (tag, count) in &data.top_tags {
                lines.push(format!("- **{}**: {}", tag, count));
            }
            lines.push(String::new());
        }

        // Score
        let score = self.calculate_productivity_score(data);
        lines.push(format!("**Productivity Score**: {}/100", score));

        lines.join("\n")
    }
}

impl Default for WeeklyDigest {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock providers for testing
    struct MockJournal {
        entries: Vec<JournalEntry>,
    }
    impl MockJournal {
        fn new(entries: Vec<JournalEntry>) -> Self {
            Self { entries }
        }
    }
    impl JournalProvider for MockJournal {
        fn list_entries(&self, _days: u32) -> Vec<JournalEntry> {
            self.entries.clone()
        }
    }

    struct MockExperiments {
        experiments: Vec<Experiment>,
    }
    impl MockExperiments {
        fn new(experiments: Vec<Experiment>) -> Self {
            Self { experiments }
        }
    }
    impl ExperimentProvider for MockExperiments {
        fn list_experiments(&self) -> Vec<Experiment> {
            self.experiments.clone()
        }
    }

    struct MockQuestions {
        questions: Vec<Question>,
    }
    impl MockQuestions {
        fn new(questions: Vec<Question>) -> Self {
            Self { questions }
        }
    }
    impl QuestionProvider for MockQuestions {
        fn list_questions(&self) -> Vec<Question> {
            self.questions.clone()
        }
    }

    #[test]
    fn test_collect_week_data_empty() {
        let digest = WeeklyDigest::new();
        let journal = MockJournal::new(vec![]);
        let experiments = MockExperiments::new(vec![]);
        let questions = MockQuestions::new(vec![]);

        let data = digest.collect_week_data(7, &journal, &experiments, &questions);
        assert_eq!(data.journal_entries, 0);
        assert_eq!(data.experiments_started, 0);
        assert_eq!(data.experiments_completed, 0);
        assert_eq!(data.questions_new, 0);
        assert_eq!(data.questions_resolved, 0);
        assert!(!data.start_date.is_empty());
        assert!(!data.end_date.is_empty());
    }

    #[test]
    fn test_collect_week_data_with_entries() {
        let digest = WeeklyDigest::new();
        let journal = MockJournal::new(vec![
            JournalEntry {
                mood: Some("productive".to_string()),
                tags: vec!["ml".to_string(), "llm".to_string()],
            },
            JournalEntry {
                mood: Some("excited".to_string()),
                tags: vec!["ml".to_string()],
            },
        ]);
        let experiments = MockExperiments::new(vec![
            Experiment {
                created_at: "2024-01-01".to_string(),
                status: "completed".to_string(),
                completed_at: Some("2024-01-02".to_string()),
            },
        ]);
        let questions = MockQuestions::new(vec![
            Question {
                created_at: "2024-01-01".to_string(),
                status: "resolved".to_string(),
                updated_at: Some("2024-01-03".to_string()),
            },
        ]);

        let data = digest.collect_week_data(7, &journal, &experiments, &questions);
        assert_eq!(data.journal_entries, 2);
        assert_eq!(data.mood_breakdown.get("productive"), Some(&1));
        assert_eq!(data.mood_breakdown.get("excited"), Some(&1));
        assert!(data.top_tags.iter().any(|(t, _)| t == "ml"));
    }

    #[test]
    fn test_calculate_productivity_score() {
        let digest = WeeklyDigest::new();
        let data = WeekData {
            journal_entries: 3,    // 15 pts
            experiments_completed: 2, // 40 pts
            questions_resolved: 2, // 30 pts
            mood_breakdown: HashMap::new(),
            top_tags: vec![],
            highlights: vec![],
            start_date: String::new(),
            end_date: String::new(),
            papers_added: 0,
            experiments_started: 0,
            questions_new: 0,
        };
        // 15 + 40 + 30 = 85
        assert_eq!(digest.calculate_productivity_score(&data), 85);
    }

    #[test]
    fn test_calculate_productivity_score_max() {
        let digest = WeeklyDigest::new();
        let mut data = WeekData {
            journal_entries: 100,
            experiments_completed: 100,
            questions_resolved: 100,
            mood_breakdown: HashMap::new(),
            top_tags: vec![],
            highlights: vec![],
            start_date: String::new(),
            end_date: String::new(),
            papers_added: 0,
            experiments_started: 0,
            questions_new: 0,
        };
        // 25 + 40 + 30 = 95, no excited mood
        assert_eq!(digest.calculate_productivity_score(&data), 95);

        data.mood_breakdown
            .insert("excited".to_string(), 1);
        // +5 bonus = 100
        assert_eq!(digest.calculate_productivity_score(&data), 100);
    }

    #[test]
    fn test_generate_summary() {
        let digest = WeeklyDigest::new();
        let data = WeekData {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-07".to_string(),
            journal_entries: 5,
            experiments_started: 3,
            experiments_completed: 1,
            questions_new: 4,
            questions_resolved: 2,
            mood_breakdown: [("productive".to_string(), 3), ("excited".to_string(), 2)]
                .into_iter()
                .collect(),
            top_tags: vec![
                ("llm".to_string(), 5),
                ("ml".to_string(), 3),
            ],
            highlights: vec!["Made breakthrough".to_string()],
            papers_added: 0,
        };

        let summary = digest.generate_summary(&data);
        assert!(summary.contains("Weekly Research Digest"));
        assert!(summary.contains("Journal entries: 5"));
        assert!(summary.contains("Productivity Score"));
    }

    #[test]
    fn test_render_markdown() {
        let digest = WeeklyDigest::new();
        let data = WeekData {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-07".to_string(),
            journal_entries: 5,
            experiments_started: 3,
            experiments_completed: 1,
            questions_new: 4,
            questions_resolved: 2,
            mood_breakdown: [("productive".to_string(), 3)].into_iter().collect(),
            top_tags: vec![("llm".to_string(), 5)],
            highlights: vec![],
            papers_added: 0,
        };

        let md = digest.render_markdown(&data);
        assert!(md.contains("# Weekly Research Digest"));
        assert!(md.contains("| Journal entries | 5 |"));
        assert!(md.contains("**Productivity Score**:"));
    }

    #[test]
    fn test_parse_date_to_timestamp() {
        // Known dates
        let ts = parse_date_to_timestamp("1970-01-01").unwrap();
        assert_eq!(ts, 0);

        let ts = parse_date_to_timestamp("1970-01-02").unwrap();
        assert_eq!(ts, 86400);

        let ts = parse_date_to_timestamp("2024-01-01").unwrap();
        // 54 years from epoch: 54*365 + 13 leap days = 19723 days
        let expected = 19723 * 86400;
        assert_eq!(ts, expected);
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01");
        assert_eq!(format_timestamp(86400), "1970-01-02");
    }
}
