use rairos_core::{Paper, PaperMetadata, ParseStatus};
use chrono::{DateTime, Utc, NaiveDate};
use std::collections::HashMap;

const ARXIV_API: &str = "http://export.arxiv.org/api/query";

pub fn search(query: &str, max_results: usize) -> Result<Vec<Paper>, String> {
    let url = format!(
        "{}?search_query=all:{}&start=0&max_results={}",
        ARXIV_API,
        urlencoding(query),
        max_results
    );

    let resp = reqwest::blocking::get(&url)
        .map_err(|e| format!("arXiv API request failed: {}", e))?;

    let text = resp.text().map_err(|e| format!("Failed to read response: {}", e))?;
    parse_arxiv_response(&text)
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
       .replace(':', "%3A")
       .replace('/', "%2F")
       .replace('(', "%28")
       .replace(')', "%29")
}

fn parse_date(s: &str) -> DateTime<Utc> {
    if let Ok(naive) = NaiveDate::parse_from_str(&s[..10.min(s.len())], "%Y-%m-%d") {
        return DateTime::from_naive_utc_and_offset(naive.and_hms_opt(0, 0, 0).unwrap_or_default(), Utc);
    }
    DateTime::default()
}

fn parse_arxiv_response(xml: &str) -> Result<Vec<Paper>, String> {
    let mut papers = Vec::new();

    let mut pos = 0;
    while let Some(entry_start) = xml[pos..].find("<entry>") {
        let abs_start = pos + entry_start;
        let Some(entry_end) = xml[abs_start..].find("</entry>") else {
            break;
        };
        let entry = &xml[abs_start..abs_start + entry_end + 8];
        pos = abs_start + entry_end + 8;

        let id = extract_tag(entry, "id").unwrap_or_default();
        let published = extract_tag(entry, "published").unwrap_or_default();
        let title = extract_tag(entry, "title").map(clean_xml).unwrap_or_default();
        let summary = extract_tag(entry, "summary").map(clean_xml).unwrap_or_default();
        let authors = extract_authors(entry);
        let categories = extract_categories(entry);
        let pdf_url = extract_pdf_url(entry);

        let arxiv_id = id
            .strip_prefix("http://arxiv.org/abs/")
            .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
            .map(|s| s.to_string());

        let id_str = arxiv_id.clone().unwrap_or_else(|| format!("paper_{}", papers.len()));

        papers.push(Paper {
            id: id_str.clone(),
            arxiv_id,
            title,
            authors,
            published: parse_date(&published),
            abstract_text: summary,
            categories,
            parse_status: ParseStatus::Pending,
            metadata: PaperMetadata {
                pdf_url: Some(pdf_url),
                ..Default::default()
            },
        });
    }

    Ok(papers)
}

fn extract_tag<'a>(s: &'a str, tag: &str) -> Option<String> {
    let start = s.find(&format!("<{}>", tag))?;
    let value_start = start + tag.len() + 2;
    let end = s[value_start..].find(&format!("</{}>", tag))?;
    Some(s[value_start..value_start + end].to_string())
}

fn clean_xml(s: String) -> String {
    s.trim()
        .replace('\n', " ")
        .replace("  ", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_authors(entry: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("<author>") {
        let abs_start = pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else { break; };
        let author_block = &entry[abs_start..abs_start + end + 9];
        if let Some(name) = extract_tag(author_block, "name") {
            authors.push(name);
        }
        pos = abs_start + end + 9;
    }
    authors
}

fn extract_categories(entry: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("term=\"") {
        let after = &entry[pos + start + 6..];
        if let Some(end) = after.find('"') {
            cats.push(after[..end].to_string());
        }
        pos += start + 6;
    }
    cats
}

fn extract_pdf_url(entry: &str) -> String {
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("<link") {
        let chunk = &entry[pos + start..];
        let Some(end) = chunk.find('>') else { break; };
        let link_tag = &chunk[..end];
        if link_tag.contains("title=\"pdf\"") {
            if let Some(href_start) = link_tag.find("href=\"") {
                let rest = &link_tag[href_start + 6..];
                if let Some(href_end) = rest.find('"') {
                    return rest[..href_end].to_string();
                }
            }
        }
        pos += start + end + 1;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arxiv_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><feed>
<entry>
  <id>http://arxiv.org/abs/2401.12345</id>
  <published>2024-01-01</published>
  <updated>2024-01-02</updated>
  <title>Test Title About Neural Networks</title>
  <summary>This is a test abstract about deep learning.</summary>
  <author><name>John Doe</name></author>
  <author><name>Jane Smith</name></author>
  <category term="cs.LG"/>
  <category term="stat.ML"/>
  <link href="http://arxiv.org/pdf/2401.12345.pdf" rel="alternate" title="pdf"/>
</entry></feed>"#;
        let papers = parse_arxiv_response(xml).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].id, "2401.12345");
        assert!(papers[0].title.contains("Test Title"));
        assert_eq!(papers[0].authors.len(), 2);
    }

    #[test]
    fn test_empty_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><feed></feed>"#;
        let papers = parse_arxiv_response(xml).unwrap();
        assert!(papers.is_empty());
    }
}
