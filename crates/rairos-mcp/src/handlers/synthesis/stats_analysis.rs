use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Clone)]
#[allow(dead_code)]
struct TestRecommendation {
    test_name: String,
    alternate_names: Vec<String>,
    use_when: Vec<String>,
    data_type: String,
    groups: String,
    assumptions: Vec<String>,
    effect_size: String,
    apa_template: String,
}

fn get_test_recommendations(data_type: &str, groups: &str, hypothesis: &str) -> Vec<TestRecommendation> {
    let mut tests = Vec::new();

    match (data_type, groups, hypothesis) {
        ("continuous", "two_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "Independent Samples t-test".to_string(),
                alternate_names: vec!["Two-sample t-test".to_string(), "Student's t-test".to_string()],
                use_when: vec!["Comparing means between two independent groups".to_string(), "Continuous outcome with normal distribution".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Homogeneity of variance".to_string(), "Independence of observations".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's d: small=0.2, medium=0.5, large=0.8".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Welch's t-test".to_string(),
                alternate_names: vec!["Welch's unequal variances t-test".to_string()],
                use_when: vec!["Two independent groups with unequal variances".to_string(), "Robust to variance heterogeneity".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Independence of observations".to_string(), "Does not assume equal variances".to_string()],
                effect_size: "Cohen's d or Hedges' g".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Mann-Whitney U Test".to_string(),
                alternate_names: vec!["Wilcoxon rank-sum test".to_string(), "Non-parametric alternative to t-test".to_string()],
                use_when: vec!["Non-normal continuous data".to_string(), "Ordinal data".to_string(), "Violation of normality assumption".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Ordinal or continuous data".to_string(), "Independence of observations".to_string(), "Similar distribution shapes (for median comparison)".to_string()],
                effect_size: "r = Z / sqrt(N): small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "U = XXX, p = .XXX, r = .XX".to_string(),
            });
        },
        ("continuous", "two_paired", _) => {
            tests.push(TestRecommendation {
                test_name: "Paired Samples t-test".to_string(),
                alternate_names: vec!["Dependent t-test".to_string(), "Matched pairs t-test".to_string()],
                use_when: vec!["Before-after measurements".to_string(), "Matched pairs design".to_string(), "Two measurements on same subjects".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two paired/matched groups".to_string(),
                assumptions: vec!["Normality of differences".to_string(), "Independence within pairs".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's d: small=0.2, medium=0.5, large=0.8".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d_z = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Wilcoxon Signed-Rank Test".to_string(),
                alternate_names: vec!["Non-parametric alternative to paired t-test".to_string()],
                use_when: vec!["Non-normal paired data".to_string(), "Ordinal paired data".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Two paired groups".to_string(),
                assumptions: vec!["Ordinal or continuous differences".to_string(), "Symmetric distribution of differences".to_string()],
                effect_size: "r = Z / sqrt(N): small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "W = XXX, p = .XXX, r = .XX".to_string(),
            });
        },
        ("continuous", "three_plus_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "One-way ANOVA".to_string(),
                alternate_names: vec!["Analysis of Variance".to_string(), "F-test".to_string()],
                use_when: vec!["Comparing means across 3+ independent groups".to_string(), "Continuous outcome".to_string(), "One categorical independent variable".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Three or more independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Homogeneity of variance".to_string(), "Independence of observations".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's f: small=0.10, medium=0.25, large=0.40 (or eta-squared: small=0.01, medium=0.06, large=0.14)".to_string(),
                apa_template: "F(df_between, df_within) = X.XX, p = .XXX, f² = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Kruskal-Wallis H Test".to_string(),
                alternate_names: vec!["Non-parametric alternative to one-way ANOVA".to_string(), "H-test".to_string()],
                use_when: vec!["Non-normal data across 3+ groups".to_string(), "Ordinal data".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Three or more independent groups".to_string(),
                assumptions: vec!["Ordinal or continuous data".to_string(), "Independence of observations".to_string(), "Similar distribution shapes".to_string()],
                effect_size: "Epsilon-squared (epsilon²): small=0.01, medium=0.06, large=0.14".to_string(),
                apa_template: "H(df) = X.XX, p = .XXX, epsilon² = .XX".to_string(),
            });
        },
        ("categorical", "two_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "Chi-square Test of Independence".to_string(),
                alternate_names: vec!["Chi-squared test".to_string(), "contingency table test".to_string()],
                use_when: vec!["Comparing proportions between groups".to_string(), "Categorical outcome with 2+ categories".to_string(), "Testing association between variables".to_string()],
                data_type: "Categorical".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Expected frequencies >= 5 in each cell (or >80% cells with >=5)".to_string(), "Independence of observations".to_string(), "Random sampling".to_string()],
                effect_size: "Cramér's V: small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "χ²(df) = X.XX, p = .XXX, V = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Fisher's Exact Test".to_string(),
                alternate_names: vec!["Fisher-Irwin test".to_string()],
                use_when: vec!["Small sample sizes".to_string(), "2x2 contingency table".to_string(), "Expected frequencies < 5".to_string()],
                data_type: "Categorical".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Hypergeometric distribution".to_string(), "Fixed marginal totals".to_string(), "Independence of observations".to_string()],
                effect_size: "Odds ratio or Cramér's V".to_string(),
                apa_template: "OR = X.XX, p = .XXX (Fisher's exact test)".to_string(),
            });
        },
        ("continuous", "correlation", _) | ("ordinal", "correlation", _) => {
            tests.push(TestRecommendation {
                test_name: "Pearson Correlation".to_string(),
                alternate_names: vec!["Pearson's r".to_string(), "Product-moment correlation".to_string()],
                use_when: vec!["Measuring linear relationship between two continuous variables".to_string(), "Both variables normally distributed".to_string()],
                data_type: "Continuous (bivariate normal)".to_string(),
                groups: "N/A (correlation)".to_string(),
                assumptions: vec!["Linearity".to_string(), "Normality of both variables".to_string(), "Homoscedasticity".to_string(), "No significant outliers".to_string()],
                effect_size: "r: small=0.1, medium=0.3, large=0.5 (or r² for variance explained)".to_string(),
                apa_template: "r(df) = .XX, p = .XXX, r² = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Spearman's Rank Correlation".to_string(),
                alternate_names: vec!["Spearman's rho".to_string(), "Rank correlation".to_string()],
                use_when: vec!["Non-normal continuous data".to_string(), "Ordinal data".to_string(), "Monotonic relationships".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "N/A (correlation)".to_string(),
                assumptions: vec!["Monotonic relationship (not necessarily linear)".to_string(), "Ordinal or continuous data".to_string(), "Independence of observations".to_string()],
                effect_size: "rho: small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "rho = .XX, p = .XXX".to_string(),
            });
        },
        ("continuous", "regression", _) => {
            tests.push(TestRecommendation {
                test_name: "Linear Regression".to_string(),
                alternate_names: vec!["OLS regression".to_string(), "Multiple linear regression".to_string()],
                use_when: vec!["Predicting continuous outcome from predictors".to_string(), "Continuous or dichotomous predictors".to_string(), "Testing relationship between variables".to_string()],
                data_type: "Continuous outcome".to_string(),
                groups: "N/A (predictive)".to_string(),
                assumptions: vec!["Linearity".to_string(), "Normality of residuals".to_string(), "Homoscedasticity".to_string(), "Independence of residuals".to_string(), "No multicollinearity (for multiple regression)".to_string()],
                effect_size: "R²: small=0.02, medium=0.13, large=0.26 (f² for added predictors)".to_string(),
                apa_template: "R² = .XX, F(df_model, df_residual) = X.XX, p = .XXX".to_string(),
            });
        },
        _ => {
            tests.push(TestRecommendation {
                test_name: "Descriptive Statistics".to_string(),
                alternate_names: vec!["Summary statistics".to_string()],
                use_when: vec!["Initial data exploration".to_string(), "Large sample sizes (n > 30)".to_string(), "Unknown distribution".to_string()],
                data_type: data_type.to_string(),
                groups: groups.to_string(),
                assumptions: vec!["No specific distributional assumptions".to_string()],
                effect_size: "N/A for descriptive statistics".to_string(),
                apa_template: "M = X.XX, SD = X.XX, Range: X-XX".to_string(),
            });
        }
    }

    if groups == "three_plus_independent" || groups == "two_paired" {
        tests.insert(0, TestRecommendation {
            test_name: "Consider repeated measures ANOVA".to_string(),
            alternate_names: vec!["RM-ANOVA".to_string(), "Within-subjects ANOVA".to_string()],
            use_when: vec!["Same subjects measured multiple times".to_string(), "Longitudinal data".to_string(), "Matched measurements".to_string()],
            data_type: "Continuous".to_string(),
            groups: "Multiple time points or conditions".to_string(),
            assumptions: vec!["Sphericity (or use Greenhouse-Geisser correction)".to_string(), "Normality".to_string(), "Independence within subjects".to_string()],
            effect_size: "Partial eta-squared (η²p): small=0.01, medium=0.06, large=0.14".to_string(),
            apa_template: "F(df_time, df_error) = X.XX, p = .XXX, η²p = .XX".to_string(),
        });
    }

    tests
}

fn build_analysis_markdown(
    research_question: &str,
    data_type: &str,
    groups: &str,
    hypothesis: &str,
    tests: &[TestRecommendation],
) -> String {
    let mut md = String::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    md.push_str("# Statistical Analysis Advisor\n\n");
    md.push_str(&format!("**Research Question:** {}\n", research_question));
    md.push_str(&format!("**Date:** {}\n\n", today));

    md.push_str("---\n\n");

    md.push_str("## Analysis Plan\n\n");
    md.push_str("Based on your inputs:\n");
    md.push_str(&format!("- **Data Type:** {}\n", data_type));
    md.push_str(&format!("- **Group Structure:** {}\n", groups));
    md.push_str(&format!("- **Hypothesis Type:** {}\n\n", hypothesis));

    md.push_str("---\n\n");
    md.push_str("## Recommended Statistical Tests\n\n");

    for (i, test) in tests.iter().enumerate() {
        md.push_str(&format!("### {}: {}\n\n", i + 1, test.test_name));
        if !test.alternate_names.is_empty() {
            md.push_str(&format!("**Also known as:** {}\n", test.alternate_names.join(", ")));
        }
        md.push_str("**Use when:**\n");
        for uw in &test.use_when {
            md.push_str(&format!("- {}\n", uw));
        }
        md.push('\n');

        md.push_str("**Assumptions:**\n");
        for a in &test.assumptions {
            md.push_str(&format!("- {}\n", a));
        }
        md.push('\n');

        md.push_str(&format!("**Effect Size:** {}\n\n", test.effect_size));

        md.push_str("**APA Format Result:**\n");
        md.push_str(&format!("```\n{}\n```\n\n", test.apa_template));

        md.push_str("---\n\n");
    }

    md.push_str("## Effect Size Interpretation Guide\n\n");
    md.push_str("| Effect Size | Small | Medium | Large |\n");
    md.push_str("|-------------|-------|--------|-------|\n");
    md.push_str("| Cohen's d | 0.2 | 0.5 | 0.8 |\n");
    md.push_str("| r (Pearson) | 0.1 | 0.3 | 0.5 |\n");
    md.push_str("| Cohen's f | 0.10 | 0.25 | 0.40 |\n");
    md.push_str("| eta-squared | 0.01 | 0.06 | 0.14 |\n");
    md.push_str("| Cramér's V | 0.1 | 0.3 | 0.5 |\n");
    md.push_str("| Odds Ratio | 1.5 | 2.5 | 4.0 |\n\n");

    md.push_str("## Sample Size Guidelines\n\n");
    md.push_str("- **t-tests:** Minimum n=30 per group for normality approximation\n");
    md.push_str("- **ANOVA:** n=30 per group recommended; at least n=20 per group\n");
    md.push_str("- **Chi-square:** Expected frequency >= 5 in 80%+ of cells\n");
    md.push_str("- **Correlation:** r=0.3 requires n~85; r=0.5 requires n~30; r=0.7 requires n~16\n");
    md.push_str("- **Regression:** n >= 50 + 8k (k = number of predictors)\n\n");

    md.push_str("---\n\n");
    md.push_str("## Reporting Checklist\n\n");
    md.push_str("- [ ] State the statistical test used\n");
    md.push_str("- [ ] Report test statistic, degrees of freedom, and p-value\n");
    md.push_str("- [ ] Report effect size with confidence intervals\n");
    md.push_str("- [ ] Check and report assumption violations\n");
    md.push_str("- [ ] Report exact p-values (not just p < .05)\n");
    md.push_str("- [ ] Include descriptive statistics (M, SD for continuous; n, % for categorical)\n\n");

    md.push_str(&format!("_Generated by Rairos on {} for statistical analysis guidance_\n", today));

    md
}

pub struct StatisticalAnalysisHandler;

#[async_trait]
impl ToolHandler for StatisticalAnalysisHandler {
    fn name(&self) -> &str { "statistical_analysis_guide" }
    fn description(&self) -> &str { "Statistical analysis advisor: recommend appropriate tests based on research question, data type, and group structure. Provides effect size guidelines and APA result templates for t-tests, ANOVA, chi-square, correlation, and regression." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("research_question".into(), ToolProperty::string("Brief description of your research question")),
                ("data_type".into(), ToolProperty::string("Type of data: 'continuous' (measurements), 'categorical' (counts/groups), or 'ordinal' (ranked data)")),
                ("groups".into(), ToolProperty::string("Group structure: 'two_independent', 'two_paired', 'three_plus_independent', 'correlation', or 'regression'")),
                ("hypothesis".into(), ToolProperty::string("Type of hypothesis: 'difference', 'association', 'prediction', or 'correlation'")),
            ].into_iter().collect(),
            vec!["research_question".into(), "data_type".into(), "groups".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let research_question = params["research_question"].as_str().ok_or("Missing research_question")?;
        let data_type = params["data_type"].as_str().ok_or("Missing data_type")?;
        let groups = params["groups"].as_str().ok_or("Missing groups")?;
        let hypothesis = params.get("hypothesis").and_then(|v| v.as_str()).unwrap_or("difference");

        let tests = get_test_recommendations(data_type, groups, hypothesis);
        let markdown = build_analysis_markdown(research_question, data_type, groups, hypothesis, &tests);

        let test_names: Vec<String> = tests.iter().map(|t| t.test_name.clone()).collect();

        Ok(serde_json::json!({
            "research_question": research_question,
            "data_type": data_type,
            "groups": groups,
            "hypothesis": hypothesis,
            "recommended_tests": test_names,
            "test_count": tests.len(),
            "markdown": markdown,
        }))
    }
}
