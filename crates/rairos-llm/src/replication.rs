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
}

pub fn keyword_check(abstract_text: &str) -> ReplicationAssessment {
    let lower = abstract_text.to_lowercase();
    let has_code = lower.contains("github") || lower.contains("code available") || lower.contains("source code") || lower.contains("open source");
    let has_data = lower.contains("dataset") || lower.contains("data available") || lower.contains("benchmark");
    let has_method = lower.contains("algorithm") || lower.contains("method") || lower.contains("architecture") || lower.contains("implementation");
    let has_env = lower.contains("docker") || lower.contains("environment") || lower.contains("python") || lower.contains("pytorch") || lower.contains("tensorflow");

    let count = [has_code, has_data, has_method, has_env].iter().filter(|&&x| x).count();
    let score = count as f64 / 4.0;

    ReplicationAssessment { score, has_code, has_data, has_method, has_env, reasoning: String::new() }
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

    ReplicationAssessment {
        score: llm_score.clamp(0.0, 1.0),
        reasoning: body,
        ..kw
    }
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
}
