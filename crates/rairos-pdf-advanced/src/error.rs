//! Error types for rairos-pdf-advanced

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfAdvancedError {
    #[error("GROBID API error: {0}")]
    GrobidError(String),

    #[error("PDF parsing error: {0}")]
    ParseError(String),

    #[error("NER extraction error: {0}")]
    NerError(String),

    #[error("Triple extraction error: {0}")]
    TripleError(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid PDF format: {0}")]
    InvalidPdf(String),

    #[error("GROBID service unavailable")]
    ServiceUnavailable,

    #[error("Entity extraction timeout")]
    Timeout,
}
