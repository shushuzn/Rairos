"""Rairos API Client

Main client for interacting with the Rairos API.
"""

import os
from typing import Optional, Dict, Any, List
from .exceptions import RairosError, AuthenticationError, RateLimitError


class RairosClient:
    """Python SDK for Rairos API."""

    BASE_URL = os.environ.get("RAIROS_API_URL", "https://api.rairos.ai/api/v1")

    def __init__(self, api_key: Optional[str] = None):
        """Initialize the Rairos client.

        Args:
            api_key: Your Rairos API key. If not provided, will look for RAIROS_API_KEY env var.
        """
        self.api_key = api_key or os.environ.get("RAIROS_API_KEY")
        if not self.api_key:
            raise AuthenticationError("API key is required. Pass api_key or set RAIROS_API_KEY env var.")

    def _request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """Make an HTTP request to the Rairos API."""
        import requests

        url = f"{self.BASE_URL}{endpoint}"
        headers = {"Authorization": f"Bearer {self.api_key}"}
        headers.update(kwargs.pop("headers", {}))

        try:
            response = requests.request(method, url, headers=headers, **kwargs)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.HTTPError as e:
            if response.status_code == 401:
                raise AuthenticationError("Invalid or expired API key")
            elif response.status_code == 429:
                raise RateLimitError("Rate limit exceeded")
            else:
                error_data = response.json() if response.content else {}
                raise RairosError(
                    error_data.get("error", {}).get("message", str(e)),
                    status_code=response.status_code
                )
        except requests.exceptions.RequestException as e:
            raise RairosError(f"Request failed: {str(e)}")

    # Authentication
    def register(self, email: str, password: str) -> Dict[str, Any]:
        """Register a new user.

        Args:
            email: User email
            password: User password (min 8 characters)

        Returns:
            Auth response with user_id and API key
        """
        return self._request(
            "POST",
            "/auth/register",
            json={"email": email, "password": password}
        )

    def login(self, email: str, password: str) -> Dict[str, Any]:
        """Login and get a new API key.

        Args:
            email: User email
            password: User password

        Returns:
            Auth response with user_id and API key
        """
        return self._request(
            "POST",
            "/auth/login",
            json={"email": email, "password": password}
        )

    # API Keys
    def list_keys(self) -> List[Dict[str, Any]]:
        """List all API keys for the current user."""
        return self._request("GET", "/keys")

    def create_key(self, name: Optional[str] = None) -> Dict[str, Any]:
        """Create a new API key.

        Args:
            name: Optional name for the key

        Returns:
            New API key details
        """
        return self._request(
            "POST",
            "/keys",
            json={"name": name} if name else {}
        )

    # Usage
    def get_usage(self) -> Dict[str, Any]:
        """Get current API usage statistics."""
        return self._request("GET", "/usage")

    # Papers
    def search_papers(
        self,
        query: Optional[str] = None,
        page: int = 1,
        per_page: int = 20
    ) -> Dict[str, Any]:
        """Search papers.

        Args:
            query: Search query
            page: Page number
            per_page: Results per page

        Returns:
            Paginated paper results
        """
        params = {"page": page, "per_page": per_page}
        if query:
            params["q"] = query

        return self._request("GET", "/papers/search", params=params)

    def get_paper(self, paper_id: str) -> Dict[str, Any]:
        """Get a specific paper by ID.

        Args:
            paper_id: Paper UUID

        Returns:
            Paper details
        """
        return self._request("GET", f"/papers/{paper_id}")

    # Gap Detection (Pro+)
    def detect_gap(self, query: str, **kwargs) -> Dict[str, Any]:
        """Detect research gaps for a query. Requires Pro tier or higher.

        Args:
            query: Research query to analyze

        Returns:
            Gap detection results
        """
        return self._request(
            "POST",
            "/gap/detect",
            json={"query": query, **kwargs}
        )

    # Research (Team+)
    def run_research(self, query: str, **kwargs) -> Dict[str, Any]:
        """Run automated research. Requires Team tier or higher.

        Args:
            query: Research query

        Returns:
            Research results
        """
        return self._request(
            "POST",
            "/research/run",
            json={"query": query, **kwargs}
        )

    # Subscriptions
    def get_tiers(self) -> Dict[str, Any]:
        """Get available subscription tiers."""
        return self._request("GET", "/tiers")

    def create_checkout(
        self,
        price_id: str,
        success_url: str = "https://rairos.ai/success",
        cancel_url: str = "https://rairos.ai/pricing"
    ) -> Dict[str, Any]:
        """Create a Stripe checkout session for subscription.

        Args:
            price_id: Stripe price ID for the tier
            success_url: URL to redirect on success
            cancel_url: URL to redirect on cancel

        Returns:
            Checkout URL and session ID
        """
        return self._request(
            "POST",
            "/subscription/checkout",
            json={
                "price_id": price_id,
                "success_url": success_url,
                "cancel_url": cancel_url
            }
        )

    def create_portal(self, return_url: str = "https://rairos.ai/dashboard") -> Dict[str, Any]:
        """Create a Stripe customer portal session.

        Args:
            return_url: URL to redirect after using the portal

        Returns:
            Portal URL
        """
        return self._request(
            "POST",
            "/subscription/portal",
            json={"return_url": return_url}
        )

    def get_subscription_status(self) -> Dict[str, Any]:
        """Get current subscription status."""
        return self._request("GET", "/subscription/status")
