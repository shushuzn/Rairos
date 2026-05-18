use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::llm_client;
use async_trait::async_trait;
use serde_json::Value;

pub struct ReplicationCheckHandler;

#[async_trait]
impl ToolHandler for ReplicationCheckHandler {
    fn name(&self) -> &str { "replication_check" }
    fn description(&self) -> &str { "Check a paper's reproducibility" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID")),
                ("include_abstract".into(), ToolProperty::string("Abstract text (optional)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str())
            .or_else(|| params.get("paper_id").and_then(|v| v.as_str()))
            .ok_or("Missing arxiv_id or paper_id")?;
        let abstract_text = params.get("include_abstract").and_then(|v| v.as_str()).unwrap_or("");
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or(arxiv_id);

        let result = if let Some(client) = llm_client() {
            rairos_llm::replication::llm_assess_replication(client.as_ref(), crate::llm_handlers::helpers::llm_model(), title, abstract_text).await
        } else {
            rairos_llm::replication::keyword_check(abstract_text)
        };
        Ok(serde_json::json!({
            "arxiv_id": arxiv_id, "score": result.score,
            "has_code": result.has_code, "has_data": result.has_data,
            "has_method": result.has_method, "has_env": result.has_env,
            "reasoning": result.reasoning,
        }))
    }
}

pub struct ReplicationCompareHandler;

#[async_trait]
impl ToolHandler for ReplicationCompareHandler {
    fn name(&self) -> &str { "replication_compare" }
    fn description(&self) -> &str { "Compare reproducibility of two papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id_1".into(), ToolProperty::string("arXiv ID of the first paper")),
                ("arxiv_id_2".into(), ToolProperty::string("arXiv ID of the second paper")),
            ].into_iter().collect(),
            vec!["arxiv_id_1".into(), "arxiv_id_2".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id_1 = params["arxiv_id_1"].as_str().ok_or("Missing arxiv_id_1")?;
        let arxiv_id_2 = params["arxiv_id_2"].as_str().ok_or("Missing arxiv_id_2")?;

        let checker = rairos_replication_checker::ReplicationChecker::new();

        let report1 = checker.check_paper(arxiv_id_1, arxiv_id_1, "", "");
        let report2 = checker.check_paper(arxiv_id_2, arxiv_id_2, "", "");

        let easier_id = if report1.difficulty_score < report2.difficulty_score {
            report1.paper_id.clone()
        } else {
            report2.paper_id.clone()
        };

        Ok(serde_json::json!({
            "paper_1": report1,
            "paper_2": report2,
            "easier_to_reproduce": easier_id,
            "comparison": {
                "difficulty_diff": (report1.difficulty_score - report2.difficulty_score).abs(),
            },
        }))
    }
}
