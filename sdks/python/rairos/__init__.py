"""Rairos API Python SDK

A Python SDK for the Rairos API platform.
"""

__version__ = "0.2.0"

from .client import RairosClient
from .exceptions import (
    RairosError,
    AuthenticationError,
    RateLimitError,
    ValidationError,
    NotFoundError,
    ForbiddenError,
    ServerError,
    PaymentError,
    raise_from_response,
)

__all__ = [
    "RairosClient",
    "RairosError",
    "AuthenticationError",
    "RateLimitError",
    "ValidationError",
    "NotFoundError",
    "ForbiddenError",
    "ServerError",
    "PaymentError",
    "raise_from_response",
]
