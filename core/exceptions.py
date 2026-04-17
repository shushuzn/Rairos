"""Exception hierarchy for AI Research OS."""


class AIResearchOSError(Exception):
    """Base exception for all AI Research OS errors."""

    def __init__(self, message: str = "", cause: Exception = None):
        super().__init__(message)
        self.cause = cause
        if cause:
            self.__cause__ = cause


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
