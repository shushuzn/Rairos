use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

#[derive(serde::Serialize)]
struct ScenarioBranch {
    branch_id: String,
    branch_name: String,
    probability_pct: u8,
    timeframe: String,
    confidence: String,
    narrative: String,
    key_assumptions: Vec<String>,
    trigger_conditions: Vec<String>,
    immediate_consequences: String,
    thirty_day_consequences: String,
    six_month_consequences: String,
    required_response: String,
    what_most_miss: String,
}

fn analyze_scenarios(question: &str, _context: &str) -> Vec<ScenarioBranch> {
    let q_lower = question.to_lowercase();

    let (domain, focus) = if q_lower.contains("drug") || q_lower.contains("therapy") || q_lower.contains("clinical") || q_lower.contains("patient") {
        ("clinical research", "therapeutic efficacy and patient outcomes")
    } else if q_lower.contains("ai") || q_lower.contains("machine learning") || q_lower.contains("model") {
        ("AI/ML technology", "model performance and deployment")
    } else if q_lower.contains("climate") || q_lower.contains("environment") || q_lower.contains("carbon") {
        ("environmental science", "ecological and atmospheric systems")
    } else if q_lower.contains("economic") || q_lower.contains("market") || q_lower.contains("financial") || q_lower.contains("revenue") {
        ("economics/finance", "market dynamics and financial performance")
    } else {
        ("general research", "scientific advancement and knowledge")
    };

    let mut scenarios = Vec::new();

    scenarios.push(ScenarioBranch {
        branch_id: "omega_best".to_string(),
        branch_name: "Best Case".to_string(),
        probability_pct: 15,
        timeframe: "12-24 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Everything aligns optimally. Key assumptions validate, favorable conditions emerge, and the research trajectory exceeds expectations. For {} domain, this means {} accelerates dramatically with strong evidence supporting the primary hypothesis.", domain, focus),
        key_assumptions: vec![
            "All primary hypothesis assumptions hold under scrutiny".to_string(),
            format!("Supporting evidence from {} studies converges", domain),
            "Key stakeholders commit full resources".to_string(),
            "No major competing alternatives emerge".to_string(),
        ],
        trigger_conditions: vec![
            "Early signals exceed baseline thresholds by >2x".to_string(),
            "Key opinion leaders publicly endorse approach".to_string(),
            "Funding secured ahead of schedule".to_string(),
        ],
        immediate_consequences: "Momentum builds rapidly; additional collaborators seek involvement; media coverage amplifies visibility".to_string(),
        thirty_day_consequences: "Expanded team hired; follow-on funding confirmed; competing groups seek partnership".to_string(),
        six_month_consequences: "Full validation complete; manuscript submitted to top venue; industry partnership formalized".to_string(),
        required_response: "Scale resources proportionally; protect intellectual property; maintain quality under acceleration pressure".to_string(),
        what_most_miss: "Success attracts detractors and creates unrealistic expectations that can derail when normal setbacks occur".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "alpha_likely".to_string(),
        branch_name: "Likely Case".to_string(),
        probability_pct: 45,
        timeframe: "6-18 months".to_string(),
        confidence: "HIGH".to_string(),
        narrative: format!("The most probable path materializes. Core hypothesis holds with moderate effect sizes, methodology proves sound, and incremental progress continues. For {} domain, this means {} develops as expected with no major surprises.", domain, focus),
        key_assumptions: vec![
            format!("Core {} assumptions remain valid", domain),
            "Methodology produces reproducible results".to_string(),
            "Resource levels remain stable".to_string(),
            "No paradigm-shifting alternatives appear".to_string(),
        ],
        trigger_conditions: vec![
            "Results fall within expected confidence intervals".to_string(),
            "Peer feedback confirms approach validity".to_string(),
            "Publication timeline proceeds as planned".to_string(),
        ],
        immediate_consequences: "Steady progress; team maintains course; stakeholders remain engaged".to_string(),
        thirty_day_consequences: "Draft manuscript completed; internal review conducted; next-phase planning begins".to_string(),
        six_month_consequences: "Publication submitted; follow-on proposal drafted; core methodology established as standard".to_string(),
        required_response: "Maintain rigorous standards; document all findings thoroughly; build on incremental wins".to_string(),
        what_most_miss: "Incremental progress can mask underlying structural weaknesses that only appear under stress".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "delta_worst".to_string(),
        branch_name: "Worst Case".to_string(),
        probability_pct: 20,
        timeframe: "3-12 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Multiple assumptions fail simultaneously. Key results do not replicate, methodology flaws emerge, and resource constraints force difficult choices. For {} domain, this means {} stalls and credibility suffers.", domain, focus),
        key_assumptions: vec![
            format!("{} hypothesis is fundamentally sound", domain),
            "Sufficient sample size achievable".to_string(),
            "Data quality meets standards".to_string(),
            "Timeline is achievable".to_string(),
        ],
        trigger_conditions: vec![
            "Primary outcome measure shows null or negative effect".to_string(),
            "Replication attempt fails".to_string(),
            "Funding source signals concern".to_string(),
        ],
        immediate_consequences: "Stakeholder confidence erodes; team morale impacted; timeline slips".to_string(),
        thirty_day_consequences: "Emergency re-evaluation; methodology audit; potential pivot or termination".to_string(),
        six_month_consequences: "Project restructured or cancelled; lessons documented; team reassigned".to_string(),
        required_response: "Conduct honest post-mortem; preserve useful learnings; rebuild trust through transparency".to_string(),
        what_most_miss: "The specific failure mode contains information about what WOULD work — but only if analyzed objectively".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "psi_wildcard".to_string(),
        branch_name: "Wild Card".to_string(),
        probability_pct: 8,
        timeframe: "1-24 months (unpredictable)".to_string(),
        confidence: "LOW".to_string(),
        narrative: format!("An unexpected variable enters that nobody anticipated. This black swan event reshapes the landscape entirely. For {} domain, this could mean a breakthrough discovery, a safety crisis, or a disruptive technology renders current approach obsolete.", domain),
        key_assumptions: vec![
            "Current understanding captures relevant variables".to_string(),
            "No external disruption possible".to_string(),
            "Timeline is within predictable window".to_string(),
        ],
        trigger_conditions: vec![
            "Unexpected result defies all models".to_string(),
            "External event creates sudden change".to_string(),
            "Competing breakthrough announced".to_string(),
        ],
        immediate_consequences: "Complete reorientation required; existing plans become irrelevant".to_string(),
        thirty_day_consequences: "New strategy formulated; team restructured; stakeholders reassessed".to_string(),
        six_month_consequences: "Either pivot fully executed or graceful exit completed".to_string(),
        required_response: "Build organizational agility; maintain optionality; avoid overcommitment to single path".to_string(),
        what_most_miss: "Wild cards are only wild in retrospect — early signals always existed for those paying attention".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "phi_contrarian".to_string(),
        branch_name: "Contrarian Case".to_string(),
        probability_pct: 7,
        timeframe: "12-36 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("The opposite of conventional wisdom proves true. The consensus view that {} is the right approach turns out to be wrong. This creates both risk for current efforts and opportunity for those who anticipate the shift.", domain),
        key_assumptions: vec![
            format!("Consensus view on {} is correct", domain),
            "Current evidence supports prevailing theory".to_string(),
            "Established methods are optimal".to_string(),
        ],
        trigger_conditions: vec![
            "Minority view gains unexpected support".to_string(),
            "Contrarian data published to acclaim".to_string(),
            "Regulatory or funder stance shifts".to_string(),
        ],
        immediate_consequences: "Current approach questioned; early adopters of contrarian view gain advantage".to_string(),
        thirty_day_consequences: "Debate intensifies; funding bodies reconsider; team must decide on pivot".to_string(),
        six_month_consequences: "Field reorients around new consensus; late movers face disadvantage".to_string(),
        required_response: "Monitor contrarian signals actively; avoid dismissing minority views prematurely".to_string(),
        what_most_miss: "The contrarian view is right more often than expected — and at the exact moment when admitting it feels like the biggest risk".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "infinity_second".to_string(),
        branch_name: "Second Order".to_string(),
        probability_pct: 5,
        timeframe: "12-48 months".to_string(),
        confidence: "LOW".to_string(),
        narrative: "First-order effects trigger cascading consequences that nobody predicted. Initial success (or failure) creates chain reaction of unintended outcomes. The secondary consequences prove more significant than the primary event itself.".to_string(),
        key_assumptions: vec![
            "First-order effects remain contained".to_string(),
            "No significant side effects emerge".to_string(),
            "System remains in equilibrium".to_string(),
        ],
        trigger_conditions: vec![
            "Success attracts attention from unexpected quarters".to_string(),
            "Scaling reveals hidden complexities".to_string(),
            "Unintended consequences begin appearing".to_string(),
        ],
        immediate_consequences: "Focus shifts from primary goal to managing cascades".to_string(),
        thirty_day_consequences: "Second-order stakeholders demand input; resource reallocation required".to_string(),
        six_month_consequences: "Ecosystem around the work has formed; original team has limited control".to_string(),
        required_response: "Map potential second-order effects proactively; build governance mechanisms early".to_string(),
        what_most_miss: "The most important consequences of any action are always the ones you didn't think to look for".to_string(),
    });

    scenarios
}

pub struct WhatIfOracleHandler;

#[async_trait]
impl ToolHandler for WhatIfOracleHandler {
    fn name(&self) -> &str { "what_if_oracle" }
    fn description(&self) -> &str { "Explore multi-branch scenario analysis for a research question using the What-If Oracle framework (0·IF·1). Generates 6 scenario branches: Best, Likely, Worst, Wild Card, Contrarian, and Second Order cases." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("question".into(), ToolProperty::string("Research question to analyze (e.g., 'What if we discover a safe and effective CRISPR therapy for sickle cell disease within 5 years?')")),
                ("context".into(), ToolProperty::string("Additional context or constraints (e.g., 'Current funding: $2M, Timeline: 3 years, Team: 5 researchers')")),
                ("mode".into(), ToolProperty::string("Analysis mode: 'quick' (3 branches) or 'deep' (6 branches, default: deep)")),
            ].into_iter().collect(),
            vec!["question".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let question = params["question"].as_str().ok_or("Missing question")?;
        let context = params.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("deep");

        let all_scenarios = analyze_scenarios(question, context);

        let scenarios: Vec<ScenarioBranch> = if mode == "quick" {
            all_scenarios.into_iter().filter(|s| {
                s.branch_id == "omega_best" || s.branch_id == "alpha_likely" || s.branch_id == "delta_worst"
            }).collect()
        } else {
            all_scenarios
        };

        let total_pct: u8 = scenarios.iter().map(|s| s.probability_pct).sum();

        let branches_json: Vec<Value> = scenarios.iter().map(|s| {
            serde_json::json!({
                "branch_id": s.branch_id,
                "branch_name": s.branch_name,
                "probability_pct": s.probability_pct,
                "timeframe": s.timeframe,
                "confidence": s.confidence,
                "narrative": s.narrative,
                "key_assumptions": s.key_assumptions,
                "trigger_conditions": s.trigger_conditions,
                "consequences": {
                    "immediate": s.immediate_consequences,
                    "thirty_day": s.thirty_day_consequences,
                    "six_month": s.six_month_consequences,
                },
                "required_response": s.required_response,
                "what_most_miss": s.what_most_miss,
            })
        }).collect();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let mut synthesis = String::new();
        synthesis.push_str("## Probability Distribution\n\n");
        synthesis.push_str("| Scenario | Probability |\n");
        synthesis.push_str("|----------|-------------|\n");
        for s in &scenarios {
            let bar = "█".repeat((s.probability_pct / 5) as usize);
            let dots = "░".repeat(20 - (s.probability_pct / 5) as usize);
            synthesis.push_str(&format!("| {} {} | {}% |\n", bar, dots, s.probability_pct));
        }
        synthesis.push_str(&format!("\n**Total probability:** {}% (normalized)\n\n", total_pct));

        synthesis.push_str("## Synthesis\n\n");
        synthesis.push_str("**Robust Actions** (beneficial across multiple branches):\n\n");
        synthesis.push_str("1. Maintain methodological rigor regardless of early results\n");
        synthesis.push_str("2. Build in flexibility to pivot when evidence demands\n");
        synthesis.push_str("3. Document all findings thoroughly for future reference\n");
        synthesis.push_str("4. Cultivate stakeholder relationships beyond the primary funder\n\n");

        synthesis.push_str("**Hedge Actions** (protect against worst case without sacrificing upside):\n\n");
        synthesis.push_str("1. Establish replication protocol before primary analysis\n");
        synthesis.push_str("2. Maintain runway for at least 6 months beyond planned end\n");
        synthesis.push_str("3. Develop contingency plans for both null and positive results\n\n");

        synthesis.push_str("**Decision Triggers** (signals to update branch probabilities):\n\n");
        synthesis.push_str("- Increase Best Case probability if: early results exceed thresholds, key endorsements received\n");
        synthesis.push_str("- Increase Worst Case probability if: replication fails, funding signals concern\n");
        synthesis.push_str("- Increase Wild Card probability if: results defy all models, external disruption occurs\n");
        synthesis.push_str("- Increase Contrarian probability if: minority view gains unexpected traction\n\n");

        synthesis.push_str("**The 1% Insight**\n\n");
        synthesis.push_str(&format!(
            "The most actionable insight from this scenario analysis: the specific branch that feels least comfortable to plan for is often the one that contains the highest learning potential. For the question '{}', the gap between what the analysis reveals and what you immediately want to act on is where the real strategic value lies.\n\n",
            question.chars().take(60).collect::<String>()
        ));

        synthesis.push_str(&format!("_Analyzed: {} | Mode: {} | Branches: {}_\n", today, mode, scenarios.len()));

        Ok(serde_json::json!({
            "question": question,
            "context": context,
            "mode": mode,
            "branches": branches_json,
            "synthesis": synthesis,
            "total_probability_pct": total_pct,
            "date": today,
        }))
    }
}
