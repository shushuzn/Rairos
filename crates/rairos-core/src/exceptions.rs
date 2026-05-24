//! rairos-exceptions — Exception hierarchy for AI Research OS.
//!
//! Ported from `core/exceptions.py`.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
    pub has_cause: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<i32>,
}

#[derive(Debug)]
pub struct AIResearchOSError {
    pub message: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
    pub error_info: ErrorInfo,
}

impl AIResearchOSError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            cause: None,
            error_info: ErrorInfo {
                error_type: "AIResearchOSError".to_string(),
                message: message.to_string(),
                has_cause: false,
                cause: None,
                field: None,
                value_type: None,
                retries: None,
            },
        }
    }

    pub fn with_cause<E>(message: &str, cause: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let cause_str = cause.to_string();
        Self {
            message: message.to_string(),
            cause: Some(Box::new(cause)),
            error_info: ErrorInfo {
                error_type: "AIResearchOSError".to_string(),
                message: message.to_string(),
                has_cause: true,
                cause: Some(cause_str),
                field: None,
                value_type: None,
                retries: None,
            },
        }
    }

    pub fn get_error_info(&self) -> ErrorInfo {
        self.error_info.clone()
    }
}

impl fmt::Display for AIResearchOSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AIResearchOSError {
    fn cause(&self) -> Option<&dyn Error> {
        self.cause.as_ref().map(|e| e.as_ref() as &dyn Error)
    }
}

#[derive(Debug)]
pub struct PDFParseError {
    pub message: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl PDFParseError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            cause: None,
        }
    }
}

impl fmt::Display for PDFParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PDFParseError: {}", self.message)
    }
}

impl Error for PDFParseError {}

#[derive(Debug)]
pub struct APIClientError {
    pub message: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl APIClientError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            cause: None,
        }
    }
}

impl fmt::Display for APIClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "APIClientError: {}", self.message)
    }
}

impl Error for APIClientError {}

#[derive(Debug)]
pub struct NetworkError {
    pub message: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl NetworkError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            cause: None,
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkError: {}", self.message)
    }
}

impl Error for NetworkError {}

#[derive(Debug)]
pub struct RateLimitError {
    pub message: String,
}

impl RateLimitError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RateLimitError: {}", self.message)
    }
}

impl Error for RateLimitError {}

#[derive(Debug)]
pub struct PaperNotFoundError {
    pub message: String,
}

impl PaperNotFoundError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for PaperNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PaperNotFoundError: {}", self.message)
    }
}

impl Error for PaperNotFoundError {}

#[derive(Debug)]
pub struct DatabaseError {
    pub message: String,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl DatabaseError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            cause: None,
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseError: {}", self.message)
    }
}

impl Error for DatabaseError {}

#[derive(Debug)]
pub struct CacheError {
    pub message: String,
}

impl CacheError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CacheError: {}", self.message)
    }
}

impl Error for CacheError {}

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidationError: {}", self.message)
    }
}

impl Error for ValidationError {}

#[derive(Debug)]
pub struct ParseTimeoutError {
    pub message: String,
}

impl ParseTimeoutError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ParseTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseTimeoutError: {}", self.message)
    }
}

impl Error for ParseTimeoutError {}

#[derive(Debug)]
pub struct LLMCacheError {
    pub message: String,
}

impl LLMCacheError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for LLMCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LLMCacheError: {}", self.message)
    }
}

impl Error for LLMCacheError {}

#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl ConfigError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConfigError: {}", self.message)
    }
}

impl Error for ConfigError {}

#[derive(Debug)]
pub struct RetryExhaustedError {
    pub message: String,
    pub retries: i32,
}

impl RetryExhaustedError {
    pub fn new(message: &str, retries: i32) -> Self {
        Self {
            message: message.to_string(),
            retries,
        }
    }
}

impl fmt::Display for RetryExhaustedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetryExhaustedError: {}", self.message)
    }
}

impl Error for RetryExhaustedError {}

#[derive(Debug)]
pub struct InvalidInputError {
    pub message: String,
    pub field: Option<String>,
    pub value_type: Option<String>,
    pub error_info: ErrorInfo,
}

impl InvalidInputError {
    pub fn new(message: &str, field: Option<&str>, value: Option<&dyn fmt::Debug>) -> Self {
        let field_str = field.map(String::from);
        let value_type_str = value.map(|_| "Unknown".to_string());
        Self {
            message: message.to_string(),
            field: field_str.clone(),
            value_type: value_type_str.clone(),
            error_info: ErrorInfo {
                error_type: "InvalidInputError".to_string(),
                message: message.to_string(),
                has_cause: false,
                cause: None,
                field: field_str,
                value_type: value_type_str,
                retries: None,
            },
        }
    }

    pub fn get_error_info(&self) -> ErrorInfo {
        self.error_info.clone()
    }
}

impl fmt::Display for InvalidInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InvalidInputError: {}", self.message)
    }
}

impl Error for InvalidInputError {}

#[derive(Debug)]
pub struct MissingDependencyError {
    pub message: String,
    pub dependency: Option<String>,
}

impl MissingDependencyError {
    pub fn new(message: &str, dependency: Option<&str>) -> Self {
        Self {
            message: message.to_string(),
            dependency: dependency.map(String::from),
        }
    }
}

impl fmt::Display for MissingDependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MissingDependencyError: {}", self.message)
    }
}

impl Error for MissingDependencyError {}

pub fn format_error_message(error: &dyn Error) -> String {
    let err_str = error.to_string();
    if let Some(source) = error.source() {
        format!("[{}] {}\nCaused by: {}", error, err_str, source)
    } else {
        format!("[{}] {}", error, err_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_research_os_error_new() {
        let err = AIResearchOSError::new("test error");
        let info = err.get_error_info();
        assert_eq!(info.error_type, "AIResearchOSError");
        assert_eq!(info.message, "test error");
        assert!(!info.has_cause);
    }

    #[test]
    fn test_pdf_parse_error_display() {
        let err = PDFParseError::new("failed to extract text");
        assert!(err.to_string().contains("PDFParseError"));
    }

    #[test]
    fn test_api_client_error_display() {
        let err = APIClientError::new("API call failed");
        assert!(err.to_string().contains("APIClientError"));
    }

    #[test]
    fn test_network_error_display() {
        let err = NetworkError::new("connection refused");
        assert!(err.to_string().contains("NetworkError"));
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = RateLimitError::new("rate limit exceeded");
        assert!(err.to_string().contains("RateLimitError"));
    }

    #[test]
    fn test_paper_not_found_error_display() {
        let err = PaperNotFoundError::new("paper abc123 not found");
        assert!(err.to_string().contains("PaperNotFoundError"));
    }

    #[test]
    fn test_database_error_display() {
        let err = DatabaseError::new("query failed");
        assert!(err.to_string().contains("DatabaseError"));
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::new("cache miss");
        assert!(err.to_string().contains("CacheError"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("invalid input");
        assert!(err.to_string().contains("ValidationError"));
    }

    #[test]
    fn test_parse_timeout_error_display() {
        let err = ParseTimeoutError::new("parsing timed out");
        assert!(err.to_string().contains("ParseTimeoutError"));
    }

    #[test]
    fn test_llm_cache_error_display() {
        let err = LLMCacheError::new("cache write failed");
        assert!(err.to_string().contains("LLMCacheError"));
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::new("missing config key");
        assert!(err.to_string().contains("ConfigError"));
    }

    #[test]
    fn test_retry_exhausted_error() {
        let err = RetryExhaustedError::new("all retries failed", 3);
        assert_eq!(err.retries, 3);
        assert!(err.to_string().contains("RetryExhaustedError"));
    }

    #[test]
    fn test_invalid_input_error() {
        let err = InvalidInputError::new("invalid field", Some("email"), None);
        let info = err.get_error_info();
        assert_eq!(info.error_type, "InvalidInputError");
        assert_eq!(info.field, Some("email".to_string()));
    }

    #[test]
    fn test_missing_dependency_error() {
        let err = MissingDependencyError::new("missing dep", Some("torch"));
        assert_eq!(err.dependency, Some("torch".to_string()));
    }

    #[test]
    fn test_format_error_message() {
        let err = AIResearchOSError::new("test error message");
        let msg = format_error_message(&err);
        assert!(msg.contains("test error message"));
    }
}
