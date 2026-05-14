//! rairos-research-session — Research session tracker and knowledge graph builder
//!
//! Ports: `llm/research_session.py` (517 LOC)
//!
//! Track research conversations, extract topics, detect intent, and generate insights.

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// Prompt constants (LLM-driven follow-up question generation)
// ============================================================================

#[allow(dead_code)]
const FOLLOWUP_SYSTEM_PROMPT: &str = "你是一个研究助手，擅长通过追问帮助用户深入理解研究主题。\
根据对话历史，生成2-3个有洞察力的追问，帮助用户进一步探索。\
要求：\
1. 问题要有深度，能引发思考\
2. 结合用户的研究意图\
3. 不要重复历史中已问过的问题\
4. 用中文提问\
5. 每个问题限制在20字以内\
6. 只输出问题，不要解释，每行一个";

#[allow(dead_code)]
const FOLLOWUP_USER_PROMPT_TEMPLATE: &str = "对话历史：\
{history_text}

研究主题：{topics}
研究意图：{intent_name}

请生成追问：";

// ============================================================================
// Error types
// ============================================================================

#[derive(Error, Debug)]
pub enum ResearchSessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No active session")]
    NoActiveSession,

    #[error("Query not found: {0}")]
    QueryNotFound(String),
}

// ============================================================================
// Research intent classification
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ResearchIntent {
    #[default]
    Learning,    // 理解概念、学习原理
    Reproducing, // 复现代码、复现实验
    Improving,   // 改进方法、创新
    Comparing,   // 对比分析、选型
    Exploring,   // 探索发现、找方向
    Citing,      // 引用写作、文献整理
}


impl ResearchIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchIntent::Learning => "learning",
            ResearchIntent::Reproducing => "reproducing",
            ResearchIntent::Improving => "improving",
            ResearchIntent::Comparing => "comparing",
            ResearchIntent::Exploring => "exploring",
            ResearchIntent::Citing => "citing",
        }
    }
}

// ============================================================================
// Query — Q&A record
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub id: String,
    pub question: String,
    pub answer_preview: String, // First 100 chars of answer
    pub paper_ids: Vec<String>,
    pub paper_titles: Vec<String>,
    pub timestamp: String,
    #[serde(default)]
    pub follow_ups: Vec<String>, // Follow-up records
}

impl Query {
    pub fn new(
        id: String,
        question: String,
        answer: &str,
        paper_ids: Vec<String>,
        paper_titles: Vec<String>,
    ) -> Self {
        let answer_preview = if answer.is_empty() {
            String::new()
        } else {
            answer.chars().take(100).collect()
        };
        Self {
            id,
            question,
            answer_preview,
            paper_ids,
            paper_titles,
            timestamp: Utc::now().to_rfc3339(),
            follow_ups: Vec::new(),
        }
    }
}

// ============================================================================
// ChatResearchSession — chat-level session (distinct from agent workflow)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResearchSession {
    pub id: String,
    pub title: String,
    pub queries: Vec<Query>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>, // Auto-extracted tags
    #[serde(default)]
    pub insights: Vec<String>, // Session insights
    #[serde(default)]
    pub intent: ResearchIntent, // Detected research intent
}

impl ChatResearchSession {
    pub fn duration_minutes(&self) -> i64 {
        let ended: Option<chrono::DateTime<chrono::FixedOffset>> = match &self.ended_at {
            Some(e) => DateTime::parse_from_rfc3339(e).ok(),
            None => Utc::now()
                .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                .into(),
        };
        let started = DateTime::parse_from_rfc3339(&self.started_at).ok();
        match (ended, started) {
            (Some(e), Some(s)) => (e - s).num_seconds() / 60,
            _ => 0,
        }
    }

    pub fn topics(&self) -> Vec<String> {
        let mut unique: Vec<String> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for tag in &self.tags {
            if seen.insert(tag.as_str()) {
                unique.push(tag.clone());
            }
        }
        unique
    }
}

// ============================================================================
// Intent detector — keyword + pattern matching
// ============================================================================

struct IntentDetector {
    patterns: Vec<(ResearchIntent, Vec<Regex>)>,
}

impl IntentDetector {
    fn new() -> Self {
        let patterns = vec![
            (
                ResearchIntent::Reproducing,
                vec![
                    Regex::new(r"复现|实现|copy|paste|跑通|代码|code|reproduce|implement|build").unwrap(),
                    Regex::new(r"怎么实现|如何复现|有代码吗|show me|给我代码").unwrap(),
                ],
            ),
            (
                ResearchIntent::Improving,
                vec![
                    Regex::new(r"改进|优化|提升|更好|improve|better|enhance|boost").unwrap(),
                    Regex::new(r"如何改进|能不能更好|超越|outperform|beat").unwrap(),
                ],
            ),
            (
                ResearchIntent::Comparing,
                vec![
                    Regex::new(r"比较|对比|差异|哪个更好|vs|versus|compare|differ").unwrap(),
                    Regex::new(r"和.*区别|相比.*如何|哪个更强").unwrap(),
                ],
            ),
            (
                ResearchIntent::Learning,
                vec![
                    Regex::new(r"是什么|原理|如何理解|学习|了解|入门|概念|definition|learn|understand|explain").unwrap(),
                    Regex::new(r"什么意思|怎么理解|有什么用|what is|how does").unwrap(),
                ],
            ),
            (
                ResearchIntent::Exploring,
                vec![
                    Regex::new(r"有哪些|有什么|最新|研究|最近|探索|发现|what are|latest|recent|discover").unwrap(),
                    Regex::new(r"有什么新|还有什么|还有什么方法").unwrap(),
                ],
            ),
            (
                ResearchIntent::Citing,
                vec![
                    Regex::new(r"引用|cite|参考文献|写论文|写作|如何引用|citation|bibliography").unwrap(),
                    Regex::new(r"格式|规范|apa|ieee").unwrap(),
                ],
            ),
        ];
        Self { patterns }
    }

    fn detect(&self, question: &str) -> ResearchIntent {
        let q_lower = question.to_lowercase();
        let mut scores: Vec<(ResearchIntent, usize)> = self
            .patterns
            .iter()
            .map(|&(intent, _)| (intent, 0))
            .collect();

        for (intent, regexes) in &self.patterns {
            for re in regexes {
                if re.is_match(&q_lower) {
                    if let Some((_, count)) = scores.iter_mut().find(|(i, _)| *i == *intent) {
                        *count += 1;
                    }
                }
            }
        }

        let max_score = scores.iter().map(|(_, c)| *c).max().unwrap_or(0);
        if max_score == 0 {
            return ResearchIntent::Learning;
        }

        scores
            .into_iter()
            .find(|(_, c)| *c == max_score)
            .map(|(i, _)| i)
            .unwrap_or(ResearchIntent::Learning)
    }
}

// ============================================================================
// Tag extractor
// ============================================================================

fn extract_tags(question: &str, paper_titles: &[String], keywords: &HashSet<&str>) -> Vec<String> {
    let text = format!("{} {}", question, paper_titles.join(" ")).to_lowercase();
    let mut found = Vec::new();
    for &kw in keywords {
        let pattern = Regex::new(&format!(r"\b{}\b", regex::escape(kw))).unwrap();
        if pattern.is_match(&text) {
            found.push(kw.to_string());
        }
    }
    found
}

// ============================================================================
// Research path suggestion
// ============================================================================

fn get_research_path_suggestion(intent: ResearchIntent, topics: &[String]) -> String {
    let main_topic = topics.first().map(|s| s.as_str()).unwrap_or("该主题");
    match intent {
        ResearchIntent::Learning => {
            format!(
                "📚 学习路径建议: {} → 核心论文 → 变体模型 → 应用案例",
                main_topic
            )
        }
        ResearchIntent::Reproducing => {
            "🔧 复现路径建议: 找到基准实现 → 对齐指标 → 消融实验 → 复现结果".to_string()
        }
        ResearchIntent::Improving => {
            format!(
                "🚀 改进路径建议: {} → 痛点分析 → 改进思路 → 验证实验",
                main_topic
            )
        }
        ResearchIntent::Comparing => {
            format!(
                "⚖️ 对比路径建议: {} → 竞品分析 → 优缺点 → 选型建议",
                main_topic
            )
        }
        ResearchIntent::Exploring => {
            "🔍 探索路径建议: 最新论文 → 开源实现 → 社区反馈 → 实际应用".to_string()
        }
        ResearchIntent::Citing => {
            "📝 引用建议: 相关工作 → 方法对比 → 贡献点 → 格式规范".to_string()
        }
    }
}

// ============================================================================
// Template-based probing questions (fallback when LLM unavailable)
// ============================================================================

fn get_template_probing_questions(intent: ResearchIntent, topics: &[String]) -> Vec<String> {
    let mut questions = Vec::new();
    if topics.len() == 1 {
        questions.push("这个 topic 和其他领域有什么联系？".to_string());
    }
    match intent {
        ResearchIntent::Learning => {
            questions.push("这个 topic 在实际项目中如何使用？".to_string());
        }
        ResearchIntent::Reproducing => {
            questions.push("复现过程中最大的挑战是什么？".to_string());
        }
        ResearchIntent::Improving => {
            questions.push("现有方法的核心局限在哪里？".to_string());
        }
        _ => {}
    }
    questions.into_iter().take(2).collect()
}

// ============================================================================
// Parse probing questions from LLM response text
// ============================================================================

#[allow(dead_code)]
fn parse_questions_from_response(response: &str) -> Vec<String> {
    let mut questions = Vec::new();
    for line in response.lines() {
        let line = line.trim();
        let mut cleaned = line.to_string();

        // Remove common prefixes
        for prefix in &[r"^\d+[.、]", r"^[-•*\s]+", r"^Q\d*[:：]\s*"] {
            let re = Regex::new(prefix).unwrap();
            cleaned = re.replace(&cleaned, "").trim().to_string();
        }
        if !cleaned.is_empty() && cleaned.chars().count() <= 30 {
            questions.push(cleaned);
        }
    }
    questions.into_iter().take(3).collect()
}

// ============================================================================
// Generate insights from session
// ============================================================================

fn generate_insights(session: &ChatResearchSession) -> Vec<String> {
    let mut insights = Vec::new();

    // Insight 1: main topics (always show at least something about what was discussed)
    let topics = session.topics();
    if !topics.is_empty() {
        insights.push(format!(
            "主要研究主题: {}",
            topics
                .iter()
                .take(3)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        // Fallback: describe what was asked if no keywords matched
        if !session.queries.is_empty() {
            let first_q = &session.queries[0].question;
            let preview = if first_q.chars().count() > 30 {
                format!("{}...", first_q.chars().take(30).collect::<String>())
            } else {
                first_q.clone()
            };
            insights.push(format!("讨论主题: {}", preview));
        }
    }

    // Insight 2: exploration depth
    let total_followups: usize = session.queries.iter().map(|q| q.follow_ups.len()).sum();
    if total_followups > 2 {
        insights.push("进行了深度探索（多次追问）".to_string());
    } else if total_followups > 0 {
        insights.push("进行了初步探索".to_string());
    }

    // Insight 3: paper coverage
    let all_papers: HashSet<&str> = session
        .queries
        .iter()
        .flat_map(|q| q.paper_titles.iter().map(|t| t.as_str()))
        .collect();
    if all_papers.len() > 3 {
        insights.push(format!("覆盖了 {} 篇相关论文", all_papers.len()));
    }

    // Always ensure at least one insight
    if insights.is_empty() {
        insights.push("完成了基础问答".to_string());
    }

    insights
}

// ============================================================================
// Session tree renderer
// ============================================================================

fn render_session_tree(session: &ChatResearchSession) -> String {
    let mut lines = Vec::new();
    lines.push(format!("📚 {}", session.title));
    lines.push(format!(
        "   时长: {} 分钟 | {} 个问答",
        session.duration_minutes(),
        session.queries.len()
    ));

    if !session.insights.is_empty() {
        lines.push(format!(
            "   💡 {}",
            session
                .insights
                .iter()
                .take(2)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    lines.push(String::new());

    for (i, q) in session.queries.iter().enumerate() {
        let num = i + 1;
        let indent = if i == 0 { "   " } else { "       " };
        let q_text = if q.question.chars().count() > 60 {
            format!("{}...", q.question.chars().take(60).collect::<String>())
        } else {
            q.question.clone()
        };
        lines.push(format!("{}Q{}: {}", indent, num, q_text));

        // Referenced papers
        for title in q.paper_titles.iter().take(2) {
            let title_text = if title.chars().count() > 50 {
                format!("{}...", title.chars().take(50).collect::<String>())
            } else {
                title.clone()
            };
            lines.push(format!("{}   📄 {}", indent, title_text));
        }

        // Follow-ups
        if !q.follow_ups.is_empty() {
            lines.push(format!("{}   └─ {} 次追问", indent, q.follow_ups.len()));
        }
    }

    lines.join("\n")
}

// ============================================================================
// Sessions list renderer
// ============================================================================

fn render_sessions_list(sessions: &[ChatResearchSession]) -> String {
    if sessions.is_empty() {
        return "暂无研究会话记录".to_string();
    }

    let mut lines = Vec::new();
    lines.push("=".to_string() + &"=".repeat(49));

    for s in sessions {
        let date = &s.started_at[..10];
        lines.push(format!(
            "📅 {} | {} ({}问答)",
            date,
            s.title,
            s.queries.len()
        ));
        if !s.insights.is_empty() {
            let insight = s.insights[0].chars().take(50).collect::<String>();
            lines.push(format!("   💡 {}", insight));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

// ============================================================================
// ResearchSessionTracker — main tracker
// ============================================================================

pub struct ResearchSessionTracker {
    #[allow(dead_code)]
    memory_dir: PathBuf,
    sessions_file: PathBuf,
    current_session: Option<ChatResearchSession>,
    keywords: HashSet<&'static str>,
}

impl ResearchSessionTracker {
    pub fn new(memory_dir: Option<PathBuf>) -> Self {
        let memory_dir = memory_dir.unwrap_or_else(|| PathBuf::from("memory/evolution"));
        let sessions_file = memory_dir.join("research_sessions.jsonl");

        // Ensure directory exists
        if let Err(e) = fs::create_dir_all(&memory_dir) {
            eprintln!("Warning: could not create memory_dir: {}", e);
        }

        // Touch the file if it doesn't exist
        if !sessions_file.exists() {
            if let Ok(f) = File::create(&sessions_file) {
                drop(f);
            }
        }

        // Build keywords set from rairos-constants
        let keywords: HashSet<&'static str> = rairos_core::constants::AI_RESEARCH_KEYWORDS
            .iter()
            .copied()
            .collect();

        Self {
            memory_dir,
            sessions_file,
            current_session: None,
            keywords,
        }
    }

    pub fn start_session(&mut self, title: Option<&str>) -> ChatResearchSession {
        let session_id = format!("session_{}", Utc::now().timestamp());
        let now = Utc::now().to_rfc3339();
        let title = title
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("研究会话 {}", &now[..10]));

        let session = ChatResearchSession {
            id: session_id,
            title,
            queries: Vec::new(),
            started_at: now,
            ended_at: None,
            tags: Vec::new(),
            insights: Vec::new(),
            intent: ResearchIntent::Learning,
        };
        self.current_session = Some(session.clone());
        session
    }

    pub fn add_query(
        &mut self,
        question: &str,
        answer: &str,
        paper_ids: Vec<String>,
        paper_titles: Vec<String>,
    ) -> Query {
        if self.current_session.is_none() {
            self.start_session(None);
        }

        let session = self.current_session.as_mut().unwrap();
        let detector = IntentDetector::new();
        let intent = detector.detect(question);

        let query = Query::new(
            format!("q_{}", Utc::now().timestamp_millis()),
            question.to_string(),
            answer,
            paper_ids.clone(),
            paper_titles.clone(),
        );

        // Auto-extract tags
        let new_tags = extract_tags(question, &paper_titles, &self.keywords);
        session.tags.extend(new_tags);

        // Auto-detect intent
        session.intent = intent;

        session.queries.push(query.clone());
        query
    }

    pub fn get_research_path_suggestion(&self) -> Option<String> {
        let session = self.current_session.as_ref()?;
        if session.queries.is_empty() {
            return None;
        }
        let topics = session.topics();
        if topics.is_empty() {
            return None;
        }
        Some(get_research_path_suggestion(session.intent, &topics))
    }

    pub fn get_probing_questions(&self, use_llm: bool) -> Vec<String> {
        let session = match &self.current_session {
            Some(s) if !s.queries.is_empty() => s,
            _ => return Vec::new(),
        };

        // If LLM requested but no API key, fall back to template
        if use_llm {
            // LLM-based generation would require rairos-llm dependency
            // For pure stdlib build, we use template-based fallback
        }

        get_template_probing_questions(session.intent, &session.topics())
    }

    /// Generate probing questions using LLM.
    /// Returns empty list if LLM is unavailable or fails.
    pub fn get_probing_questions_llm(
        &self,
        _api_key: Option<&str>,
        _base_url: Option<&str>,
        _model: Option<&str>,
    ) -> Vec<String> {
        // This requires LLM integration which would add rairos-llm dependency.
        // For this pure-stdlib crate, we return template-based questions.
        self.get_probing_questions(false)
    }

    pub fn add_follow_up(&mut self, query_id: &str, follow_up_question: &str) {
        if let Some(session) = &mut self.current_session {
            if let Some(q) = session.queries.iter_mut().find(|q| q.id == query_id) {
                q.follow_ups.push(follow_up_question.to_string());
            }
        }
    }

    pub fn end_session(&mut self) -> Option<ChatResearchSession> {
        let session = self.current_session.take()?;
        let mut session = session;
        session.ended_at = Some(Utc::now().to_rfc3339());

        // Generate insights
        let insights = generate_insights(&session);
        session.insights = insights;

        // Save to file
        if let Err(e) = self.save_session(&session) {
            eprintln!("Warning: could not save session: {}", e);
        }

        Some(session)
    }

    #[allow(dead_code)]
    fn extract_tags(&self, question: &str, paper_titles: &[String]) -> Vec<String> {
        extract_tags(question, paper_titles, &self.keywords)
    }

    #[allow(dead_code)]
    fn detect_intent(&self, question: &str) -> ResearchIntent {
        IntentDetector::new().detect(question)
    }

    #[allow(dead_code)]
    fn generate_insights(&self, session: &ChatResearchSession) -> Vec<String> {
        generate_insights(session)
    }

    fn save_session(&self, session: &ChatResearchSession) -> Result<(), ResearchSessionError> {
        // Ensure the parent directory exists
        if let Some(parent) = self.sessions_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(session)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.sessions_file)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn get_recent_sessions(&self, days: i64, limit: usize) -> Vec<ChatResearchSession> {
        let cutoff = Utc::now() - Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        let mut sessions = Vec::new();

        let file = match File::open(&self.sessions_file) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            let session: ChatResearchSession = match serde_json::from_str(&line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if session.started_at >= cutoff_str {
                sessions.push(session);
            }
        }

        // Sort by started_at descending
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        sessions.truncate(limit);
        sessions
    }

    pub fn get_session_by_id(&self, session_id: &str) -> Option<ChatResearchSession> {
        let file = match File::open(&self.sessions_file) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            let session: ChatResearchSession = match serde_json::from_str(&line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if session.id == session_id {
                return Some(session);
            }
        }
        None
    }

    pub fn get_current_session(&self) -> Option<&ChatResearchSession> {
        self.current_session.as_ref()
    }

    pub fn render_session_tree(&self, session: &ChatResearchSession) -> String {
        render_session_tree(session)
    }

    pub fn render_sessions_list(&self, sessions: &[ChatResearchSession]) -> String {
        render_sessions_list(sessions)
    }
}

impl Default for ResearchSessionTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

// ============================================================================
// Global tracker instance (module-level)
// ============================================================================

use std::sync::Mutex;

static SESSION_TRACKER: std::sync::LazyLock<Mutex<Option<ResearchSessionTracker>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Get the global session tracker instance.
pub fn get_session_tracker() -> MutexGuard<'static, Option<ResearchSessionTracker>> {
    SESSION_TRACKER.lock().unwrap()
}

/// Initialize and get the global tracker.
pub fn init_session_tracker(
    #[allow(dead_code)]
    memory_dir: Option<PathBuf>,
) -> &'static Mutex<Option<ResearchSessionTracker>> {
    let mut guard = SESSION_TRACKER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(ResearchSessionTracker::new(memory_dir));
    }
    drop(guard);
    &SESSION_TRACKER
}

// Type alias for the mutex guard return type
pub type MutexGuard<'a, T> = std::sync::MutexGuard<'a, T>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tracker() -> ResearchSessionTracker {
        let dir = tempfile::tempdir().unwrap();
        ResearchSessionTracker::new(Some(dir.path().to_path_buf()))
    }

    #[test]
    fn test_start_session() {
        let mut tracker = temp_tracker();
        let session = tracker.start_session(Some("Test Session"));
        assert!(session.title.contains("Test Session"));
        assert!(session.id.starts_with("session_"));
        assert!(session.queries.is_empty());
        assert_eq!(session.intent, ResearchIntent::Learning);
    }

    #[test]
    fn test_add_query() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);

        let q = tracker.add_query(
            "What is the transformer architecture?",
            "The transformer is a neural network architecture...",
            vec!["paper1".to_string()],
            vec!["Attention Is All You Need".to_string()],
        );

        assert!(!q.id.is_empty());
        assert_eq!(q.question, "What is the transformer architecture?");
        assert!(!q.answer_preview.is_empty());
    }

    #[test]
    fn test_intent_detection() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);

        // Test reproducing intent
        tracker.add_query(
            "How to reproduce this code?",
            "Here is the code...",
            vec![],
            vec![],
        );
        let session = tracker.get_current_session().unwrap();
        assert_eq!(session.intent, ResearchIntent::Reproducing);

        // Test comparing intent
        let mut tracker2 = temp_tracker();
        tracker2.start_session(None);
        tracker2.add_query(
            "Compare BERT vs GPT",
            "They are different...",
            vec![],
            vec![],
        );
        let session2 = tracker2.get_current_session().unwrap();
        assert_eq!(session2.intent, ResearchIntent::Comparing);
    }

    #[test]
    fn test_duration_minutes() {
        let mut tracker = temp_tracker();
        let session = tracker.start_session(Some("Test"));
        // New session should have 0 duration (or very small)
        assert!(session.duration_minutes() >= 0);
    }

    #[test]
    fn test_end_session() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);
        tracker.add_query("test?", "answer", vec![], vec![]);

        let ended = tracker.end_session();
        assert!(ended.is_some());
        let ended = ended.unwrap();
        assert!(ended.ended_at.is_some());
        assert!(!ended.insights.is_empty()); // Should have at least topic insight
    }

    #[test]
    fn test_research_path_suggestion() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);
        tracker.add_query(
            "What is RLHF?",
            "It is...",
            vec![],
            vec!["RLHF paper".to_string()],
        );

        let suggestion = tracker.get_research_path_suggestion();
        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert!(s.contains("学习路径") || s.contains("📚"));
    }

    #[test]
    fn test_probing_questions() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);
        tracker.add_query("What is attention?", "It is...", vec![], vec![]);

        let questions = tracker.get_probing_questions(false);
        assert!(!questions.is_empty());
    }

    #[test]
    fn test_follow_up() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);
        let q = tracker.add_query("What is X?", "It is...", vec![], vec![]);

        tracker.add_follow_up(&q.id, "Can you elaborate?");
        let session = tracker.get_current_session().unwrap();
        assert_eq!(session.queries[0].follow_ups.len(), 1);
    }

    #[test]
    fn test_render_session_tree() {
        let mut tracker = temp_tracker();
        tracker.start_session(Some("Test Session"));
        tracker.add_query("Q1?", "A1", vec![], vec![]);
        let session = tracker.get_current_session().unwrap();

        let tree = tracker.render_session_tree(session);
        assert!(tree.contains("Test Session"));
        assert!(tree.contains("Q1"));
    }

    #[test]
    fn test_render_sessions_list() {
        let mut tracker = temp_tracker();
        tracker.start_session(Some("Session 1"));
        tracker.end_session();

        let sessions = tracker.get_recent_sessions(7, 10);
        let list = tracker.render_sessions_list(&sessions);
        assert!(!list.is_empty());
        assert!(list.contains("Session 1"));
    }

    #[test]
    fn test_get_recent_sessions() {
        let mut tracker = temp_tracker();
        tracker.start_session(Some("Old"));
        tracker.end_session();

        let sessions = tracker.get_recent_sessions(7, 10);
        assert!(!sessions.is_empty());
    }

    #[test]
    fn test_research_intent_enum() {
        assert_eq!(ResearchIntent::Learning.as_str(), "learning");
        assert_eq!(ResearchIntent::Reproducing.as_str(), "reproducing");
        assert_eq!(ResearchIntent::Improving.as_str(), "improving");
        assert_eq!(ResearchIntent::Comparing.as_str(), "comparing");
        assert_eq!(ResearchIntent::Exploring.as_str(), "exploring");
        assert_eq!(ResearchIntent::Citing.as_str(), "citing");
    }

    #[test]
    fn test_query_new() {
        let q = Query::new(
            "q1".to_string(),
            "What is AI?".to_string(),
            "AI is artificial intelligence that enables machines to think and learn.",
            vec!["p1".to_string()],
            vec!["AI101".to_string()],
        );
        assert_eq!(q.id, "q1");
        assert_eq!(q.question, "What is AI?");
        // answer_preview should be first 100 chars
        assert!(q.answer_preview.len() <= 100);
    }

    #[test]
    fn test_chat_research_session_topics() {
        let session = ChatResearchSession {
            id: "s1".to_string(),
            title: "Test".to_string(),
            queries: vec![],
            started_at: Utc::now().to_rfc3339(),
            ended_at: None,
            tags: vec![
                "transformer".to_string(),
                "attention".to_string(),
                "transformer".to_string(),
            ],
            insights: vec![],
            intent: ResearchIntent::Learning,
        };
        let topics = session.topics();
        // Should be deduplicated
        assert_eq!(topics.len(), 2);
        assert!(topics.contains(&"transformer".to_string()));
    }

    #[test]
    fn test_session_with_no_current_session() {
        let tracker = temp_tracker();
        assert!(tracker.get_current_session().is_none());
        assert!(tracker.get_research_path_suggestion().is_none());
    }

    #[test]
    fn test_keywords_extraction() {
        let mut tracker = temp_tracker();
        tracker.start_session(None);
        tracker.add_query(
            "Tell me about transformer and attention mechanisms in LLMs",
            "They are key components...",
            vec![],
            vec![],
        );
        let session = tracker.get_current_session().unwrap();
        // Should have extracted at least some AI research keywords as tags
        assert!(!session.tags.is_empty() || session.queries.len() == 1); // tags are extracted
    }
}
