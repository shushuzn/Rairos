use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct PdfDownloadHandler;

#[async_trait]
impl ToolHandler for PdfDownloadHandler {
    fn name(&self) -> &str { "pdf_download" }
    fn description(&self) -> &str { "Download a PDF from arXiv" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv paper ID"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        std::fs::create_dir_all(&pdf_dir).map_err(|e| format!("Failed to create pdfs dir: {}", e))?;
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));
        let url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);

        if !pdf_path.exists() {
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {}", e))?;
            rt.block_on(rairos_pdf::download_pdf(&url, &pdf_path))
                .map_err(|e| format!("Download failed: {}", e))?;
        }

        let size_bytes = std::fs::metadata(&pdf_path).map(|m| m.len()).unwrap_or(0);
        Ok(serde_json::json!({
            "saved_path": pdf_path.to_string_lossy(),
            "size_bytes": size_bytes,
            "url": url,
        }))
    }
}

pub struct PdfExtractTextHandler;

#[async_trait]
impl ToolHandler for PdfExtractTextHandler {
    fn name(&self) -> &str { "pdf_extract_text" }
    fn description(&self) -> &str { "Extract plain text from a PDF file" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper"))].into_iter().collect(),
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

        let text = rairos_pdf::extract_pdf_text(&pdf_path)
            .map_err(|e| format!("Text extraction failed: {}", e))?;

        let char_count = text.chars().count();
        Ok(serde_json::json!({
            "text": text,
            "char_count": char_count,
        }))
    }
}

pub struct PdfExtractStructuredHandler;

#[async_trait]
impl ToolHandler for PdfExtractStructuredHandler {
    fn name(&self) -> &str { "pdf_extract_structured" }
    fn description(&self) -> &str { "Extract structured content from PDF (text blocks, tables, math)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper"))].into_iter().collect(),
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

        let text = rairos_pdf::extract_pdf_text(&pdf_path)
            .map_err(|e| format!("Text extraction failed: {}", e))?;

        let text_blocks: Vec<Value> = text.lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, l)| serde_json::json!({
                "index": i,
                "text": l,
                "length": l.len(),
            }))
            .collect();

        let sections = rairos_pdf::segment_into_sections(&text, 20);
        let section_list: Vec<Value> = sections.iter()
            .map(|(name, content)| serde_json::json!({
                "section": name,
                "content_length": content.len(),
                "preview": content.chars().take(200).collect::<String>(),
            }))
            .collect();

        let math_count = text.lines().filter(|l| l.contains("\\(") || l.contains("\\[") || l.contains("$$")).count();

        Ok(serde_json::json!({
            "text_blocks": text_blocks,
            "section_count": sections.len(),
            "sections": section_list,
            "math_count": math_count,
            "total_chars": text.len(),
        }))
    }
}
