use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct PaperParseFullHandler;

#[async_trait]
impl ToolHandler for PaperParseFullHandler {
    fn name(&self) -> &str { "paper_parse_full" }
    fn description(&self) -> &str { "Download and fully parse a paper (PDF, equations, claims, algorithms) by arXiv ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID to parse")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let content = rairos_pdf::paper_parser::download_and_parse(arxiv_id).await;
        Ok(serde_json::json!(content))
    }
}

pub struct ReplicationCheckSimpleHandler;

#[async_trait]
impl ToolHandler for ReplicationCheckSimpleHandler {
    fn name(&self) -> &str { "replication_check_simple" }
    fn description(&self) -> &str { "Check a paper for replication feasibility using code/dependency detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("full_text".into(), ToolProperty::string("Paper full text (optional)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into(), "abstract_text".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params["abstract_text"].as_str().ok_or("Missing abstract_text")?;
        let full_text = params.get("full_text").and_then(|v| v.as_str()).unwrap_or("");

        let checker = rairos_replication::ReplicationChecker::new();
        let report = checker.check_paper(paper_id, title, abstract_text, full_text);
        let rendered = checker.render_report(&report);

        Ok(serde_json::json!({
            "content": [{"type": "text", "text": rendered}],
            "report": serde_json::to_value(&report).unwrap_or_default(),
        }))
    }
}

pub struct GitHubRepoMetadataHandler;

#[async_trait]
impl ToolHandler for GitHubRepoMetadataHandler {
    fn name(&self) -> &str { "github_repo_metadata" }
    fn description(&self) -> &str { "Fetch GitHub repository metadata (stars, forks, language, license, etc.)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("owner".into(), ToolProperty::string("Repository owner (user or organization)")),
                ("repo".into(), ToolProperty::string("Repository name")),
                ("include_readme".into(), ToolProperty::string("Include README preview: \"true\" or \"false\" (default: false)")),
            ].into_iter().collect(),
            vec!["owner".into(), "repo".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let owner = params["owner"].as_str().ok_or("Missing owner")?;
        let repo = params["repo"].as_str().ok_or("Missing repo")?;
        let include_readme = params.get("include_readme")
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let github = rairos_replication_checker::GitHubClient::new();
        let metadata = github.get_repo_metadata(owner, repo).await
            .map_err(|e| format!("Failed to fetch repo metadata: {}", e))?;

        let mut result = serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "## {}\nStars: {} | Forks: {} | Language: {}\nLicense: {}\nCreated: {} | Last push: {}\nOpen Issues: {}\nTopics: {}",
                    metadata.full_name,
                    metadata.stars,
                    metadata.forks,
                    metadata.language.as_deref().unwrap_or("N/A"),
                    metadata.license.as_deref().unwrap_or("N/A"),
                    metadata.created_at,
                    metadata.pushed_at,
                    metadata.open_issues,
                    metadata.topics.join(", ")
                )
            }],
            "metadata": metadata,
        });

        if include_readme {
            match github.get_readme_preview(owner, repo, 500).await {
                Ok(readme) => {
                    if let Some(content) = result["content"].as_array_mut() {
                        if let Some(text) = content[0].as_object_mut() {
                            text.insert("text".to_string(), serde_json::json!(format!("{}\n\n## README Preview\n{}", text["text"], readme)));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch README: {}", e);
                }
            }
        }

        Ok(result)
    }
}

pub struct HuggingFaceDatasetHandler;

#[async_trait]
impl ToolHandler for HuggingFaceDatasetHandler {
    fn name(&self) -> &str { "huggingface_dataset_metadata" }
    fn description(&self) -> &str { "Fetch HuggingFace dataset metadata (downloads, tags, papers with code)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("dataset_id".into(), ToolProperty::string("Dataset ID (e.g., imagenet-1k or ILSVRC/imagenet-1k)")),
                ("search".into(), ToolProperty::string("Search query to find datasets (alternative to dataset_id)")),
                ("limit".into(), ToolProperty::string("Max results when searching (default: 5)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let client = rairos_replication_checker::HuggingFaceClient::new();

        if let Some(search) = params.get("search").and_then(|v| v.as_str()) {
            let limit = params.get("limit")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(5);

            let datasets = client.search_datasets(search, limit).await
                .map_err(|e| format!("Failed to search datasets: {}", e))?;

            let content: Vec<String> = datasets.iter().map(|d| {
                format!(
                    "## {}\nDownloads: {} | Tags: {}\n",
                    d.id,
                    d.downloads,
                    d.tags.iter().take(5).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                )
            }).collect();

            Ok(serde_json::json!({
                "content": [{"type": "text", "text": content.join("\n")}],
                "datasets": datasets,
            }))
        } else {
            let dataset_id = params["dataset_id"].as_str()
                .ok_or("Missing dataset_id or search parameter")?;

            let meta = client.get_dataset_metadata(dataset_id).await
                .map_err(|e| format!("Failed to fetch dataset metadata: {}", e))?;

            Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "## {}\nDownloads: {}\nTags: {}\nPapers with Code: {}",
                        meta.id,
                        meta.downloads,
                        meta.tags.iter().take(10).map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                        meta.papers_with_code.map(|n| n.to_string()).unwrap_or_else(|| "N/A".to_string())
                    )
                }],
                "metadata": meta,
            }))
        }
    }
}

pub struct PdfExtractAdvancedHandler;

#[async_trait]
impl ToolHandler for PdfExtractAdvancedHandler {
    fn name(&self) -> &str { "pdf_extract_advanced" }
    fn description(&self) -> &str { "Extract text from PDF with advanced fallback methods, section segmentation, and block detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

        if !pdf_path.exists() {
            return Err("PDF not found. Call pdf_download first.".into());
        }

        let text = rairos_pdf::pdf_parser2::extract_pdf_text_with_fallback(&pdf_path)
            .map_err(|e| format!("Advanced text extraction failed: {}", e))?;

        let sections = rairos_pdf::segment_into_sections(&text, 20);
        let section_list: Vec<Value> = sections.iter()
            .map(|(name, content)| serde_json::json!({
                "section": name,
                "content_length": content.len(),
                "preview": content.chars().take(200).collect::<String>(),
            }))
            .collect();

        Ok(serde_json::json!({
            "text": text,
            "char_count": text.chars().count(),
            "sections": section_list,
            "section_count": sections.len(),
        }))
    }
}
