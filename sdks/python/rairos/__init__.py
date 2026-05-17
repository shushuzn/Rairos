"""Rairos API Python SDK

A Python SDK for the Rairos API platform.
"""

__version__ = "0.1.0"

from .client import RairosClient
from .exceptions import RairosError, AuthenticationError, RateLimitError

__all__ = [
    "RairosClient",
    "RairosError",
    "AuthenticationError",
    "RateLimitError",
]
