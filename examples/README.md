# Rairos SDK Examples

Example code demonstrating how to use the Rairos Python and JavaScript SDKs.

## Python Examples

All Python examples are in `python/`.

| Example | Description |
|---------|-------------|
| `01_basic_usage.py` | Initialize client, search papers, check usage |
| `02_api_key_management.py` | List, create, and rotate API keys |
| `03_subscription_management.py` | Get tiers, check status, create checkout |
| `04_gap_detection.py` | Detect research gaps (requires Pro tier) |
| `05_usage_dashboard.py` | Detailed usage statistics and trends |

### Running Python Examples

```bash
# Set your API key
export RAIROS_API_KEY=your_key_here

# Run an example
python examples/python/01_basic_usage.py
```

## JavaScript Examples

| Example | Description |
|---------|-------------|
| `basic_usage.js` | Initialize client, search papers, check usage |

### Running JavaScript Examples

```bash
# Set your API key
export RAIROS_API_KEY=your_key_here

# Run an example
node examples/js/basic_usage.js
```

## Prerequisites

- Python 3.8+ or Node.js 16+
- Rairos API key (get one at https://rairos.ai)

## Installation

```bash
# Python SDK
pip install rairos

# JavaScript SDK
npm install rairos
```
