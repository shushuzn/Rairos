//! Replication Checker — assesses reproducibility from paper metadata + abstract.
//!
//! Mirrors llm/replication_checker.py

use crate::LlmClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationAssessment {
    pub score: f64,           // 0.0 (hard) - 1.0 (easy)
    pub has_code: bool,
    pub has_data: bool,
    pub has_method: bool,
    pub has_env: bool,
    pub reasoning: String,
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReplicationVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

impl ReplicationVerificationResult {
    pub fn valid() -> Self {
        Self { is_valid: true, warnings: Vec::new() }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { is_valid: warnings.is_empty(), warnings }
    }
}

pub fn keyword_check(abstract_text: &str) -> ReplicationAssessment {
    let lower = abstract_text.to_lowercase();
    let has_code = lower.contains("github") || lower.contains("code available") || lower.contains("source code") || lower.contains("open source");
    let has_data = lower.contains("dataset") || lower.contains("data available") || lower.contains("benchmark");
    let has_method = lower.contains("algorithm") || lower.contains("method") || lower.contains("architecture") || lower.contains("implementation");
    let has_env = lower.contains("docker") || lower.contains("environment") || lower.contains("python") || lower.contains("pytorch") || lower.contains("tensorflow");

    let count = [has_code, has_data, has_method, has_env].iter().filter(|&&x| x).count();
    let score = count as f64 / 4.0;

    ReplicationAssessment { score, has_code, has_data, has_method, has_env, reasoning: String::new(), verification_warnings: Vec::new() }
}

pub async fn llm_assess_replication(
    llm: &dyn LlmClient,
    model: &str,
    title: &str,
    abstract_text: &str,
) -> ReplicationAssessment {
    let kw = keyword_check(abstract_text);
    let prompt = format!(
        "Assess the reproducibility of this paper (score 0.0-1.0).\n\
        Consider: code availability, dataset access, method clarity, environment specs.\n\n\
        Title: {}\nAbstract: {}\n\nKeyword signals: code={}, data={}, method={}, env={}",
        title, abstract_text, kw.has_code, kw.has_data, kw.has_method, kw.has_env
    );

    let msg = crate::Message { role: "user".to_string(), content: prompt };
    let body = match llm.complete(vec![msg], model, 0.2, 500).await {
        Ok(crate::LlmResponse::NonStream(ns)) => ns.content,
        _ => return ReplicationAssessment { score: kw.score, ..kw },
    };

    let llm_score = body.lines()
        .find_map(|l| {
            let l = l.trim();
            if l.starts_with("Score:") || l.starts_with("score:") {
                let rest = l.trim_start_matches("Score:").trim_start_matches("score:").trim();
                rest.split_whitespace().next().and_then(|s| s.parse::<f64>().ok())
            } else { None }
        })
        .unwrap_or(kw.score);

    let clamped_score = llm_score.clamp(0.0, 1.0);
    let verification = verify_replication_assessment(llm, model, title, abstract_text, clamped_score).await;

    ReplicationAssessment {
        score: clamped_score,
        reasoning: body,
        verification_warnings: verification.warnings,
        ..kw
    }
}

const VERIFY_REPLICATION_PROMPT: &str = r#"你是一个严谨的可复现性评估验证助手。检查以下评估是否合理。

标题: {title}
摘要: {abstract}
评分: {score}

请验证：
1. 评分是否与论文的可复现性特征匹配？
2. reasoning是否与评分一致？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1"]}}

如果评估合理，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_replication_assessment(
    llm: &dyn LlmClient,
    model: &str,
    title: &str,
    abstract_text: &str,
    score: f64,
) -> ReplicationVerificationResult {
    if score == 0.0 || score == 1.0 {
        return ReplicationVerificationResult::valid();
    }

    let prompt = VERIFY_REPLICATION_PROMPT
        .replace("{title}", &title.chars().take(100).collect::<String>())
        .replace("{abstract}", &abstract_text.chars().take(200).collect::<String>())
        .replace("{score}", &format!("{:.2}", score));

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 300).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_verification_result(&ns.content)
        }
        _ => ReplicationVerificationResult::valid(),
    }
}

fn parse_verification_result(content: &str) -> ReplicationVerificationResult {
    let content = content.trim();

    let _is_valid = if content.contains("\"is_valid\": true") || content.contains("\"is_valid\":true") {
        true
    } else if content.contains("\"is_valid\": false") || content.contains("\"is_valid\":false") {
        false
    } else {
        return ReplicationVerificationResult::valid();
    };

    let mut warnings = Vec::new();
    if let Some(start) = content.find("\"warnings\":") {
        let warnings_str = &content[start..];
        if let Some(arr_start) = warnings_str.find('[') {
            if let Some(arr_end) = warnings_str.find(']') {
                let items = &warnings_str[arr_start + 1..arr_end];
                for item in items.split(',') {
                    let item = item.trim().trim_matches('"').trim_matches(|c| c == '"' || c == ' ');
                    if !item.is_empty() && item != "[]" && item != "warnings" {
                        warnings.push(item.to_string());
                    }
                }
            }
        }
    }

    ReplicationVerificationResult::with_warnings(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_check_code_available() {
        let r = keyword_check("We release our code on GitHub.");
        assert!(r.has_code);
        assert!(!r.has_data);
    }

    #[test]
    fn test_keyword_check_full_replication() {
        let r = keyword_check("Code available on GitHub. Dataset released. Algorithm described. PyTorch implementation.");
        assert!(r.has_code && r.has_data && r.has_method && r.has_env);
        assert!((r.score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_keyword_check_has_verification_warnings() {
        let r = keyword_check("Code available on GitHub.");
        assert!(r.verification_warnings.is_empty());
    }

    #[test]
    fn test_replication_verification_result_valid() {
        let result = ReplicationVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_replication_verification_result_with_warnings() {
        let warnings = vec!["评分不合理".to_string()];
        let result = ReplicationVerificationResult::with_warnings(warnings);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_parse_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_verification_result(json);
        assert!(result.is_valid);
    }

    #[test]
    fn test_parse_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["评分与论文不符"]}"#;
        let result = parse_verification_result(json);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_parse_verification_result_malformed() {
        let json = "not json";
        let result = parse_verification_result(json);
        assert!(result.is_valid);
    }
}
