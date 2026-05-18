use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::{llm_client, llm_model};
use async_trait::async_trait;
use rairos_experiment_tracker::ExperimentTracker;
use rairos_llm::LlmClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TopicDiscoveryHandler;

#[async_trait]
impl ToolHandler for TopicDiscoveryHandler {
    fn name(&self) -> &str { "topic_discovery" }
    fn description(&self) -> &str { "Suggest new arXiv subscription topics from research gaps and recent papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("recent_gaps".into(), ToolProperty::string("Recent gap objects as JSON array")),
                ("recent_papers".into(), ToolProperty::string("Recent paper objects with title/abstract as JSON array")),
                ("gap_clusters".into(), ToolProperty::string("Gap cluster objects as JSON array")),
                ("gap_trends".into(), ToolProperty::string("Gap type trend map as JSON object {{type: 'rising'|'stable'|'declining'}}")),
                ("max_suggestions".into(), ToolProperty::integer("Maximum suggestions to return (default 10)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let recent_gaps: Vec<Value> = params.get("recent_gaps")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok())
            .unwrap_or_default();
        let recent_papers: Vec<Value> = params.get("recent_papers")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok())
            .unwrap_or_default();
        let gap_clusters: Vec<Value> = params.get("gap_clusters")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok())
            .unwrap_or_default();
        let gap_trends: std::collections::HashMap<String, String> = params.get("gap_trends")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<std::collections::HashMap<String, String>>(s).ok())
            .unwrap_or_default();
        let max_suggestions = params.get("max_suggestions")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let discoverer = rairos_topic_discovery::TopicDiscoverer::new();
        let suggestions = discoverer.suggest_new_topics(
            &recent_gaps, &recent_papers, &gap_clusters, &gap_trends, max_suggestions,
        );

        let results: Vec<Value> = suggestions.into_iter().map(|s| {
            serde_json::json!({
                "topic": s.topic,
                "source": s.source,
                "confidence": s.confidence,
                "reason": s.reason,
                "gap_type": s.gap_type,
                "keywords": s.keywords,
            })
        }).collect();

        Ok(serde_json::json!({
            "content": [{"type": "text", "text": serde_json::to_string_pretty(&results).unwrap_or_default()}],
            "suggestions": results,
            "count": results.len(),
        }))
    }
}

pub struct OrchestratorRunCycleHandler;

#[async_trait]
impl ToolHandler for OrchestratorRunCycleHandler {
    fn name(&self) -> &str { "orchestrator_run_cycle" }
    fn description(&self) -> &str { "Run one autonomous research cycle — check subscriptions, detect gaps, generate alerts" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("interval_minutes".into(), ToolProperty::integer("Check interval in minutes (default 30)")),
                ("min_gap_severity".into(), ToolProperty::string("Minimum gap severity for alert (LOW/MEDIUM/HIGH, default MEDIUM)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let interval = params.get("interval_minutes").and_then(|v| v.as_i64()).unwrap_or(30) as i32;
        let min_severity = params.get("min_gap_severity")
            .and_then(|v| v.as_str())
            .unwrap_or("MEDIUM")
            .to_string();

        let config = rairos_orchestrator::OrchestratorConfig {
            interval_minutes: interval,
            min_gap_severity_for_alert: min_severity,
            ..Default::default()
        };

        let orchestrator = rairos_orchestrator::AutonomousOrchestrator::new(config, false);
        let alerts = orchestrator.run_cycle().await
            .map_err(|e| format!("Orchestrator cycle failed: {}", e))?;

        let alert_list: Vec<Value> = alerts.iter().map(|a| {
            serde_json::json!({
                "alert_id": a.alert_id,
                "topic": a.topic,
                "triggered_by": a.triggered_by,
                "trigger_title": a.trigger_title,
                "gaps_found": a.gaps_found,
                "top_gap_title": a.top_gap_title,
                "top_gap_type": a.top_gap_type,
                "severity": a.severity,
                "gene_pool_score": a.gene_pool_score,
            })
        }).collect();

        Ok(serde_json::json!({
            "content": [{"type": "text", "text": serde_json::to_string_pretty(&alert_list).unwrap_or_default()}],
            "alerts": alert_list,
            "alert_count": alert_list.len(),
        }))
    }
}

pub struct DeepResearchRunHandler;

#[async_trait]
impl ToolHandler for DeepResearchRunHandler {
    fn name(&self) -> &str { "deep_research_run" }
    fn description(&self) -> &str { "Run deep research agent on a topic — searches arXiv, detects gaps, generates insights" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Research query or topic")),
                ("max_iterations".into(), ToolProperty::integer("Maximum research iterations (default 3)")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing query")?.to_string();
        let max_iterations = params.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let result = tokio::task::spawn_blocking(move || {
            let config = rairos_deep_research::DeepResearchConfig {
                query,
                max_iterations,
                ..Default::default()
            };
            let mut agent = rairos_deep_research::DeepResearchAgent::new(config);
            agent.run()
        }).await
            .map_err(|e| format!("Spawn error: {}", e))?
            .map_err(|e| format!("Deep research failed: {}", e))?;

        Ok(serde_json::json!({
            "content": [{"type": "text", "text": result.report}],
            "session_id": result.session_id,
            "iterations": result.iterations,
            "papers_found": result.papers.len(),
            "gaps_found": result.gaps.len(),
            "thoughts_count": result.thoughts.len(),
            "duration_seconds": result.duration_seconds,
            "report_preview": result.report.chars().take(500).collect::<String>(),
        }))
    }
}

pub struct ParallelResearchRunHandler;

#[async_trait]
impl ToolHandler for ParallelResearchRunHandler {
    fn name(&self) -> &str { "parallel_research_run" }
    fn description(&self) -> &str { "Run parallel deep research across multiple gap clusters — each cluster gets an independent agent" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Overall research topic")),
                ("gap_clusters".into(), ToolProperty::string("JSON array of gap clusters: [{cluster_id, gaps: [{title, gap_type, novelty_score}], gap_type, keywords}]")),
                ("max_concurrency".into(), ToolProperty::integer("Max parallel agents (default 3)")),
                ("max_iterations".into(), ToolProperty::integer("Max iterations per agent (default 2)")),
            ].into_iter().collect(),
            vec!["topic".into(), "gap_clusters".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let clusters_str = params["gap_clusters"].as_str().ok_or("Missing gap_clusters")?;
        let max_concurrency = params.get("max_concurrency").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let max_iterations = params.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

        let gap_clusters: Vec<rairos_parallel_research::GapCluster> =
            serde_json::from_str(clusters_str)
                .map_err(|e| format!("Invalid gap_clusters JSON: {}", e))?;

        let coordinator = rairos_parallel_research::ParallelResearchCoordinator::new(
            max_concurrency,
            max_iterations,
            300,
        );

        let orchestrator: Arc<dyn rairos_parallel_research::Orchestrator> =
            Arc::new(rairos_parallel_research::DeepResearchOrchestrator);

        let result = coordinator.run(topic, gap_clusters, None, orchestrator).await;

        Ok(serde_json::json!(result))
    }
}

pub struct ResearchRunHandler;

#[async_trait]
impl ToolHandler for ResearchRunHandler {
    fn name(&self) -> &str { "research_run" }
    fn description(&self) -> &str { "Search arXiv, save papers to DB, generate report" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic to search for")),
                ("limit".into(), ToolProperty::integer("Maximum results (default 5, max 20)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let limit = (params["limit"].as_u64().unwrap_or(5) as usize).min(20);

        let arxiv_url = "http://export.arxiv.org/api/query";
        let url = format!("{}?search_query=all:{}&start=0&max_results={}", arxiv_url, topic.replace(' ', "+"), limit);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = crate::handlers::parse_arxiv_response(&text);

        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = rairos_core::Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;

        let mut saved = 0;
        for p in &papers {
            let arxiv_id = p["arxiv_id"].as_str().unwrap_or("");
            if arxiv_id.is_empty() { continue; }
            let title = p["title"].as_str().unwrap_or("");
            let abstract_text = p["abstract"].as_str().unwrap_or("");
            let authors: Vec<String> = p["authors"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let categories: Vec<String> = p["categories"].as_array()
                .map(|c| c.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let paper = rairos_core::Paper::with_metadata(
                Some(arxiv_id.to_string()),
                title.to_string(),
                abstract_text.to_string(),
                authors,
                categories,
                rairos_core::PaperMetadata::default(),
            );
            if db.insert_paper(&paper).is_ok() {
                saved += 1;
            }
        }

        Ok(serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "papers_saved": saved,
            "status": "completed",
        }))
    }
}

pub struct HypothesisGenerateHandler;

#[async_trait]
impl ToolHandler for HypothesisGenerateHandler {
    fn name(&self) -> &str { "hypothesis_generate" }
    fn description(&self) -> &str { "Generate testable research hypotheses from topic + gap context with KG paper context, Gene Pool prior success/failure signal, experiment designs, risk assessment, and scoring" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("gap_context".into(), ToolProperty::string("Context from gap detection (optional)")),
                ("gap_type".into(), ToolProperty::string("Type of gap (optional, auto-detected from context)")),
                ("creative".into(), ToolProperty::string("Generate creative cross-domain hypotheses (true/false, default false)")),
                ("submit_to_genepool".into(), ToolProperty::string("Auto-submit high-scoring hypotheses to GenePool (true/false, default true)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_context = params.get("gap_context").and_then(|v| v.as_str()).unwrap_or("");
        let creative = params.get("creative").and_then(|v| v.as_str()).unwrap_or("false") == "true";
        let auto_submit = params.get("submit_to_genepool").and_then(|v| v.as_str()).unwrap_or("true") == "true";

        if let Some(client) = llm_client() {
            let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
            let result = gen.generate_llm(
                client, llm_model(), topic, gap_context, creative, auto_submit,
            ).await;
            return Ok(serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({
                "topic": topic, "summary": "Error serializing result", "hypotheses": []
            })));
        }

        let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
        let result = gen.generate(topic, gap_context, creative);
        Ok(serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({
            "topic": topic, "summary": "Error serializing result", "hypotheses": []
        })))
    }
}

fn compute_hypothesis_verdict(experiments: &[&rairos_experiment_tracker::Experiment]) -> (String, String) {
    if experiments.is_empty() {
        return ("INCONCLUSIVE".into(), "no experiments recorded".into());
    }

    let has_validated = experiments.iter().any(|e| {
        e.status == "completed"
            && e.results.get("verdict")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == "validated")
    });
    let has_rejected = experiments.iter().any(|e| e.status == "failed");

    if has_validated && has_rejected {
        ("MIXED".into(), "both validated and rejected experiments exist".into())
    } else if has_validated {
        ("VALIDATED".into(), "all experiments succeeded".into())
    } else if has_rejected {
        ("REJECTED".into(), "all experiments failed".into())
    } else {
        ("INCONCLUSIVE".into(), "no completed experiments yet".into())
    }
}

pub struct HypothesisListHandler;

#[async_trait]
impl ToolHandler for HypothesisListHandler {
    fn name(&self) -> &str { "hypothesis_list" }
    fn description(&self) -> &str { "List all tracked hypotheses with verdict status, success scores, and linked experiments" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("status".into(), ToolProperty::string("Optional filter: validated, rejected, mixed, inconclusive")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let status_filter = params.get("status").and_then(|v| v.as_str());

        let pool = rairos_research::gene_pool::GenePool::new();
        let all_capsules = pool.load_capsules();
        let hypothesis_capsules: Vec<_> = all_capsules.iter()
            .filter(|c| !c.hypothesis_id.is_empty())
            .collect();

        let tracker = ExperimentTracker::new(None);
        let all_experiments = tracker.list_experiments(None, None, None);

        let mut exp_by_hid: HashMap<&str, Vec<&rairos_experiment_tracker::Experiment>> = HashMap::new();
        for exp in &all_experiments {
            if !exp.hypothesis_id.is_empty() {
                exp_by_hid.entry(exp.hypothesis_id.as_str())
                    .or_default()
                    .push(exp);
            }
        }

        let mut rows: Vec<serde_json::Value> = Vec::new();
        for cap in &hypothesis_capsules {
            let hid = &cap.hypothesis_id;
            let experiments = exp_by_hid.get(hid.as_str()).cloned().unwrap_or_default();

            let (verdict, detail) = compute_hypothesis_verdict(&experiments);
            let linked_count = experiments.len();

            if let Some(filter) = status_filter {
                if filter.to_uppercase() != verdict { continue; }
            }

            rows.push(serde_json::json!({
                "hypothesis_id": hid,
                "title": cap.action_gap_title,
                "gap_type": cap.action_gap_type,
                "success_score": cap.outcome_success_score,
                "feedback_count": cap.feedback_count,
                "created_at": cap.created_at,
                "verdict": verdict,
                "detail": detail,
                "linked_experiments": linked_count,
                "experiments": experiments.iter().map(|e| serde_json::json!({
                    "id": e.id,
                    "name": e.name,
                    "status": e.status,
                })).collect::<Vec<_>>(),
            }));
        }

        Ok(serde_json::json!({
            "total": rows.len(),
            "hypotheses": rows,
        }))
    }
}
