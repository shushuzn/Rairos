use crate::handlers::helpers::{data_dir, parse_arxiv_citation};
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

fn format_author_human(authors: &[Value]) -> String {
    if authors.is_empty() {
        return String::new();
    }
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{} {}", given, family))
        }
    }).collect();
    if formatted.len() <= 6 {
        formatted.join(", ")
    } else {
        format!("{} et al.", formatted[0])
    }
}

fn generate_bibtex_key(authors: &[Value], year: &str, _title: &str) -> String {
    let first_author = authors.first()
        .and_then(|a| a.get("family").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    format!("{}{}", first_author.to_lowercase(), year)
}

fn format_authors_bibtex(authors: &[Value]) -> String {
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{{{}, {}}}", family, given))
        }
    }).collect();
    formatted.join(" and ")
}

pub struct PaperFormatCitationHandler;

#[async_trait]
impl ToolHandler for PaperFormatCitationHandler {
    fn name(&self) -> &str { "paper_format_citation" }
    fn description(&self) -> &str { "Format a paper citation in multiple styles (APA, Nature, Vancouver, Chicago, IEEE, BibTeX) from DOI, PMID, or arXiv ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("identifier".into(), ToolProperty::string("DOI (e.g., 10.1038/s41586-021-03819-2), PMID (e.g., 34265844), or arXiv ID (e.g., 2103.14030)")),
                ("style".into(), ToolProperty::string("Citation style: apa, nature, vancouver, chicago, ieee, bibtex, or all (default: all)")),
            ].into_iter().collect(),
            vec!["identifier".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let identifier = params["identifier"].as_str().ok_or("Missing identifier")?;
        let style = params.get("style").and_then(|v| v.as_str()).unwrap_or("all");

        let client = crate::handlers::helpers::http_client_default()?;

        let (metadata, id_type) = if identifier.starts_with("10.") {
            let url = format!("https://doi.org/{}", identifier);
            let resp = client.get(&url)
                .header("Accept", "application/json")
                .send().await.map_err(|e| format!("CrossRef request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("DOI not found: {}", identifier));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "doi".to_string())
        } else if identifier.chars().all(|c| c.is_ascii_digit()) {
            let url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
                identifier
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("PubMed request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("PubMed request failed: {}", resp.status()));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "pmid".to_string())
        } else if identifier.contains("/") || identifier.starts_with("arxiv:") {
            let arxiv_id = identifier.trim_start_matches("arxiv:");
            let url = format!(
                "https://export.arxiv.org/api/query?id_list={}&max_results=1",
                arxiv_id
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("arXiv request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("arXiv request failed: {}", resp.status()));
            }
            let body = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
            let parsed = parse_arxiv_citation(&body)?;
            (serde_json::json!({ "entry": parsed }), "arxiv".to_string())
        } else {
            return Err("Invalid identifier. Use DOI (10.xxxx), PMID (digits), or arXiv ID (e.g. 2103.14030)".into());
        };

        let mut title = String::new();
        let mut authors: Vec<Value> = Vec::new();
        let mut year = String::new();
        let mut journal = String::new();
        let mut volume = String::new();
        let mut issue = String::new();
        let mut pages = String::new();
        let mut doi = String::new();
        let mut url = String::new();

        if id_type == "doi" {
            if let Some(msg) = metadata.get("message").or(metadata.get("response")) {
                title = msg.get("title").and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if let Some(a) = msg.get("author").or(msg.get("author")).and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = msg.get("published").and_then(|v| v.get("date-parts"))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_i64())
                    .map(|y| y.to_string())
                    .unwrap_or_default();
                if year.is_empty() {
                    year = msg.get("created").and_then(|v| v.get("date-parts"))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.as_i64())
                        .map(|y| y.to_string())
                        .unwrap_or_default();
                }
                journal = msg.get("container-title")
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                volume = msg.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = msg.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = msg.get("page").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = msg.get("DOI").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                url = format!("https://doi.org/{}", doi);
            }
        } else if id_type == "pmid" {
            if let Some(result) = metadata.get("result").and_then(|v| v.get(identifier)) {
                title = result.get("title").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                if let Some(a) = result.get("authors").and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = result.get("pubdate").and_then(|v| v.as_str())
                    .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = result.get("source").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                volume = result.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = result.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = result.get("pages").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = result.get("elocationid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches("pii: ").to_string())
                    .unwrap_or_default();
                url = format!("https://pubmed.ncbi.nlm.nih.gov/{}", identifier);
            }
        } else if id_type == "arxiv" {
            if let Some(entry) = metadata.get("entry").or(metadata.as_array().and_then(|v| v.get(0))) {
                title = entry.get("title").and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if let Some(a) = entry.get("author").and_then(|v| v.as_array()) {
                    authors = a.iter().filter_map(|author| {
                        let name = author.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let parts: Vec<&str> = name.split_whitespace().collect();
                        let family = parts.last().map(|s| *s).unwrap_or("");
                        let given = if parts.len() > 1 { parts[..parts.len()-1].join(" ") } else { String::new() };
                        if family.is_empty() { None } else { Some(serde_json::json!({ "family": family, "given": given })) }
                    }).collect();
                }
                year = entry.get("published").or(entry.get("updated"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('-').next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = entry.get("journal-ref")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "arXiv preprint".to_string());
                url = entry.get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if entry.get("doi").and_then(|v| v.as_str()).is_some() {
                    doi = entry.get("doi").and_then(|v| v.as_str()).unwrap_or("").to_string();
                } else {
                    let arxiv_id_val = entry.get("arxiv_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(identifier);
                    doi = format!("10.48550/arXiv.{}", arxiv_id_val);
                    url = format!("https://arxiv.org/abs/{}", arxiv_id_val);
                }
            }
        }

        if title.is_empty() {
            return Err("Could not extract paper metadata".into());
        }

        let author_str = format_author_human(&authors);

        let mut citations = serde_json::json!({});

        if style == "all" || style == "apa" {
            citations["apa"] = serde_json::json!(format!(
                "{}. ({}). {}. {}{}{}{}.",
                author_str, year,
                title,
                if !journal.is_empty() { format!("{}. ", journal) } else { String::new() },
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "--")) } else { String::new() }
            ));
        }

        if style == "all" || style == "nature" {
            let nature_journal = if journal.is_empty() { String::new() } else { journal.clone() };
            citations["nature"] = serde_json::json!(format!(
                "{} {} {} {} {}{}{}.",
                author_str.split(',').next().unwrap_or(&author_str).split_whitespace().last().unwrap_or(""),
                if !year.is_empty() { &year } else { "s" },
                title,
                nature_journal,
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "-")) } else { String::new() },
                if !doi.is_empty() { format!(" https://doi.org/{}", doi) } else { String::new() }
            ));
        }

        if style == "all" || style == "vancouver" {
            let numbered_authors: Vec<String> = authors.iter().map(|a| {
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}{}", initials, family)
            }).collect();
            let vancouver_author = if numbered_authors.len() <= 6 {
                numbered_authors.join(", ")
            } else {
                format!("{} et al.", numbered_authors[..5].join(", "))
            };
            citations["vancouver"] = serde_json::json!(format!(
                "{} {}. {}. {}{}{}:{}",
                vancouver_author, year, title, journal,
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { pages.replace("-", "-") } else { "".into() }
            ));
        }

        if style == "all" || style == "chicago" {
            citations["chicago"] = serde_json::json!(format!(
                "{} \"{}\"{} {}{}{}{}.",
                author_str,
                title,
                if !journal.is_empty() { format!(", {}", journal) } else { String::new() },
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !year.is_empty() { format!(" ({})", year) } else { String::new() },
                if !pages.is_empty() { format!(": {}", pages.replace("-", "-")) } else { String::new() }
            ));
        }

        if style == "all" || style == "ieee" {
            let ieee_authors: Vec<String> = authors.iter().map(|a| {
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}. {}", initials, family)
            }).collect();
            let ieee_author = if ieee_authors.len() <= 3 {
                ieee_authors.join(", ")
            } else {
                format!("{} et al.", ieee_authors.iter().take(2).cloned().collect::<Vec<_>>().join(", "))
            };
            let ieee_str = format!(
                "{} {}, \"{}\" {}{}{}{}.",
                ieee_author, year, title,
                if !journal.is_empty() { format!("{}", journal) } else { String::new() },
                if !volume.is_empty() { format!(", vol. {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !pages.is_empty() { format!(", pp. {}", pages.replace("-", "--")) } else { String::new() }
            );
            citations["ieee"] = serde_json::json!(ieee_str);
        }

        if style == "all" || style == "bibtex" {
            let bibtex_key = generate_bibtex_key(&authors, &year, &title);
            let bibtex_authors = format_authors_bibtex(&authors);
            let bibtex_abstract = metadata.get("message")
                .and_then(|m| m.get("abstract"))
                .and_then(|v| v.as_str())
                .map(|s| format!("\n  abstract = {{{}}}", s.trim()))
                .unwrap_or_default();
            citations["bibtex"] = serde_json::json!(format!(
                "@article{{{},\n  author = {{{}}}\n  title = {{{}}}\n  journal = {{{}}}\n  year = {{{}}}{}{}{}{}{}\n}}",
                bibtex_key,
                bibtex_authors,
                title,
                journal,
                year,
                if !volume.is_empty() { format!("\n  volume = {{{}}}", volume) } else { String::new() },
                if !issue.is_empty() { format!("\n  number = {{{}}}", issue) } else { String::new() },
                if !pages.is_empty() { format!("\n  pages = {{{}}}", pages.replace("-", "--")) } else { String::new() },
                if !doi.is_empty() { format!("\n  doi = {{{}}}", doi) } else { String::new() },
                bibtex_abstract
            ));
        }

        Ok(serde_json::json!({
            "identifier": identifier,
            "id_type": id_type,
            "title": title,
            "authors": author_str,
            "year": year,
            "journal": journal,
            "doi": doi,
            "url": url,
            "citations": citations,
        }))
    }
}
