use rairos_core::constants::{GP_DIR_NAME, GENE_POOL_JSONL, TAGS_FILE};
use serde_json::Value;
use std::io::BufRead;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use tokio::io::AsyncWriteExt;

static KG: OnceCell<rairos_kg::KnowledgeGraph> = OnceCell::const_new();

pub async fn kg() -> &'static rairos_kg::KnowledgeGraph {
    KG.get_or_init(|| async {
        let db_path = rairos_kg::KnowledgeGraph::db_path();
        rairos_kg::KnowledgeGraph::with_db(db_path)
            .await
            .expect("Failed to initialize knowledge graph")
    }).await
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    home_dir().join(".ai_research_os")
}

pub fn tags_path() -> PathBuf {
    data_dir().join(TAGS_FILE)
}

pub fn gene_pool_path() -> PathBuf {
    data_dir().join(GP_DIR_NAME).join(GENE_POOL_JSONL)
}

pub fn read_jsonl(path: &PathBuf) -> Vec<Value> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = std::io::BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let t = line.trim();
            if t.is_empty() { None } else { serde_json::from_str(t).ok() }
        })
        .collect()
}

/// Async version of read_jsonl for use in async contexts
pub async fn read_jsonl_async(path: &PathBuf) -> Vec<Value> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() { None } else { serde_json::from_str(t).ok() }
        })
        .collect()
}

pub async fn append_jsonl(path: &PathBuf, value: &Value) -> Result<(), String> {
    if let Some(p) = path.parent() { tokio::fs::create_dir_all(p).await.map_err(|e| e.to_string())?; }
    let line = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let mut file = tokio::fs::OpenOptions::new().create(true).write(true).append(true).open(path)
        .await.map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
    file.write_all(b"\n").await.map_err(|e| e.to_string())
}

pub async fn write_jsonl(path: &PathBuf, items: &[Value]) -> Result<(), String> {
    if let Some(p) = path.parent() { tokio::fs::create_dir_all(p).await.map_err(|e| e.to_string())?; }
    let mut file = tokio::fs::File::create(path).await.map_err(|e| e.to_string())?;
    for item in items {
        let line = serde_json::to_string(item).map_err(|e| e.to_string())?;
        file.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
        file.write_all(b"\n").await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn chrono_now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn parse_arxiv_response(xml: &str) -> Vec<Value> {
    let mut papers = Vec::new();
    let mut pos = 0;
    while let Some(entry_start) = xml[pos..].find("<entry>") {
        let abs_start = pos + entry_start;
        let Some(entry_end) = xml[abs_start..].find("</entry>") else { break; };
        let entry = &xml[abs_start..abs_start + entry_end + 8];
        pos = abs_start + entry_end + 8;

        let id = extract_tag(entry, "id").unwrap_or_default();
        let published = extract_tag(entry, "published").unwrap_or_default();
        let title = extract_tag(entry, "title").map(clean_xml).unwrap_or_default();
        let summary = extract_tag(entry, "summary").map(clean_xml).unwrap_or_default();
        let authors = extract_authors(entry);
        let categories = extract_categories(entry);

        let arxiv_id = id.strip_prefix("http://arxiv.org/abs/")
            .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
            .map(|s| s.to_string()).unwrap_or_default();

        papers.push(serde_json::json!({
            "arxiv_id": arxiv_id, "title": title, "abstract": summary,
            "authors": authors, "categories": categories, "published": published,
            "pdf_url": format!("https://arxiv.org/pdf/{}.pdf", arxiv_id), "abs_url": id,
        }));
    }
    papers
}

fn extract_tag(s: &str, tag: &str) -> Option<String> {
    let start = s.find(&format!("<{}>", tag))?;
    let value_start = start + tag.len() + 2;
    let end = s[value_start..].find(&format!("</{}>", tag))?;
    Some(s[value_start..value_start + end].to_string())
}

fn clean_xml(s: String) -> String {
    s.trim().replace('\n', " ").replace("  ", " ")
        .replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&apos;", "'")
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_authors(entry: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("<author>") {
        let abs_start = pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else { break; };
        let ab = &entry[abs_start..abs_start + end + 9];
        if let Some(n) = extract_tag(ab, "name") { authors.push(n); }
        pos = abs_start + end + 9;
    }
    authors
}

fn extract_categories(entry: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("term=\"") {
        let after = &entry[pos + start + 6..];
        if let Some(end) = after.find('"') { cats.push(after[..end].to_string()); }
        pos += start + 6;
    }
    cats
}

pub fn parse_arxiv_citation(xml: &str) -> Result<serde_json::Value, String> {
    let entry_text: Option<String> = if let Some(start) = xml.find("<entry>") {
        let abs_start = start;
        let end = xml[abs_start..].find("</entry>").ok_or("No </entry> found")?;
        Some(xml[abs_start..abs_start + end + 8].to_string())
    } else {
        None
    };
    let entry = entry_text.ok_or("No entry found in arXiv response")?;

    let title = extract_tag(&entry, "title").map(clean_xml).unwrap_or_default();
    let id = extract_tag(&entry, "id").unwrap_or_default();
    let published = extract_tag(&entry, "published").unwrap_or_default();
    let journal_ref = extract_tag(&entry, "journal-ref").unwrap_or_default();
    let doi = extract_tag(&entry, "doi").unwrap_or_default();

    let mut authors: Vec<serde_json::Value> = Vec::new();
    let mut a_pos = 0;
    while let Some(start) = entry[a_pos..].find("<author>") {
        let abs_start = a_pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else { break; };
        let ab = &entry[abs_start..abs_start + end + 9];
        if let Some(name) = extract_tag(ab, "name") {
            let parts: Vec<&str> = name.split_whitespace().collect();
            let family = parts.last().unwrap_or(&"").to_string();
            let given = if parts.len() > 1 { parts[..parts.len()-1].join(" ") } else { String::new() };
            authors.push(serde_json::json!({ "family": family, "given": given }));
        }
        a_pos = abs_start + end + 9;
    }

    let arxiv_id = id.strip_prefix("http://arxiv.org/abs/")
        .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
        .unwrap_or(&id).to_string();

    Ok(serde_json::json!({
        "title": title,
        "id": id,
        "published": published,
        "journal-ref": journal_ref,
        "doi": doi,
        "authors": authors,
        "arxiv_id": arxiv_id,
    }))
}

pub fn http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

pub fn http_client_default() -> Result<reqwest::Client, String> {
    http_client(15)
}
