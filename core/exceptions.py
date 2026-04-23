"""Exception hierarchy for AI Research OS."""
from typing import Any, Dict



class AIResearchOSError(Exception):
    """Base exception for all AI Research OS errors."""

    def __init__(self, message: str = "", cause: Exception = None):
        super().__init__(message)
        self.cause = cause
        if cause:
            self.__cause__ = cause

    def get_error_info(self) -> Dict[str, Any]:
        """Get detailed error information for debugging."""
        info = {
            "error_type": self.__class__.__name__,
            "message": str(self),
            "has_cause": self.cause is not None,
        }
        if self.cause:
            info["cause"] = str(self.cause)
        return info


class PDFParseError(AIResearchOSError):
    """Raised when PDF text extraction fails irrecoverably."""


class APIClientError(AIResearchOSError):
    """Raised when an external API call fails after all retries."""


class NetworkError(APIClientError):
    """Raised on network-level failures (DNS, timeout, connection refused)."""


class RateLimitError(APIClientError):
    """Raised when an API rate limit is hit and retries are exhausted."""


class PaperNotFoundError(AIResearchOSError):
    """Raised when a paper ID is not found in the database."""


class DatabaseError(AIResearchOSError):
    """Raised when a database operation fails."""


class CacheError(AIResearchOSError):
    """Raised when a cache operation fails non-critically."""


class ValidationError(AIResearchOSError):
    """Raised when input validation fails."""


class ParseTimeoutError(PDFParseError):
    """Raised when PDF parsing exceeds the time limit."""


class LLMCacheError(AIResearchOSError):
    """Raised when LLM cache operation fails."""


class ConfigError(AIResearchOSError):
    """Raised when configuration is invalid or missing."""


class RetryExhaustedError(AIResearchOSError):
    """Raised when all retry attempts have been exhausted."""
    
    def __init__(self, message: str = "", cause: Exception = None, retries: int = 0):
        super().__init__(message, cause)
        self.retries = retries


class InvalidInputError(ValidationError):
    """Raised when input validation fails due to invalid data format or type."""
    
    def __init__(self, message: str = "", field: str = None, value: Any = None):
        super().__init__(message)
        self.field = field
        self.value = value
    
    def get_error_info(self) -> Dict[str, Any]:
        info = super().get_error_info()
        if self.field:
            info["field"] = self.field
        if self.value is not None:
            info["value_type"] = type(self.value).__name__
        return info


class MissingDependencyError(AIResearchOSError):
    """Raised when a required dependency is not installed."""
    
    def __init__(self, message: str = "", dependency: str = None):
        super().__init__(message)
        self.dependency = dependency


def format_error_message(error: Exception) -> str:
    """Format an error message for user display."""
    if isinstance(error, AIResearchOSError):
        error_info = error.get_error_info()
        parts = [f"[{error_info['error_type']}] {error_info['message']}"]
        if error_info.get('cause'):
            parts.append(f"Caused by: {error_info['cause']}")
        return "\n".join(parts)
    else:
        return str(error)
