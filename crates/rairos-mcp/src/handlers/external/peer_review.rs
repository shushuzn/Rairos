use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Default)]
struct PeerReviewChecklist {
    has_abstract: bool,
    has_introduction: bool,
    has_methods: bool,
    has_results: bool,
    has_discussion: bool,
    has_references: bool,
    has_ethics_statement: bool,
    has_conflict_of_interest: bool,
    has_limitations: bool,
    has_data_availability: bool,
    has_sample_size_justification: bool,
    has_statistical_tests: bool,
    has_confidence_intervals: bool,
    has_effect_sizes: bool,
    has_replicates: bool,
    novelty_score: u8,
    methodology_score: u8,
    clarity_score: u8,
    reproducibility_score: u8,
}

impl PeerReviewChecklist {
    fn evaluate(title: &str, abstract_text: &str, sections: &str) -> Self {
        let text_lower = format!("{} {} {}", title, abstract_text, sections).to_lowercase();
        let mut checklist = PeerReviewChecklist::default();

        checklist.has_abstract = !abstract_text.is_empty();
        checklist.has_introduction = text_lower.contains("introduction") || text_lower.contains("background");
        checklist.has_methods = text_lower.contains("method") || text_lower.contains("experiment") || text_lower.contains("procedure");
        checklist.has_results = text_lower.contains("result") || text_lower.contains("finding") || text_lower.contains("outcome");
        checklist.has_discussion = text_lower.contains("discussion") || text_lower.contains("conclusion");
        checklist.has_references = text_lower.contains("reference") || text_lower.contains("citation") || sections.len() > 5000;
        checklist.has_ethics_statement = text_lower.contains("ethics") || text_lower.contains("irb") || text_lower.contains("approval") || text_lower.contains("consent");
        checklist.has_conflict_of_interest = text_lower.contains("conflict") || text_lower.contains("coi") || text_lower.contains("disclosure");
        checklist.has_limitations = text_lower.contains("limitation") || text_lower.contains("caveat");
        checklist.has_data_availability = text_lower.contains("data availability") || text_lower.contains("supplementary") || text_lower.contains("repository");
        checklist.has_sample_size_justification = text_lower.contains("sample size") || text_lower.contains("power analysis") || text_lower.contains("n =");
        checklist.has_statistical_tests = text_lower.contains("p-value") || text_lower.contains("t-test") || text_lower.contains("anova") || text_lower.contains("regression") || text_lower.contains("wilcoxon") || text_lower.contains("mann-whitney");
        checklist.has_confidence_intervals = text_lower.contains("confidence interval") || text_lower.contains("ci:");
        checklist.has_effect_sizes = text_lower.contains("effect size") || text_lower.contains("cohen") || text_lower.contains("odds ratio");
        checklist.has_replicates = text_lower.contains("replicate") || text_lower.contains("triplicate") || text_lower.contains("n = 3") || text_lower.contains("n=3");

        checklist.novelty_score = if text_lower.contains("novel") || text_lower.contains("first") || text_lower.contains("new method") || text_lower.contains("state-of-the-art") || text_lower.contains("sota") { 5 } else if text_lower.contains("improve") || text_lower.contains("advance") { 4 } else if text_lower.contains("build") || text_lower.contains("extend") { 3 } else { 2 };
        checklist.methodology_score = if checklist.has_methods && checklist.has_statistical_tests && checklist.has_sample_size_justification { 5 } else if checklist.has_methods { 3 } else { 1 };
        checklist.clarity_score = if text_lower.len() > 2000 { 4 } else if text_lower.len() > 500 { 3 } else { 2 };
        checklist.reproducibility_score = if checklist.has_data_availability && checklist.has_methods && checklist.has_replicates { 5 } else if checklist.has_data_availability || checklist.has_methods { 3 } else { 1 };

        checklist
    }

    fn overall_score(&self) -> f64 {
        (self.novelty_score as f64 + self.methodology_score as f64 + self.clarity_score as f64 + self.reproducibility_score as f64) / 4.0
    }

    fn recommendation(&self) -> &'static str {
        let score = self.overall_score();
        if score >= 4.0 { "Accept" }
        else if score >= 3.0 { "Minor Revision" }
        else if score >= 2.0 { "Major Revision" }
        else { "Reject" }
    }

    fn major_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_methods { issues.push("Missing or inadequate Methods section"); }
        if !self.has_results { issues.push("Missing or inadequate Results section"); }
        if !self.has_discussion { issues.push("Missing or inadequate Discussion/Conclusion section"); }
        if !self.has_statistical_tests { issues.push("No mention of statistical tests used for analysis"); }
        if self.methodology_score < 3 { issues.push("Methodology appears insufficiently detailed for reproducibility"); }
        if !self.has_data_availability { issues.push("No data availability statement — reproducibility concern"); }
        if self.reproducibility_score < 2 { issues.push("Low reproducibility score — missing key elements"); }
        issues
    }

    fn minor_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_abstract { issues.push("Abstract missing or empty"); }
        if !self.has_ethics_statement { issues.push("Ethics statement not explicitly mentioned"); }
        if !self.has_conflict_of_interest { issues.push("Conflict of interest statement not provided"); }
        if !self.has_limitations { issues.push("Limitations section missing — important for reader assessment"); }
        if !self.has_sample_size_justification { issues.push("Sample size justification or power analysis not described"); }
        if !self.has_confidence_intervals { issues.push("Confidence intervals not reported alongside point estimates"); }
        if !self.has_effect_sizes { issues.push("Effect sizes not explicitly reported — limits interpretability"); }
        if !self.has_replicates { issues.push("Number of replicates or independent experiments not clearly stated"); }
        issues
    }
}

pub struct PaperPeerReviewHandler;

#[async_trait]
impl ToolHandler for PaperPeerReviewHandler {
    fn name(&self) -> &str { "paper_peer_review" }
    fn description(&self) -> &str { "Generate a structured peer review for a scientific paper with compliance checklist, major/minor issues, and recommendation" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("sections".into(), ToolProperty::string("Full text of paper sections (introduction, methods, results, discussion)")),
                ("checklist_type".into(), ToolProperty::string("Optional: CONSORT (clinical trials), STROBE (observational), PRISMA (meta-analyses), or general (default)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params.get("abstract_text").and_then(|v| v.as_str()).unwrap_or("");
        let sections = params.get("sections").and_then(|v| v.as_str()).unwrap_or("");
        let checklist_type = params.get("checklist_type").and_then(|v| v.as_str()).unwrap_or("general");

        let checklist = PeerReviewChecklist::evaluate(title, abstract_text, sections);
        let overall_score = checklist.overall_score();
        let recommendation = checklist.recommendation();
        let major_issues = checklist.major_issues();
        let minor_issues = checklist.minor_issues();

        let mut compliance = serde_json::json!({
            "abstract": checklist.has_abstract,
            "introduction": checklist.has_introduction,
            "methods": checklist.has_methods,
            "results": checklist.has_results,
            "discussion": checklist.has_discussion,
            "references": checklist.has_references,
            "ethics_statement": checklist.has_ethics_statement,
            "conflict_of_interest": checklist.has_conflict_of_interest,
            "limitations": checklist.has_limitations,
            "data_availability": checklist.has_data_availability,
            "sample_size_justification": checklist.has_sample_size_justification,
            "statistical_tests": checklist.has_statistical_tests,
            "confidence_intervals": checklist.has_confidence_intervals,
            "effect_sizes": checklist.has_effect_sizes,
            "replicates": checklist.has_replicates,
        });

        let sections_lower = sections.to_lowercase();

        if checklist_type == "CONSORT" {
            compliance["consort_checklist"] = serde_json::json!({
                "title_and_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_intervention": checklist.has_methods,
                "methods_outcomes": checklist.has_results,
                "methods_sample_size": checklist.has_sample_size_justification,
                "results_numbers_analyzed": checklist.has_results,
                "results_harms": sections_lower.contains("adverse") || sections_lower.contains("side effect"),
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
            });
        } else if checklist_type == "STROBE" {
            compliance["strobe_checklist"] = serde_json::json!({
                "title_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_study_design": checklist.has_methods,
                "methods_setting": checklist.has_methods,
                "methods_participants": sections_lower.contains("participant") || sections_lower.contains("patient"),
                "methods_variables": checklist.has_methods,
                "methods_data_sources": checklist.has_methods,
                "methods_bias": checklist.has_methods,
                "methods_quantitative": checklist.has_statistical_tests,
                "results_participants": checklist.has_results,
                "results_descriptive": checklist.has_results,
                "results_outcome_data": checklist.has_results,
                "discussion_key_results": checklist.has_discussion,
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
                "discussion_funding": sections_lower.contains("funding") || sections_lower.contains("grant"),
            });
        } else if checklist_type == "PRISMA" {
            compliance["prisma_checklist"] = serde_json::json!({
                "title": checklist.has_abstract,
                "abstract": checklist.has_abstract,
                "introduction_eligibility_criteria": checklist.has_introduction,
                "introduction_information_sources": sections_lower.contains("database") || sections_lower.contains("search"),
                "introduction_search_strategy": sections_lower.contains("search"),
                "methods_study_selection": checklist.has_methods,
                "methods_data_extraction": checklist.has_methods,
                "methods_risk_of_bias": checklist.has_methods,
                "methods_results_synthesis": checklist.has_results,
                "results_study_selection": checklist.has_results,
                "results_study_characteristics": checklist.has_results,
                "results_risk_of_bias": checklist.has_results,
                "results_results_synthesis": checklist.has_results,
                "discussion_limitations": checklist.has_limitations,
                "discussion_conclusions": checklist.has_discussion,
                "discussion_registration": sections_lower.contains("registration") || sections_lower.contains("protocol"),
            });
        }

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "title": title,
            "checklist_type": checklist_type,
            "overall_score": overall_score,
            "recommendation": recommendation,
            "dimension_scores": {
                "novelty": checklist.novelty_score,
                "methodology": checklist.methodology_score,
                "clarity": checklist.clarity_score,
                "reproducibility": checklist.reproducibility_score,
            },
            "compliance": compliance,
            "major_issues": major_issues,
            "minor_issues": minor_issues,
            "review_summary": format!(
                "This paper '{}' receives an overall score of {:.1}/5.0 and a recommendation of {}. \
                The review identified {} major issue(s) and {} minor issue(s). \
                Key strengths: novelty ({}/5), methodology ({}/5), clarity ({}/5), reproducibility ({}/5). \
                {}",
                title, overall_score, recommendation,
                major_issues.len(), minor_issues.len(),
                checklist.novelty_score, checklist.methodology_score, checklist.clarity_score, checklist.reproducibility_score,
                if major_issues.is_empty() { "No major issues identified." } else { major_issues[0] }
            ),
        }))
    }
}
