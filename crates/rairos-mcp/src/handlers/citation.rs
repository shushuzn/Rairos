use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum CitationStyle {
    Apa,
    Nature,
    Vancouver,
}

impl CitationStyle {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "apa" => Some(CitationStyle::Apa),
            "nature" => Some(CitationStyle::Nature),
            "vancouver" => Some(CitationStyle::Vancouver),
            _ => None,
        }
    }
}

pub fn format_citation_apa(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() == 1 {
        authors[0].clone()
    } else if authors.len() == 2 {
        format!("{} & {}", authors[0], authors[1])
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} ({}). {}. *{}*. https://doi.org/{}", author_str, year, title, journal, doi)
}

pub fn format_citation_nature(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() <= 5 {
        authors.join(", ")
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} {} {} {} {}", author_str, title, journal, year, doi)
}

pub fn format_citation_vancouver(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() <= 6 {
        authors.join(", ")
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} {}. {}. {}. {}:{}:{}", author_str, title, journal, year, journal, year, doi)
}

pub struct CiteFetchHandler;

#[async_trait]
impl ToolHandler for CiteFetchHandler {
    fn name(&self) -> &str { "cite_fetch" }
    fn description(&self) -> &str { "Fetch citation metadata for a paper from Semantic Scholar" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID to fetch citations for")),
            ].into_iter().collect(),
            vec!["paper_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}?fields=title,citationCount,externalIds",
            paper_id
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let cited_by = data["citationCount"].as_u64().unwrap_or(0) as usize;
        let title = data["title"].as_str().unwrap_or("Unknown");

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "title": title,
            "cited_by_count": cited_by,
            "citations": [],
        }))
    }
}

pub struct PaperSearchMultiHandler;

#[async_trait]
impl ToolHandler for PaperSearchMultiHandler {
    fn name(&self) -> &str { "paper_search_multi" }
    fn description(&self) -> &str { "Search papers across multiple academic databases using Semantic Scholar" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Search query")),
                ("limit".into(), ToolProperty::integer("Maximum results (default 10, max 100)")),
                ("year_from".into(), ToolProperty::integer("Filter papers from this year")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing required parameter: query")?;
        let limit = (params["limit"].as_u64().unwrap_or(10) as usize).min(100);
        let year_from = params.get("year_from").and_then(|v| v.as_u64()).map(|y| y as i32);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let mut url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields=title,year,abstract,citationCount,authors,openAccessPdf,externalIds&limit={}",
            urlencoding::encode(query),
            limit
        );
        if let Some(year) = year_from {
            url.push_str(&format!("&year={}-", year));
        }

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let results = data["data"].as_array().cloned().unwrap_or_default();
        let total = data["total"].as_u64().unwrap_or(0);

        let papers: Vec<Value> = results.into_iter().map(|p| {
            serde_json::json!({
                "title": p["title"].as_str().unwrap_or(""),
                "year": p["year"].as_i64().unwrap_or(0),
                "abstract": p["abstract"].as_str().unwrap_or(""),
                "citationCount": p["citationCount"].as_u64().unwrap_or(0),
                "openAccessPdf": p["openAccessPdf"]["url"].as_str(),
                "doi": p["externalIds"]["DOI"].as_str(),
                "arxivId": p["externalIds"]["ArXiv"].as_str(),
                "authors": p["authors"].as_array().map(|arr| arr.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>()).unwrap_or_default(),
            })
        }).collect();

        Ok(serde_json::json!({"papers": papers, "total": total}))
    }
}

pub struct PaperLookupDoiHandler;

#[async_trait]
impl ToolHandler for PaperLookupDoiHandler {
    fn name(&self) -> &str { "paper_lookup_doi" }
    fn description(&self) -> &str { "Look up paper metadata by DOI using Crossref" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("doi".into(), ToolProperty::string("DOI to look up")),
            ].into_iter().collect(),
            vec!["doi".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let doi = params["doi"].as_str().ok_or("Missing required parameter: doi")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let url = format!(
            "https://api.crossref.org/works/{}?mailto=rairos@example.com",
            urlencoding::encode(doi)
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Lookup failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Crossref API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let msg = &data["message"];

        let authors: Vec<String> = if let Some(author_arr) = msg["author"].as_array() {
            author_arr.iter()
                .map(|a| {
                    let given = a["given"].as_str().unwrap_or("");
                    let family = a["family"].as_str().unwrap_or("");
                    format!("{} {}", given, family).trim().to_string()
                })
                .collect()
        } else {
            Vec::new()
        };

        let title = msg["title"].as_array().and_then(|t| t[0].as_str()).unwrap_or("");
        let journal = msg["container-title"].as_array().and_then(|j| j[0].as_str()).unwrap_or("");
        let year = msg["published"]["date-parts"]
            .as_array()
            .and_then(|d| d[0].as_array())
            .and_then(|y| y[0].as_i64())
            .unwrap_or(0);

        Ok(serde_json::json!({
            "doi": doi,
            "title": title,
            "authors": authors,
            "journal": journal,
            "year": year,
            "citedByCount": msg["is-referenced-by-count"].as_u64().unwrap_or(0),
            "abstract": msg["abstract"].as_str().unwrap_or(""),
        }))
    }
}

pub struct PaperCitationsHandler;

#[async_trait]
impl ToolHandler for PaperCitationsHandler {
    fn name(&self) -> &str { "paper_citations" }
    fn description(&self) -> &str { "Get citation chain for a paper (papers that cite it and papers it cites)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID, DOI, or arXiv ID")),
                ("limit".into(), ToolProperty::integer("Maximum citations per direction (default 20, max 100)")),
            ].into_iter().collect(),
            vec!["paper_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing required parameter: paper_id")?;
        let limit = (params["limit"].as_u64().unwrap_or(20) as usize).min(100);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let fields = "title,year,citationCount,externalIds";
        let citing_url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}/citations?fields={}&limit={}",
            paper_id, fields, limit
        );
        let refs_url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}/references?fields={}&limit={}",
            paper_id, fields, limit
        );

        let citing_resp = client.get(&citing_url).send().await.map_err(|e| format!("Request failed: {}", e))?;
        let refs_resp = client.get(&refs_url).send().await.map_err(|e| format!("Request failed: {}", e))?;

        let citing: Vec<Value> = if citing_resp.status().is_success() {
            if let Ok(data) = citing_resp.json::<serde_json::Value>().await {
                data["data"].as_array().cloned().unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let citing = &p["citingPaper"];
                        serde_json::json!({
                            "paperId": citing["paperId"].as_str().unwrap_or(""),
                            "title": citing["title"].as_str().unwrap_or(""),
                            "year": citing["year"].as_i64().unwrap_or(0),
                            "citationCount": citing["citationCount"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            } else { Vec::new() }
        } else { Vec::new() };

        let references: Vec<Value> = if refs_resp.status().is_success() {
            if let Ok(data) = refs_resp.json::<serde_json::Value>().await {
                data["data"].as_array().cloned().unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let referenced = &p["referencedPaper"];
                        serde_json::json!({
                            "paperId": referenced["paperId"].as_str().unwrap_or(""),
                            "title": referenced["title"].as_str().unwrap_or(""),
                            "year": referenced["year"].as_i64().unwrap_or(0),
                            "citationCount": referenced["citationCount"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            } else { Vec::new() }
        } else { Vec::new() };

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "citing": citing,
            "references": references,
            "citingCount": citing.len(),
            "referenceCount": references.len(),
        }))
    }
}

pub struct PaperVerifyCitationsHandler;

#[async_trait]
impl ToolHandler for PaperVerifyCitationsHandler {
    fn name(&self) -> &str { "paper_verify_citations" }
    fn description(&self) -> &str { "Verify DOIs and format citations in APA, Nature, or Vancouver style" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("dois".into(), ToolProperty::string("Comma-separated DOIs to verify")),
                ("style".into(), ToolProperty::string("Citation style: apa, nature, or vancouver (default: apa)")),
            ].into_iter().collect(),
            vec!["dois".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let dois_str = params["dois"].as_str().ok_or("Missing required parameter: dois")?;
        let style_str = params.get("style").and_then(|v| v.as_str()).unwrap_or("apa");
        let style = CitationStyle::from_str(style_str).ok_or_else(|| format!("Invalid style: {}. Use apa, nature, or vancouver.", style_str))?;

        let dois: Vec<&str> = dois_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if dois.is_empty() {
            return Err("No DOIs provided".to_string());
        }
        if dois.len() > 50 {
            return Err("Maximum 50 DOIs at a time".to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let mut results = Vec::new();
        for doi in dois {
            let url = format!(
                "https://api.crossref.org/works/{}?mailto=rairos@example.com",
                urlencoding::encode(doi)
            );

            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    results.push(serde_json::json!({
                        "doi": doi,
                        "verified": false,
                        "error": format!("Request failed: {}", e),
                    }));
                    continue;
                }
            };

            if !resp.status().is_success() {
                results.push(serde_json::json!({
                    "doi": doi,
                    "verified": false,
                    "error": format!("Crossref API returned {}", resp.status()),
                }));
                continue;
            }

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    results.push(serde_json::json!({
                        "doi": doi,
                        "verified": false,
                        "error": format!("Parse failed: {}", e),
                    }));
                    continue;
                }
            };

            let msg = &data["message"];

            let authors: Vec<String> = if let Some(author_arr) = msg["author"].as_array() {
                author_arr.iter()
                    .map(|a| {
                        let given = a["given"].as_str().unwrap_or("");
                        let family = a["family"].as_str().unwrap_or("");
                        format!("{} {}", given, family).trim().to_string()
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let title = msg["title"].as_array().and_then(|t| t[0].as_str()).unwrap_or("");
            let journal = msg["container-title"].as_array().and_then(|j| j[0].as_str()).unwrap_or("");
            let year = msg["published"]["date-parts"]
                .as_array()
                .and_then(|d| d[0].as_array())
                .and_then(|y| y[0].as_i64())
                .unwrap_or(0);

            let formatted = match style {
                CitationStyle::Apa => format_citation_apa(&authors, title, journal, year, doi),
                CitationStyle::Nature => format_citation_nature(&authors, title, journal, year, doi),
                CitationStyle::Vancouver => format_citation_vancouver(&authors, title, journal, year, doi),
            };

            results.push(serde_json::json!({
                "doi": doi,
                "verified": true,
                "title": title,
                "authors": authors,
                "journal": journal,
                "year": year,
                "formatted": formatted,
            }));
        }

        let verified_count = results.iter().filter(|r| r["verified"].as_bool().unwrap_or(false)).count();
        Ok(serde_json::json!({
            "citations": results,
            "total": results.len(),
            "verified": verified_count,
            "style": style_str,
        }))
    }
}
