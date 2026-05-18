use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rairos_core::Database;
use rairos_experiment_tracker::ExperimentTracker;
use serde_json::Value;
use std::collections::HashMap;

fn parse_stance_type(s: &str) -> Result<rairos_research_memory::StanceType, String> {
    match s.to_lowercase().as_str() {
        "supported" => Ok(rairos_research_memory::StanceType::Supported),
        "rejected" => Ok(rairos_research_memory::StanceType::Rejected),
        "deferred" => Ok(rairos_research_memory::StanceType::Deferred),
        "qualified" => Ok(rairos_research_memory::StanceType::Qualified),
        _ => Err(format!("Invalid stance type: '{}' — expected supported/rejected/deferred/qualified", s)),
    }
}

fn research_memory_add_stance_impl(memory: &mut rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let topic = params.get("topic").and_then(|v| v.as_str()).ok_or("Missing topic")?;
    let claim = params.get("claim").and_then(|v| v.as_str()).ok_or("Missing claim")?;
    let stance_str = params.get("stance").and_then(|v| v.as_str()).ok_or("Missing stance")?;
    let stance = parse_stance_type(stance_str)?;
    let evidence_refs: Vec<String> = params.get("evidence_refs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let reasoning = params.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
    let confidence = params.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let tags: Vec<String> = params.get("tags")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");

    let s = memory.add_stance(topic, claim, stance, evidence_refs, reasoning, confidence, tags, notes);
    Ok(serde_json::json!({
        "stance_id": s.stance_id,
        "topic": s.topic,
        "claim": s.claim,
        "stance": s.stance.to_string(),
        "confidence": s.confidence,
        "message": "Stance recorded",
    }))
}

fn research_memory_list_stances_impl(memory: &rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let topic = params.get("topic").and_then(|v| v.as_str());
    let stances = memory.get_stances(topic, None);
    Ok(serde_json::json!({ "stances": stances, "total": stances.len() }))
}

fn research_memory_check_paper_impl(memory: &mut rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str()).ok_or("Missing arxiv_id")?;
    let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let claim = params.get("claim").and_then(|v| v.as_str()).unwrap_or("");
    let mut paper = std::collections::HashMap::new();
    paper.insert("arxiv_id".to_string(), arxiv_id.to_string());
    paper.insert("title".to_string(), title.to_string());
    paper.insert("claim".to_string(), claim.to_string());
    let anomalies = memory.check_paper_against_stances(&paper, false, None, None, None);
    Ok(serde_json::json!({ "anomalies": anomalies, "total": anomalies.len() }))
}

fn research_memory_anomalies_impl(memory: &rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let anomalies = memory.get_recent_anomalies(limit);
    Ok(serde_json::json!({ "anomalies": anomalies, "total": anomalies.len() }))
}

macro_rules! make_research_memory_handler {
    ($name:ident, $tool_name:expr, $desc:expr, $impl_fn:ident) => {
        pub struct $name;
        #[async_trait]
        impl ToolHandler for $name {
            fn name(&self) -> &str { $tool_name }
            fn description(&self) -> &str { $desc }
            fn input_schema(&self) -> ToolInputSchema {
                ToolInputSchema::object(
                    vec![
                        ("topic".into(), ToolProperty::string("Research topic")),
                        ("claim".into(), ToolProperty::string("The claim being evaluated")),
                        ("stance".into(), ToolProperty::string("Stance: supported/rejected/deferred/qualified")),
                        ("evidence_refs".into(), ToolProperty::string("JSON array of evidence references")),
                        ("reasoning".into(), ToolProperty::string("Reasoning behind the stance")),
                        ("confidence".into(), ToolProperty::string("Confidence score 0.0-1.0")),
                        ("arxiv_id".into(), ToolProperty::string("arXiv ID for check_paper")),
                        ("limit".into(), ToolProperty::integer("Max results (default 20)")),
                    ].into_iter().collect(),
                    vec![],
                )
            }
            async fn call(&self, params: Value) -> Result<Value, String> {
                let mut memory = rairos_research_memory::ResearchMemory::new();
                $impl_fn(&mut memory, &params)
            }
        }
    };
}

make_research_memory_handler!(ResearchMemoryAddStanceHandler, "research_memory_add_stance", "Record a new research stance", research_memory_add_stance_impl);
make_research_memory_handler!(ResearchMemoryListStancesHandler, "research_memory_list_stances", "List all research stances", research_memory_list_stances_impl);
make_research_memory_handler!(ResearchMemoryCheckPaperHandler, "research_memory_check_paper", "Check a paper against prior research stances", research_memory_check_paper_impl);
make_research_memory_handler!(ResearchMemoryAnomaliesHandler, "research_memory_anomalies", "List recent research memory anomalies", research_memory_anomalies_impl);

pub struct LeaderboardHandler;

#[async_trait]
impl ToolHandler for LeaderboardHandler {
    fn name(&self) -> &str { "leaderboard" }
    fn description(&self) -> &str { "Benchmark Leaderboard: ranked paper2code implementations by pass_rate + coverage" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, rankings, entry (default: status)")),
                ("arxiv_id".into(), ToolProperty::string("arXiv ID for entry action")),
                ("sort_by".into(), ToolProperty::string("Sort: combined, pass_rate, coverage (default: combined)")),
                ("limit".into(), ToolProperty::integer("Max results (default: 20)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str());
        let sort_by = params.get("sort_by").and_then(|v| v.as_str()).unwrap_or("combined");
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        Ok(rairos_leaderboard::leaderboard_action(action, arxiv_id, sort_by, limit))
    }
}

pub struct ImpactLeaderboardHandler;

#[async_trait]
impl ToolHandler for ImpactLeaderboardHandler {
    fn name(&self) -> &str { "impact_leaderboard" }
    fn description(&self) -> &str { "Get overall impact leaderboard from local database" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("limit".into(), ToolProperty::integer("Max results (default: 20)")),
                ("year_min".into(), ToolProperty::integer("Minimum year (default: 2020)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let _year_min = params.get("year_min").and_then(|v| v.as_u64()).unwrap_or(2020) as i32;
        Ok(rairos_leaderboard::leaderboard_action("rankings", None, "combined", limit))
    }
}

pub struct ClaimGraphHandler;

#[async_trait]
impl ToolHandler for ClaimGraphHandler {
    fn name(&self) -> &str { "claim_graph" }
    fn description(&self) -> &str { "Cross-paper numerical claim tracking with contradiction detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, add_claim, add_edge, contradictions (default: status)")),
                ("paper_id".into(), ToolProperty::string("Paper ID for add_claim")),
                ("claim_type".into(), ToolProperty::string("Claim type: accuracy, efficiency, scalability, etc.")),
                ("value".into(), ToolProperty::string("Numeric value of the claim")),
                ("source_text".into(), ToolProperty::string("Source text for the claim")),
                ("from_paper".into(), ToolProperty::string("Source paper ID for edge")),
                ("to_paper".into(), ToolProperty::string("Target paper ID for edge")),
                ("improvement_ratio".into(), ToolProperty::string("Improvement ratio for improvement edges")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let paper_id = params.get("paper_id").and_then(|v| v.as_str());
        let claim_type = params.get("claim_type").and_then(|v| v.as_str());
        let value = params.get("value").and_then(|v| v.as_f64());
        let source_text = params.get("source_text").and_then(|v| v.as_str());
        let from_paper = params.get("from_paper").and_then(|v| v.as_str());
        let to_paper = params.get("to_paper").and_then(|v| v.as_str());
        let improvement_ratio = params.get("improvement_ratio").and_then(|v| v.as_f64());
        Ok(rairos_claimgraph::claim_graph_action(
            action, paper_id, claim_type, value, source_text, from_paper, to_paper, improvement_ratio,
        ))
    }
}

pub struct TagAllHandler;

#[async_trait]
impl ToolHandler for TagAllHandler {
    fn name(&self) -> &str { "tag_all" }
    fn description(&self) -> &str { "List all tags in the system from the database" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;
        let tags = db.list_tags().map_err(|e| format!("List tags error: {}", e))?;
        let names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
        Ok(serde_json::json!({"tags": names, "count": names.len()}))
    }
}

pub struct ReviewListHandler;

#[async_trait]
impl ToolHandler for ReviewListHandler {
    fn name(&self) -> &str { "review_list" }
    fn description(&self) -> &str { "List saved simulated reviews" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let reviews = rairos_review_simulator::list_reviews(20);
        Ok(serde_json::json!({"reviews": reviews, "count": reviews.len()}))
    }
}

pub struct ExperimentRecordHandler;

#[async_trait]
impl ToolHandler for ExperimentRecordHandler {
    fn name(&self) -> &str { "experiment_record" }
    fn description(&self) -> &str { "Record an experiment result for a hypothesis — also updates the GenePool capsule's success score when completed" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("hypothesis_id".into(), ToolProperty::string("ID of the hypothesis")),
                ("name".into(), ToolProperty::string("Name of the experiment")),
                ("result".into(), ToolProperty::string("Result: validated, rejected, failed, running, or completed")),
                ("metrics".into(), ToolProperty::string("Optional JSON object of metrics")),
            ].into_iter().collect(),
            vec!["hypothesis_id".into(), "name".into(), "result".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let hypothesis_id = params["hypothesis_id"].as_str().ok_or("Missing hypothesis_id")?;
        let name = params["name"].as_str().ok_or("Missing name")?;
        let result = params["result"].as_str().ok_or("Missing result")?;

        let tracker = ExperimentTracker::new(None);
        let exp = tracker.run(name, "", "", hypothesis_id, None, None);

        let metrics: Option<serde_json::Value> = params.get("metrics")
            .and_then(|v| {
                if v.is_string() { serde_json::from_str(v.as_str()?).ok() } else { Some(v.clone()) }
            });

        match result.to_lowercase().as_str() {
            "rejected" | "failed" => {
                tracker.fail(&exp.id, result);
                let pool = rairos_research::gene_pool::GenePool::new();
                let _ = pool.update_capsule_by_hypothesis_id(hypothesis_id, 0.2, 1);
            }
            _ => {
                let mut results = HashMap::new();
                results.insert("verdict".to_string(), serde_json::json!(result));
                if let Some(m) = metrics {
                    results.insert("metrics".to_string(), m);
                }
                tracker.complete(&exp.id, Some(results));

                let pool = rairos_research::gene_pool::GenePool::new();
                let score = match result.to_lowercase().as_str() {
                    "validated" => 0.9,
                    "completed" => 0.8,
                    _ => 0.6,
                };
                let _ = pool.update_capsule_by_hypothesis_id(hypothesis_id, score, 1);
            }
        }

        Ok(serde_json::json!({
            "experiment_id": exp.id,
            "hypothesis_id": hypothesis_id,
            "status": if matches!(result.to_lowercase().as_str(), "rejected" | "failed") { "failed" } else { "completed" },
            "message": format!("Experiment recorded: {} -> {}. GenePool capsule updated.", name, result),
        }))
    }
}

pub struct LitReviewListHandler;

#[async_trait]
impl ToolHandler for LitReviewListHandler {
    fn name(&self) -> &str { "litreview_list" }
    fn description(&self) -> &str { "List all saved literature reviews" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let cwd = std::env::current_dir().map_err(|e| format!("CWD error: {}", e))?;
        let reviews_dir = cwd.join("data").join("litreviews");
        let mut reviews = Vec::new();

        if reviews_dir.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&reviews_dir)
                .map_err(|e| format!("Read dir error: {}", e))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let n = name.to_string_lossy();
                    n.starts_with("litreview_") && n.ends_with(".md")
                })
                .collect();
            entries.sort_by(|a, b| {
                let a_m = a.metadata().ok().and_then(|m| m.modified().ok());
                let b_m = b.metadata().ok().and_then(|m| m.modified().ok());
                b_m.cmp(&a_m)
            });
            for entry in entries.iter().take(20) {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let lines: Vec<&str> = content.lines().collect();
                let title = lines.first()
                    .map(|l| l.trim_start_matches("# ").trim().to_string())
                    .unwrap_or_else(|| {
                        entry.file_name().to_string_lossy().replace(".md", "")
                    });
                let mut date = String::new();
                for line in lines.iter().skip(1).take(4) {
                    if let Some(pos) = line.find("Generated:") {
                        date = line[pos + 10..].trim().to_string();
                        break;
                    }
                }
                let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                reviews.push(serde_json::json!({
                    "filename": entry.file_name().to_string_lossy(),
                    "topic": title,
                    "date": date,
                    "size_bytes": size,
                }));
            }
        }

        Ok(serde_json::json!({"reviews": reviews, "count": reviews.len()}))
    }
}

pub struct ReviewSimulateHandler;

#[async_trait]
impl ToolHandler for ReviewSimulateHandler {
    fn name(&self) -> &str { "review_simulate" }
    fn description(&self) -> &str { "Simulate adversarial peer reviewers on a paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper to review")),
                ("persona".into(), ToolProperty::string("Reviewer persona (e.g. 'methodologist', 'all' for consensus)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let persona = params.get("persona").and_then(|v| v.as_str()).unwrap_or("all");

        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;
        let paper = db.get_paper_by_arxiv(arxiv_id)
            .map_err(|e| format!("DB query error: {}", e))?
            .ok_or_else(|| format!("Paper not found: {}", arxiv_id))?;

        let full_text = format!("{}\n\n{}", paper.title, paper.abstract_text);

        let simulator = rairos_review_simulator::ReviewSimulator::new();
        let review = if persona != "all" {
            let personas = rairos_review_simulator::default_personas();
            let selected = personas.into_iter().find(|p| {
                p.name.to_lowercase().starts_with(&persona.to_lowercase())
            }).ok_or_else(|| format!("Unknown persona: {}", persona))?;
            simulator.review(&full_text, Some(&paper.title), Some(&selected), None, None, None).await
                .map_err(|e| format!("Review error: {}", e))?
        } else {
            simulator.review(&full_text, Some(&paper.title), None, None, None, None).await
                .map_err(|e| format!("Review error: {}", e))?
        };

        rairos_review_simulator::save_review(&review);

        Ok(serde_json::json!({
            "review_id": review.review_id,
            "persona": review.persona,
            "overall_score": review.overall_score,
            "summary": review.summary,
            "strengths": review.strengths,
            "weaknesses": review.weaknesses,
            "recommendation": review.recommendation,
            "annotation_count": review.annotations.len(),
        }))
    }
}
