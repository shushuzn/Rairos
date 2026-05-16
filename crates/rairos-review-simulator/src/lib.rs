//! Rairos Review Simulator — LLM-powered adversarial peer reviewer simulator

#![allow(clippy::type_repetition_in_bounds)]
//!
//! Simulates adversarial peer reviewers stress-testing a paper or proposal.
//! Plays hostile reviewer personas to surface weaknesses before submission.
//!
//! Replaces: llm/review_simulator.py

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ReviewSimulatorError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("LLM API error: {0}")]
    LlmApi(String),

    #[error("Failed to parse LLM response: {0}")]
    ParseError(String),

    #[error("No API key configured")]
    NoApiKey,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// Enums
// ============================================================================

/// Review dimensions that a reviewer can focus on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDimension {
    Methodology,
    NoveltyContribution,
    ClarityPresentation,
    BaselinesComparison,
    Reproducibility,
    Overclaiming,
    RelatedWork,
}

impl ReviewDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewDimension::Methodology => "methodology",
            ReviewDimension::NoveltyContribution => "novelty_contribution",
            ReviewDimension::ClarityPresentation => "clarity_presentation",
            ReviewDimension::BaselinesComparison => "baselines_comparison",
            ReviewDimension::Reproducibility => "reproducibility",
            ReviewDimension::Overclaiming => "overclaiming",
            ReviewDimension::RelatedWork => "related_work",
        }
    }
}

impl std::fmt::Display for ReviewDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Severity level of a review annotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical, // must fix before submission
    Major,    // significant weakness
    Minor,    // optional improvement
    Praise,   // genuinely good
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Major => "major",
            Severity::Minor => "minor",
            Severity::Praise => "praise",
        }
    }

    pub fn score(&self) -> i32 {
        match self {
            Severity::Critical => 4,
            Severity::Major => 3,
            Severity::Minor => 1,
            Severity::Praise => 0,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Recommendation outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Accept,
    Borderline,
    Reject,
    StrongReject,
}

impl Recommendation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Recommendation::Accept => "accept",
            Recommendation::Borderline => "borderline",
            Recommendation::Reject => "reject",
            Recommendation::StrongReject => "strong reject",
        }
    }
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// A single annotated comment on the paper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAnnotation {
    pub annotation_id: String,
    pub dimension: ReviewDimension,
    pub severity: Severity,
    /// "abstract", "introduction", "section 3", "table 2", etc.
    pub location: String,
    /// one-line summary of the issue
    pub headline: String,
    /// detailed reviewer comment
    pub comment: String,
    /// concrete fix suggestion
    #[serde(default)]
    pub suggestion: String,
    /// optional specific location (e.g., "page 5, line 12")
    #[serde(default)]
    pub page_line: String,
    /// Unix timestamp
    pub created_at: i64,
}

impl ReviewAnnotation {
    pub fn new(
        dimension: ReviewDimension,
        severity: Severity,
        location: &str,
        headline: &str,
        comment: &str,
    ) -> Self {
        Self {
            annotation_id: Uuid::new_v4().to_string()[..8].to_string(),
            dimension,
            severity,
            location: location.to_string(),
            headline: headline.to_string(),
            comment: comment.to_string(),
            suggestion: String::new(),
            page_line: String::new(),
            created_at: Utc::now().timestamp(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = suggestion.to_string();
        self
    }

    pub fn with_page_line(mut self, page_line: &str) -> Self {
        self.page_line = page_line.to_string();
        self
    }

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn created_at_iso(&self) -> String {
        DateTime::<Utc>::from_timestamp(self.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }
}

/// A simulated reviewer persona with a specific lens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPersona {
    pub name: String,
    pub focus: Vec<ReviewDimension>,
    pub tone: String,
    pub priority_instructions: String,
}

impl ReviewPersona {
    pub fn new(
        name: &str,
        focus: Vec<ReviewDimension>,
        tone: &str,
        priority_instructions: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            focus,
            tone: tone.to_string(),
            priority_instructions: priority_instructions.to_string(),
        }
    }

    /// Create the default methodology reviewer persona
    pub fn methodology_reviewer() -> Self {
        Self::new(
            "Methodology Reviewer",
            vec![
                ReviewDimension::Methodology,
                ReviewDimension::BaselinesComparison,
                ReviewDimension::Reproducibility,
            ],
            "hostile",
            "Focus on whether the methodology is sound, whether experiments are properly controlled, whether baselines are fair and complete, and whether the approach can be reproduced from the description.",
        )
    }

    /// Create the default contributions reviewer persona
    pub fn contributions_reviewer() -> Self {
        Self::new(
            "Contributions Reviewer",
            vec![
                ReviewDimension::NoveltyContribution,
                ReviewDimension::Overclaiming,
            ],
            "hostile",
            "Focus on whether the claimed contributions are genuinely novel, whether the paper overstated its significance, whether the novelty compared to prior work is real or incremental, and whether the 'novel' aspects are clearly articulated.",
        )
    }

    /// Create the default clarity reviewer persona
    pub fn clarity_reviewer() -> Self {
        Self::new(
            "Clarity Reviewer",
            vec![
                ReviewDimension::ClarityPresentation,
                ReviewDimension::RelatedWork,
            ],
            "constructive",
            "Focus on whether the paper is clearly written, whether the problem motivation is understandable, whether related work adequately situates the contribution, whether figures and tables are self-contained, and whether the writing obscures weaknesses.",
        )
    }

    /// Create the default ethics & scope reviewer persona
    pub fn ethics_scope_reviewer() -> Self {
        Self::new(
            "Ethics & Scope Reviewer",
            vec![
                ReviewDimension::Overclaiming,
                ReviewDimension::Reproducibility,
            ],
            "critical",
            "Focus on whether the paper makes claims beyond what the experiments support, whether limitations are honestly discussed, whether potential misuse cases are noted, and whether the scope of claimed applicability matches the evidence.",
        )
    }
}

/// Complete simulated review from one persona
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedReview {
    pub review_id: String,
    pub persona: String,
    /// 1–10 overall score
    pub overall_score: f64,
    /// 2–3 sentence overall assessment
    pub summary: String,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub annotations: Vec<ReviewAnnotation>,
    /// accept / borderline / reject / strong reject
    pub recommendation: String,
    /// Unix timestamp
    pub created_at: i64,
}

impl SimulatedReview {
    pub fn new(persona: &str) -> Self {
        Self {
            review_id: Uuid::new_v4().to_string()[..8].to_string(),
            persona: persona.to_string(),
            overall_score: 5.0,
            summary: String::new(),
            strengths: Vec::new(),
            weaknesses: Vec::new(),
            annotations: Vec::new(),
            recommendation: "borderline".to_string(),
            created_at: Utc::now().timestamp(),
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.overall_score = score;
        self
    }

    pub fn with_summary(mut self, summary: &str) -> Self {
        self.summary = summary.to_string();
        self
    }

    pub fn with_strengths(mut self, strengths: Vec<&str>) -> Self {
        self.strengths = strengths.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_weaknesses(mut self, weaknesses: Vec<&str>) -> Self {
        self.weaknesses = weaknesses.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_annotations(mut self, annotations: Vec<ReviewAnnotation>) -> Self {
        self.annotations = annotations;
        self
    }

    pub fn with_recommendation(mut self, recommendation: &str) -> Self {
        self.recommendation = recommendation.to_string();
        self
    }

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn created_at_iso(&self) -> String {
        DateTime::<Utc>::from_timestamp(self.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }
}

// ============================================================================
// Default Review Personas
// ============================================================================

/// Returns all default review personas
pub fn default_personas() -> Vec<ReviewPersona> {
    vec![
        ReviewPersona::methodology_reviewer(),
        ReviewPersona::contributions_reviewer(),
        ReviewPersona::clarity_reviewer(),
        ReviewPersona::ethics_scope_reviewer(),
    ]
}

// ============================================================================
// LLM API Client (internal)
// ============================================================================

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

fn get_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

fn resolve_credentials(
    explicit_base_url: Option<&str>,
    explicit_api_key: Option<&str>,
    explicit_model: Option<&str>,
) -> (String, String, String) {
    let base_url = explicit_base_url
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| get_env("OPENAI_BASE_URL"))
        .or_else(|| get_env("MINIMAX_CN_BASE_URL"))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let api_key = explicit_api_key
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| get_env("OPENAI_API_KEY"))
        .or_else(|| get_env("MINIMAX_CN_API_KEY"))
        .unwrap_or_default();

    let model = explicit_model
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| get_env("LLM_MODEL"))
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    (base_url, api_key, model)
}

/// Call the LLM chat completions API
async fn call_llm_chat_completions(
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ReviewSimulatorError> {
    if api_key.is_empty() {
        return Err(ReviewSimulatorError::NoApiKey);
    }

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    #[derive(Serialize)]
    struct ChatMessage {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<ChatMessage>,
    }

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(ReviewSimulatorError::LlmApi(format!(
            "Status {}: {}",
            status, text
        )));
    }

    #[derive(Deserialize)]
    struct ApiResponse {
        choices: Vec<Choice>,
    }

    #[derive(Deserialize)]
    struct Choice {
        message: MessageContent,
    }

    #[derive(Deserialize)]
    struct MessageContent {
        content: String,
    }

    let api_resp: ApiResponse = response.json().await?;
    api_resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| ReviewSimulatorError::ParseError("No choices in response".to_string()))
}

// ============================================================================
// Review Simulator Core
// ============================================================================

/// Simulate adversarial peer reviewers for a paper or proposal.
#[derive(Debug, Clone)]
pub struct ReviewSimulator {
    personas: Vec<ReviewPersona>,
}

impl Default for ReviewSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewSimulator {
    pub fn new() -> Self {
        Self {
            personas: default_personas(),
        }
    }

    pub fn with_personas(mut self, personas: Vec<ReviewPersona>) -> Self {
        self.personas = personas;
        self
    }

    /// Run a simulated review on the paper text.
    ///
    /// If `persona` is None, runs all personas and returns a merged consensus review.
    pub async fn review(
        &self,
        paper_text: &str,
        title: Option<&str>,
        persona: Option<&ReviewPersona>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        model: Option<&str>,
    ) -> Result<SimulatedReview, ReviewSimulatorError> {
        let (base_url, api_key, model) = resolve_credentials(base_url, api_key, model);

        if let Some(p) = persona {
            self.review_with_persona(
                p,
                paper_text,
                title.unwrap_or(""),
                &base_url,
                &api_key,
                &model,
            )
            .await
        } else {
            let mut reviews = Vec::new();
            for p in &self.personas {
                let r = self
                    .review_with_persona(
                        p,
                        paper_text,
                        title.unwrap_or(""),
                        &base_url,
                        &api_key,
                        &model,
                    )
                    .await?;
                reviews.push(r);
            }
            Ok(self.merge_reviews(&reviews))
        }
    }

    async fn review_with_persona(
        &self,
        persona: &ReviewPersona,
        paper_text: &str,
        title: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
    ) -> Result<SimulatedReview, ReviewSimulatorError> {
        let focus_dims = persona
            .focus
            .iter()
            .map(|d| d.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = format!(
            r#"You are simulating a {} peer reviewer for an academic paper.
Your persona: **{}** — you specialize in: {}

{}

Be adversarial. Find real weaknesses. Do not be polite.

PAPER TITLE: {}
PAPER TEXT (or key sections):
---
{}
---

TASK: Produce a structured adversarial review. Respond ONLY with valid JSON (no markdown, no explanation):

{{
  "overall_score": <1-10, where 1=reject, 10=accept>,
  "summary": "<2-3 sentence overall assessment>",
  "strengths": ["<strength 1>", "<strength 2>"],
  "weaknesses": ["<weakness 1>", "<weakness 2>", "<weakness 3>"],
  "recommendation": "accept | borderline | reject | strong reject",
  "annotations": [
    {{
      "dimension": "methodology" | "novelty_contribution" | "clarity_presentation" | "baselines_comparison" | "reproducibility" | "overclaiming" | "related_work",
      "severity": "critical | major | minor | praise",
      "location": "abstract | introduction | related work | methodology | experiments | results | discussion | table N | figure N",
      "headline": "<one-line summary of the issue>",
      "comment": "<detailed reviewer comment, 2-3 sentences>",
      "suggestion": "<concrete fix suggestion, 1-2 sentences>"
    }}
  ]
}}

Only include annotations for genuine issues. Maximum 6 annotations per review. If something is genuinely good, use severity "praise"."#,
            persona.tone,
            persona.name,
            focus_dims,
            persona.priority_instructions,
            title,
            &paper_text.chars().take(8000).collect::<String>()
        );

        let system_prompt = "You are an adversarial peer reviewer. Be critical but constructive. Respond with valid JSON only.";

        let response =
            call_llm_chat_completions(base_url, api_key, model, system_prompt, &prompt).await?;

        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(response.trim()).map_err(|e| {
            ReviewSimulatorError::ParseError(format!(
                "Invalid JSON: {} - Response: {}",
                e,
                &response[..response.len().min(200)]
            ))
        })?;

        let overall_score = parsed
            .get("overall_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let summary = parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let strengths: Vec<String> = parsed
            .get("strengths")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let weaknesses: Vec<String> = parsed
            .get("weaknesses")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let recommendation = parsed
            .get("recommendation")
            .and_then(|v| v.as_str())
            .unwrap_or("borderline")
            .to_string();

        let annotations: Vec<ReviewAnnotation> = parsed
            .get("annotations")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_annotation).collect())
            .unwrap_or_default();

        Ok(SimulatedReview {
            review_id: Uuid::new_v4().to_string()[..8].to_string(),
            persona: persona.name.clone(),
            overall_score,
            summary,
            strengths,
            weaknesses,
            annotations,
            recommendation,
            created_at: Utc::now().timestamp(),
        })
    }

    fn merge_reviews(&self, reviews: &[SimulatedReview]) -> SimulatedReview {
        if reviews.is_empty() {
            return SimulatedReview::new("Consensus Panel (0 reviewers)");
        }

        let total_score: f64 = reviews.iter().map(|r| r.overall_score).sum();
        let avg_score = total_score / reviews.len() as f64;

        // Collect all annotations
        let mut all_annotations: Vec<ReviewAnnotation> =
            reviews.iter().flat_map(|r| r.annotations.clone()).collect();

        // Deduplicate by headline similarity (first 40 chars)
        let mut seen_headlines = std::collections::HashSet::new();
        all_annotations.sort_by(|a, b| b.severity.score().cmp(&a.severity.score()));
        all_annotations.retain(|a| {
            let h = a
                .headline
                .chars()
                .take(40)
                .collect::<String>()
                .to_lowercase();
            seen_headlines.insert(h)
        });

        // Majority vote on recommendation
        let recommendations: Vec<&str> =
            reviews.iter().map(|r| r.recommendation.as_str()).collect();
        let final_rec = majority_vote(&recommendations)
            .map(String::from)
            .unwrap_or_else(|| "borderline".to_string());

        // Collect unique strengths and weaknesses
        let strengths: Vec<String> = reviews
            .iter()
            .flat_map(|r| r.strengths.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        let weaknesses: Vec<String> = reviews
            .iter()
            .flat_map(|r| r.weaknesses.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(6)
            .collect();

        SimulatedReview {
            review_id: Uuid::new_v4().to_string()[..8].to_string(),
            persona: format!("Consensus Panel ({} reviewers)", reviews.len()),
            overall_score: (avg_score * 10.0).round() / 10.0,
            summary: format!(
                "Consensus review from {} adversarial reviewers.",
                reviews.len()
            ),
            strengths,
            weaknesses,
            annotations: all_annotations.into_iter().take(8).collect(),
            recommendation: final_rec,
            created_at: Utc::now().timestamp(),
        }
    }

    /// Focus review on a single dimension only.
    pub async fn review_by_dimension(
        &self,
        paper_text: &str,
        dimension: ReviewDimension,
        base_url: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<Vec<ReviewAnnotation>, ReviewSimulatorError> {
        let (base_url, api_key, model) = resolve_credentials(base_url, api_key, None);

        let persona = ReviewPersona {
            name: format!("Focused {} Reviewer", dimension),
            focus: vec![dimension],
            tone: "hostile".to_string(),
            priority_instructions: format!(
                "You specialize exclusively in {}. Be extremely thorough in your area of expertise.",
                dimension
            ),
        };

        let review = self
            .review_with_persona(&persona, paper_text, "", &base_url, &api_key, &model)
            .await?;

        Ok(review.annotations)
    }
}

fn majority_vote<'a>(items: &[&'a str]) -> Option<&'a str> {
    let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for item in items {
        *freq.entry(item).or_insert(0) += 1;
    }
    // Use max_by_key on a tuple (count, key) to ensure deterministic result on ties
    freq.into_iter()
        .max_by_key(|(k, v)| (*v, *k))
        .map(|(v, _)| v)
}

fn parse_annotation(value: &serde_json::Value) -> Option<ReviewAnnotation> {
    let obj = value.as_object()?;

    let dimension_str = obj.get("dimension")?.as_str()?;
    let dimension = match dimension_str {
        "methodology" => ReviewDimension::Methodology,
        "novelty_contribution" => ReviewDimension::NoveltyContribution,
        "clarity_presentation" => ReviewDimension::ClarityPresentation,
        "baselines_comparison" => ReviewDimension::BaselinesComparison,
        "reproducibility" => ReviewDimension::Reproducibility,
        "overclaiming" => ReviewDimension::Overclaiming,
        "related_work" => ReviewDimension::RelatedWork,
        _ => return None,
    };

    let severity_str = obj.get("severity")?.as_str()?;
    let severity = match severity_str {
        "critical" => Severity::Critical,
        "major" => Severity::Major,
        "minor" => Severity::Minor,
        "praise" => Severity::Praise,
        _ => return None,
    };

    Some(ReviewAnnotation {
        annotation_id: Uuid::new_v4().to_string()[..8].to_string(),
        dimension,
        severity,
        location: obj
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        headline: obj
            .get("headline")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        comment: obj
            .get("comment")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        suggestion: obj
            .get("suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        page_line: obj
            .get("page_line")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        created_at: Utc::now().timestamp(),
    })
}

// ============================================================================
// Storage
// ============================================================================

fn get_review_path() -> std::path::PathBuf {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai_research_os")
        .join("review_simulator");
    std::fs::create_dir_all(&path).ok();
    path
}

/// Save a review to disk.
pub fn save_review(review: &SimulatedReview) -> std::path::PathBuf {
    let path = get_review_path();
    let filepath = path.join(format!("review_{}.json", review.review_id));
    let json = serde_json::to_string_pretty(review).unwrap_or_default();
    std::fs::write(&filepath, json).ok();
    filepath
}

/// Load a saved review.
pub fn load_review(review_id: &str) -> Option<SimulatedReview> {
    let path = get_review_path().join(format!("review_{}.json", review_id));
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Convert created_at from ISO string back to i64 if needed
    if let Some(ts) = value.get("created_at") {
        if ts.is_string() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts.as_str()?) {
                let mut new_value = value.clone();
                new_value["created_at"] = serde_json::json!(dt.timestamp());
                return serde_json::from_value(new_value).ok();
            }
        }
    }

    serde_json::from_value(value).ok()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewListItem {
    pub review_id: String,
    pub persona: String,
    pub overall_score: f64,
    pub recommendation: String,
    pub created_at: String,
    pub annotation_count: usize,
}

/// List saved reviews.
pub fn list_reviews(limit: usize) -> Vec<ReviewListItem> {
    let path = get_review_path();
    let mut items = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&path)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            name.to_string_lossy().starts_with("review_")
                && name.to_string_lossy().ends_with(".json")
        })
        .collect();

    entries.sort_by(|a, b| {
        let a_meta = a.metadata().ok();
        let b_meta = b.metadata().ok();
        let a_time = a_meta.as_ref().and_then(|m| m.modified().ok());
        let b_time = b_meta.as_ref().and_then(|m| m.modified().ok());
        b_time.cmp(&a_time)
    });

    for entry in entries.iter().take(limit) {
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let review_id = value
            .get("review_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let persona = value
            .get("persona")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let overall_score = value
            .get("overall_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let recommendation = value
            .get("recommendation")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Handle both string and i64 created_at
        let created_at = if let Some(ts) = value.get("created_at") {
            if let Some(s) = ts.as_str() {
                s.to_string()
            } else if let Some(i) = ts.as_i64() {
                DateTime::<Utc>::from_timestamp(i, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let annotation_count = value
            .get("annotations")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        items.push(ReviewListItem {
            review_id,
            persona,
            overall_score,
            recommendation,
            created_at,
            annotation_count,
        });
    }

    items
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_dimension_display() {
        assert_eq!(ReviewDimension::Methodology.to_string(), "methodology");
        assert_eq!(
            ReviewDimension::NoveltyContribution.to_string(),
            "novelty_contribution"
        );
    }

    #[test]
    fn test_severity_score() {
        assert_eq!(Severity::Critical.score(), 4);
        assert_eq!(Severity::Major.score(), 3);
        assert_eq!(Severity::Minor.score(), 1);
        assert_eq!(Severity::Praise.score(), 0);
    }

    #[test]
    fn test_review_persona_construction() {
        let persona = ReviewPersona::new(
            "Test Reviewer",
            vec![
                ReviewDimension::Methodology,
                ReviewDimension::Reproducibility,
            ],
            "hostile",
            "Focus on methodology",
        );
        assert_eq!(persona.name, "Test Reviewer");
        assert_eq!(persona.focus.len(), 2);
        assert_eq!(persona.tone, "hostile");
    }

    #[test]
    fn test_simulated_review_builder() {
        let review = SimulatedReview::new("Test Persona")
            .with_score(7.5)
            .with_summary("Good paper overall.")
            .with_strengths(vec!["Clear writing", "Solid experiments"])
            .with_weaknesses(vec!["Limited baselines"])
            .with_recommendation("accept");

        assert_eq!(review.overall_score, 7.5);
        assert_eq!(review.summary, "Good paper overall.");
        assert_eq!(review.strengths.len(), 2);
        assert_eq!(review.weaknesses.len(), 1);
        assert_eq!(review.recommendation, "accept");
    }

    #[test]
    fn test_review_annotation_builder() {
        let annotation = ReviewAnnotation::new(
            ReviewDimension::Methodology,
            Severity::Major,
            "introduction",
            "Weak baseline comparison",
            "The baselines selected are not competitive.",
        )
        .with_suggestion("Add established methods from the field.")
        .with_page_line("page 3, line 12");

        assert_eq!(annotation.dimension, ReviewDimension::Methodology);
        assert_eq!(annotation.severity, Severity::Major);
        assert_eq!(annotation.location, "introduction");
        assert_eq!(
            annotation.suggestion,
            "Add established methods from the field."
        );
    }

    #[test]
    fn test_review_simulator_default() {
        let simulator = ReviewSimulator::default();
        assert_eq!(simulator.personas.len(), 4);
    }

    #[test]
    fn test_review_simulator_with_custom_personas() {
        let custom = ReviewPersona::new(
            "Custom",
            vec![ReviewDimension::ClarityPresentation],
            "constructive",
            "Focus on clarity.",
        );
        let simulator = ReviewSimulator::new().with_personas(vec![custom.clone()]);
        assert_eq!(simulator.personas.len(), 1);
        assert_eq!(simulator.personas[0].name, "Custom");
    }

    #[test]
    fn test_simulated_review_serialization() {
        let review = SimulatedReview::new("Test Persona")
            .with_score(6.0)
            .with_summary("Decent paper.");

        let json = serde_json::to_string_pretty(&review).unwrap();
        assert!(json.contains("\"overallScore\": 6"));
        assert!(json.contains("\"persona\": \"Test Persona\""));
    }

    #[test]
    fn test_annotation_serialization() {
        let annotation = ReviewAnnotation::new(
            ReviewDimension::Overclaiming,
            Severity::Critical,
            "abstract",
            "Overstated contributions",
            "The abstract claims X but the paper only shows Y.",
        );

        let json = serde_json::to_string_pretty(&annotation).unwrap();
        assert!(json.contains("\"dimension\": \"overclaiming\""));
        assert!(json.contains("\"severity\": \"critical\""));
    }

    #[test]
    fn test_majority_vote() {
        // Single votes - highest count wins
        assert_eq!(
            majority_vote(&["accept", "accept", "reject"]),
            Some("accept")
        );
        assert_eq!(
            majority_vote(&["reject", "reject", "reject"]),
            Some("reject")
        );
        // No ties - deterministic
        assert_eq!(majority_vote(&["borderline", "accept"]), Some("borderline"));
        assert_eq!(majority_vote(&[]), None);
    }

    #[test]
    fn test_merge_reviews_empty() {
        let simulator = ReviewSimulator::new();
        let merged = simulator.merge_reviews(&[]);
        assert_eq!(merged.persona, "Consensus Panel (0 reviewers)");
    }

    #[test]
    fn test_merge_reviews_multiple() {
        let simulator = ReviewSimulator::new();
        let reviews = vec![
            SimulatedReview::new("R1")
                .with_score(6.0)
                .with_recommendation("accept"),
            SimulatedReview::new("R2")
                .with_score(4.0)
                .with_recommendation("reject"),
        ];
        let merged = simulator.merge_reviews(&reviews);
        assert_eq!(merged.overall_score, 5.0);
        assert_eq!(merged.persona, "Consensus Panel (2 reviewers)");
    }

    #[test]
    fn test_parse_annotation_valid() {
        let json = serde_json::json!({
            "dimension": "methodology",
            "severity": "major",
            "location": "introduction",
            "headline": "Test headline",
            "comment": "Test comment",
            "suggestion": "Test suggestion"
        });
        let annotation = parse_annotation(&json);
        assert!(annotation.is_some());
        let a = annotation.unwrap();
        assert_eq!(a.dimension, ReviewDimension::Methodology);
        assert_eq!(a.severity, Severity::Major);
    }

    #[test]
    fn test_parse_annotation_invalid() {
        let json = serde_json::json!({
            "dimension": "invalid_dimension",
            "severity": "major"
        });
        assert!(parse_annotation(&json).is_none());
    }

    #[test]
    fn test_review_list_item_serialization() {
        let item = ReviewListItem {
            review_id: "abc123".to_string(),
            persona: "Test".to_string(),
            overall_score: 7.0,
            recommendation: "accept".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            annotation_count: 3,
        };
        let json = serde_json::to_string_pretty(&item).unwrap();
        assert!(json.contains("\"review_id\": \"abc123\""));
        assert!(json.contains("\"annotation_count\": 3"));
    }

    #[test]
    fn test_default_personas() {
        let personas = default_personas();
        assert_eq!(personas.len(), 4);
        assert_eq!(personas[0].name, "Methodology Reviewer");
        assert_eq!(personas[1].name, "Contributions Reviewer");
        assert_eq!(personas[2].name, "Clarity Reviewer");
        assert_eq!(personas[3].name, "Ethics & Scope Reviewer");
    }

    #[test]
    fn test_persona_focus_dimensions() {
        let m = ReviewPersona::methodology_reviewer();
        assert!(m.focus.contains(&ReviewDimension::Methodology));
        assert!(m.focus.contains(&ReviewDimension::BaselinesComparison));
        assert!(m.focus.contains(&ReviewDimension::Reproducibility));

        let c = ReviewPersona::contributions_reviewer();
        assert!(c.focus.contains(&ReviewDimension::NoveltyContribution));
        assert!(c.focus.contains(&ReviewDimension::Overclaiming));
    }
}
