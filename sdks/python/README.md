# Rairos Python SDK

[![PyPI version](https://img.shields.io/pypi/v/rairos.svg)](https://pypi.org/project/rairos/)
[![Python versions](https://img.shields.io/pypi/pyversions/rairos.svg)](https://pypi.org/project/rairos/)
[![License](https://img.shields.io/pypi/l/rairos.svg)](https://github.com/shushuzn/Rairos/blob/main/sdks/python/pyproject.toml)

Python SDK for the Rairos API platform.

## Installation

```bash
# From PyPI (coming soon)
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

# Create checkout session
checkout = client.create_checkout(
    price_id="price_pro_monthly",
    success_url="https://yourapp.com/success",
    cancel_url="https://yourapp.com/pricing"
)
print(checkout["checkout_url"])  # Redirect user to this URL

# Get subscription status
status = client.get_subscription_status()
```

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
except RateLimitError:
    print("Rate limit exceeded - wait and retry")
except RairosError as e:
    print(f"API error: {e.message}")
```

## License

MIT License
