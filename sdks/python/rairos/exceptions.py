"""Rairos SDK Exceptions."""


class RairosError(Exception):
    """Base exception for Rairos SDK errors."""

    def __init__(self, message: str, status_code: int = None):
        super().__init__(message)
        self.message = message
        self.status_code = status_code


class AuthenticationError(RairosError):
    """Raised when authentication fails."""
    pass


class RateLimitError(RairosError):
    """Raised when rate limit is exceeded."""
    pass


class ValidationError(RairosError):
    """Raised when request validation fails."""
    pass


class NotFoundError(RairosError):
    """Raised when a resource is not found."""
    pass


class ForbiddenError(RairosError):
    """Raised when access to a resource is forbidden."""
    pass
