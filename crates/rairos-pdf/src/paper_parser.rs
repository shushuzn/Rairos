//! Paper Parser — Download and parse research papers into structured PaperContent.
//!
//! Used by the paper2code integration pipeline:
//!   `download_and_parse(arxiv_id) → PaperContent`
//!   `parse_existing_pdf(pdf_path, arxiv_id) → PaperContent`
//!
//! PaperContent feeds into:
//!   - code_generator: generate code skeleton from paper content
//!   - test_extractor: extract testable assertions from paper content
//!
//! Python original: `research_loop/paper_parser.py` (325 lines)

use crate::provenance::{AlgorithmSource, ClaimSource, EquationSource, PaperLocation};
use rairos_code_generator::PaperContent as CodeGenPaperContent;
use regex::Regex;
use std::sync::LazyLock;

// Algorithm fingerprint regex patterns
static RE_EQ_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:softmax|attention|matmul|@|linear|layer.?norm|residual|dropout|encoder|decoder|self.?attention|cross.?attention|multi.?head|positional|embedding|relu|gelu|swiglu|feed.?forward|normalization|convolution|pooling|gru|lstm|rnn|transformer|cross.?entropy|BCE|CE|adam|sgd|rmsprop|weight)").expect("valid regex")
});

static RE_STRIP_VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-_]?[0-9]+$").expect("valid regex")
});

static RE_STRIP_NON_ALPHA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^a-z]+").expect("valid regex")
});

static RE_ARXIV_ID_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"v\d+$").expect("valid regex")
});
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperContent {
    /// ArXiv identifier
    pub arxiv_id: String,
    /// Paper title
    pub title: String,
    /// List of author names
    pub authors: Vec<String>,
    /// Paper abstract
    pub abstract_text: String,
    /// Publication date
    pub published: String,
    /// Last updated date
    pub updated: String,
    /// Algorithm descriptions extracted from PDF
    pub algorithm_descriptions: Vec<String>,
    /// Equations extracted from PDF
    pub equations: Vec<String>,
    /// Claims extracted from PDF
    pub claims: Vec<String>,
    /// Hyperparameters extracted from PDF
    pub hyperparameters: HashMap<String, String>,
    /// Datasets mentioned in paper
    pub datasets: Vec<String>,
    /// Methods used in the paper
    pub methods: Vec<String>,
    /// ArXiv categories
    pub categories: Vec<String>,
    /// Cross-paper dedup: structural fingerprint of the algorithm
    pub algorithm_fingerprint: String,
    /// Provenance: source locations for extracted content
    pub equation_sources: Vec<EquationSource>,
    pub claim_sources: Vec<ClaimSource>,
    pub algorithm_sources: Vec<AlgorithmSource>,
}

impl From<PaperContent> for CodeGenPaperContent {
    fn from(pc: PaperContent) -> Self {
        CodeGenPaperContent {
            title: pc.title,
            arxiv_id: pc.arxiv_id,
            abstract_text: pc.abstract_text,
            authors: pc.authors,
            algorithm_descriptions: pc.algorithm_descriptions,
            equations: pc.equations,
            claims: pc.claims,
            hyperparameters: pc.hyperparameters,
            datasets: pc.datasets,
            methods: pc.methods,
        }
    }
}

// ============================================================================
// Algorithm Fingerprint
// ============================================================================

/// Compute a structural fingerprint of an algorithm from paper content.
///
/// Two papers implementing the same algorithm (e.g., "Attention is All You Need"
/// variants) should produce the same fingerprint even with different notation.
/// This enables cross-paper dedup: same algorithm → same fingerprint.
///
/// Fingerprint is derived from:
/// 1. Equation structure (variables, operations, layout) — stripped of notation variants
/// 2. Method names (e.g., "self-attention", "feed-forward")
/// 3. Hyperparameter names (not values) — structural signature
pub fn compute_algorithm_fingerprint(content: &PaperContent) -> String {
    let mut signals: Vec<String> = Vec::new();

    // 1. Equations: extract structural skeleton (op signature only, no vars)
    for eq in &content.equations {
        let eq_lower = eq.to_lowercase();
        let ops: Vec<String> = RE_EQ_OPS
            .find_iter(&eq_lower)
            .map(|m| m.as_str().to_string())
            .collect();
        if !ops.is_empty() {
            let mut sorted_ops = ops;
            sorted_ops.sort();
            sorted_ops.dedup();
            signals.push(format!("eq:{}", sorted_ops.join("|")));
        }
    }

    // 2. Method names — canonical form with synonym collapsing
    let synonym_groups: Vec<Vec<&str>> = vec![
        vec![
            "feedforward",
            "feedforwardnetwork",
            "feedforwardlayer",
            "feedforwardblock",
            "feedforwardsublayer",
        ],
        vec!["selfattention", "selfattention"],
        vec!["multiheadattention", "multihead"],
        vec!["residual", "residualconnection", "skipconnection"],
        vec!["encoder", "encoderlayer", "encoderblock"],
        vec!["decoder", "decoderlayer", "decoderblock"],
        vec![
            "attention",
            "selfattention",
            "crossattention",
            "multiheadattention",
        ],
        vec!["layer_norm", "layernorm", "ln"],
        vec!["convolution", "conv", "convlayer"],
    ];

    for method in &content.methods {
        let mut m = method.to_lowercase();
        m = RE_STRIP_VERSION.replace_all(&m, "").to_string();
        m = RE_STRIP_NON_ALPHA.replace_all(&m, "").to_string();

        // Collapse synonym groups to canonical name
        for group in &synonym_groups {
            if group.contains(&m.as_str()) {
                m = group[0].to_string();
                break;
            }
        }

        if !m.is_empty() {
            signals.push(format!("method:{}", m));
        }
    }

    // 3. Hyperparameter names (structural, not values)
    let mut hp_names: Vec<String> = content.hyperparameters.keys().cloned().collect();
    hp_names.sort();
    if !hp_names.is_empty() {
        signals.push(format!("hpn:{}", hp_names.join("|")));
    }

    // 4. Datasets intentionally excluded — same algorithm can be evaluated on
    // different benchmarks (WMT, Wikitext, etc.). Dataset differences should NOT
    // make two implementations of the same algorithm look different.

    let combined = signals.join(";");
    let hash = Sha256::digest(combined.as_bytes());
    hash[..8].iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// ArXiv API Types
// ============================================================================

const ARXIV_API: &str = "https://export.arxiv.org/api/query";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ArXivFeed {
    #[serde(rename = "entry")]
    entries: Vec<ArXivEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ArXivEntry {
    #[serde(rename = "id")]
    entry_id: String,
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "summary")]
    summary: String,
    #[serde(rename = "author")]
    authors: Vec<ArXivAuthor>,
    #[serde(rename = "published")]
    published: String,
    #[serde(rename = "updated")]
    updated: Option<String>,
    #[serde(rename = "category")]
    categories: Vec<ArXivCategory>,
    #[serde(rename = "link")]
    links: Vec<ArXivLink>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ArXivAuthor {
    #[serde(rename = "name")]
    name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ArXivCategory {
    #[serde(rename = "term")]
    term: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ArXivLink {
    #[serde(rename = "href")]
    href: String,
    #[serde(rename = "type")]
    link_type: Option<String>,
    #[serde(rename = "title")]
    title: Option<String>,
}

fn extract_text_from_xml(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let content_start = start + open.len();
        if let Some(end) = xml[content_start..].find(&close) {
            return Some(xml[content_start..content_start + end].trim().to_string());
        }
    }
    None
}

fn extract_author_names(xml: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let name_re = Regex::new(r"<author>.*?<name>([^<]+)</name>.*?</author>").expect("valid regex");
    for cap in name_re.captures_iter(xml) {
        if let Some(name) = cap.get(1) {
            authors.push(name.as_str().to_string());
        }
    }
    authors
}

fn extract_categories(xml: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let cat_re = Regex::new(r#"<category term="([^"]+)""#).expect("valid regex");
    for cap in cat_re.captures_iter(xml) {
        if let Some(cat) = cap.get(1) {
            cats.push(cat.as_str().to_string());
        }
    }
    cats
}

#[allow(dead_code)]
fn extract_pdf_url(links: &[ArXivLink]) -> Option<String> {
    for link in links {
        if link.link_type.as_deref() == Some("application/pdf") {
            return Some(link.href.clone());
        }
        if link.title.as_deref() == Some("pdf") {
            return Some(link.href.clone());
        }
    }
    None
}

// ============================================================================
// Download and Parse
// ============================================================================

/// Download paper metadata from arXiv API and parse into PaperContent.
///
/// Uses the arXiv API for metadata extraction.
/// Falls back to minimal metadata if API is unavailable.
pub async fn download_and_parse(arxiv_id: &str) -> PaperContent {
    // Normalize arxiv_id
    let aid = arxiv_id
        .trim()
        .split(".org/abs/")
        .last()
        .unwrap_or(arxiv_id.trim())
        .trim();
    let aid = RE_ARXIV_ID_SUFFIX
        .replace_all(aid, "")
        .to_string();

    // Fetch from arXiv API
    let url = format!("{}?id={}", ARXIV_API, aid);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.text().await {
                    Ok(xml) => {
                        return parse_arxiv_response(&xml, &aid);
                    }
                    Err(_) => {
                        return _minimal_content(&aid);
                    }
                }
            }
            _minimal_content(&aid)
        }
        Err(_) => _minimal_content(&aid),
    }
}

fn parse_arxiv_response(xml: &str, arxiv_id: &str) -> PaperContent {
    let title = extract_text_from_xml(xml, "title")
        .map(|s| s.replace('\n', " ").trim().to_string())
        .unwrap_or_else(|| format!("Paper {}", arxiv_id));

    let summary = extract_text_from_xml(xml, "summary")
        .map(|s| s.replace('\n', " ").trim().to_string())
        .unwrap_or_default();

    let authors = extract_author_names(xml);

    let published = extract_text_from_xml(xml, "published").unwrap_or_default();

    let updated = extract_text_from_xml(xml, "updated").unwrap_or_default();

    let categories = extract_categories(xml);

    // Extract PDF URL from links
    let link_re = Regex::new(r#"<link href="([^"]+\.pdf)" type="application/pdf""#).expect("valid regex");
    let pdf_url = link_re
        .captures_iter(xml)
        .next()
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    let mut content = PaperContent {
        arxiv_id: arxiv_id.to_string(),
        title,
        authors,
        abstract_text: summary,
        published,
        updated,
        categories,
        ..Default::default()
    };

    // Download and enrich from PDF if available
    if let Some(url) = pdf_url {
        if let Ok(pdf_data) = tokio::runtime::Handle::current().block_on(download_pdf(&url)) {
            let pdf_path_str = format!("{}.pdf", arxiv_id.replace(['.', '/'], "_"));
            let pdf_path = Path::new(&pdf_path_str);
            if std::fs::write(pdf_path, &pdf_data).is_ok() {
                _enrich_from_pdf(&mut content, pdf_path);
                let _ = std::fs::remove_file(pdf_path);
            }
        }
    }

    content
}

async fn download_pdf(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client.get(url).send().await?;
    let bytes = resp.bytes().await?;
    Ok(bytes.to_vec())
}

/// Parse an already-downloaded PDF into PaperContent.
pub fn parse_existing_pdf(pdf_path: &str, arxiv_id: &str) -> PaperContent {
    let path = Path::new(pdf_path);
    if !path.exists() {
        return _minimal_content(arxiv_id);
    }

    let mut content = _minimal_content(arxiv_id);
    _enrich_from_pdf(&mut content, path);
    content
}

// ============================================================================
// PDF Enrichment
// ============================================================================

/// Extract algorithm descriptions, equations, claims from PDF text with provenance.
fn _enrich_from_pdf(content: &mut PaperContent, pdf_path: &Path) {
    let Ok(doc) = lopdf::Document::load(pdf_path) else {
        return;
    };

    // Extract text from all pages
    let mut pages_text: Vec<String> = Vec::new();
    let mut page_offsets: Vec<usize> = Vec::new();

    let mut current_offset = 0;
    for page_num in 1..=doc.get_pages().len() as u32 {
        if let Ok(text) = doc.extract_text(&[page_num]) {
            page_offsets.push(current_offset);
            pages_text.push(text);
            current_offset += pages_text.last().map(|s| s.len()).unwrap_or(0);
        }
    }

    let full_text = pages_text.join("\n");
    let text_lower = full_text.to_lowercase();

    fn match_to_location(char_start: usize, page_offsets: &[usize]) -> PaperLocation {
        #[allow(clippy::filter_next)]
        let page_idx = page_offsets
            .iter()
            .enumerate()
            .filter(|(_, &off)| char_start >= off)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        PaperLocation {
            section: "unknown".to_string(),
            page: (page_idx + 1) as u32,
            char_start: char_start as u32,
            char_end: char_start as u32,
        }
    }

    // Algorithm descriptions: look for "algorithm", "method", "approach" sections
    let algo_pattern = Regex::new(
        r"(?:algorithm|method|approach|procedure|technique)s?[:\s]+([A-Z][^.!?\n]{50,500}?(?:\d+\.|\n){1,3})"
    ).expect("valid regex");

    for cap in algo_pattern.captures_iter(&full_text[..10000.min(full_text.len())]) {
        if let Some(desc) = cap.get(1) {
            let desc_str = desc.as_str().trim();
            if desc_str.len() > 30 {
                let idx = content.algorithm_descriptions.len() as u32;
                let loc = match_to_location(desc.start(), &page_offsets);
                content
                    .algorithm_descriptions
                    .push(desc_str[..300.min(desc_str.len())].to_string());
                content.algorithm_sources.push(AlgorithmSource {
                    index: idx,
                    description: desc_str[..300.min(desc_str.len())].to_string(),
                    location: loc,
                });
            }
        }
    }

    // Equations: look for display math
    let eq_pattern = Regex::new(r"\$\$(.+?)\$\$|\$(.+?)\$").expect("valid regex");
    for cap in eq_pattern.captures_iter(&full_text) {
        let eq = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().trim())
            .unwrap_or("");
        if !eq.is_empty() && eq.len() > 5 {
            let idx = content.equations.len() as u32;
            let loc = match_to_location(cap.get(0).expect("valid regex").start(), &page_offsets);
            content.equations.push(eq[..200.min(eq.len())].to_string());
            content.equation_sources.push(EquationSource {
                index: idx,
                equation: eq[..200.min(eq.len())].to_string(),
                location: loc,
            });
        }
    }

    // Claims: look for "we show", "prove", "demonstrate", "our results"
    let claim_patterns = [
        r"(?:we show|we prove|we demonstrate|our results? show)[^.!?\n]{10,200}",
        r"(?:the (?:model|method|algorithm) achieves?|performance reaches?)[^.!?\n]{10,200}",
    ];

    for pat in &claim_patterns {
        let re = Regex::new(pat).expect("valid regex");
        for cap in re.captures_iter(&text_lower) {
            if let Some(m) = cap.get(0) {
                let claim = m.as_str().trim();
                if claim.len() > 20 {
                    let idx = content.claims.len() as u32;
                    let loc = match_to_location(m.start(), &page_offsets);
                    content
                        .claims
                        .push(claim[..300.min(claim.len())].to_string());
                    content.claim_sources.push(ClaimSource {
                        index: idx,
                        claim: claim[..300.min(claim.len())].to_string(),
                        location: loc,
                    });
                }
            }
        }
    }

    // Hyperparameters: look for "learning rate", "batch size", etc.
    let hp_patterns = [
        (r"learning\s*rate[:\s]+[\d.e\-]+", "learning_rate"),
        (r"batch\s*size[:\s]+\d+", "batch_size"),
        (r"epochs?[:\s]+\d+", "epochs"),
        (r"dropout[:\s]+[\d.]+", "dropout"),
        (r"hidden\s*layer[s]?[:\s]+\d+", "hidden_size"),
    ];

    for (pat, name) in &hp_patterns {
        let re = Regex::new(pat).expect("valid regex");
        for cap in re.captures_iter(&text_lower) {
            if let Some(m) = cap.get(0) {
                let val = m.as_str().split(':').next_back().unwrap_or("").trim();
                if !val.is_empty() {
                    content
                        .hyperparameters
                        .insert(name.to_string(), val.to_string());
                }
            }
        }
    }

    // Datasets: look for common dataset names
    let dataset_names = [
        "imagenet",
        "cifar-10",
        "cifar-100",
        "mnist",
        "wikitext",
        "glue",
        "squad",
        "arxiv",
        "pubmed",
        "openwebtext",
        "pile",
        "the pile",
        "alpaca",
        "dolly",
        "hh-rlhf",
    ];

    let text_lower_short = text_lower[..20000.min(text_lower.len())].to_string();
    for ds in &dataset_names {
        if text_lower_short.contains(ds) {
            content.datasets.push(ds.to_string());
        }
    }
}

/// Return minimal PaperContent when full parsing fails.
fn _minimal_content(input: &str) -> PaperContent {
    // Handle full URL or bare arxiv_id
    let aid = input
        .trim()
        .split(".org/abs/")
        .last()
        .unwrap_or(input.trim())
        .trim();
    let aid = RE_ARXIV_ID_SUFFIX
        .replace_all(aid, "")
        .to_string();

    PaperContent {
        arxiv_id: aid.clone(),
        title: format!("Paper {}", aid),
        ..Default::default()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_empty() {
        let content = PaperContent::default();
        let fp = compute_algorithm_fingerprint(&content);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 16);
    }

    #[test]
    fn test_fingerprint_same_algorithm() {
        let mut content1 = PaperContent::default();
        content1
            .equations
            .push("Attention(Q,K,V) = softmax(QK^T / sqrt(d_k))V".to_string());
        content1.methods.push("multi-head attention".to_string());

        let mut content2 = PaperContent::default();
        content2
            .equations
            .push("Attention(Q,K,V) = softmax(QK^T / sqrt(d))V".to_string());
        content2.methods.push("multiheadattention".to_string());

        let fp1 = compute_algorithm_fingerprint(&content1);
        let fp2 = compute_algorithm_fingerprint(&content2);

        // Same structural fingerprint expected due to synonym collapsing
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_minimal_content() {
        let content = _minimal_content("1706.03762v5");
        assert_eq!(content.arxiv_id, "1706.03762");
        assert_eq!(content.title, "Paper 1706.03762");
    }

    #[test]
    fn test_minimal_content_with_url() {
        let content = _minimal_content("https://arxiv.org/abs/1706.03762v1");
        assert_eq!(content.arxiv_id, "1706.03762");
    }

    #[test]
    fn test_paper_content_default() {
        let pc = PaperContent::default();
        assert!(pc.algorithm_descriptions.is_empty());
        assert!(pc.equations.is_empty());
        assert!(pc.claims.is_empty());
        assert!(pc.hyperparameters.is_empty());
        assert!(pc.datasets.is_empty());
        assert!(pc.equation_sources.is_empty());
        assert!(pc.claim_sources.is_empty());
        assert!(pc.algorithm_sources.is_empty());
    }

    #[test]
    fn test_paper_content_from_trait() {
        let pc = PaperContent {
            arxiv_id: "1706.03762".to_string(),
            title: "Attention Is All You Need".to_string(),
            authors: vec!["Vaswani".to_string()],
            abstract_text: "The dominant sequence transduction models.".to_string(),
            published: "2017-06-12".to_string(),
            updated: "".to_string(),
            algorithm_descriptions: vec!["Transformer architecture".to_string()],
            equations: vec!["Attention(Q,K,V) = softmax(QK^T / sqrt(d_k))V".to_string()],
            claims: vec!["Outperforms existing models".to_string()],
            hyperparameters: HashMap::new(),
            datasets: vec![],
            methods: vec![],
            categories: vec!["cs.CL".to_string()],
            algorithm_fingerprint: "".to_string(),
            equation_sources: vec![],
            claim_sources: vec![],
            algorithm_sources: vec![],
        };

        let _: CodeGenPaperContent = pc.into();
        // If it compiles, the trait impl works
    }
}
