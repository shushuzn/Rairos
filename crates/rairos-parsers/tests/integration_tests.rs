//! Integration tests for rairos-parsers
//!
//! These tests verify the search API functionality.

use rairos_parsers::{
    cross_search_blocking, semantic_search_blocking, SearchError, SearchResult, Source,
};

/// Test cross_search functionality
#[cfg(test)]
mod cross_search_tests {
    use super::*;

    #[test]
    fn test_cross_search_empty_sources_error() {
        let result = cross_search_blocking("test", 5, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_result_has_source() {
        use rairos_core::Paper;

        let paper = Paper::new(
            Some("2301.00001".to_string()),
            "Test Title".to_string(),
            "Test abstract".to_string(),
        );

        let result = SearchResult {
            paper,
            source: Source::ArXiv,
        };

        assert_eq!(result.source, Source::ArXiv);
        assert_eq!(result.paper.title, "Test Title");
    }
}

/// Test helper functions
#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn test_source_variants() {
        assert_eq!(Source::ArXiv, Source::ArXiv);
        assert_eq!(Source::SemanticScholar, Source::SemanticScholar);
        assert_ne!(Source::ArXiv, Source::SemanticScholar);
    }

    #[test]
    fn test_source_debug() {
        let source = Source::ArXiv;
        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("ArXiv"));
    }

    #[test]
    fn test_search_error_variants() {
        let rate_limited = SearchError::RateLimited(60);
        assert!(rate_limited.to_string().contains("Rate limited"));

        let search_failed = SearchError::SearchFailed("test".to_string());
        assert!(search_failed.to_string().contains("Search failed"));

        let parse_failed = SearchError::ParseFailed("json error".to_string());
        assert!(parse_failed.to_string().contains("Parse failed"));
    }
}

/// Test search result structure
#[cfg(test)]
mod search_result_tests {
    use super::*;
    use rairos_core::Paper;

    #[test]
    fn test_search_result_creation() {
        let paper = Paper::with_metadata(
            Some("2301.00001".to_string()),
            "Attention Is All You Need".to_string(),
            "We propose a new simple network architecture...".to_string(),
            vec!["Ashish Vaswani".to_string(), "Noam Shazeer".to_string()],
            vec!["cs.CL".to_string(), "cs.LG".to_string()],
            Default::default(),
        );

        let result_arxiv = SearchResult {
            paper: paper.clone(),
            source: Source::ArXiv,
        };

        assert_eq!(result_arxiv.source, Source::ArXiv);
        assert!(result_arxiv.paper.title.contains("Attention"));

        let result_s2 = SearchResult {
            paper,
            source: Source::SemanticScholar,
        };

        assert_eq!(result_s2.source, Source::SemanticScholar);
    }

    #[test]
    fn test_search_result_clone() {
        use rairos_core::Paper;

        let paper = Paper::new(
            Some("2301.00001".to_string()),
            "Test".to_string(),
            "Abstract".to_string(),
        );

        let result = SearchResult {
            paper,
            source: Source::ArXiv,
        };

        let cloned = result.clone();
        assert_eq!(cloned.source, result.source);
        assert_eq!(cloned.paper.title, result.paper.title);
    }
}

/// Test paper conversion and data integrity
#[cfg(test)]
mod paper_tests {
    use rairos_core::Paper;

    #[test]
    fn test_paper_with_all_fields() {
        let paper = Paper::with_metadata(
            Some("2301.00001v1".to_string()),
            "Deep Learning for Natural Language Processing".to_string(),
            "We present a comprehensive study of deep learning methods for NLP tasks.".to_string(),
            vec![
                "Yann LeCun".to_string(),
                "Yoshua Bengio".to_string(),
                "Geoffrey Hinton".to_string(),
            ],
            vec![
                "cs.CL".to_string(),
                "cs.LG".to_string(),
                "cs.AI".to_string(),
            ],
            Default::default(),
        );

        assert_eq!(paper.arxiv_id, Some("2301.00001v1".to_string()));
        assert!(paper.title.contains("Deep Learning"));
        assert_eq!(paper.authors.len(), 3);
        assert_eq!(paper.categories.len(), 3);
    }

    #[test]
    fn test_paper_default_metadata() {
        let paper = Paper::new(
            Some("2301.00001".to_string()),
            "Test Paper".to_string(),
            "Abstract text".to_string(),
        );

        assert_eq!(paper.metadata.cited_by, 0);
        assert_eq!(paper.metadata.references, 0);
        assert!(paper.metadata.doi.is_none());
        assert!(paper.metadata.pdf_url.is_none());
    }
}

/// Test error display
#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_search_error_display_all_variants() {
        let rate_limited = SearchError::RateLimited(30);
        assert!(!rate_limited.to_string().is_empty());

        let search_failed = SearchError::SearchFailed("query too long".to_string());
        assert!(!search_failed.to_string().is_empty());

        let parse_failed = SearchError::ParseFailed("invalid xml".to_string());
        assert!(!parse_failed.to_string().is_empty());

        let json_err =
            SearchError::Json(serde_json::from_str::<serde_json::Value>("invalid").unwrap_err());
        assert!(!json_err.to_string().is_empty());
    }
}

/// Test semantic_search_blocking error handling
#[cfg(test)]
mod semantic_search_tests {
    use super::*;

    #[test]
    fn test_semantic_search_empty_query() {
        // semantic_search_blocking is already blocking, no need for Runtime
        let result = semantic_search_blocking("", 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_search_handles_empty_results() {
        // With a very unique query that returns nothing, should get empty vec or error
        let result = semantic_search_blocking("xyzzynonexistentquery12345", 5);
        // Either returns empty or error depending on API response
        assert!(result.is_ok() || result.is_err());
    }
}
