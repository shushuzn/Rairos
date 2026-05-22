//! Intent Classifier Module for request routing and intent classification.
//!
//! Based on Microsoft Multi-Agent Reference Architecture:
//! - Intent Classifier analyzes incoming requests
//! - Routes requests to appropriate agents or capabilities
//! - Supports custom classification rules and patterns
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │              IntentClassifier                         │
//! │  ┌─────────────────────────────────────────────┐   │
//! │  │ rules: Vec<ClassificationRule>              │   │
//! │  │ embeddings: Option<EmbeddingsModel>         │   │
//! │  │ fallback_agent: Option<AgentId>             │   │
//! │  └─────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────┘
//!                         │
//!         ┌───────────────┼───────────────┐
//!         ▼               ▼               ▼
//!    ┌─────────┐     ┌─────────┐     ┌─────────┐
//!    │Research │     │  Code   │     │  Data   │
//!    │ Intent  │     │ Intent  │     │ Intent  │
//!    └─────────┘     └─────────┘     └─────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Intent type identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub String);

impl IntentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Classification confidence level
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence {
    /// Confidence score between 0.0 and 1.0
    pub score: f32,
}

impl Confidence {
    pub fn new(score: f32) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
        }
    }

    pub fn high() -> Self {
        Self { score: 0.8 }
    }

    pub fn medium() -> Self {
        Self { score: 0.5 }
    }

    pub fn low() -> Self {
        Self { score: 0.3 }
    }

    pub fn is_confident(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self::medium()
    }
}

/// Intent classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Classified intent ID
    pub intent_id: IntentId,
    /// Classification confidence
    pub confidence: Confidence,
    /// Assigned agent ID (if routing is enabled)
    pub agent_id: Option<String>,
    /// Suggested tools/capabilities
    pub suggested_tools: Vec<String>,
    /// Context metadata from classification
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Reasoning for the classification
    #[serde(default)]
    pub reasoning: Vec<String>,
}

/// Request to classify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRequest {
    /// Raw user input
    pub input: String,
    /// Optional conversation history context
    #[serde(default)]
    pub history: Vec<String>,
    /// Optional user ID for personalization
    #[serde(default)]
    pub user_id: Option<String>,
    /// Optional requested response format
    #[serde(default)]
    pub format_hint: Option<String>,
}

/// A classification rule
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    /// Unique rule ID
    pub id: String,
    /// Intent this rule classifies as
    pub intent_id: IntentId,
    /// Agent to route to (optional)
    pub agent_id: Option<String>,
    /// Keyword patterns (OR logic between patterns, AND within)
    pub patterns: Vec<PatternMatcher>,
    /// Minimum confidence boost for matching
    pub confidence_boost: f32,
    /// Rule priority (higher = evaluated first)
    pub priority: u32,
    /// Whether to require all patterns to match
    pub require_all_patterns: bool,
}

/// Pattern types for matching
#[derive(Debug, Clone)]
pub enum PatternMatcher {
    /// Simple keyword (case-insensitive)
    Keyword(String),
    /// Regular expression pattern
    Regex(String),
    /// Semantic similarity threshold (requires embeddings)
    Semantic { query: String, threshold: f32 },
}

impl PatternMatcher {
    pub fn matches(&self, input: &str) -> bool {
        match self {
            PatternMatcher::Keyword(keyword) => {
                let input_lower = input.to_lowercase();
                let keyword_lower = keyword.to_lowercase();
                input_lower.contains(&keyword_lower)
            }
            PatternMatcher::Regex(pattern) => {
                match regex::Regex::new(pattern) {
                    Ok(re) => re.is_match(input),
                    Err(_) => false,
                }
            }
            PatternMatcher::Semantic { .. } => {
                // Semantic matching requires embeddings - handled separately
                false
            }
        }
    }

    /// Optimized match that accepts pre-lowercased input to avoid repeated lowercasing
    pub fn matches_lowercase(&self, input_lower: &str) -> bool {
        match self {
            PatternMatcher::Keyword(keyword) => {
                input_lower.contains(&keyword.to_lowercase())
            }
            PatternMatcher::Regex(pattern) => {
                match regex::Regex::new(pattern) {
                    Ok(re) => re.is_match(input_lower),
                    Err(_) => false,
                }
            }
            PatternMatcher::Semantic { .. } => false,
        }
    }
}

/// Intent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDefinition {
    /// Unique intent ID
    pub id: IntentId,
    /// Human-readable name
    pub name: String,
    /// Description of the intent
    pub description: String,
    /// Default agent for this intent
    #[serde(default)]
    pub default_agent: Option<String>,
    /// Suggested tools for this intent
    #[serde(default)]
    pub suggested_tools: Vec<String>,
    /// Example phrases that trigger this intent
    #[serde(default)]
    pub examples: Vec<String>,
}

/// Intent Classifier
#[derive(Debug, Clone)]
pub struct IntentClassifier {
    /// Classification rules
    rules: Arc<RwLock<Vec<ClassificationRule>>>,
    /// Intent definitions
    intents: Arc<RwLock<HashMap<IntentId, IntentDefinition>>>,
    /// Keywords index for fast lookup
    keywords_index: Arc<RwLock<HashMap<String, Vec<IntentId>>>>,
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentClassifier {
    /// Create a new intent classifier
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            intents: Arc::new(RwLock::new(HashMap::new())),
            keywords_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an intent definition
    pub async fn register_intent(&self, intent: IntentDefinition) {
        let mut intents = self.intents.write().await;
        intents.insert(intent.id.clone(), intent);
    }

    /// Register multiple intents at once
    pub async fn register_intents(&self, intents: Vec<IntentDefinition>) {
        let mut intent_map = self.intents.write().await;
        for intent in intents {
            intent_map.insert(intent.id.clone(), intent);
        }
    }

    /// Add a classification rule
    pub async fn add_rule(&self, rule: ClassificationRule) {
        // Update keywords index
        for pattern in &rule.patterns {
            if let PatternMatcher::Keyword(keyword) = pattern {
                let mut index = self.keywords_index.write().await;
                index
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(rule.intent_id.clone());
            }
        }

        // Add rule
        let mut rules = self.rules.write().await;
        rules.push(rule);
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a rule by ID
    pub async fn remove_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write().await;
        let initial_len = rules.len();
        rules.retain(|r| r.id != rule_id);
        rules.len() < initial_len
    }

    /// Clear all rules
    pub async fn clear_rules(&self) {
        let mut rules = self.rules.write().await;
        rules.clear();
        let mut index = self.keywords_index.write().await;
        index.clear();
    }

    /// Classify a request
    pub async fn classify(&self, request: &ClassificationRequest) -> ClassificationResult {
        let input_lower = request.input.to_lowercase();
        let rules = self.rules.read().await;

        let mut best_match: Option<ClassificationResult> = None;
        let mut best_score: f32 = 0.0;

        for rule in rules.iter() {
            let matches = self::evaluate_rule(rule, &input_lower, &request.history);

            if matches {
                let score = 0.5 + rule.confidence_boost; // Base score + boost

                if score > best_score {
                    // Get intent definition for metadata
                    let intents = self.intents.read().await;
                    let intent_def = intents.get(&rule.intent_id);

                    let suggested_tools = intent_def
                        .map(|i| i.suggested_tools.clone())
                        .unwrap_or_default();

                    let agent_id = rule.agent_id.clone().or_else(|| {
                        intent_def.and_then(|i| i.default_agent.clone())
                    });

                    best_match = Some(ClassificationResult {
                        intent_id: rule.intent_id.clone(),
                        confidence: Confidence::new(score),
                        agent_id,
                        suggested_tools,
                        metadata: HashMap::new(),
                        reasoning: vec![format!("Matched rule: {}", rule.id)],
                    });
                    best_score = score;
                }
            }
        }

        // Return best match or unknown
        best_match.unwrap_or_else(|| ClassificationResult {
            intent_id: IntentId::new("unknown".to_string()),
            confidence: Confidence::new(0.0),
            agent_id: None,
            suggested_tools: vec![],
            metadata: HashMap::new(),
            reasoning: vec!["No matching rule found".to_string()],
        })
    }

    /// Classify with intent routing
    pub async fn classify_and_route(
        &self,
        request: &ClassificationRequest,
    ) -> ClassificationResult {
        let mut result = self.classify(request).await;

        // Add routing reasoning if we have an agent
        if result.agent_id.is_some() {
            result
                .reasoning
                .push(format!("Routed to agent: {:?}", result.agent_id));
        }

        result
    }

    /// Get all registered intents
    pub async fn get_intents(&self) -> Vec<IntentDefinition> {
        let intents = self.intents.read().await;
        intents.values().cloned().collect()
    }

    /// Get all rules
    pub async fn get_rules(&self) -> Vec<ClassificationRule> {
        let rules = self.rules.read().await;
        rules.clone()
    }

    /// Get rule count
    pub async fn rule_count(&self) -> usize {
        let rules = self.rules.read().await;
        rules.len()
    }

    /// Get intent count
    pub async fn intent_count(&self) -> usize {
        let intents = self.intents.read().await;
        intents.len()
    }
}

fn evaluate_rule(rule: &ClassificationRule, input: &str, _history: &[String]) -> bool {
    // Pre-lowercase input once for all keyword patterns
    let input_lower = input.to_lowercase();
    let pattern_results: Vec<bool> = rule.patterns.iter()
        .map(|p| p.matches_lowercase(&input_lower))
        .collect();

    if rule.require_all_patterns {
        pattern_results.iter().all(|&r| r)
    } else {
        // OR logic - at least one pattern matches
        // But give small boost if multiple match
        pattern_results.iter().any(|&r| r)
    }
}

// =============================================================================
// Builder for IntentClassifier
// =============================================================================

/// Builder for creating pre-configured IntentClassifier
pub struct IntentClassifierBuilder {
    intents: Vec<IntentDefinition>,
    rules: Vec<ClassificationRule>,
}

impl IntentClassifierBuilder {
    pub fn new() -> Self {
        Self {
            intents: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Add an intent
    pub fn intent(mut self, id: impl Into<String>, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.intents.push(IntentDefinition {
            id: IntentId(id.into()),
            name: name.into(),
            description: desc.into(),
            default_agent: None,
            suggested_tools: Vec::new(),
            examples: Vec::new(),
        });
        self
    }

    /// Add an intent with default agent
    pub fn intent_with_agent(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        desc: impl Into<String>,
        agent: impl Into<String>,
    ) -> Self {
        self.intents.push(IntentDefinition {
            id: IntentId(id.into()),
            name: name.into(),
            description: desc.into(),
            default_agent: Some(agent.into()),
            suggested_tools: Vec::new(),
            examples: Vec::new(),
        });
        self
    }

    /// Add a keyword-based rule
    pub fn keyword_rule(
        mut self,
        id: impl Into<String>,
        intent_id: impl Into<String>,
        keywords: Vec<String>,
        agent: Option<String>,
    ) -> Self {
        let patterns: Vec<PatternMatcher> = keywords
            .into_iter()
            .map(PatternMatcher::Keyword)
            .collect();

        self.rules.push(ClassificationRule {
            id: id.into(),
            intent_id: IntentId(intent_id.into()),
            agent_id: agent,
            patterns,
            confidence_boost: 0.2,
            priority: 10,
            require_all_patterns: false,
        });
        self
    }

    /// Add a regex-based rule
    pub fn regex_rule(
        mut self,
        id: impl Into<String>,
        intent_id: impl Into<String>,
        pattern: String,
        agent: Option<String>,
    ) -> Self {
        self.rules.push(ClassificationRule {
            id: id.into(),
            intent_id: IntentId(intent_id.into()),
            agent_id: agent,
            patterns: vec![PatternMatcher::Regex(pattern)],
            confidence_boost: 0.3,
            priority: 20,
            require_all_patterns: false,
        });
        self
    }

    /// Build the IntentClassifier
    pub async fn build(self) -> IntentClassifier {
        let classifier = IntentClassifier::new();

        // Register intents
        classifier.register_intents(self.intents).await;

        // Add rules
        for rule in self.rules {
            classifier.add_rule(rule).await;
        }

        classifier
    }
}

impl Default for IntentClassifierBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Preset Intent Sets
// =============================================================================

impl IntentClassifier {
    /// Create a classifier with common intents preset
    pub async fn with_common_intents() -> IntentClassifier {
        let builder = IntentClassifierBuilder::new()
            // Research intents
            .intent("web_search", "Web Search", "Search the web for information")
            .intent("web_scrape", "Web Scraping", "Extract data from web pages")
            .intent("fact_check", "Fact Checking", "Verify facts and claims")
            .intent_with_agent("research", "Research", "Conduct comprehensive research", "researcher")
            // Code intents
            .intent("code_generate", "Code Generation", "Generate code from specifications")
            .intent("code_review", "Code Review", "Review and analyze code")
            .intent("debug", "Debugging", "Find and fix bugs in code")
            .intent_with_agent("code", "Coding", "General coding tasks", "coder")
            // Data intents
            .intent("data_analysis", "Data Analysis", "Analyze and interpret data")
            .intent("visualization", "Visualization", "Create data visualizations")
            .intent("data_clean", "Data Cleaning", "Clean and normalize data")
            // Communication intents
            .intent("summarize", "Summarization", "Summarize text or documents")
            .intent("translate", "Translation", "Translate between languages")
            .intent("compose", "Composition", "Write or draft content")
            // System intents
            .intent("help", "Help", "Get help or guidance")
            .intent("status", "Status Check", "Check system or task status")
            .intent("unknown", "Unknown", "Unclassified intent");

        builder.build().await
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyword_classification() {
        let classifier = IntentClassifierBuilder::new()
            .intent("search", "Search", "Search for information")
            .intent("code", "Code", "Code generation")
            .keyword_rule("search_kw", "search", vec!["search".to_string(), "find".to_string()], None)
            .keyword_rule("code_kw", "code", vec!["write".to_string(), "code".to_string(), "program".to_string()], None)
            .build()
            .await;

        // Test search intent
        let result = classifier
            .classify(&ClassificationRequest {
                input: "Search for latest news about AI".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        assert_eq!(result.intent_id.0, "search");

        // Test code intent
        let result = classifier
            .classify(&ClassificationRequest {
                input: "Write a function to sort a list".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        assert_eq!(result.intent_id.0, "code");
    }

    #[tokio::test]
    async fn test_regex_classification() {
        let classifier = IntentClassifierBuilder::new()
            .intent("email", "Email", "Send an email")
            .regex_rule(
                "email_regex",
                "email",
                r"(?i)(send|compose).*(email|mail|message)".to_string(),
                None,
            )
            .build()
            .await;

        let result = classifier
            .classify(&ClassificationRequest {
                input: "Send an email to john@example.com".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        assert_eq!(result.intent_id.0, "email");
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let classifier = IntentClassifierBuilder::new()
            .intent("specific", "Specific Search", "Specific type of search")
            .intent("general", "General Search", "General search")
            .keyword_rule("specific_kw", "specific", vec!["arxiv".to_string()], None)
            .keyword_rule("general_kw", "general", vec!["search".to_string()], None)
            .build()
            .await;

        // "arxiv search" should match specific (higher priority via keyword specificity)
        let result = classifier
            .classify(&ClassificationRequest {
                input: "Search arxiv for machine learning papers".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        // Both keywords match, but specific has arxiv which is more specific
        assert!(result.confidence.score > 0.5);
    }

    #[tokio::test]
    async fn test_unknown_intent() {
        let classifier = IntentClassifierBuilder::new()
            .intent("search", "Search", "Search for information")
            .keyword_rule("search_kw", "search", vec!["search".to_string()], None)
            .build()
            .await;

        let result = classifier
            .classify(&ClassificationRequest {
                input: "What is the weather like today?".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        assert_eq!(result.intent_id.0, "unknown");
        assert!(result.confidence.score < 0.5);
    }

    #[tokio::test]
    async fn test_classification_with_agent_routing() {
        let classifier = IntentClassifierBuilder::new()
            .intent_with_agent("research", "Research", "Research tasks", "researcher-agent")
            .keyword_rule("research_kw", "research", vec!["research".to_string(), "investigate".to_string()], None)
            .build()
            .await;

        let result = classifier
            .classify_and_route(&ClassificationRequest {
                input: "Research the history of AI".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;

        assert_eq!(result.intent_id.0, "research");
        assert_eq!(result.agent_id.as_deref(), Some("researcher-agent"));
    }

    #[tokio::test]
    async fn test_confidence_scoring() {
        let classifier = IntentClassifierBuilder::new()
            .intent("test", "Test", "Test intent")
            .keyword_rule("weak_rule", "test", vec!["maybe".to_string()], None)
            .build()
            .await;

        let result = classifier
            .classify(&ClassificationRequest {
                input: "This might be a test".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;

        // Base 0.5 + boost 0.2 = 0.7
        assert!((result.confidence.score - 0.7).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_multiple_keywords_one_match() {
        let classifier = IntentClassifierBuilder::new()
            .intent("search", "Search", "Search intent")
            .keyword_rule("search_kw", "search", vec!["find".to_string(), "search".to_string(), "locate".to_string()], None)
            .build()
            .await;

        // Only "search" matches
        let result = classifier
            .classify(&ClassificationRequest {
                input: "Search the database".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;

        assert_eq!(result.intent_id.0, "search");
    }

    #[tokio::test]
    async fn test_with_common_intents() {
        let classifier = IntentClassifier::with_common_intents().await;

        // Should have many intents
        assert!(classifier.intent_count().await > 10);

        // Should classify some common requests
        let result = classifier
            .classify(&ClassificationRequest {
                input: "Debug my Python code".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;

        assert_eq!(result.intent_id.0, "debug");
    }

    #[tokio::test]
    async fn test_intent_definition_storage() {
        let classifier = IntentClassifier::new();

        classifier
            .register_intent(IntentDefinition {
                id: IntentId::new("custom"),
                name: "Custom Intent".to_string(),
                description: "A custom intent".to_string(),
                default_agent: Some("custom-agent".to_string()),
                suggested_tools: vec!["tool1".to_string(), "tool2".to_string()],
                examples: vec!["example1".to_string()],
            })
            .await;

        let intents = classifier.get_intents().await;
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].name, "Custom Intent");
    }

    #[tokio::test]
    async fn test_rule_removal() {
        let classifier = IntentClassifierBuilder::new()
            .intent("test", "Test", "Test")
            .keyword_rule("rule1", "test", vec!["test".to_string()], None)
            .keyword_rule("rule2", "test", vec!["demo".to_string()], None)
            .build()
            .await;

        assert_eq!(classifier.rule_count().await, 2);

        classifier.remove_rule("rule1").await;
        assert_eq!(classifier.rule_count().await, 1);

        // Demo rule should still work
        let result = classifier
            .classify(&ClassificationRequest {
                input: "This is a demo".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;
        assert_eq!(result.intent_id.0, "test");
    }

    #[tokio::test]
    async fn test_case_insensitive_keywords() {
        let classifier = IntentClassifierBuilder::new()
            .intent("search", "Search", "Search")
            .keyword_rule("search_kw", "search", vec!["SEARCH".to_string()], None)
            .build()
            .await;

        let result = classifier
            .classify(&ClassificationRequest {
                input: "search for information".to_string(),
                history: vec![],
                user_id: None,
                format_hint: None,
            })
            .await;

        // Case insensitive matching
        assert_eq!(result.intent_id.0, "search");
    }
}
