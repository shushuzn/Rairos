use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::llm_client;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct TrustScorerComputeHandler;

#[async_trait]
impl ToolHandler for TrustScorerComputeHandler {
    fn name(&self) -> &str { "trust_scorer_compute" }
    fn description(&self) -> &str { "Compute per-category trust scores from capsule quality data" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("scores".into(), ToolProperty::string("JSON array of {category: string, score: number} objects"))].into_iter().collect(),
            vec!["scores".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let entries: Vec<(String, f64)> = if let Some(arr) = params["scores"].as_array() {
            arr.iter().filter_map(|v| {
                let cat = v["category"].as_str()?;
                let score = v["score"].as_f64()?;
                Some((cat.to_string(), score))
            }).collect()
        } else if let Some(s) = params["scores"].as_str() {
            serde_json::from_str(s).map_err(|e| format!("Invalid scores JSON: {}", e))?
        } else {
            return Err("Missing scores: provide JSON array or JSON string".into());
        };
        let refs: Vec<(&str, f64)> = entries.iter().map(|(c, s)| (c.as_str(), *s)).collect();
        let result = rairos_llm::trust_scorer::compute_trust(&refs);
        Ok(serde_json::json!(result))
    }
}

pub struct RouteQueryHandler;

#[async_trait]
impl ToolHandler for RouteQueryHandler {
    fn name(&self) -> &str { "routeplan_create" }
    fn description(&self) -> &str { "Create a research route plan from a hypothesis (LLM-backed, keyword fallback)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("hypothesis".into(), ToolProperty::string("Research hypothesis to investigate")),
                ("goal".into(), ToolProperty::string("What the plan should determine")),
                ("known_papers".into(), ToolProperty::string("JSON array of {arxiv_id, title} (optional)")),
            ].into_iter().collect(),
            vec!["hypothesis".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let hypothesis = params["hypothesis"].as_str().ok_or("Missing hypothesis")?;
        let goal = params.get("goal").and_then(|v| v.as_str()).unwrap_or("Test the hypothesis");

        if let Some(client) = llm_client() {
            let known_papers: Vec<String> = params.get("known_papers")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().map(|p| {
                    let title = p.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let arxiv_id = p.get("arxiv_id").and_then(|i| i.as_str()).unwrap_or("");
                    format!("{} ({})", title, arxiv_id)
                }).collect())
                .unwrap_or_default();

            let plan = rairos_llm::route_planner::create_plan(
                client.as_ref(), crate::llm_handlers::helpers::llm_model(), hypothesis, goal, &known_papers,
            ).await;
            let _ = rairos_llm::route_planner::save_plan(&plan);
            return Ok(serde_json::json!(plan));
        }

        let route = rairos_llm::semantic_router::route_by_keyword(hypothesis);
        Ok(serde_json::json!({"semantic_route": route, "note": "No LLM available — keyword routing only. Set OPENAI_API_KEY or ANTHROPIC_API_KEY for full plan generation."}))
    }
}

pub struct RoutePlanListHandler;

#[async_trait]
impl ToolHandler for RoutePlanListHandler {
    fn name(&self) -> &str { "routeplan_list" }
    fn description(&self) -> &str { "List all research plans" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let plans = rairos_llm::route_planner::list_plans(20);
        let list: Vec<Value> = plans.iter().map(|p| {
            let progress = p.get_progress();
            serde_json::json!({
                "plan_id": p.plan_id,
                "hypothesis": p.hypothesis.chars().take(80).collect::<String>(),
                "goal": p.goal.chars().take(80).collect::<String>(),
                "status": p.status,
                "step_count": p.steps.len(),
                "progress": progress.progress_pct,
                "revision_count": p.revision_count,
                "created_at": p.created_at,
                "updated_at": p.updated_at,
            })
        }).collect();
        Ok(serde_json::json!({"plans": list, "count": list.len()}))
    }
}

pub struct RoutePlanUpdateStepHandler;

#[async_trait]
impl ToolHandler for RoutePlanUpdateStepHandler {
    fn name(&self) -> &str { "routeplan_update_step" }
    fn description(&self) -> &str { "Update a step status in a research plan" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("plan_id".into(), ToolProperty::string("ID of the plan")),
                ("step_id".into(), ToolProperty::string("ID of the step to update")),
                ("status".into(), ToolProperty::string("New status: pending, in_progress, completed, blocked, failed, skipped")),
                ("result".into(), ToolProperty::string("Result details (optional)")),
                ("notes".into(), ToolProperty::string("Notes (optional)")),
            ].into_iter().collect(),
            vec!["plan_id".into(), "step_id".into(), "status".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let plan_id = params["plan_id"].as_str().ok_or("Missing plan_id")?;
        let step_id = params["step_id"].as_str().ok_or("Missing step_id")?;
        let status = params["status"].as_str().ok_or("Missing status")?;
        let result = params.get("result").and_then(|v| v.as_str()).unwrap_or("");
        let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");

        let plan = rairos_llm::route_planner::update_step(plan_id, step_id, status, result, notes)
            .ok_or_else(|| format!("Plan {} or step {} not found", plan_id, step_id))?;

        let ready: Vec<Value> = plan.get_ready_steps().iter().map(|s| {
            serde_json::json!({"step_id": s.step_id, "description": s.description})
        }).collect();

        Ok(serde_json::json!({
            "plan_id": plan.plan_id,
            "step_id": step_id,
            "status": status,
            "progress": plan.get_progress(),
            "ready_steps": ready,
        }))
    }
}

pub struct RoutePlanReviseHandler;

#[async_trait]
impl ToolHandler for RoutePlanReviseHandler {
    fn name(&self) -> &str { "routeplan_revise" }
    fn description(&self) -> &str { "Revise a plan when dead ends are hit" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("plan_id".into(), ToolProperty::string("ID of the plan to revise")),
                ("reason".into(), ToolProperty::string("Reason for the revision")),
            ].into_iter().collect(),
            vec!["plan_id".into(), "reason".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let plan_id = params["plan_id"].as_str().ok_or("Missing plan_id")?;
        let reason = params["reason"].as_str().ok_or("Missing reason")?;

        let new_plan = rairos_llm::route_planner::revise_plan(plan_id, reason)
            .ok_or_else(|| format!("Plan {} not found", plan_id))?;

        Ok(serde_json::json!({
            "new_plan_id": new_plan.plan_id,
            "old_plan_id": plan_id,
            "revision_count": new_plan.revision_count,
            "step_count": new_plan.steps.len(),
            "progress": new_plan.get_progress(),
        }))
    }
}
