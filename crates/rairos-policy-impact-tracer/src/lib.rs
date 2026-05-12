//! rairos-policy-impact-tracer — Policy Impact Tracer for AI Research OS.
//!
//! Ported from `llm/policy_impact_tracer.py`.
//!
//! Tracks AI policy/regulation developments and maps their impact to research domains.
//!
//! ## Architecture
//!
//! - `check_policy_impact()` — pure function, no external dependencies
//! - `get_impacted_capsules()` — requires a `CapsuleProvider` implementation
//! - `render_policy_tracer_html()` — renders the full HTML report

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regulation {
    pub name: String,
    pub jurisdiction: String,
    pub effective_date: String,
    pub affected_domains: Vec<String>,
    pub affected_gap_types: Vec<String>,
    pub keywords: Vec<String>,
    #[serde(rename = "priority_boost")]
    pub priority_boost: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyImpact {
    #[serde(rename = "regulation_id")]
    pub regulation_id: String,
    #[serde(rename = "regulation_name")]
    pub regulation_name: String,
    pub jurisdiction: String,
    #[serde(rename = "effective_date")]
    pub effective_date: String,
    #[serde(rename = "affected_domains")]
    pub affected_domains: Vec<String>,
    #[serde(rename = "priority_boost")]
    pub priority_boost: HashMap<String, f64>,
    #[serde(rename = "match_reason")]
    pub match_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactedCapsule {
    #[serde(rename = "capsule_id")]
    pub capsule_id: String,
    #[serde(rename = "gap_title")]
    pub gap_title: String,
    #[serde(rename = "gap_type")]
    pub gap_type: String,
    pub regulation: String,
    #[serde(rename = "priority_boost")]
    pub priority_boost: f64,
}

// ─── Regulations ─────────────────────────────────────────────────────────────

fn eu_ai_act() -> Regulation {
    let mut pb = HashMap::new();
    pb.insert("evaluation_gap".to_string(), 0.3);
    pb.insert("generalization_gap".to_string(), 0.1);
    Regulation {
        name: "EU AI Act".to_string(),
        jurisdiction: "European Union".to_string(),
        effective_date: "2025-08".to_string(),
        affected_domains: vec!["cs.AI".to_string(), "cs.LG".to_string(), "cs.CY".to_string(), "cs.CV".to_string()],
        affected_gap_types: vec!["evaluation_gap".to_string(), "scalability_issue".to_string(), "method_limitation".to_string()],
        keywords: vec!["eu ai act".to_string(), "high-risk".to_string(), "conformity".to_string(), "prohibited ai".to_string()],
        priority_boost: pb,
    }
}

fn us_ai_eo() -> Regulation {
    let mut pb = HashMap::new();
    pb.insert("theoretical_gap".to_string(), 0.2);
    pb.insert("evaluation_gap".to_string(), 0.2);
    Regulation {
        name: "US AI Executive Order".to_string(),
        jurisdiction: "United States".to_string(),
        effective_date: "2024-01".to_string(),
        affected_domains: vec!["cs.AI".to_string(), "cs.LG".to_string()],
        affected_gap_types: vec!["safety".to_string(), "alignment".to_string(), "evaluation".to_string()],
        keywords: vec!["executive order ai".to_string(), "safety".to_string(), "us government ai".to_string()],
        priority_boost: pb,
    }
}

fn gdpr_ai() -> Regulation {
    let mut pb = HashMap::new();
    pb.insert("dataset_gap".to_string(), 0.3);
    pb.insert("evaluation_gap".to_string(), 0.1);
    Regulation {
        name: "GDPR for AI Systems".to_string(),
        jurisdiction: "European Union".to_string(),
        effective_date: "2024-05".to_string(),
        affected_domains: vec!["cs.AI".to_string(), "cs.LG".to_string(), "cs.CY".to_string()],
        affected_gap_types: vec!["evaluation_gap".to_string(), "dataset_gap".to_string()],
        keywords: vec!["gdpr".to_string(), "data protection".to_string(), "privacy ai".to_string(), "personal data ai".to_string()],
        priority_boost: pb,
    }
}

fn china_ai() -> Regulation {
    let mut pb = HashMap::new();
    pb.insert("scalability_issue".to_string(), 0.2);
    pb.insert("method_limitation".to_string(), 0.1);
    Regulation {
        name: "China AI Regulations".to_string(),
        jurisdiction: "China".to_string(),
        effective_date: "2024-01".to_string(),
        affected_domains: vec!["cs.AI".to_string(), "cs.LG".to_string(), "cs.CL".to_string()],
        affected_gap_types: vec!["method_limitation".to_string(), "scalability_issue".to_string()],
        keywords: vec!["china ai regulation".to_string(), "generative ai china".to_string(), "china ml policy".to_string()],
        priority_boost: pb,
    }
}

/// All known regulations.
pub fn regulations() -> HashMap<&'static str, Regulation> {
    HashMap::from([
        ("EU_AI_Act", eu_ai_act()),
        ("US_AI_Executive_Order", us_ai_eo()),
        ("GDPR_AI", gdpr_ai()),
        ("China_AI_Regulation", china_ai()),
    ])
}

// ─── CapsuleProvider trait ────────────────────────────────────────────────────

/// Trait for loading Gene Pool capsules.
/// Implement this to integrate with the actual capsule storage backend.
pub trait CapsuleProvider {
    fn load_capsules(&self) -> Vec<HashMap<String, serde_json::Value>>;
}

// ─── Core logic ──────────────────────────────────────────────────────────────

/// Check which regulations a paper relates to.
pub fn check_policy_impact(paper: &HashMap<String, serde_json::Value>) -> Vec<PolicyImpact> {
    let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let abstract_text = paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("");
    let text = (title.to_string() + " " + abstract_text).to_lowercase();

    let cats: std::collections::HashSet<_> = paper
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let regs = regulations();
    let mut results = Vec::new();

    for (rid, reg) in &regs {
        // Keyword match
        let kw_match = reg.keywords.iter().any(|kw| text.contains(&kw.to_lowercase()));
        // Category match
        let cat_match = reg.affected_domains.iter().any(|c| cats.contains(c.as_str()));

        if kw_match || cat_match {
            results.push(PolicyImpact {
                regulation_id: rid.to_string(),
                regulation_name: reg.name.clone(),
                jurisdiction: reg.jurisdiction.clone(),
                effective_date: reg.effective_date.clone(),
                affected_domains: reg.affected_domains.clone(),
                priority_boost: reg.priority_boost.clone(),
                match_reason: if kw_match { "keyword".to_string() } else { "category".to_string() },
            });
        }
    }
    results
}

/// Return Gene Pool capsules whose gap types are affected by current regulations.
/// Uses the provided `CapsuleProvider` to load capsules.
pub fn get_impacted_capsules<P: CapsuleProvider>(provider: &P) -> Vec<ImpactedCapsule> {
    let capsules = provider.load_capsules();
    let regs = regulations();
    let mut impacted: Vec<ImpactedCapsule> = Vec::new();

    for cap in capsules {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !status.is_empty() && status != "active" {
            continue;
        }

        let gap_type = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .or_else(|| cap.get("trigger_gap_type").and_then(|v| v.as_str()))
            .unwrap_or("");

        for reg in regs.values() {
            if reg.affected_gap_types.iter().any(|gt| gt == gap_type) {
                let boost = reg.priority_boost.get(gap_type).copied().unwrap_or(0.0);
                impacted.push(ImpactedCapsule {
                    capsule_id: cap.get("capsule_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    gap_title: cap.get("action_gap_title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    gap_type: gap_type.to_string(),
                    regulation: reg.name.clone(),
                    priority_boost: boost,
                });
                break;
            }
        }
    }

    impacted.sort_by(|a, b| b.priority_boost.partial_cmp(&a.priority_boost).unwrap_or(std::cmp::Ordering::Equal));
    impacted
}

/// Render the full Policy Tracer HTML report.
/// Uses `provider` to load capsules; passes `None` to skip capsule section.
pub fn render_policy_tracer_html<C: CapsuleProvider>(provider: Option<&C>) -> String {
    let impacted = match provider {
        Some(p) => get_impacted_capsules(p),
        None => Vec::new(),
    };
    let regs = regulations();

    let mut lines = Vec::new();
    lines.push("<div class=\"policy-tracer\">".to_string());
    lines.push("<h3>🏛️ Policy Impact Tracer</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>\
         Maps AI regulations to affected Gene Pool gaps. \
         Priority weights increase for gap types targeted by new policies.</p>".to_string(),
    );

    // Regulation list
    for (_rid, reg) in &regs {
        lines.push(format!(
            "<div style='border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:10px;border-left:4px solid #D4A055'>\
              <div style='display:flex;justify-content:space-between'>\
                <div style='font-weight:700;font-size:13px'>{}</div>\
                <div style='font-size:11px;color:#A89E8C'>{} · effective {}</div>\
              </div>\
              <div style='font-size:12px;color:#7a7570;margin-top:4px'>Affected: {}</div>\
            </div>",
            html_escape(&reg.name),
            html_escape(&reg.jurisdiction),
            html_escape(&reg.effective_date),
            html_escape(&reg.affected_domains.join(", ")),
        ));
    }

    // Impacted capsules
    lines.push(format!(
        "<h4 style='font-size:13px;font-weight:700;color:#333;margin-top:20px;margin-bottom:10px'>\
         Policy-Impacted Capsules ({})</h4>",
        impacted.len()
    ));

    if impacted.is_empty() {
        lines.push(
            "<p style='color:#A89E8C;font-size:13px'>\
             No capsules directly affected by current regulations.</p>".to_string(),
        );
    } else {
        for cap in impacted.iter().take(10) {
            let boost_pct = (cap.priority_boost * 100.0) as i32;
            lines.push(format!(
                "<div style='display:flex;justify-content:space-between;align-items:center;\
                 padding:8px 12px;background:#f8f4ef;border-radius:4px;margin-bottom:6px'>\
                  <div>\
                    <div style='font-size:12px;font-weight:600;color:#2a2a2a'>{}</div>\
                    <div style='font-size:11px;color:#A89E8C'>{} · {}</div>\
                  </div>\
                  <div style='color:#6BBF8A;font-size:12px;font-weight:700'>+{}% priority</div>\
                </div>",
                html_escape(&cap.gap_title[..cap.gap_title.len().min(55)]),
                html_escape(&cap.gap_type),
                html_escape(&cap.regulation),
                boost_pct,
            ));
        }
    }

    lines.push("<style>.policy-tracer { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubProvider {
        capsules: Vec<HashMap<String, serde_json::Value>>,
    }
    impl CapsuleProvider for StubProvider {
        fn load_capsules(&self) -> Vec<HashMap<String, serde_json::Value>> {
            self.capsules.clone()
        }
    }

    #[test]
    fn test_check_policy_impact_keyword() {
        let mut paper = HashMap::new();
        paper.insert("title".to_string(), serde_json::json!("EU AI Act compliance in NLP systems"));
        paper.insert("abstract".to_string(), serde_json::json!("We study high-risk AI systems under the EU AI Act."));
        paper.insert("categories".to_string(), serde_json::json!(["cs.CL"]));

        let impacts = check_policy_impact(&paper);
        assert!(!impacts.is_empty());
        assert!(impacts.iter().any(|i| i.regulation_id.contains("EU")), "expected an EU regulation impact, got: {:?}", impacts.iter().map(|i| i.regulation_id.clone()).collect::<Vec<_>>());
        assert_eq!(impacts[0].match_reason, "keyword");
    }

    #[test]
    fn test_check_policy_impact_category() {
        let mut paper = HashMap::new();
        paper.insert("title".to_string(), serde_json::json!("Deep Learning for Computer Vision"));
        paper.insert("abstract".to_string(), serde_json::json!("Novel approach to visual recognition."));
        paper.insert("categories".to_string(), serde_json::json!(["cs.CV", "cs.LG"]));

        let impacts = check_policy_impact(&paper);
        // cs.CV matches EU_AI_Act by category
        assert!(!impacts.is_empty());
    }

    #[test]
    fn test_check_policy_impact_no_match() {
        let mut paper = HashMap::new();
        paper.insert("title".to_string(), serde_json::json!("Distributed Systems Consensus"));
        paper.insert("abstract".to_string(), serde_json::json!("Raft consensus algorithm."));
        paper.insert("categories".to_string(), serde_json::json!(["cs.DC"]));

        let impacts = check_policy_impact(&paper);
        assert!(impacts.is_empty());
    }

    #[test]
    fn test_get_impacted_capsules() {
        let capsules = vec![
            {
                let mut c = HashMap::new();
                c.insert("status".to_string(), serde_json::json!("active"));
                c.insert("action_gap_type".to_string(), serde_json::json!("evaluation_gap"));
                c.insert("action_gap_title".to_string(), serde_json::json!("Need better evals for LLMs"));
                c.insert("capsule_id".to_string(), serde_json::json!("c1"));
                c
            },
            {
                let mut c = HashMap::new();
                c.insert("status".to_string(), serde_json::json!(""));
                c.insert("action_gap_type".to_string(), serde_json::json!("evaluation_gap"));
                c.insert("action_gap_title".to_string(), serde_json::json!("Another eval gap"));
                c.insert("capsule_id".to_string(), serde_json::json!("c2"));
                c
            },
        ];
        let provider = StubProvider { capsules };
        let impacted = get_impacted_capsules(&provider);
        // Only status="" or "active" with evaluation_gap
        // c1: active + evaluation_gap → impacted (EU_AI_Act: evaluation_gap +0.3)
        // c2: "" + evaluation_gap → impacted (empty status is included per Python logic)
        assert!(!impacted.is_empty());
        // Should be sorted by priority_boost desc
        for window in impacted.windows(2) {
            assert!(window[0].priority_boost >= window[1].priority_boost);
        }
    }

    #[test]
    fn test_render_html() {
        let capsules = vec![
            {
                let mut c = HashMap::new();
                c.insert("status".to_string(), serde_json::json!("active"));
                c.insert("action_gap_type".to_string(), serde_json::json!("evaluation_gap"));
                c.insert("action_gap_title".to_string(), serde_json::json!("Need better evals"));
                c.insert("capsule_id".to_string(), serde_json::json!("c1"));
                c
            },
        ];
        let provider = StubProvider { capsules };
        let html = render_policy_tracer_html(Some(&provider));
        assert!(html.contains("Policy Impact Tracer"));
        assert!(html.contains("EU AI Act"));
        assert!(html.contains("Need better evals"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<foo>"), "&lt;foo&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }
}
