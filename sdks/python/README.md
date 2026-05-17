# Rairos Python SDK

[![PyPI version](https://img.shields.io/pypi/v/rairos.svg)](https://pypi.org/project/rairos/)
[![Python versions](https://img.shields.io/pypi/pyversions/rairos.svg)](https://pypi.org/project/rairos/)
[![License](https://img.shields.io/pypi/l/rairos.svg)](https://github.com/shushuzn/Rairos/blob/main/sdks/python/pyproject.toml)

Python SDK for the Rairos API platform.

## Installation

```bash
# From PyPI
pip install rairos

# From source
cd sdks/python && pip install -e . && cd ../..
```

## Quick Start

```python
from rairos import RairosClient

# Initialize client
client = RairosClient(api_key="your_api_key_here")

# Search papers
results = client.search_papers(query="machine learning", page=1, per_page=20)
print(results)

# Detect research gaps (requires Pro tier)
gaps = client.detect_gap(query="neural network architecture")
print(gaps)

# Get usage statistics
usage = client.get_usage()
print(usage)
```

## Authentication

You can provide your API key in three ways:

1. **Environment variable** (recommended):
   ```bash
   export RAIROS_API_KEY=your_api_key_here
   ```

2. **Constructor parameter**:
   ```python
   client = RairosClient(api_key="your_api_key_here")
   ```

3. **Register/Login** to get a new API key:
   ```python
   # Register
   auth = client.register(email="user@example.com", password="secure_password")

   # Login
   auth = client.login(email="user@example.com", password="secure_password")
   print(auth["api_key"])  # Use this key
   ```

## Configuration

```python
client = RairosClient(
    api_key="your_api_key",
    max_retries=3,        # Max retry attempts (default: 3)
    retry_delay=1.0,       # Initial retry delay in seconds (default: 1.0)
    timeout=30,            # Request timeout in seconds (default: 30)
    base_url="https://api.rairos.ai/api/v1"  # Override if needed
)
```

## Rate Limiting

Each tier has daily request limits and per-minute rate limits:

| Tier | Daily Requests | Rate Limit/min |
|------|---------------|----------------|
| Free | 100 | 10 |
| Pro | 10,000 | 60 |
| Team | 100,000 | 300 |
| Enterprise | Unlimited | Custom |

When rate limited, the API returns HTTP 429 with:
- `X-RateLimit-Limit`: Your rate limit
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Unix timestamp when limit resets

## API Key Management

```python
# List all your API keys
keys = client.list_keys()
print(keys)

# Create a new API key
new_key = client.create_key(name="production")
print(new_key["api_key"])  # Save this, shown only once!

# Rotate an existing key (old key valid for 24h grace period)
rotated = client.rotate_key(
    key_id="key_xxx",
    grace_period_hours=24
)
```

## Usage Statistics

```python
# Get current usage summary
usage = client.get_usage()
print(f"Used {usage['requests_used']} / {usage['requests_limit']}")

# Get detailed dashboard with endpoint breakdown
dashboard = client.get_usage_dashboard()
print(dashboard["endpoints"])  # Usage by endpoint
print(dashboard["trends"])      # Usage over time
```

## API Reference

### Papers

```python
# Search papers
results = client.search_papers(query="quantum computing")

# Get specific paper
paper = client.get_paper(paper_id="uuid-here")
```

### Gap Detection (Pro tier)

```python
# Detect research gaps
gaps = client.detect_gap(
    query="What are the gaps in transformer architecture research?",
    categories=["cs.AI", "cs.LG"]
)
```

### Research (Team tier)

```python
# Run automated research
results = client.run_research(
    query="Analyze the state of AI safety research in 2025"
)
```

### Subscriptions

```python
# Get available tiers
tiers = client.get_tiers()

# Create Stripe checkout session
checkout = client.create_checkout(
    price_id="price_xxx",
    success_url="https://yourapp.com/success",
    cancel_url="https://yourapp.com/pricing"
)
print(checkout["checkout_url"])

# Open Stripe customer portal (manage subscription, update payment)
portal = client.create_portal(
    return_url="https://yourapp.com/dashboard"
)
print(portal["portal_url"])

# Get current subscription status
status = client.get_subscription_status()
```

## Error Reference

| Error | HTTP Status | Description |
|-------|-------------|-------------|
| `AuthenticationError` | 401 | Invalid or expired API key |
| `RateLimitError` | 429 | Rate limit exceeded |
| `ValidationError` | 400 | Invalid request parameters |
| `NotFoundError` | 404 | Resource not found |
| `ForbiddenError` | 403 | Tier does not have access |
| `PaymentError` | 402 | Payment failed |
| `ServerError` | 500 | Internal server error |

## Error Handling

```python
from rairos import (
    RairosClient,
    RairosError,
    AuthenticationError,
    RateLimitError
)

try:
    client = RairosClient(api_key="invalid_key")
    results = client.search_papers(query="test")
except AuthenticationError:
    print("Invalid API key")
except RateLimitError as e:
    print(f"Rate limit exceeded - retry after {e.reset_at}")
except RairosError as e:
    print(f"API error: {e.message}")
```

## License

MIT License
