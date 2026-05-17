"""Rairos API Client

Main client for interacting with the Rairos API.
"""

import os
import time
from typing import Optional, Dict, Any, List
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


class RairosClient:
    """Python SDK for Rairos API.

    Example:
        >>> client = RairosClient(api_key="your-api-key")
        >>> usage = client.get_usage()
        >>> papers = client.search_papers(query="machine learning")
    """

    BASE_URL = os.environ.get("RAIROS_API_URL", "https://api.rairos.ai/api/v1")

    def __init__(
        self,
        api_key: Optional[str] = None,
        max_retries: int = 3,
        retry_delay: float = 1.0,
        timeout: Optional[int] = 30
    ):
        """Initialize the Rairos client.

        Args:
            api_key: Your Rairos API key. If not provided, will look for RAIROS_API_KEY env var.
            max_retries: Maximum number of retries for failed requests (default: 3)
            retry_delay: Initial delay between retries in seconds (default: 1.0)
            timeout: Request timeout in seconds (default: 30)
        """
        self.api_key = api_key or os.environ.get("RAIROS_API_KEY")
        if not self.api_key:
            raise AuthenticationError("API key is required. Pass api_key or set RAIROS_API_KEY env var.")

        self.max_retries = max_retries
        self.retry_delay = retry_delay
        self.timeout = timeout

    def _request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """Make an HTTP request to the Rairos API with automatic retry.

        Args:
            method: HTTP method (GET, POST, etc.)
            endpoint: API endpoint path
            **kwargs: Additional arguments passed to requests.request

        Returns:
            Parsed JSON response

        Raises:
            AuthenticationError: If API key is invalid
            RateLimitError: If rate limit is exceeded
            RairosError: For other errors
        """
        import requests
        from requests.adapters import HTTPAdapter
        from urllib3.util.retry import Retry

        url = f"{self.BASE_URL}{endpoint}"
        headers = {"Authorization": f"Bearer {self.api_key}"}
        headers.update(kwargs.pop("headers", {}))

        timeout = kwargs.pop("timeout", self.timeout)

        session = requests.Session()
        retry_strategy = Retry(
            total=self.max_retries,
            backoff_factor=1,
            status_forcelist=[429, 500, 502, 503, 504],
            allowed_methods=["HEAD", "GET", "OPTIONS", "POST"],
            raise_on_status=False
        )
        adapter = HTTPAdapter(max_retries=retry_strategy)
        session.mount("http://", adapter)
        session.mount("https://", adapter)

        try:
            response = session.request(
                method,
                url,
                headers=headers,
                timeout=timeout,
                **kwargs
            )

            if response.status_code == 401:
                raise AuthenticationError(
                    "Invalid or expired API key",
                    details={"endpoint": endpoint}
                )

            if response.status_code == 429:
                error_data = {}
                try:
                    error_data = response.json()
                except ValueError:
                    pass

                limit = error_data.get("error", {}).get("limit")
                reset_at = error_data.get("error", {}).get("reset_at")

                raise RateLimitError(
                    "Rate limit exceeded. Please retry after the reset time.",
                    limit=limit,
                    reset_at=reset_at,
                    details=error_data
                )

            if not response.ok:
                error_data = {}
                try:
                    error_data = response.json()
                except ValueError:
                    error_data = {"error": {"message": response.text or "Unknown error"}}

                raise_from_response(response.status_code, error_data)

            if response.content:
                return response.json()
            return {}

        except requests.exceptions.Timeout:
            raise RairosError(
                f"Request timed out after {timeout}s: {endpoint}",
                code="TIMEOUT",
                details={"endpoint": endpoint, "timeout": timeout}
            )
        except requests.exceptions.ConnectionError as e:
            raise RairosError(
                f"Connection failed: {str(e)}",
                code="CONNECTION_ERROR",
                details={"endpoint": endpoint}
            )
        except RairosError:
            raise
        except Exception as e:
            raise RairosError(
                f"Request failed: {str(e)}",
                code="UNKNOWN_ERROR",
                details={"endpoint": endpoint, "error": str(e)}
            )

    # Authentication
    def register(self, email: str, password: str) -> Dict[str, Any]:
        """Register a new user.

        Args:
            email: User email
            password: User password (min 8 characters)

        Returns:
            Auth response with user_id and API key

        Raises:
            ValidationError: If email or password is invalid
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
        """List all API keys for the current user.

        Returns:
            List of API key objects
        """
        return self._request("GET", "/keys")

    def create_key(self, name: Optional[str] = None) -> Dict[str, Any]:
        """Create a new API key.

        Args:
            name: Optional name for the key

        Returns:
            New API key details including the raw key (only shown once)
        """
        return self._request(
            "POST",
            "/keys",
            json={"name": name} if name else {}
        )

    def rotate_key(
        self,
        key_id: str,
        grace_period_hours: int = 24
    ) -> Dict[str, Any]:
        """Rotate an API key with a grace period.

        The old key remains valid during the grace period.

        Args:
            key_id: ID of the key to rotate
            grace_period_hours: Hours until old key expires (default: 24)

        Returns:
            New key details and old key expiration time
        """
        return self._request(
            "POST",
            "/keys/rotate",
            json={
                "key_id": key_id,
                "grace_period_hours": grace_period_hours
            }
        )

    # Usage
    def get_usage(self) -> Dict[str, Any]:
        """Get current API usage statistics.

        Returns:
            Usage statistics including tier, requests used/remaining
        """
        return self._request("GET", "/usage")

    def get_usage_dashboard(self) -> Dict[str, Any]:
        """Get detailed usage dashboard with breakdowns.

        Returns:
            Detailed usage stats including endpoint breakdown and trends
        """
        return self._request("GET", "/usage/dashboard")

    # Papers
    def search_papers(
        self,
        query: Optional[str] = None,
        page: int = 1,
        per_page: int = 20
    ) -> Dict[str, Any]:
        """Search papers.

        Args:
            query: Search query (searches title and abstract)
            page: Page number (default: 1)
            per_page: Results per page (default: 20, max: 100)

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
            **kwargs: Additional parameters for gap detection

        Returns:
            Gap detection results with identified gaps

        Raises:
            ForbiddenError: If tier is insufficient
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
            **kwargs: Additional parameters for research

        Returns:
            Research results

        Raises:
            ForbiddenError: If tier is insufficient
        """
        return self._request(
            "POST",
            "/research/run",
            json={"query": query, **kwargs}
        )

    # Subscriptions
    def get_tiers(self) -> Dict[str, Any]:
        """Get available subscription tiers.

        Returns:
            List of available tiers with pricing
        """
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
        """Get current subscription status.

        Returns:
            Subscription status including tier and Stripe info
        """
        return self._request("GET", "/subscription/status")
