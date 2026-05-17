"""Rairos SDK Exceptions."""

from datetime import datetime
from typing import Optional


class RairosError(Exception):
    """Base exception for Rairos SDK errors.

    Attributes:
        message: Error message
        code: Error code (e.g., 'RATE_LIMITED', 'AUTH')
        status_code: HTTP status code if available
        details: Additional error details from API
    """

    def __init__(
        self,
        message: str,
        code: str = "ERROR",
        status_code: Optional[int] = None,
        details: Optional[dict] = None
    ):
        super().__init__(message)
        self.message = message
        self.code = code
        self.status_code = status_code
        self.details = details or {}

    def __str__(self) -> str:
        if self.code != "ERROR":
            return f"[{self.code}] {self.message}"
        return self.message


class AuthenticationError(RairosError):
    """Raised when authentication fails (invalid or expired API key).

    HTTP Status: 401
    Error Codes: AUTH, INVALID_API_KEY, UNAUTHORIZED
    """

    def __init__(self, message: str = "Authentication failed", details: Optional[dict] = None):
        super().__init__(message, code="AUTH", status_code=401, details=details)


class RateLimitError(RairosError):
    """Raised when rate limit is exceeded.

    HTTP Status: 429
    Error Code: RATE_LIMITED

    Attributes:
        limit: The rate limit that was exceeded
        reset_at: When the rate limit will reset
    """

    def __init__(
        self,
        message: str = "Rate limit exceeded",
        limit: Optional[int] = None,
        reset_at: Optional[datetime] = None,
        details: Optional[dict] = None
    ):
        super().__init__(message, code="RATE_LIMITED", status_code=429, details=details)
        self.limit = limit
        self.reset_at = reset_at

    def __str__(self) -> str:
        parts = [f"[RATE_LIMITED] {self.message}"]
        if self.limit:
            parts.append(f"Limit: {self.limit}")
        if self.reset_at:
            parts.append(f"Resets at: {self.reset_at.isoformat()}")
        return " | ".join(parts)


class ValidationError(RairosError):
    """Raised when request validation fails.

    HTTP Status: 400
    Error Codes: VALIDATION_ERROR, BAD_REQUEST
    """

    def __init__(self, message: str = "Validation failed", details: Optional[dict] = None):
        super().__init__(message, code="VALIDATION_ERROR", status_code=400, details=details)


class NotFoundError(RairosError):
    """Raised when a resource is not found.

    HTTP Status: 404
    Error Codes: NOT_FOUND
    """

    def __init__(self, message: str = "Resource not found", details: Optional[dict] = None):
        super().__init__(message, code="NOT_FOUND", status_code=404, details=details)


class ForbiddenError(RairosError):
    """Raised when access to a resource is forbidden (insufficient tier).

    HTTP Status: 403
    Error Codes: FORBIDDEN, INSUFFICIENT_PERMISSIONS
    """

    def __init__(self, message: str = "Access forbidden", details: Optional[dict] = None):
        super().__init__(message, code="FORBIDDEN", status_code=403, details=details)


class ServerError(RairosError):
    """Raised when an internal server error occurs.

    HTTP Status: 500+
    Error Codes: INTERNAL_ERROR, DATABASE_ERROR, SERVER_ERROR
    """

    def __init__(self, message: str = "Internal server error", details: Optional[dict] = None):
        super().__init__(message, code="SERVER_ERROR", status_code=500, details=details)


class PaymentError(RairosError):
    """Raised when a payment-related error occurs.

    HTTP Status: 402
    Error Codes: PAYMENT_ERROR
    """

    def __init__(self, message: str = "Payment error", details: Optional[dict] = None):
        super().__init__(message, code="PAYMENT_ERROR", status_code=402, details=details)


def parse_error_response(response_data: dict) -> tuple[str, dict]:
    """Parse error response from API.

    Returns:
        Tuple of (error_code, error_details)
    """
    error = response_data.get("error", {})
    code = error.get("code", "ERROR")
    details = {
        "limit": error.get("limit"),
        "reset_at": error.get("reset_at"),
    }
    return code, details


def raise_from_response(status_code: int, response_data: dict):
    """Raise appropriate exception from API error response.

    Args:
        status_code: HTTP status code
        response_data: Parsed JSON response

    Raises:
        Appropriate RairosError subclass
    """
    code, details = parse_error_response(response_data)
    message = response_data.get("error", {}).get("message", "Unknown error")

    error_mapping = {
        400: (ValidationError, "VALIDATION_ERROR"),
        401: (AuthenticationError, "AUTH"),
        403: (ForbiddenError, "FORBIDDEN"),
        404: (NotFoundError, "NOT_FOUND"),
        402: (PaymentError, "PAYMENT_ERROR"),
        429: (RateLimitError, "RATE_LIMITED"),
    }

    if status_code in error_mapping:
        exc_class, expected_code = error_mapping[status_code]
        if status_code == 429 and details.get("limit"):
            reset_at_str = details.get("reset_at")
            reset_at = datetime.fromisoformat(reset_at_str.replace("Z", "+00:00")) if reset_at_str else None
            raise exc_class(message, limit=details["limit"], reset_at=reset_at, details=details)
        raise exc_class(message, details=details)

    if status_code >= 500:
        raise ServerError(message, details=details)

    raise RairosError(message, code=code, status_code=status_code, details=details)
