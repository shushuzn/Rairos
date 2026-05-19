use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::gene_pool_data_dir;
use async_trait::async_trait;
use rairos_llm::insight::evolution::EvolutionEngine;
use rairos_llm::insight::storage::CapsuleStorage;
use serde_json::Value;

pub struct GapDetectHandler;

#[async_trait]
impl ToolHandler for GapDetectHandler {
    fn name(&self) -> &str { "gap_detect" }
    fn description(&self) -> &str { "Detect research gaps from the paper corpus for a topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("topic".into(), ToolProperty::string("Research topic to analyze"))].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let gaps = rairos_llm::gap_detector::detect_gaps_keyword(topic);
        Ok(serde_json::json!({
            "topic": topic,
            "gaps": gaps.iter().map(|g| serde_json::json!({
                "type": g.gap_type.as_str(), "description": g.description,
                "evidence_papers": g.evidence_papers, "confidence": g.confidence,
            })).collect::<Vec<_>>(),
            "total": gaps.len(),
        }))
    }
}

pub struct GapSubmitHandler;

#[async_trait]
impl ToolHandler for GapSubmitHandler {
    fn name(&self) -> &str { "gap_submit" }
    fn description(&self) -> &str { "Submit a new research gap directly to the Gene Pool as a CapsuleGene" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("gap_type".into(), ToolProperty::string("Type of research gap")),
                ("title".into(), ToolProperty::string("Gap title")),
                ("description".into(), ToolProperty::string("Gap description (optional)")),
                ("success_score".into(), ToolProperty::string("Success score 0.0-1.0 (default 0.8)")),
            ].into_iter().collect(),
            vec!["topic".into(), "gap_type".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_type = params["gap_type"].as_str().ok_or("Missing required parameter: gap_type")?;
        let title = params["title"].as_str().ok_or("Missing required parameter: title")?;
        let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let success_score = params.get("success_score").and_then(|v| v.as_f64()).unwrap_or(0.8);

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        let capsule = storage.encode_capsule(
            topic, gap_type, title, description, success_score,
            "active", "", "", None, None, &data_dir,
        ).map_err(|e| format!("encode_capsule failed: {}", e))?;

        Ok(serde_json::json!({
            "capsule_id": capsule.capsule_id,
            "topic": topic,
            "gap_type": gap_type,
            "title": title,
            "status": capsule.status,
            "message": format!("Gap '{}' submitted to Gene Pool successfully", title),
        }))
    }
}

pub struct GapEvolveHandler;

#[async_trait]
impl ToolHandler for GapEvolveHandler {
    fn name(&self) -> &str { "gap_evolve" }
    fn description(&self) -> &str { "Run Gene Pool evolution cycle for a topic — audit, propose, evaluate, apply" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic to evolve for")),
                ("gap_type".into(), ToolProperty::string("Optional gap type filter")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_type = params.get("gap_type").and_then(|v| v.as_str());

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        let capsules = storage.load_all_capsules()
            .map_err(|e| format!("Failed to load capsules: {}", e))?;

        if capsules.is_empty() {
            return Ok(serde_json::json!({
                "topic": topic,
                "gap_type": gap_type,
                "audit": { "total": 0, "avg_quality": 0.0, "candidates": 0, "to_retire": 0 },
                "proposed": 0,
                "evaluated": 0,
                "result": { "added": 0, "retired": 0, "total_capsules": 0, "avg_quality": 0.0 },
                "note": "Gene pool is empty — submit gaps first with gap_submit",
            }));
        }

        let mut engine = EvolutionEngine::new(capsules);
        let result = engine.evolve(topic, gap_type);

        let evolved = engine.get_capsules().to_vec();
        storage.save_capsules(&evolved)
            .map_err(|e| format!("Failed to persist evolved gene pool: {}", e))?;

        let audit = result.get("audit").and_then(|v| v.as_object()).cloned().unwrap_or_default();
        let result_data = result.get("result").and_then(|v| v.as_object()).cloned().unwrap_or_default();

        Ok(serde_json::json!({
            "topic": topic,
            "gap_type": gap_type,
            "audit": audit,
            "proposed": result.get("proposed").and_then(|v| v.as_u64()).unwrap_or(0),
            "evaluations": result.get("evaluations").and_then(|v| v.as_u64()).unwrap_or(0),
            "result": result_data,
        }))
    }
}
