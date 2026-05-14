use rairos_llm::{LlmClient, Message};
use serde::{Deserialize, Serialize};

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HypothesisType {
    #[serde(rename = "exploration")] Exploration,
    #[serde(rename = "exploitation")] Exploitation,
    #[serde(rename = "verification")] Verification,
    #[serde(rename = "generalization")] Generalization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    #[serde(rename = "low")] Low,
    #[serde(rename = "medium")] Medium,
    #[serde(rename = "high")] High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDesign {
    pub experiment_type: String,
    pub hypothesis: String,
    pub variables: Vec<String>,
    pub control_group: bool,
    pub sample_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentiationPoint {
    pub dimension: String,
    pub our_approach: String,
    pub baseline: String,
    pub improvement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub technical_risk: String,
    pub likelihood: RiskLevel,
    pub impact: RiskLevel,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchHypothesis {
    pub hypothesis_type: HypothesisType,
    pub statement: String,
    pub null_hypothesis: Option<String>,
    pub alternative_hypothesis: Option<String>,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisResult {
    pub hypothesis: String,
    pub result: String,
    pub accepted: bool,
    pub p_value: Option<f64>,
    pub evidence: Vec<String>,
}

// ─── Prompt Templates ─────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are a research hypothesis generation and experiment design expert.

Given a research context, generate a specific, testable hypothesis with:
1. A clear statement
2. Type classification (exploration/exploitation/verification/generalization)
3. Null and alternative hypotheses
4. Confidence level estimate (0.0-1.0)

Format your response as JSON:
{
  "hypothesis_type": "exploration",
  "statement": "...",
  "null_hypothesis": "...",
  "alternative_hypothesis": "...",
  "confidence_level": 0.95
}"#;

const EXPERIMENT_SYSTEM: &str = r#"You are an experiment design expert.
Design a controlled experiment to test the given hypothesis. Include:
1. Experiment type (controlled_trial / ablation / comparative / case_study)
2. Variables (independent, dependent, controlled)
3. Whether a control group is needed
4. Recommended sample size

Format as JSON."#;

// ─── HypothesisGenerator ──────────────────────────────────────────────────────

pub struct HypothesisGenerator;

impl HypothesisGenerator {
    pub fn new() -> Self { Self }

    /// Generate a hypothesis using LLM, with placeholder fallback
    pub async fn generate_hypothesis_llm(
        &self,
        llm: &dyn LlmClient,
        model: &str,
        context: &str,
    ) -> ResearchHypothesis {
        let msg = Message {
            role: "user".to_string(),
            content: format!("Research context: {}\n\nGenerate a specific, testable hypothesis based on this context.", context),
        };

        match llm.complete(vec![msg], model, 0.3, 1000).await {
            Ok(rairos_llm::LlmResponse::NonStream(ns)) => {
                serde_json::from_str(&ns.content).unwrap_or_else(|_| self.fallback_hypothesis(context))
            }
            _ => self.fallback_hypothesis(context),
        }
    }

    /// Generate hypothesis without LLM (placeholder)
    pub fn generate_hypothesis(&self, context: &str) -> ResearchHypothesis {
        self.fallback_hypothesis(context)
    }

    fn fallback_hypothesis(&self, context: &str) -> ResearchHypothesis {
        ResearchHypothesis {
            hypothesis_type: HypothesisType::Exploration,
            statement: format!("Based on: {}", context),
            null_hypothesis: None,
            alternative_hypothesis: None,
            confidence_level: 0.95,
        }
    }

    pub async fn design_experiment_llm(
        &self,
        llm: &dyn LlmClient,
        model: &str,
        hypothesis: &ResearchHypothesis,
    ) -> ExperimentDesign {
        let msg = Message {
            role: "user".to_string(),
            content: format!("Design an experiment to test this hypothesis: {}", hypothesis.statement),
        };

        match llm.complete(vec![msg], model, 0.3, 1000).await {
            Ok(rairos_llm::LlmResponse::NonStream(ns)) => {
                serde_json::from_str(&ns.content).unwrap_or_else(|_| self.fallback_experiment(hypothesis))
            }
            _ => self.fallback_experiment(hypothesis),
        }
    }

    pub fn design_experiment(&self, hypothesis: &ResearchHypothesis) -> ExperimentDesign {
        self.fallback_experiment(hypothesis)
    }

    fn fallback_experiment(&self, hypothesis: &ResearchHypothesis) -> ExperimentDesign {
        ExperimentDesign {
            experiment_type: "controlled_trial".to_string(),
            hypothesis: hypothesis.statement.clone(),
            variables: vec!["independent".to_string(), "dependent".to_string()],
            control_group: true,
            sample_size: Some(100),
        }
    }

    pub fn assess_risk(&self, _experiment: &ExperimentDesign) -> RiskAssessment {
        RiskAssessment {
            technical_risk: "Standard experimental setup".to_string(),
            likelihood: RiskLevel::Medium,
            impact: RiskLevel::Medium,
            mitigation: "Use established methodologies".to_string(),
        }
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hypothesis() {
        let gen = HypothesisGenerator::new();
        let h = gen.generate_hypothesis("Transformer architecture");
        assert!(!h.statement.is_empty());
    }

    #[test]
    fn test_design_experiment() {
        let gen = HypothesisGenerator::new();
        let h = gen.generate_hypothesis("Test");
        let exp = gen.design_experiment(&h);
        assert!(!exp.experiment_type.is_empty());
    }

    #[test]
    fn test_assess_risk() {
        let gen = HypothesisGenerator::new();
        let h = gen.generate_hypothesis("Test");
        let exp = gen.design_experiment(&h);
        let risk = gen.assess_risk(&exp);
        assert!(!risk.technical_risk.is_empty());
    }

    #[test]
    fn test_hypothesis_serialization() {
        let h = ResearchHypothesis {
            hypothesis_type: HypothesisType::Exploration,
            statement: "Test hypothesis".to_string(),
            null_hypothesis: Some("Null".to_string()),
            alternative_hypothesis: Some("Alt".to_string()),
            confidence_level: 0.95,
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains("exploration"));
    }
}
