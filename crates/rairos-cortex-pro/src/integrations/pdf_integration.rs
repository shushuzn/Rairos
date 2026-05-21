//! PDF and literature integration for rairos-cortex-pro.
//!
//! Provides functionality for parsing PDFs, extracting entities,
//! and mining literature for materials science research.

use serde::{Deserialize, Serialize};

/// A parsed document from PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    /// Document ID
    pub id: String,
    /// Document title
    pub title: String,
    /// Authors
    pub authors: Vec<String>,
    /// Abstract
    pub abstract_text: String,
    /// Full text (if available)
    pub full_text: Option<String>,
    /// Extracted entities
    pub entities: Vec<LiteratureEntity>,
    /// Extracted triples (subject-predicate-object)
    pub triples: Vec<LiteratureTriple>,
}

/// An entity extracted from literature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteratureEntity {
    /// Entity text
    pub text: String,
    /// Entity type
    pub entity_type: EntityType,
    /// Confidence score
    pub confidence: f32,
}

/// Type of literature entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    /// Chemical material (e.g., Bi2Te3)
    Material,
    /// Property (e.g., ZT, formation energy)
    Property,
    /// Method (e.g., DFT, CGCNN)
    Method,
    /// Application (e.g., thermoelectric)
    Application,
    /// Dataset
    Dataset,
}

impl Default for EntityType {
    fn default() -> Self {
        EntityType::Material
    }
}

/// A triple extracted from literature (subject-predicate-object).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteratureTriple {
    /// Subject
    pub subject: String,
    /// Predicate/relationship
    pub predicate: String,
    /// Object
    pub object: String,
    /// Confidence score
    pub confidence: f32,
}

impl Default for LiteratureTriple {
    fn default() -> Self {
        Self {
            subject: String::new(),
            predicate: String::new(),
            object: String::new(),
            confidence: 0.0,
        }
    }
}

/// Literature mining service.
#[derive(Debug, Clone)]
pub struct LiteratureService {
    enabled: bool,
}

impl LiteratureService {
    /// Create a new literature service.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled service.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Check if this service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Parse a PDF document.
    pub async fn parse_pdf(
        &self,
        _pdf_data: &[u8],
    ) -> Result<ParsedDocument, LiteratureServiceError> {
        if !self.enabled {
            return Err(LiteratureServiceError::Disabled);
        }
        // In full implementation, would call rairos-pdf-advanced
        Ok(ParsedDocument {
            id: "doc-placeholder".to_string(),
            title: "Placeholder Title".to_string(),
            authors: vec![],
            abstract_text: String::new(),
            full_text: None,
            entities: vec![],
            triples: vec![],
        })
    }

    /// Extract entities from text.
    pub async fn extract_entities(
        &self,
        _text: &str,
    ) -> Result<Vec<LiteratureEntity>, LiteratureServiceError> {
        if !self.enabled {
            return Err(LiteratureServiceError::Disabled);
        }
        Ok(vec![])
    }

    /// Extract triples from text.
    pub async fn extract_triples(
        &self,
        _text: &str,
    ) -> Result<Vec<LiteratureTriple>, LiteratureServiceError> {
        if !self.enabled {
            return Err(LiteratureServiceError::Disabled);
        }
        Ok(vec![])
    }

    /// Search for papers by topic.
    pub async fn search_papers(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<ParsedDocument>, LiteratureServiceError> {
        if !self.enabled {
            return Err(LiteratureServiceError::Disabled);
        }
        Ok(vec![])
    }
}

impl Default for LiteratureService {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for literature operations.
#[derive(Debug, thiserror::Error)]
pub enum LiteratureServiceError {
    #[error("Literature service is disabled")]
    Disabled,
    #[error("Literature error: {0}")]
    LiteratureError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_document() {
        let doc = ParsedDocument {
            id: "paper-1".to_string(),
            title: "Thermoelectric Properties of Bi2Te3".to_string(),
            authors: vec!["Author 1".to_string(), "Author 2".to_string()],
            abstract_text: "We studied thermoelectric properties...".to_string(),
            full_text: None,
            entities: vec![],
            triples: vec![],
        };
        assert!(doc.title.contains("Bi2Te3"));
    }

    #[test]
    fn test_literature_service_disabled() {
        let service = LiteratureService::disabled();
        assert!(!service.is_enabled());
    }
}
