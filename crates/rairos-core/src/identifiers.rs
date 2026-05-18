//! rairos-identifiers — Canonical identifier parsing and normalization.
use std::sync::LazyLock;
use regex::Regex;

static DOI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(https?://(dx\.)?doi\.org/)?10\.\d{4,9}/\S+$").unwrap()
});

static DOI_STRIP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:https?://)?(?:dx\.)?doi\.org/").unwrap()
});

static ARXIV_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?").unwrap()
});

static ARXIV_NEW_STYLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{4}\.\d{4,5}(v\d+)?$").unwrap()
});

static ARXIV_OLD_STYLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z\-]+/\d{7}(v\d+)?$").unwrap()
});

static ARXIV_DOI_PREFIX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)10\.48550/arXiv\.(\d{4}\.\d{4,5})(v\d+)?").unwrap()
});

static ARXIV_PREFIX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^arxiv:(\d{4}\.\d{4,5})(v\d+)?$").unwrap()
});

pub fn is_probably_doi(s: &str) -> bool {
    DOI_REGEX.is_match(s.trim())
}

pub fn normalize_doi(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let stripped = DOI_STRIP_REGEX.replace(s, "");
    if !stripped.starts_with("10.") || !stripped.contains('/') {
        return None;
    }
    Some(stripped.trim_end_matches('.').trim().to_string())
}

pub fn normalize_arxiv_id(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(caps) = ARXIV_URL_REGEX.captures(s) {
        let id = caps.get(1).unwrap().as_str().to_string();
        let version = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        return Some(format!("{}{}", id, version));
    }
    if ARXIV_NEW_STYLE_REGEX.is_match(s) {
        return Some(s.to_string());
    }
    if ARXIV_OLD_STYLE_REGEX.is_match(s) {
        return Some(s.to_string());
    }
    if let Some(caps) = ARXIV_DOI_PREFIX_REGEX.captures(s) {
        let id = caps.get(1).unwrap().as_str().to_string();
        let version = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        return Some(format!("{}{}", id, version));
    }
    if let Some(caps) = ARXIV_PREFIX_REGEX.captures(s) {
        let id = caps.get(1).unwrap().as_str().to_string();
        let version = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        return Some(format!("{}{}", id, version));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierType {
    Doi,
    Arxiv,
    Unknown,
}

pub fn classify(s: &str) -> IdentifierType {
    let doi_match = is_probably_doi(s);
    let doi_norm = normalize_doi(s);
    let arxiv_norm = normalize_arxiv_id(s);
    if doi_match || doi_norm.is_some() {
        return IdentifierType::Doi;
    }
    if arxiv_norm.is_some() {
        return IdentifierType::Arxiv;
    }
    IdentifierType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_probably_doi() {
        assert!(is_probably_doi("10.1038/nature12373"));
        assert!(is_probably_doi("10.48550/arXiv.2401.12345"));
        assert!(is_probably_doi("https://doi.org/10.1038/nature12373"));
        assert!(is_probably_doi("https://dx.doi.org/10.1038/nature12373"));
        assert!(!is_probably_doi("2401.12345"));
        assert!(!is_probably_doi("not a doi"));
    }
    #[test]
    fn test_normalize_doi() {
        assert_eq!(
            normalize_doi("https://doi.org/10.1038/nature12373"),
            Some("10.1038/nature12373".to_string())
        );
        assert_eq!(
            normalize_doi("dx.doi.org/10.1038/nature12373"),
            Some("10.1038/nature12373".to_string())
        );
        assert_eq!(
            normalize_doi("10.1038/nature12373."),
            Some("10.1038/nature12373".to_string())
        );
        assert_eq!(normalize_doi(""), None);
        assert_eq!(normalize_doi("  "), None);
    }
    #[test]
    fn test_normalize_arxiv_new_style() {
        assert_eq!(
            normalize_arxiv_id("2401.12345"),
            Some("2401.12345".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("2401.12345v3"),
            Some("2401.12345v3".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_url_abs() {
        assert_eq!(
            normalize_arxiv_id("https://arxiv.org/abs/2401.12345"),
            Some("2401.12345".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("https://arxiv.org/abs/2401.12345v3"),
            Some("2401.12345v3".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_url_pdf() {
        assert_eq!(
            normalize_arxiv_id("https://arxiv.org/pdf/2401.12345.pdf"),
            Some("2401.12345".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("https://arxiv.org/pdf/2401.12345v2.pdf"),
            Some("2401.12345v2".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_old_style() {
        assert_eq!(
            normalize_arxiv_id("cs/0701234"),
            Some("cs/0701234".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("cs/0701234v2"),
            Some("cs/0701234v2".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_doi_prefix() {
        assert_eq!(
            normalize_arxiv_id("10.48550/arXiv.2401.12345"),
            Some("2401.12345".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("10.48550/arXiv.2401.12345v3"),
            Some("2401.12345v3".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_prefix() {
        assert_eq!(
            normalize_arxiv_id("arXiv:2401.12345"),
            Some("2401.12345".to_string())
        );
    }
    #[test]
    fn test_normalize_arxiv_invalid() {
        assert_eq!(normalize_arxiv_id(""), None);
        assert_eq!(normalize_arxiv_id("not an arxiv id"), None);
        assert_eq!(normalize_arxiv_id("10.1038/nature12373"), None);
    }
    #[test]
    fn test_classify() {
        assert_eq!(classify("10.1038/nature12373"), IdentifierType::Doi);
        assert_eq!(classify("2401.12345"), IdentifierType::Arxiv);
        assert_eq!(classify("cs/0701234"), IdentifierType::Arxiv);
        assert_eq!(classify("10.48550/arXiv.2401.12345"), IdentifierType::Doi);
        assert_eq!(classify("random string"), IdentifierType::Unknown);
    }
}
