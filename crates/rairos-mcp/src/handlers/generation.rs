use crate::handlers::helpers::{data_dir, chrono_now};
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

pub struct PaperVisualizeTrendsHandler;

#[async_trait]
impl ToolHandler for PaperVisualizeTrendsHandler {
    fn name(&self) -> &str { "paper_visualize_trends" }
    fn description(&self) -> &str { "Generate a publication-quality bar chart of research trends" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("trends_json".into(), ToolProperty::string("JSON array of {keyword, count} objects")),
                ("chart_type".into(), ToolProperty::string("Chart type: bar (default), line")),
                ("title".into(), ToolProperty::string("Chart title")),
                ("journal".into(), ToolProperty::string("Target journal: default, nature, science, cell")),
            ].into_iter().collect(),
            vec!["trends_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let trends_str = params["trends_json"].as_str().ok_or("Missing trends_json")?;
        let chart_type = params.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Research Trends");
        let journal = params.get("journal").and_then(|v| v.as_str()).unwrap_or("default");

        let trends: Vec<(String, usize)> = serde_json::from_str(trends_str)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        if trends.is_empty() {
            return Err("No trends data provided".to_string());
        }

        let labels: Vec<String> = trends.iter().map(|(k, _)| k.clone()).collect();
        let values: Vec<f64> = trends.iter().map(|(_, v)| *v as f64).collect();

        let data = serde_json::json!({
            "labels": labels,
            "values": values,
            "xlabel": "Keyword",
            "ylabel": "Frequency"
        });

        let output_dir = data_dir().join("visualizations");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
        let output_path = output_dir.join(format!("trends_{}.png", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));

        let data_str = serde_json::to_string(&data).map_err(|e| e.to_string())?;

        let python_cmd = std::env::var("RAIROS_VIZ_HELPER")
            .unwrap_or_else(|_| "python3".to_string());

        let mut cmd = std::process::Command::new(&python_cmd);
        cmd.arg("/root/Rairos/scripts/viz_helper.py")
            .arg("--type").arg(chart_type)
            .arg("--data").arg(&data_str)
            .arg("--output").arg(output_path.to_str().unwrap())
            .arg("--title").arg(title)
            .arg("--journal").arg(journal);

        let output = cmd.output().map_err(|e| format!("Failed to run viz helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("viz_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "trends_count": trends.len(),
            "chart_type": chart_type,
        }))
    }
}

pub struct PaperVisualizeRadarHandler;

#[async_trait]
impl ToolHandler for PaperVisualizeRadarHandler {
    fn name(&self) -> &str { "paper_visualize_radar" }
    fn description(&self) -> &str { "Generate a radar chart for paper rubric scores" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("scores_json".into(), ToolProperty::string("JSON object with axis names and scores, e.g. {\"Novelty\": 8, \"Leverage\": 7}")),
                ("title".into(), ToolProperty::string("Chart title")),
                ("journal".into(), ToolProperty::string("Target journal: default, nature, science, cell")),
            ].into_iter().collect(),
            vec!["scores_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let scores_str = params["scores_json"].as_str().ok_or("Missing scores_json")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Paper Scores");
        let journal = params.get("journal").and_then(|v| v.as_str()).unwrap_or("default");

        let scores: serde_json::Map<String, serde_json::Value> = serde_json::from_str(scores_str)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        if scores.is_empty() {
            return Err("No scores data provided".to_string());
        }

        let axes: Vec<String> = scores.keys().cloned().collect();
        let values: Vec<f64> = scores.values()
            .filter_map(|v| v.as_f64().or_else(|| v.as_u64().map(|x| x as f64)))
            .collect();

        if axes.len() != values.len() {
            return Err("Axes and scores count mismatch".to_string());
        }

        let data = serde_json::json!({
            "axes": axes,
            "scores": values,
            "max_score": 10
        });

        let output_dir = data_dir().join("visualizations");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
        let output_path = output_dir.join(format!("radar_{}.png", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));

        let data_str = serde_json::to_string(&data).map_err(|e| e.to_string())?;

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/viz_helper.py")
            .arg("--type").arg("radar")
            .arg("--data").arg(&data_str)
            .arg("--output").arg(output_path.to_str().unwrap())
            .arg("--title").arg(title)
            .arg("--journal").arg(journal);

        let output = cmd.output().map_err(|e| format!("Failed to run viz helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("viz_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "axes": axes,
            "scores": values,
        }))
    }
}

pub struct PaperCriticalAnalysisHandler;

#[async_trait]
impl ToolHandler for PaperCriticalAnalysisHandler {
    fn name(&self) -> &str { "paper_critical_analysis" }
    fn description(&self) -> &str { "Evaluate a paper for methodological quality, biases, and evidence strength using critical thinking frameworks" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract".into(), ToolProperty::string("Paper abstract")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into(), "abstract".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params["abstract"].as_str().ok_or("Missing abstract")?;

        let checker = rairos_replication_checker::CriticalThinkingChecker::new();
        let report = checker.analyze(paper_id, title, abstract_text);

        Ok(serde_json::json!({
            "paper_id": report.paper_id,
            "study_design": report.study_design,
            "design_quality_score": report.design_quality_score,
            "evidence_quality": report.evidence_quality,
            "overall_score": report.overall_score,
            "biases": report.biases.iter().map(|b| serde_json::json!({
                "type": b.bias_type,
                "severity": b.severity,
                "description": b.description,
                "indicator": b.indicator,
            })).collect::<Vec<_>>(),
            "statistical_concerns": report.statistical_concerns.iter().map(|c| serde_json::json!({
                "type": c.concern_type,
                "severity": c.severity,
                "description": c.description,
                "suggestion": c.suggestion,
            })).collect::<Vec<_>>(),
            "logical_fallacies": report.logical_fallacies,
            "strengths": report.strengths,
            "recommendations": report.recommendations,
        }))
    }
}

pub struct PaperGenerateReviewPdfHandler;

#[async_trait]
impl ToolHandler for PaperGenerateReviewPdfHandler {
    fn name(&self) -> &str { "paper_generate_review_pdf" }
    fn description(&self) -> &str { "Generate a PDF literature review from structured content" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("review_json".into(), ToolProperty::string("JSON object with title, topic, abstract, sections, references")),
                ("output_path".into(), ToolProperty::string("Output PDF file path (optional, defaults to data dir)")),
            ].into_iter().collect(),
            vec!["review_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let review_json = params["review_json"].as_str().ok_or("Missing review_json")?;
        let output_path = params.get("output_path").and_then(|v| v.as_str());

        let output_dir = data_dir().join("reviews");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("review_{}.pdf", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let pdf_path = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            output_dir.join(&filename)
        };

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/pdf_helper.py")
            .arg("--type").arg("review")
            .arg("--data").arg(review_json)
            .arg("--output").arg(pdf_path.to_str().unwrap());

        let output = cmd.output().map_err(|e| format!("Failed to run pdf_helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pdf_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "pdf_path": pdf_path.to_string_lossy(),
            "status": "generated",
        }))
    }
}

pub fn build_hypothesis_markdown(topic: &str, hypotheses_json: &str) -> String {
    let hypotheses: Vec<serde_json::Value> = serde_json::from_str(hypotheses_json)
        .unwrap_or_default();

    let mut md = format!("# Hypothesis Report: {}\n\n", topic);
    md.push_str(&format!("**Generated:** {}\n\n", chrono_now()));

    md.push_str("## Executive Summary\n\n");
    md.push_str(&format!("This report presents {} research hypotheses generated for the topic: *{}*\n\n",
        hypotheses.len(), topic));

    md.push_str("---\n\n## Hypotheses\n\n");

    for (i, h) in hypotheses.iter().enumerate() {
        let title = h.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
        let hypo_type = h.get("hypothesis_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let description = h.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let evidence = h.get("evidence").and_then(|v| v.as_str()).unwrap_or("");
        let predictions = h.get("predictions").and_then(|v| v.as_str()).unwrap_or("");
        let experiments = h.get("experiments").and_then(|v| v.as_str()).unwrap_or("");

        md.push_str(&format!("### Hypothesis {}: {}\n\n", i + 1, title));
        md.push_str(&format!("**Type:** {} | **Confidence:** {}/10\n\n",
            hypo_type,
            h.get("confidence").and_then(|v| v.as_f64()).unwrap_or(5.0) as i32));

        if !description.is_empty() {
            md.push_str(&format!("**Mechanism:** {}\n\n", description));
        }

        if !evidence.is_empty() {
            md.push_str(&format!("**Supporting Evidence:** {}\n\n", evidence));
        }

        if !predictions.is_empty() {
            md.push_str(&format!("**Testable Predictions:**\n{}\n\n", predictions));
        }

        if !experiments.is_empty() {
            md.push_str(&format!("**Proposed Experiments:**\n{}\n\n", experiments));
        }

        md.push_str("---\n\n");
    }

    md.push_str("## Recommendations\n\n");
    md.push_str("Based on the generated hypotheses, the following next steps are recommended:\n\n");
    md.push_str("1. **Validate hypotheses** against existing literature\n");
    md.push_str("2. **Design experiments** to test the highest-confidence hypotheses\n");
    md.push_str("3. **Submit to GenePool** for tracking and evolution\n\n");

    md
}

pub struct HypothesisReportHandler;

#[async_trait]
impl ToolHandler for HypothesisReportHandler {
    fn name(&self) -> &str { "paper_hypothesis_report" }
    fn description(&self) -> &str { "Generate a structured hypothesis report with framework from hypothesis results" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("hypotheses_json".into(), ToolProperty::string("JSON array of hypotheses from hypothesis_generate")),
                ("output_format".into(), ToolProperty::string("Output format: markdown or pdf (default: markdown)"))
            ].into_iter().collect(),
            vec!["topic".into(), "hypotheses_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let hypotheses_json = params["hypotheses_json"].as_str().ok_or("Missing hypotheses_json")?;
        let output_format = params.get("output_format").and_then(|v| v.as_str()).unwrap_or("markdown");

        let markdown_content = build_hypothesis_markdown(topic, hypotheses_json);

        let output_dir = data_dir().join("hypotheses");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("hypothesis_report_{}.md",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let md_path = output_dir.join(&filename);

        std::fs::write(&md_path, &markdown_content).map_err(|e| e.to_string())?;

        let mut result = serde_json::json!({
            "report_path": md_path.to_string_lossy(),
            "format": "markdown",
            "topic": topic,
        });

        if output_format == "pdf" {
            let pdf_filename = format!("hypothesis_report_{}.pdf",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_path = output_dir.join(&pdf_filename);

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("markdown")
                .arg("--file").arg(md_path.to_str().unwrap())
                .arg("--output").arg(pdf_path.to_str().unwrap());

            if cmd.output().map_err(|e| e.to_string())?.status.success() {
                result = serde_json::json!({
                    "report_path": pdf_path.to_string_lossy(),
                    "markdown_path": md_path.to_string_lossy(),
                    "format": "pdf",
                    "topic": topic,
                });
            }
        }

        Ok(result)
    }
}

pub struct PaperGenerateSchematicHandler;

#[async_trait]
impl ToolHandler for PaperGenerateSchematicHandler {
    fn name(&self) -> &str { "paper_generate_schematic" }
    fn description(&self) -> &str { "Generate a scientific schematic diagram (flowchart, architecture, pathway, block, timeline) from structured data" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("diagram_type".into(), ToolProperty::string("Type: flowchart, architecture, pathway, block, timeline")),
                ("diagram_json".into(), ToolProperty::string("JSON data for the diagram (structure depends on type)")),
                ("title".into(), ToolProperty::string("Diagram title")),
            ].into_iter().collect(),
            vec!["diagram_type".into(), "diagram_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let diagram_type = params["diagram_type"].as_str().ok_or("Missing diagram_type")?;
        let diagram_json = params["diagram_json"].as_str().ok_or("Missing diagram_json")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("");

        let valid_types = ["flowchart", "architecture", "pathway", "block", "timeline"];
        if !valid_types.contains(&diagram_type) {
            return Err(format!("Invalid diagram_type: {}. Use: {}", diagram_type, valid_types.join(", ")));
        }

        let output_dir = data_dir().join("schematics");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("{}_{}.png",
            diagram_type,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let output_path = output_dir.join(&filename);

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/schematic_helper.py")
            .arg("--type").arg(diagram_type)
            .arg("--data").arg(diagram_json)
            .arg("--output").arg(output_path.to_str().unwrap());
        if !title.is_empty() {
            cmd.arg("--title").arg(title);
        }

        let output = cmd.output().map_err(|e| format!("Failed to run schematic_helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("schematic_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "diagram_type": diagram_type,
        }))
    }
}
