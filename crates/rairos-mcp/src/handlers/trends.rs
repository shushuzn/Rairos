use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct TrendsDetectTrendingHandler;

#[async_trait]
impl ToolHandler for TrendsDetectTrendingHandler {
    fn name(&self) -> &str { "trends_detect_trending" }
    fn description(&self) -> &str { "Detect trending research topics from recent arXiv papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("category".into(), ToolProperty::string("arXiv category (e.g. cs.LG, cs.CL, all)")),
                ("max_results".into(), ToolProperty::integer("Number of recent papers to analyze (default 100)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let category = params["category"].as_str().unwrap_or("cs.LG");
        let max = (params["max_results"].as_u64().unwrap_or(100) as usize).min(200);

        let papers = rairos_parser::search_arxiv_by_category(category, max)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let mut word_count: HashMap<String, usize> = HashMap::new();
        for p in &papers {
            let title = p.title.to_lowercase();
            for word in title.split_whitespace() {
                let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
                if clean.len() > 3 {
                    *word_count.entry(clean).or_default() += 1;
                }
            }
        }

        let mut trends: Vec<Value> = word_count.into_iter()
            .map(|(word, count)| serde_json::json!({"keyword": word, "count": count}))
            .collect();
        trends.sort_by(|a, b| b["count"].as_u64().cmp(&a["count"].as_u64()));
        trends.truncate(20);

        Ok(serde_json::json!({
            "trends": trends,
            "papers_analyzed": papers.len(),
            "category": category,
        }))
    }
}

pub struct TrendsPredictNextHandler;

#[async_trait]
impl ToolHandler for TrendsPredictNextHandler {
    fn name(&self) -> &str { "trends_predict_next" }
    fn description(&self) -> &str { "Predict the next heat score for a given tag using Holt's exponential smoothing" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("tag".into(), ToolProperty::string("Research tag to forecast"))].into_iter().collect(),
            vec!["tag".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let tag = params["tag"].as_str().ok_or("Missing tag")?;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let prediction = forecaster.predict_next(tag);
        serde_json::to_value(&prediction).map_err(|e| format!("Serialize error: {}", e))
    }
}

pub struct TrendsTopPredictionsHandler;

#[async_trait]
impl ToolHandler for TrendsTopPredictionsHandler {
    fn name(&self) -> &str { "trends_top_predictions" }
    fn description(&self) -> &str { "Get top-k predicted trending tags ranked by predicted_score * confidence" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("k".into(), ToolProperty::integer("Number of predictions (default 5)"))].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let k = params["k"].as_i64().unwrap_or(5) as usize;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let predictions = forecaster.get_top_predictions(k);
        serde_json::to_value(&predictions).map_err(|e| format!("Serialize error: {}", e))
    }
}

pub struct TrendsCompareTagsHandler;

#[async_trait]
impl ToolHandler for TrendsCompareTagsHandler {
    fn name(&self) -> &str { "trends_compare_tags" }
    fn description(&self) -> &str { "Compare trends trajectories of two tags side by side" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("tag_a".into(), ToolProperty::string("First tag")),
                ("tag_b".into(), ToolProperty::string("Second tag")),
            ].into_iter().collect(),
            vec!["tag_a".into(), "tag_b".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let tag_a = params["tag_a"].as_str().ok_or("Missing tag_a")?;
        let tag_b = params["tag_b"].as_str().ok_or("Missing tag_b")?;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let trajectory_a = forecaster.build_timeseries(tag_a, 12);
        let trajectory_b = forecaster.build_timeseries(tag_b, 12);
        Ok(serde_json::json!({
            "tag_a": tag_a,
            "tag_b": tag_b,
            "trajectory_a": trajectory_a,
            "trajectory_b": trajectory_b,
        }))
    }
}
