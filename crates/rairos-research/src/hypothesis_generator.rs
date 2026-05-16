//! Research Hypothesis Generator — full-featured port of Python HypothesisGenerator.
//!
//! Generates 3-5 testable research hypotheses from topic + gap context using:
//! 1. Template matching by gap type (method_limitation, unexplored_application, etc.)
//! 2. LLM-enhanced generation when a client is available
//! 3. Experiment design (baseline, variables, controls, metrics)
//! 4. Risk assessment (technical + hypothesis risk)
//! 5. Novelty / feasibility scoring
//! 6. Summary generation

use rairos_core::constants::{GP_DIR_NAME, GENE_POOL_JSONL};
use rairos_llm::{LlmClient, Message};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Type of research hypothesis — mirrors Python HypothesisType enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisType {
    Causal,
    Correlational,
    Comparative,
    Mechanistic,
    Exploratory,
}

impl HypothesisType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HypothesisType::Causal => "causal",
            HypothesisType::Correlational => "correlational",
            HypothesisType::Comparative => "comparative",
            HypothesisType::Mechanistic => "mechanistic",
            HypothesisType::Exploratory => "exploratory",
        }
    }
}

/// Risk level for a hypothesis assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
        }
    }
}

/// Experiment design for testing a hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDesign {
    pub baseline: String,
    pub variables: Vec<String>,
    pub controls: Vec<String>,
    #[serde(rename = "evaluation_metrics")]
    pub evaluation_metrics: Vec<String>,
    #[serde(rename = "expected_results")]
    pub expected_results: String,
}

impl Default for ExperimentDesign {
    fn default() -> Self {
        Self {
            baseline: "待确定".into(),
            variables: vec!["待确定".into()],
            controls: vec!["计算资源".into(), "训练数据".into(), "随机种子".into()],
            evaluation_metrics: vec!["性能指标".into(), "效率指标".into()],
            expected_results: "预期显著改进".into(),
        }
    }
}

/// How a hypothesis differs from existing work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentiationPoint {
    #[serde(rename = "compared_work")]
    pub compared_work: String,
    #[serde(rename = "our_advantage")]
    pub our_advantage: String,
    pub innovation: String,
}

/// Risk assessment for a hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    #[serde(rename = "technical_risk")]
    pub technical_risk: String,
    #[serde(rename = "hypothesis_risk")]
    pub hypothesis_risk: String,
    #[serde(rename = "technical_reason")]
    pub technical_reason: String,
    #[serde(rename = "hypothesis_reason")]
    pub hypothesis_reason: String,
    pub mitigation: Vec<String>,
}

/// A single generated research hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchHypothesis {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub hypothesis_type: String,
    #[serde(rename = "core_statement")]
    pub core_statement: String,
    #[serde(rename = "based_on")]
    pub based_on: String,
    #[serde(rename = "novelty_score")]
    pub novelty_score: f64,
    #[serde(rename = "feasibility_score")]
    pub feasibility_score: f64,
    #[serde(rename = "experiment_design")]
    pub experiment_design: ExperimentDesign,
    pub risk: Option<HypothesisRisk>,
    #[serde(rename = "gap_type")]
    pub gap_type: String,
}

/// Simplified risk output matching Python MCP response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisRisk {
    pub technical: String,
    pub hypothesis: String,
}

/// Complete hypothesis generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisResult {
    pub topic: String,
    pub summary: String,
    pub hypotheses: Vec<ResearchHypothesis>,
}

// ============================================================================
// Prompt Templates
// ============================================================================

const SYSTEM_PROMPT: &str = r#"You are a research hypothesis generation expert. Based on a research topic and gap context, generate 3-5 specific, testable hypotheses.

For each hypothesis, provide:
1. Type: causal | correlational | comparative | mechanistic | exploratory
2. Core statement: a clear, testable claim
3. Based on: what evidence or gap this comes from
4. Novelty score: 0.0 - 1.0
5. Feasibility score: 0.0 - 1.0

Output as JSON array:
[
  {
    "type": "causal",
    "title": "Hypothesis title",
    "core_statement": "Testable claim",
    "based_on": "Rationale",
    "novelty_score": 0.7,
    "feasibility_score": 0.6,
    "experiment_design": {
      "baseline": "Current SOTA method X",
      "variables": ["var1", "var2"],
      "controls": ["control1"],
      "evaluation_metrics": ["metric1"],
      "expected_results": "Expected outcome"
    },
    "risk": {
      "technical": "medium",
      "hypothesis": "low"
    }
  }
]"#;

const USER_PROMPT_TEMPLATE: &str = r#"Research topic: {topic}
Gap context: {gap_context}
Creative mode: {creative}
Related papers from knowledge graph:
{kg_context}

Gene Pool context (related prior hypotheses):
{genepool_context}

Trend context (research tag heat):
{trend_context}

Generate specific, testable research hypotheses."#;

// ============================================================================
// Template definitions
// ============================================================================

struct HypothesisTemplate {
    template: &'static str,
    hypo_type: HypothesisType,
    variables: &'static [&'static str],
}

const TEMPLATES: &[(&str, &[HypothesisTemplate])] = &[
    ("method_limitation", &[
        HypothesisTemplate {
            template: "通过在{method}中引入{improvement}，可以解决现有方法的{limitation}问题",
            hypo_type: HypothesisType::Causal,
            variables: &["{method}实现", "{improvement}参数", "{limitation}指标"],
        },
        HypothesisTemplate {
            template: "{method_A} + {method_B}的组合可以克服各自的{limitation}",
            hypo_type: HypothesisType::Comparative,
            variables: &["组合比例", "融合层位置", "训练策略"],
        },
    ]),
    ("unexplored_application", &[
        HypothesisTemplate {
            template: "{existing_method}可以应用于{new_domain}领域，并取得良好效果",
            hypo_type: HypothesisType::Exploratory,
            variables: &["领域适配", "数据准备", "评估指标"],
        },
        HypothesisTemplate {
            template: "{task}任务中的{challenge}挑战可以通过{approach}方法解决",
            hypo_type: HypothesisType::Mechanistic,
            variables: &["任务难度", "方法适用性", "资源需求"],
        },
    ]),
    ("contradiction", &[
        HypothesisTemplate {
            template: "{method_A}和{method_B}的差异源于{underlying_factor}，可通过实验验证",
            hypo_type: HypothesisType::Causal,
            variables: &["控制因素", "测量方法", "统计分析"],
        },
        HypothesisTemplate {
            template: "存在{condition}使得{method}效果优于{other_method}",
            hypo_type: HypothesisType::Comparative,
            variables: &["条件变量", "效果指标", "临界点"],
        },
    ]),
    ("scalability_issue", &[
        HypothesisTemplate {
            template: "通过{solution}可以使{method}扩展到{scale}规模",
            hypo_type: HypothesisType::Causal,
            variables: &["扩展策略", "效率指标", "质量保证"],
        },
    ]),
    ("evaluation_gap", &[
        HypothesisTemplate {
            template: "新的评估指标{metric}可以更准确地衡量{method}的{aspect}",
            hypo_type: HypothesisType::Correlational,
            variables: &["指标定义", "标注成本", "与其他指标的相关性"],
        },
    ]),
];

// ============================================================================
// KG & GenePool Integration (optional — graceful fallback if unavailable)
// ============================================================================

/// Fetch relevant paper titles from the knowledge graph for a topic.
/// Returns empty vec if KG is unavailable (no db, no results).
pub fn fetch_relevant_papers(topic: &str, limit: usize) -> Vec<String> {
    let db_path = rairos_kg::KnowledgeGraph::db_path();
    let graph = match rairos_kg::KnowledgeGraph::with_db(db_path) {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let db = match graph.database() {
        Some(d) => d,
        None => return Vec::new(),
    };
    // Extract keywords from topic and search each
    let keywords = crate::gene_pool::extract_keywords(topic);
    if keywords.is_empty() {
        return Vec::new();
    }
    let mut papers: std::collections::HashSet<String> = std::collections::HashSet::new();
    for kw in &keywords {
        if let Ok(nodes) = db.query_by_keyword(kw, limit) {
            for node in &nodes {
                // Only include paper-type nodes
                if node.node_type == "paper" && !node.label.is_empty() {
                    papers.insert(node.label.clone());
                }
            }
        }
    }
    let mut result: Vec<String> = papers.into_iter().collect();
    result.truncate(limit);
    result
}

/// Fetch related capsule context from the Gene Pool for a topic.
/// Returns a human-readable summary of related capsule success/failure history.
/// Gracefully degrades if Gene Pool is unavailable or empty.
pub fn fetch_gene_pool_context(topic: &str) -> String {
    let pool = crate::gene_pool::GenePool::new();
    let capsules = pool.load_capsules();
    if capsules.is_empty() {
        return "(Gene Pool is empty — no prior hypothesis data available)".to_string();
    }

    // Find capsules whose trigger_topic overlaps with the query topic
    let keywords = crate::gene_pool::extract_keywords(topic);
    if keywords.is_empty() {
        return String::new();
    }

    let related: Vec<_> = capsules.iter()
        .filter(|c| {
            keywords.iter().any(|kw| {
                c.trigger_topic.to_lowercase().contains(&kw.to_lowercase())
                    || c.action_gap_title.to_lowercase().contains(&kw.to_lowercase())
                    || c.trigger_keywords.iter().any(|k| k.to_lowercase().contains(&kw.to_lowercase()))
            })
        })
        .collect();

    if related.is_empty() {
        return "(No directly related hypotheses found in Gene Pool)".to_string();
    }

    // Compute aggregate stats
    let total = related.len();
    let avg_success: f64 = related.iter().map(|c| c.outcome_success_score).sum::<f64>() / total as f64;
    let total_feedback: i32 = related.iter().map(|c| c.feedback_count).sum();
    let high_success = related.iter().filter(|c| c.outcome_success_score >= 0.7).count();
    let low_success = related.iter().filter(|c| c.outcome_success_score < 0.3).count();
    let newest = related.iter().map(|c| &c.created_at).max().cloned().unwrap_or_default();

    // Show top 3 most successful capsules
    let mut sorted = related.clone();
    sorted.sort_by(|a, b| b.outcome_success_score.partial_cmp(&a.outcome_success_score).unwrap_or(std::cmp::Ordering::Equal));

    let top_examples: Vec<String> = sorted.iter().take(3).map(|c| {
        let title = if c.action_gap_title.len() > 80 { &c.action_gap_title[..77] } else { &c.action_gap_title };
        format!("  - \"{}\" (success: {:.0}%, feedback: {})", title, c.outcome_success_score * 100.0, c.feedback_count)
    }).collect();

    format!(
        "{} related prior {} in Gene Pool:\n- Average success rate: {:.0}%\n- High-success: {}  |  Low-success: {}  |  Total feedback events: {}\n- Most recent: {}\n- Examples:\n{}",
        total,
        if total == 1 { "hypothesis" } else { "hypotheses" },
        avg_success * 100.0,
        high_success, low_success, total_feedback,
        if newest.len() > 10 { &newest[..10] } else { &newest },
        top_examples.join("\n"),
    )
}

/// Fetch KG trend context for a topic: which research tags are heating up or cooling down.
/// Returns a human-readable summary of trending tags relevant to the topic.
/// Gracefully degrades if trend data is unavailable.
pub fn fetch_trend_context(topic: &str) -> String {
    let forecaster = rairos_trends::TrendForecaster::with_path("data/radar_history.json");
    if forecaster.history().is_empty() {
        return "(Trend data unavailable — no radar history found)".to_string();
    }

    let trending = forecaster.detect_trending(0.05);
    if trending.is_empty() {
        return "(No trending tags detected above noise threshold)".to_string();
    }

    // Filter trending tags relevant to the topic
    let keywords = crate::gene_pool::extract_keywords(topic);
    let relevant: Vec<_> = trending.iter()
        .filter(|(tag, _)| {
            keywords.is_empty() || keywords.iter().any(|kw| {
                tag.to_lowercase().contains(&kw.to_lowercase())
                    || kw.to_lowercase().contains(&tag.to_lowercase())
            })
        })
        .collect();

    let mut lines = Vec::new();

    if !relevant.is_empty() {
        lines.push(format!(
            "Relevant trending tags (top {} of {} total trending):",
            relevant.len().min(5),
            trending.len(),
        ));
        for (tag, slope) in relevant.iter().take(5) {
            lines.push(format!("  - {} (slope: {:.3})", tag, slope));
        }
    }

    // Show top 3 overall trending tags (for cross-domain awareness)
    let global_top: Vec<_> = trending.iter().take(3).collect();
    lines.push(format!(
        "\nTop trending across all research: {}",
        global_top.iter().map(|(t, s)| format!("{} ({:.3})", t, s)).collect::<Vec<_>>().join(", ")
    ));

    // Show 3 fastest-declining tags
    let mut all_sorted = trending.clone();
    all_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let bottom: Vec<_> = all_sorted.iter().take(3).collect();
    lines.push(format!(
        "Fastest declining: {}",
        bottom.iter().map(|(t, s)| format!("{} ({:.3})", t, s)).collect::<Vec<_>>().join(", ")
    ));

    lines.join("\n")
}

/// Re-rank hypotheses by adjusting novelty/feasibility scores based on research trend data.
///
/// Logic:
/// - Hot-trending tags → novelty penalty (crowded area), feasibility boost (more resources)
/// - Cold/declining tags → novelty boost (underexplored), feasibility penalty (harder path)
/// - Each matched tag adjusts by 0.05 (capped at 3 matches), clamped to [0.0, 1.0]
///
/// Gracefully degrades if trend data is unavailable.
pub fn re_rank_by_trends(hypotheses: &mut [ResearchHypothesis], topic: &str) {
    let forecaster = rairos_trends::TrendForecaster::with_path("data/radar_history.json");
    if forecaster.history().is_empty() {
        return;
    }

    // Get ALL tags with their slopes (very low threshold = no filter)
    let all_tags = forecaster.detect_trending(-999.0);
    if all_tags.is_empty() {
        return;
    }

    // Hot = top 5 highest-slope tags; Cold = bottom 5 (most negative)
    let hot: Vec<&str> = all_tags.iter().take(5).map(|(t, _)| t.as_str()).collect();
    let cold: Vec<&str> = all_tags.iter().rev().take(5).map(|(t, _)| t.as_str()).collect();

    let topic_keywords = crate::gene_pool::extract_keywords(topic);

    for h in hypotheses.iter_mut() {
        let kw_text = format!("{} {}", h.title, h.core_statement);
        let hy_kw = crate::gene_pool::extract_keywords(&kw_text);
        // Combine topic and hypothesis keywords for matching
        let all_kw: Vec<&str> = topic_keywords.iter().map(|s| s.as_str())
            .chain(hy_kw.iter().map(|s| s.as_str()))
            .collect();

        // Count hot-tag matches
        let hot_matches = hot.iter().filter(|tag| {
            all_kw.iter().any(|kw| tag.contains(kw) || kw.contains(*tag))
        }).count();

        // Count cold-tag matches
        let cold_matches = cold.iter().filter(|tag| {
            all_kw.iter().any(|kw| tag.contains(kw) || kw.contains(*tag))
        }).count();

        // Apply adjustments
        if hot_matches > 0 {
            let adj = 0.05 * hot_matches.min(3) as f64;
            h.novelty_score = (h.novelty_score - adj).max(0.0);
            h.feasibility_score = (h.feasibility_score + adj * 0.5).min(1.0);
        }

        if cold_matches > 0 {
            let adj = 0.05 * cold_matches.min(3) as f64;
            h.novelty_score = (h.novelty_score + adj).min(1.0);
            h.feasibility_score = (h.feasibility_score - adj * 0.5).max(0.0);
        }
    }
}

/// Submit high-scoring hypotheses to the GenePool as capsules.
/// Returns IDs of submitted capsules.
pub fn submit_hypotheses_to_genepool(
    topic: &str,
    gap_type: &str,
    hypotheses: &[ResearchHypothesis],
    score_threshold: f64,
) -> Vec<String> {
    let pool = crate::gene_pool::GenePool::new();
    let mut submitted = Vec::new();
    for h in hypotheses {
        let combined = h.novelty_score + h.feasibility_score;
        if combined >= score_threshold {
            if let Ok(id) = pool.encode_capsule(
                topic,
                if h.gap_type.is_empty() { gap_type } else { &h.gap_type },
                &h.title,
                &h.core_statement,
                combined / 2.0, // average score
                &h.id,
            ) {
                submitted.push(id);
            }
        }
    }
    submitted
}

// ============================================================================
// Hypothesis Generator
// ============================================================================

pub struct HypothesisGenerator;

impl HypothesisGenerator {
    pub fn new() -> Self { Self }

    /// Generate hypotheses with optional LLM enhancement.
    pub fn generate(
        &self,
        topic: &str,
        gap_context: &str,
        creative: bool,
    ) -> HypothesisResult {
        let gap_type = self.infer_gap_type(gap_context);
        let hypotheses = self.generate_from_templates(topic, gap_context, gap_type, creative);
        // Enrich: experiment designs, risk assessments, scores
        let mut hypotheses: Vec<ResearchHypothesis> = hypotheses.into_iter().map(|mut h| {
            h.experiment_design = self.generate_experiment_design(&h, topic);
            h.risk = Some(self.assess_risk(&h));
            h.novelty_score = self.calculate_novelty(&h, creative);
            h.feasibility_score = self.calculate_feasibility(&h);
            h
        }).collect();

        // Re-rank by trend data: adjust scores based on hot/cold research tags
        re_rank_by_trends(&mut hypotheses, topic);

        
        HypothesisResult {
            topic: topic.to_string(),
            summary: self.generate_summary(&hypotheses),
            hypotheses,
        }
    }

    /// Generate with LLM enhancement, including KG context and optional GenePool submission.
    pub async fn generate_llm(
        &self,
        llm: &dyn LlmClient,
        model: &str,
        topic: &str,
        gap_context: &str,
        creative: bool,
        auto_submit: bool,
    ) -> HypothesisResult {
        // Start with template-based hypotheses
        let gap_type = self.infer_gap_type(gap_context);
        let mut hypotheses = self.generate_from_templates(topic, gap_context, gap_type, creative);

        // Fetch KG context for the LLM prompt
        let kg_papers = fetch_relevant_papers(topic, 10);
        let kg_context = if kg_papers.is_empty() {
            "(No related papers found in knowledge graph)".to_string()
        } else {
            kg_papers.join("\n- ")
        };

        // Fetch Gene Pool context
        let genepool_context = fetch_gene_pool_context(topic);

        // Fetch KG trend context
        let trend_context = fetch_trend_context(topic);

        // Enhance with LLM
        let user_prompt = USER_PROMPT_TEMPLATE
            .replace("{topic}", topic)
            .replace("{gap_context}", if gap_context.len() > 200 { &gap_context[..200] } else { gap_context })
            .replace("{kg_context}", &kg_context)
            .replace("{genepool_context}", &genepool_context)
            .replace("{trend_context}", &trend_context)
            .replace("{creative}", if creative { "yes" } else { "no" });

        let msg = Message {
            role: "user".to_string(),
            content: user_prompt,
        };

        if let Ok(rairos_llm::LlmResponse::NonStream(ns)) = llm.complete(vec![msg], model, 0.3, 2000).await {
            if let Ok(llm_hypotheses) = self.parse_llm_response(&ns.content) {
                hypotheses.extend(llm_hypotheses);
            }
        }

        // Limit to 5
        hypotheses.truncate(5);

        // Generate experiment designs, risk assessments, scores for all
        let mut hypotheses: Vec<ResearchHypothesis> = hypotheses.into_iter().map(|mut h| {
            // Fill experiment design if empty
            if h.experiment_design.baseline.is_empty() || h.experiment_design.baseline == "待确定" {
                h.experiment_design = self.generate_experiment_design(&h, topic);
            }
            // Risk assessment
            if h.risk.is_none() {
                h.risk = Some(self.assess_risk(&h));
            }
            // Scores
            if h.novelty_score == 0.0 {
                h.novelty_score = self.calculate_novelty(&h, creative);
            }
            if h.feasibility_score == 0.0 {
                h.feasibility_score = self.calculate_feasibility(&h);
            }
            h
        }).collect();

        // Re-rank by trend data: adjust scores based on hot/cold research tags
        re_rank_by_trends(&mut hypotheses, topic);

        let result = HypothesisResult {
            topic: topic.to_string(),
            summary: self.generate_summary(&hypotheses),
            hypotheses,
        };

        // Auto-submit high-scoring hypotheses to GenePool
        if auto_submit && !result.hypotheses.is_empty() {
            let _ = submit_hypotheses_to_genepool(topic, gap_type, &result.hypotheses, 1.0);
        }

        result
    }

    // ─── Template Generation ────────────────────────────────────────────────

    fn generate_from_templates(
        &self,
        topic: &str,
        gap_context: &str,
        gap_type: &str,
        creative: bool,
    ) -> Vec<ResearchHypothesis> {
        let mut hypotheses = Vec::new();

        // Get templates for this gap type
        if let Some(templates) = TEMPLATES.iter().find(|(name, _)| *name == gap_type) {
            for (i, t) in templates.1.iter().enumerate().take(2) {
                let core_statement = self.fill_template(t.template, topic, gap_context);
                hypotheses.push(ResearchHypothesis {
                    id: self.make_id(i),
                    title: format!("假说 {}: {} 研究", i + 1, topic),
                    hypothesis_type: t.hypo_type.as_str().to_string(),
                    core_statement,
                    based_on: format!("基于{}类型", gap_type.replace('_', " ")),
                    novelty_score: 0.5,
                    feasibility_score: 0.5,
                    experiment_design: ExperimentDesign {
                        baseline: "待确定".into(),
                        variables: t.variables.iter().map(|v| v.to_string()).collect(),
                        controls: vec!["计算资源".into(), "训练数据".into(), "随机种子".into()],
                        evaluation_metrics: vec!["性能指标".into(), "效率指标".into()],
                        expected_results: "预期显著改进".into(),
                    },
                    risk: None,
                    gap_type: gap_type.to_string(),
                });
            }
        }

        // Add creative hypothesis if requested
        if creative {
            let creative_h = self.generate_creative_hypothesis(topic, gap_context);
            if let Some(h) = creative_h {
                hypotheses.push(h);
            }
        }

        hypotheses
    }

    fn fill_template(&self, template: &str, topic: &str, context: &str) -> String {
        // Extract a method name from context (simple heuristic)
        let method = if let Some(start) = context.find(char::is_uppercase) {
            let substr = &context[start..];
            if let Some(end) = substr.find(|c: char| !c.is_alphabetic() && c != ' ') {
                let candidate = &substr[..end].trim();
                if !candidate.is_empty() && candidate.len() > 2 {
                    candidate.to_string()
                } else {
                    topic.to_string()
                }
            } else {
                topic.to_string()
            }
        } else {
            topic.to_string()
        };

        let method_str: &str = &method;
        let replacements: Vec<(&str, &str)> = vec![
            ("{method}", method_str),
            ("{existing_method}", method_str),
            ("{new_domain}", "新领域"),
            ("{task}", "特定任务"),
            ("{challenge}", "核心挑战"),
            ("{approach}", "创新方法"),
            ("{method_A}", method_str),
            ("{method_B}", "对比方法"),
            ("{improvement}", "改进机制"),
            ("{limitation}", "局限性"),
            ("{solution}", "解决方案"),
            ("{scale}", "更大"),
            ("{metric}", "新指标"),
            ("{aspect}", "关键方面"),
            ("{condition}", "特定条件"),
            ("{other_method}", "其他方法"),
            ("{underlying_factor}", "底层因素"),
        ];

        let mut result = template.to_string();
        for (key, value) in &replacements {
            result = result.replace(key, value);
        }
        result
    }

    fn infer_gap_type(&self, context: &str) -> &str {
        let lower = context.to_lowercase();
        if lower.contains("limitation") || lower.contains("weakness") || lower.contains("scalability") {
            "method_limitation"
        } else if lower.contains("future") || lower.contains("unexplored") {
            "unexplored_application"
        } else if lower.contains("however") || lower.contains("contradict") {
            "contradiction"
        } else if lower.contains("scale") || lower.contains("large") {
            "scalability_issue"
        } else if lower.contains("benchmark") || lower.contains("metric") || lower.contains("evaluation") {
            "evaluation_gap"
        } else {
            "method_limitation"
        }
    }

    fn generate_creative_hypothesis(
        &self,
        topic: &str,
        _context: &str,
    ) -> Option<ResearchHypothesis> {
        // Domain detection keywords
        let domain_keywords: HashMap<&str, &[&str]> = [
            ("nlp", &["language", "text", "seq2seq", "transformer", "bert", "gpt"] as &[&str]),
            ("vision", &["image", "visual", "cnn", "vision", "detection"]),
            ("audio", &["speech", "audio", "sound", "wave"]),
            ("reasoning", &["logic", "reason", "inference", "planning"]),
        ].into_iter().collect();

        let topic_lower = topic.to_lowercase();
        let detected: Vec<&str> = domain_keywords.iter()
            .filter(|(_, keywords)| keywords.iter().any(|k| topic_lower.contains(k)))
            .map(|(name, _)| *name)
            .collect();

        if detected.is_empty() {
            return None;
        }

        Some(ResearchHypothesis {
            id: self.make_id(99),
            title: format!("跨领域假说: {}", topic),
            hypothesis_type: "exploratory".into(),
            core_statement: format!("将{}的方法/机制应用于跨领域任务可能产生意外的效果提升", topic),
            based_on: "跨领域创新思维".into(),
            novelty_score: 0.8,
            feasibility_score: 0.4,
            experiment_design: ExperimentDesign {
                baseline: "标准方法".into(),
                variables: vec!["领域迁移策略".into(), "适配层设计".into(), "预训练权重".into()],
                controls: vec!["数据集规模".into(), "模型大小".into(), "训练轮数".into()],
                evaluation_metrics: vec!["目标任务准确率".into(), "迁移效率".into()],
                expected_results: "跨领域迁移有效性".into(),
            },
            risk: None,
            gap_type: "cross_domain".into(),
        })
    }

    // ─── LLM Parsing ────────────────────────────────────────────────────────

    fn parse_llm_response(&self, content: &str) -> Result<Vec<ResearchHypothesis>, String> {
        // Try JSON array first
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
            let mut hypotheses = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let hypo_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("exploratory");
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("LLM生成假说");
                let core = item.get("core_statement").and_then(|v| v.as_str()).unwrap_or("");
                let based = item.get("based_on").and_then(|v| v.as_str()).unwrap_or("LLM增强生成");
                let novelty = item.get("novelty_score").and_then(|v| v.as_f64()).unwrap_or(0.5);
                let feasibility = item.get("feasibility_score").and_then(|v| v.as_f64()).unwrap_or(0.5);

                // Parse experiment design
                let exp = item.get("experiment_design");
                let experiment_design = ExperimentDesign {
                    baseline: exp.and_then(|e| e.get("baseline")).and_then(|v| v.as_str()).unwrap_or("待设计").into(),
                    variables: exp.and_then(|e| e.get("variables")).and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    controls: exp.and_then(|e| e.get("controls")).and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    evaluation_metrics: exp.and_then(|e| e.get("evaluation_metrics")).and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    expected_results: exp.and_then(|e| e.get("expected_results")).and_then(|v| v.as_str()).unwrap_or("待确定").into(),
                };

                // Parse risk
                let risk = item.get("risk").map(|r| HypothesisRisk {
                    technical: r.get("technical").and_then(|v| v.as_str()).unwrap_or("medium").into(),
                    hypothesis: r.get("hypothesis").and_then(|v| v.as_str()).unwrap_or("medium").into(),
                });

                hypotheses.push(ResearchHypothesis {
                    id: format!("llm_{}", i),
                    title: title.to_string(),
                    hypothesis_type: hypo_type.to_string(),
                    core_statement: core.to_string(),
                    based_on: based.to_string(),
                    novelty_score: novelty,
                    feasibility_score: feasibility,
                    experiment_design,
                    risk,
                    gap_type: self.infer_gap_type(core).to_string(),
                });
            }
            if hypotheses.is_empty() {
                return Err("Empty LLM response".into());
            }
            return Ok(hypotheses);
        }

        // Try line-based format (Python legacy format)
        let mut hypotheses = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[假说") || trimmed.starts_with("Hypothesis") || trimmed.starts_with("H") {
                let parts: Vec<&str> = trimmed.split('|').collect();
                if !parts.is_empty() {
                    let statement = if parts[0].contains(']') {
                        parts[0].split(']').nth(1).map(|s| s.trim()).unwrap_or(parts[0])
                    } else {
                        parts[0].trim()
                    };
                    if !statement.is_empty() {
                        hypotheses.push(ResearchHypothesis {
                            id: self.make_id(hypotheses.len()),
                            title: "LLM生成假说".into(),
                            hypothesis_type: "exploratory".into(),
                            core_statement: statement.to_string(),
                            based_on: "LLM增强生成".into(),
                            novelty_score: 0.6,
                            feasibility_score: 0.5,
                            experiment_design: ExperimentDesign::default(),
                            risk: None,
                            gap_type: "method_limitation".into(),
                        });
                    }
                }
            }
        }

        if hypotheses.is_empty() {
            Err("No hypotheses found in LLM response".into())
        } else {
            Ok(hypotheses)
        }
    }

    // ─── Experiment Design ──────────────────────────────────────────────────

    fn generate_experiment_design(
        &self,
        hypothesis: &ResearchHypothesis,
        topic: &str,
    ) -> ExperimentDesign {
        let mut design = ExperimentDesign::default();

        match hypothesis.hypothesis_type.as_str() {
            "comparative" => {
                design.baseline = format!("{}的标准实现", topic);
                design.evaluation_metrics.push("相对提升".into());
                design.evaluation_metrics.push("统计显著性".into());
            }
            "causal" => {
                design.controls.push("消融变量".into());
                design.controls.push("干预点".into());
            }
            "exploratory" => {
                design.evaluation_metrics.push("可行性".into());
                design.evaluation_metrics.push("资源消耗".into());
            }
            _ => {}
        }

        design
    }

    // ─── Risk Assessment ────────────────────────────────────────────────────

    fn assess_risk(&self, hypothesis: &ResearchHypothesis) -> HypothesisRisk {
        let (tech_risk, hyp_risk) = if hypothesis.core_statement.contains("新领域") {
            ("high".to_string(), "high".to_string())
        } else if hypothesis.hypothesis_type == "exploratory" {
            ("medium".to_string(), "high".to_string())
        } else {
            ("medium".to_string(), "medium".to_string())
        };

        HypothesisRisk { technical: tech_risk, hypothesis: hyp_risk }
    }

    // ─── Scoring ────────────────────────────────────────────────────────────

    fn calculate_novelty(&self, hypothesis: &ResearchHypothesis, creative: bool) -> f64 {
        let mut score: f64 = 0.5;
        if hypothesis.hypothesis_type == "exploratory" {
            score += 0.2;
        }
        if hypothesis.core_statement.contains("跨领域") {
            score += 0.3;
        }
        if creative {
            score += 0.1;
        }
        score.min(1.0)
    }

    fn calculate_feasibility(&self, hypothesis: &ResearchHypothesis) -> f64 {
        let mut score: f64 = 0.6;
        if hypothesis.experiment_design.variables.len() > 5 {
            score -= 0.1;
        }
        if hypothesis.hypothesis_type == "exploratory" {
            score -= 0.1;
        }
        score.max(0.3)
    }

    // ─── Summary ────────────────────────────────────────────────────────────

    fn generate_summary(&self, hypotheses: &[ResearchHypothesis]) -> String {
        if hypotheses.is_empty() {
            return "无法生成有效假说，请提供更多上下文".into();
        }
        let high_feasibility = hypotheses.iter().filter(|h| h.feasibility_score > 0.6).count();
        let high_novelty = hypotheses.iter().filter(|h| h.novelty_score > 0.6).count();

        let mut summary = format!("生成了 {} 个研究假说", hypotheses.len());
        if high_feasibility > 0 {
            summary.push_str(&format!("，其中 {} 个可行性较高", high_feasibility));
        }
        if high_novelty > 0 {
            summary.push_str(&format!("，{} 个创新性较高", high_novelty));
        }
        summary
    }

    // ─── Helpers ────────────────────────────────────────────────────────────

    fn make_id(&self, index: usize) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        format!("h{}_{:04x}", index % 10, ts)
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    #[test]
    fn test_generate_hypotheses() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate(
            "Transformer architecture for code generation",
            "Current methods have scalability limitations when handling long sequences",
            false,
        );
        assert!(!result.hypotheses.is_empty(), "Should generate at least 2 hypotheses");
        assert!(result.hypotheses.len() <= 5, "Should not exceed 5 hypotheses");
        assert!(!result.summary.is_empty());
        for h in &result.hypotheses {
            assert!(!h.id.is_empty());
            assert!(!h.core_statement.is_empty());
            assert!(h.novelty_score >= 0.0 && h.novelty_score <= 1.0);
            assert!(h.feasibility_score >= 0.0 && h.feasibility_score <= 1.0);
        }
    }

    #[test]
    fn test_creative_hypothesis() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate(
            "Language model pre-training",
            "Exploring new training paradigms",
            true,
        );
        let creative = result.hypotheses.iter().find(|h| h.gap_type == "cross_domain");
        assert!(creative.is_some(), "Should generate creative cross-domain hypothesis");
        let c = creative.unwrap();
        assert_eq!(c.hypothesis_type, "exploratory");
        assert!(c.novelty_score > 0.6);
    }

    #[test]
    fn test_gap_type_inference() {
        let gen = HypothesisGenerator::new();
        assert_eq!(gen.infer_gap_type("memory limitation issue"), "method_limitation");
        assert_eq!(gen.infer_gap_type("future unexplored direction"), "unexplored_application");
        assert_eq!(gen.infer_gap_type("however contradictory evidence"), "contradiction");
        assert_eq!(gen.infer_gap_type("scale to large production"), "scalability_issue");
        assert_eq!(gen.infer_gap_type("evaluation benchmark metric"), "evaluation_gap");
    }

    #[test]
    fn test_llm_parse_response() {
        let gen = HypothesisGenerator::new();
        let json = r#"[
            {
                "type": "causal",
                "title": "Test Hypothesis",
                "core_statement": "X causes Y in Z context",
                "based_on": "Prior work suggests correlation",
                "novelty_score": 0.7,
                "feasibility_score": 0.6,
                "experiment_design": {
                    "baseline": "Standard approach",
                    "variables": ["X", "Y"],
                    "controls": ["Z"],
                    "evaluation_metrics": ["accuracy"],
                    "expected_results": "X improves Y by 10%"
                },
                "risk": {
                    "technical": "low",
                    "hypothesis": "medium"
                }
            }
        ]"#;
        let result = gen.parse_llm_response(json);
        assert!(result.is_ok(), "Should parse valid JSON: {:?}", result.err());
        let hypotheses = result.unwrap();
        assert_eq!(hypotheses.len(), 1);
        assert_eq!(hypotheses[0].hypothesis_type, "causal");
        assert!((hypotheses[0].novelty_score - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_llm_parse_line_format() {
        let gen = HypothesisGenerator::new();
        let lines = "[假说1] Hypothesis A | Gap type | Expected result\n[假说2] Hypothesis B | Another gap | Different result\n";
        let result = gen.parse_llm_response(lines);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_empty_context_fallback() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate("Quantum computing", "", false);
        assert!(!result.hypotheses.is_empty(), "Should still generate with empty context");
        assert_eq!(result.hypotheses[0].gap_type, "method_limitation");
    }

    #[test]
    fn test_risk_assessment() {
        let gen = HypothesisGenerator::new();
        let h = ResearchHypothesis {
            id: "test".into(),
            title: "Test".into(),
            hypothesis_type: "exploratory".into(),
            core_statement: "新领域探索假说".into(),
            based_on: "Test".into(),
            novelty_score: 0.5,
            feasibility_score: 0.5,
            experiment_design: ExperimentDesign::default(),
            risk: None,
            gap_type: "test".into(),
        };
        let risk = gen.assess_risk(&h);
        assert_eq!(risk.technical, "high");
        assert_eq!(risk.hypothesis, "high");
    }

    #[test]
    fn test_summary_generation() {
        let gen = HypothesisGenerator::new();
        let hypotheses = vec![
            ResearchHypothesis {
                id: "1".into(), title: "H1".into(), hypothesis_type: "causal".into(),
                core_statement: "A causes B".into(), based_on: "Gap".into(),
                novelty_score: 0.8, feasibility_score: 0.9,
                experiment_design: ExperimentDesign::default(), risk: None,
                gap_type: "method_limitation".into(),
            },
            ResearchHypothesis {
                id: "2".into(), title: "H2".into(), hypothesis_type: "exploratory".into(),
                core_statement: "C might apply to D".into(), based_on: "Gap".into(),
                novelty_score: 0.4, feasibility_score: 0.3,
                experiment_design: ExperimentDesign::default(), risk: None,
                gap_type: "unexplored_application".into(),
            },
        ];
        let summary = gen.generate_summary(&hypotheses);
        assert!(summary.contains("2 个研究假说"));
        assert!(summary.contains("可行性较高"));
        assert!(summary.contains("创新性较高"));
    }

    // ─── Comprehensive E2E Tests ────────────────────────────────────────────

    #[test]
    fn test_all_gap_types_generate_hypotheses() {
        let gen = HypothesisGenerator::new();
        let test_cases = vec![
            ("method_limitation", "Current approach has a scalability limitation"),
            ("unexplored_application", "This remains a future work direction"),
            ("contradiction", "However these contradictory results suggest"),
            ("scalability_issue", "The method does not scale to large production"),
            ("evaluation_gap", "No benchmark metric exists for evaluation"),
        ];
        for (expected_type, context) in &test_cases {
            let result = gen.generate("Test topic", context, false);
            assert!(!result.hypotheses.is_empty(),
                "Should generate at least 1 hypothesis for gap type '{}'", expected_type);
            // At least one hypothesis should match the gap type
            let matched = result.hypotheses.iter().any(|h| h.gap_type == *expected_type);
            assert!(matched,
                "Gap type '{}' should match for context: {}", expected_type, context);
            assert!(!result.summary.is_empty());
            // Verify scores
            for h in &result.hypotheses {
                assert!(h.novelty_score >= 0.0 && h.novelty_score <= 1.0,
                    "novelty_score {} out of range for {}", h.novelty_score, h.gap_type);
                assert!(h.feasibility_score >= 0.0 && h.feasibility_score <= 1.0,
                    "feasibility_score {} out of range for {}", h.feasibility_score, h.gap_type);
            }
        }
    }

    #[test]
    fn test_output_schema_guarantee() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate("Transformer efficiency", "Memory footprint limitation", false);
        for h in &result.hypotheses {
            // All required fields must be present and non-empty
            assert!(!h.id.is_empty(), "id must not be empty");
            assert!(!h.title.is_empty(), "title must not be empty");
            assert!(!h.hypothesis_type.is_empty(), "type must not be empty");
            assert!(!h.core_statement.is_empty(), "core_statement must not be empty");
            assert!(!h.based_on.is_empty(), "based_on must not be empty");
            assert!(!h.gap_type.is_empty(), "gap_type must not be empty");

            // Experiment design must have all fields
            assert!(!h.experiment_design.baseline.is_empty(), "experiment baseline must not be empty");
            assert!(!h.experiment_design.variables.is_empty(), "experiment variables must not be empty");
            assert!(!h.experiment_design.controls.is_empty(), "experiment controls must not be empty");
            assert!(!h.experiment_design.evaluation_metrics.is_empty(), "experiment metrics must not be empty");
            assert!(!h.experiment_design.expected_results.is_empty(), "experiment expected_results must not be empty");

            // Risk must be present
            assert!(h.risk.is_some(), "risk must be present");
            if let Some(ref r) = h.risk {
                assert!(!r.technical.is_empty(), "risk.technical must not be empty");
                assert!(!r.hypothesis.is_empty(), "risk.hypothesis must not be empty");
                assert!(r.technical == "low" || r.technical == "medium" || r.technical == "high",
                    "risk.technical must be low/medium/high, got: {}", r.technical);
            }
        }
    }

    #[test]
    fn test_multiple_hypotheses_count() {
        let gen = HypothesisGenerator::new();
        // With creative=false, should get 2 template hypotheses
        let result = gen.generate("Vision transformers", "Limitation in attention computation", false);
        assert_eq!(result.hypotheses.len(), 2,
            "Should generate exactly 2 template hypotheses without creative");

        // With creative=true and matching domain, should get 3
        // "transformer" should trigger NLP domain; "contradiction" has 2 templates
        let result_c = gen.generate("Transformer language model training",
            "However contradictory results found", true);
        assert_eq!(result_c.hypotheses.len(), 3,
            "Should generate 3 hypotheses with creative (contradiction has 2 templates + 1 creative), got {}",
            result_c.hypotheses.len());

        // Non-matching domain should still get 2
        let result_neutral = gen.generate("General topic",
            "Some limitation in approach", true);
        assert_eq!(result_neutral.hypotheses.len(), 2,
            "Should generate 2 hypotheses when no domain matches");
    }

    #[test]
    fn test_experiment_design_by_type() {
        let gen = HypothesisGenerator::new();
        // Comparative type should add specific metrics
        let result = gen.generate(
            "Algorithm comparison",
            "However contradictory findings exist",
            false,
        );
        for h in &result.hypotheses {
            if h.hypothesis_type == "comparative" {
                let has_rel_metric = h.experiment_design.evaluation_metrics
                    .iter().any(|m| m.contains("相对提升"));
                assert!(has_rel_metric,
                    "Comparative hypothesis should contain comparison metrics: {:?}",
                    h.experiment_design.evaluation_metrics);
            }
        }
    }

    #[test]
    fn test_llm_parse_partial_data() {
        let gen = HypothesisGenerator::new();
        // Minimal JSON with only required fields
        let json = r#"[
            {
                "type": "mechanistic",
                "core_statement": "Test mechanism hypothesis"
            }
        ]"#;
        let result = gen.parse_llm_response(json);
        assert!(result.is_ok(), "Should parse minimal JSON: {:?}", result.err());
        let hypotheses = result.unwrap();
        assert_eq!(hypotheses.len(), 1);
        assert_eq!(hypotheses[0].hypothesis_type, "mechanistic");
        assert_eq!(hypotheses[0].core_statement, "Test mechanism hypothesis");
        // Defaults should be filled
        assert!(!hypotheses[0].title.is_empty());
        assert_eq!(hypotheses[0].novelty_score, 0.5);
    }

    #[test]
    fn test_llm_parse_multi_hypothesis() {
        let gen = HypothesisGenerator::new();
        let json = r#"[
            {"type": "causal", "core_statement": "H1", "novelty_score": 0.9},
            {"type": "correlational", "core_statement": "H2", "novelty_score": 0.5},
            {"type": "comparative", "core_statement": "H3", "novelty_score": 0.3},
            {"type": "mechanistic", "core_statement": "H4"},
            {"type": "exploratory", "core_statement": "H5"}
        ]"#;
        let result = gen.parse_llm_response(json);
        assert!(result.is_ok());
        let hypotheses = result.unwrap();
        assert_eq!(hypotheses.len(), 5);
        assert_eq!(hypotheses[0].hypothesis_type, "causal");
        assert_eq!(hypotheses[4].hypothesis_type, "exploratory");
    }

    #[test]
    fn test_llm_parse_empty_json_array() {
        let gen = HypothesisGenerator::new();
        let result = gen.parse_llm_response("[]");
        assert!(result.is_err(), "Empty array should be error");
    }

    #[test]
    fn test_llm_parse_invalid_json() {
        let gen = HypothesisGenerator::new();
        let result = gen.parse_llm_response("not json at all");
        assert!(result.is_err(), "Invalid JSON should be error");
    }

    #[test]
    fn test_llm_parse_malformed_lines() {
        let gen = HypothesisGenerator::new();
        // Lines that start with H pattern but have no useful content
        let lines = "not a hypothesis\n[假说] \n";
        let result = gen.parse_llm_response(lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_creative_domain_detection() {
        let gen = HypothesisGenerator::new();
        // NLP domain
        let r = gen.generate("Transformer language model", "Limitation", true);
        assert!(r.hypotheses.iter().any(|h| h.gap_type == "cross_domain"),
            "NLP topic should trigger creative hypothesis");

        // Vision domain
        let r = gen.generate("Image classification with CNN", "Limitation", true);
        assert!(r.hypotheses.iter().any(|h| h.gap_type == "cross_domain"),
            "Vision topic should trigger creative hypothesis");

        // Non-matching domain
        let r = gen.generate("Something completely different", "Limitation", true);
        assert!(!r.hypotheses.iter().any(|h| h.gap_type == "cross_domain"),
            "Non-matching topic should NOT trigger creative hypothesis");
    }

    #[test]
    fn test_risk_for_all_types() {
        let gen = HypothesisGenerator::new();
        let types = vec!["causal", "correlational", "comparative", "mechanistic", "exploratory"];
        for hypo_type in &types {
            let h = ResearchHypothesis {
                id: "test".into(),
                title: format!("Test {}", hypo_type),
                hypothesis_type: hypo_type.to_string(),
                core_statement: if *hypo_type == "exploratory" {
                    "新领域探索".into()
                } else {
                    format!("Standard {} hypothesis", hypo_type)
                },
                based_on: "Test".into(),
                novelty_score: 0.5,
                feasibility_score: 0.5,
                experiment_design: ExperimentDesign::default(),
                risk: None,
                gap_type: "test".into(),
            };
            let risk = gen.assess_risk(&h);
            // Exploratory with 新领域 should be high/high
            if *hypo_type == "exploratory" && h.core_statement.contains("新领域") {
                assert_eq!(risk.technical, "high", "{} with 新领域 should be high", hypo_type);
                assert_eq!(risk.hypothesis, "high", "{} with 新领域 should be high", hypo_type);
            } else {
                // Standard types should not crash
                assert!(!risk.technical.is_empty());
                assert!(!risk.hypothesis.is_empty());
            }
        }
    }

    #[test]
    fn test_scoring_ranges() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate("Test topic for scoring", "Limitation in approach", false);
        for h in &result.hypotheses {
            assert!(h.novelty_score >= 0.0, "novelty must be >= 0");
            assert!(h.novelty_score <= 1.0, "novelty must be <= 1");
            assert!(h.feasibility_score >= 0.0, "feasibility must be >= 0");
            assert!(h.feasibility_score <= 1.0, "feasibility must be <= 1");
        }
    }

    #[test]
    fn test_summary_edge_cases() {
        let gen = HypothesisGenerator::new();
        // Empty list
        let s = gen.generate_summary(&[]);
        assert_eq!(s, "无法生成有效假说，请提供更多上下文");

        // Single hypothesis with mixed scores
        let h = ResearchHypothesis {
            id: "1".into(), title: "H".into(), hypothesis_type: "causal".into(),
            core_statement: "Test".into(), based_on: "Gap".into(),
            novelty_score: 0.3, feasibility_score: 0.3,
            experiment_design: ExperimentDesign::default(), risk: None,
            gap_type: "test".into(),
        };
        let s = gen.generate_summary(&[h]);
        assert!(s.contains("1 个研究假说"));
        assert!(!s.contains("可行性较高"), "Low feasibility should not mention high");
        assert!(!s.contains("创新性较高"), "Low novelty should not mention high");
    }

    #[test]
    fn test_multi_hypothesis_diverse_types() {
        let gen = HypothesisGenerator::new();
        let result = gen.generate(
            "Deep learning optimization",
            "Memory limitation in large batch training on GPU clusters",
            false,
        );
        assert!(result.hypotheses.len() >= 2, "Should have at least 2 hypotheses");
        // All hypotheses should have unique IDs
        let ids: std::collections::HashSet<&str> = result.hypotheses.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids.len(), result.hypotheses.len(), "All hypothesis IDs should be unique");
        // At least one should be causal (from method_limitation template)
        let has_causal = result.hypotheses.iter().any(|h| h.hypothesis_type == "causal");
        assert!(has_causal, "method_limitation should generate causal hypotheses");
    }

    #[test]
    fn test_generate_with_all_types_inferable() {
        let gen = HypothesisGenerator::new();
        // A context that mentions many gap-type keywords
        let rich_context = "The current method has a scalability limitation and we found contradictory \
            results. Future work should explore benchmark metrics for evaluation.";
        let result = gen.generate("Rich topic", rich_context, true);
        // Should still work and produce valid output
        assert!(!result.hypotheses.is_empty());
        assert!(!result.summary.is_empty());
        for h in &result.hypotheses {
            assert!(!h.id.is_empty());
            assert!(h.novelty_score > 0.0);
        }
    }

    // ─── Hypothesis Lifecycle E2E ──────────────────────────────────────────

    #[test]
    fn test_hypothesis_lifecycle_e2e() {
        // Use temp dir to avoid touching real data
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Relaxed);
        let tid = std::thread::current().id();
        let dir = std::env::temp_dir().join(format!("rairos_e2e_{:?}_{}_{}", tid, std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        let gp_dir = dir.join(GP_DIR_NAME);
        std::fs::create_dir_all(&gp_dir).unwrap();

        let pool = crate::gene_pool::GenePool {
            base_dir: gp_dir.clone(),
            jsonl_path: gp_dir.join(GENE_POOL_JSONL),
            events_path: gp_dir.join("events.jsonl"),
        };

        let gen = HypothesisGenerator::new();

        // ── Phase 1: Generate hypotheses ──────────────────────────────────
        let result = gen.generate(
            "attention mechanism in transformers",
            "Current methods have scalability limitations when handling long sequences",
            false,
        );
        assert!(!result.hypotheses.is_empty(), "Should generate hypotheses");
        assert!(result.hypotheses.len() <= 5, "Max 5 hypotheses");
        for h in &result.hypotheses {
            assert!(!h.id.is_empty(), "Each hypothesis needs an ID");
            assert!(h.novelty_score >= 0.0 && h.novelty_score <= 1.0);
            assert!(h.feasibility_score >= 0.0 && h.feasibility_score <= 1.0);
        }

        // ── Phase 2: Submit first hypothesis to GenePool ──────────────────
        let hid = &result.hypotheses[0].id;
        pool.encode_capsule(
            "attention mechanism in transformers",
            "method_limitation",
            &result.hypotheses[0].title,
            &result.hypotheses[0].core_statement,
            (result.hypotheses[0].novelty_score + result.hypotheses[0].feasibility_score) / 2.0,
            hid,
        ).expect("encode_capsule");

        // ── Phase 3: Verify hypothesis_id stored on capsule ───────────────
        let capsules = pool.load_capsules();
        let found: Vec<_> = capsules.iter().filter(|c| !c.hypothesis_id.is_empty()).collect();
        assert_eq!(found.len(), 1, "Exactly one capsule with hypothesis_id");
        assert_eq!(found[0].hypothesis_id, *hid, "hypothesis_id should match");

        let initial_score = found[0].outcome_success_score;
        let initial_feedback = found[0].feedback_count;

        // ── Phase 4: Update capsule score (simulate validated experiment) ─
        pool.update_capsule_by_hypothesis_id(hid, 0.9, 1)
            .expect("update_capsule_by_hypothesis_id");

        // ── Phase 5: Verify score update ──────────────────────────────────
        let capsules_after = pool.load_capsules();
        let updated: Vec<_> = capsules_after.iter()
            .filter(|c| c.hypothesis_id == *hid)
            .collect();
        assert_eq!(updated.len(), 1, "Found capsule after update");
        let expected = initial_score * 0.7 + 0.9 * 0.3;
        assert!((updated[0].outcome_success_score - expected).abs() < 0.001,
            "Score EMA: expected {:.4}, got {:.4}",
            expected, updated[0].outcome_success_score);
        assert_eq!(updated[0].feedback_count, initial_feedback + 1,
            "Feedback should increment");

        // ── Phase 6: Simulate rejection update ────────────────────────────
        let score_before_rejection = updated[0].outcome_success_score;
        pool.update_capsule_by_hypothesis_id(hid, 0.2, 1)
            .expect("Second update");
        let final_capsules = pool.load_capsules();
        let rejected: Vec<_> = final_capsules.iter()
            .filter(|c| c.hypothesis_id == *hid)
            .collect();
        assert_eq!(rejected.len(), 1, "Found after rejection");
        let expected2 = score_before_rejection * 0.7 + 0.2 * 0.3;
        assert!((rejected[0].outcome_success_score - expected2).abs() < 0.001,
            "Rejection EMA: expected {:.4}, got {:.4}",
            expected2, rejected[0].outcome_success_score);

        // ── Phase 7: Lifecycle event recorded ────────────────────────────
        let lc = std::fs::read_to_string(gp_dir.join("lifecycle_events.jsonl")).unwrap_or_default();
        assert!(lc.contains("experiment_feedback"), "Lifecycle event recorded");
        assert!(lc.contains(hid), "Lifecycle event references hypothesis_id");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_hypothesis_e2e_submit_and_update() {
        // Tests: generate → encode_capsule with hypothesis_id → update_capsule → lifecycle
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Relaxed);
        let tid = std::thread::current().id();
        let dir = std::env::temp_dir().join(format!("rairos_e2e_{:?}_{}_{}", tid, std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        let gp_dir = dir.join(GP_DIR_NAME);
        std::fs::create_dir_all(&gp_dir).unwrap();

        let pool = crate::gene_pool::GenePool {
            base_dir: gp_dir.clone(),
            jsonl_path: gp_dir.join(GENE_POOL_JSONL),
            events_path: gp_dir.join("events.jsonl"),
        };

        let gen = HypothesisGenerator::new();
        let result = gen.generate("deep learning optimization", "Memory limitation", false);
        assert!(!result.hypotheses.is_empty());

        // Submit hypothesis to GenePool with hypothesis_id
        let hid = &result.hypotheses[0].id;
        pool.encode_capsule("deep learning optimization", "method_limitation",
            &result.hypotheses[0].title, &result.hypotheses[0].core_statement,
            (result.hypotheses[0].novelty_score + result.hypotheses[0].feasibility_score) / 2.0,
            hid,
        ).expect("encode_capsule");

        // Verify capsule has hypothesis_id
        let before = pool.load_capsules();
        let cap = before.iter().find(|c| c.hypothesis_id == *hid).expect("Capsule should exist");
        let score_before = cap.outcome_success_score;
        let fb_before = cap.feedback_count;

        // Simulate experiment feedback (validated → score 0.9)
        pool.update_capsule_by_hypothesis_id(hid, 0.9, 1).expect("update_capsule");

        let after = pool.load_capsules();
        let updated = after.iter().find(|c| c.hypothesis_id == *hid).expect("Should still exist");
        let expected = score_before * 0.7 + 0.9 * 0.3;
        assert!((updated.outcome_success_score - expected).abs() < 0.001,
            "Score EMA: {:.4} != {:.4}", updated.outcome_success_score, expected);
        assert_eq!(updated.feedback_count, fb_before + 1, "Feedback ++");

        // Lifecycle event recorded
        let lc = std::fs::read_to_string(gp_dir.join("lifecycle_events.jsonl")).unwrap_or_default();
        assert!(lc.contains("experiment_feedback"), "Should have lifecycle event");
        assert!(lc.contains(hid), "Should reference hypothesis_id");

        std::fs::remove_dir_all(&dir).ok();
    }
}
