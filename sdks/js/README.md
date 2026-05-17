# Rairos JavaScript SDK

[![npm version](https://img.shields.io/npm/v/rairos.svg)](https://www.npmjs.com/package/rairos)
[![License](https://img.shields.io/npm/l/rairos.svg)](https://github.com/shushuzn/Rairos/blob/main/sdks/js/package.json)

JavaScript/TypeScript SDK for the Rairos API platform.

## Installation

```bash
# From npm
npm install rairos

# From source
cd sdks/js && npm install && cd ../..
```

## Quick Start

```javascript
const { RairosClient } = require('rairos');

// Initialize client
const client = new RairosClient('your_api_key_here');

// Search papers
const results = await client.searchPapers({ query: 'machine learning', page: 1 });
console.log(results);

// Detect research gaps (requires Pro tier)
const gaps = await client.detectGap('neural network architecture');
console.log(gaps);

// Get usage statistics
const usage = await client.getUsage();
console.log(usage);
```

## TypeScript Support

```typescript
import { RairosClient } from 'rairos';

const client = new RairosClient(process.env.RAIROS_API_KEY!);

const results = await client.searchPapers({
  query: 'quantum computing',
  page: 1,
  perPage: 20
});
```

## Authentication

You can provide your API key in three ways:

1. **Environment variable** (recommended):
   ```bash
   export RAIROS_API_KEY=your_api_key_here
   ```

2. **Constructor parameter**:
   ```javascript
   const client = new RairosClient('your_api_key_here');
   ```

3. **Register/Login** to get a new API key:
   ```javascript
   // Register
   const auth = await client.register('user@example.com', 'secure_password');

   // Login
   const auth = await client.login('user@example.com', 'secure_password');
   console.log(auth.api_key);  // Use this key
   ```

## Configuration

```javascript
const client = new RairosClient('your_api_key', {
  maxRetries: 3,      // Max retry attempts (default: 3)
  retryDelay: 1.0,    // Initial retry delay in seconds (default: 1.0)
  timeout: 30,        // Request timeout in seconds (default: 30)
  baseUrl: 'https://api.rairos.ai/api/v1'  // Override if needed
});
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

```javascript
// List all your API keys
const keys = await client.listKeys();
console.log(keys);

// Create a new API key
const newKey = await client.createKey({ name: 'production' });
console.log(newKey.apiKey);  // Save this, shown only once!

// Rotate an existing key (old key valid for 24h grace period)
const rotated = await client.rotateKey({
  keyId: 'key_xxx',
  gracePeriodHours: 24
});
```

## Usage Statistics

```javascript
// Get current usage summary
const usage = await client.getUsage();
console.log(`Used ${usage.requestsUsed} / ${usage.requestsLimit}`);

// Get detailed dashboard with endpoint breakdown
const dashboard = await client.getUsageDashboard();
console.log(dashboard.endpoints);  // Usage by endpoint
console.log(dashboard.trends);     // Usage over time
```

## API Reference

### Papers

```javascript
// Search papers
const results = await client.searchPapers({ query: 'transformers' });

// Get specific paper
const paper = await client.getPaper('uuid-here');
```

### Gap Detection (Pro tier)

```javascript
const gaps = await client.detectGap(
  'What are the gaps in transformer architecture research?',
  { categories: ['cs.AI', 'cs.LG'] }
);
```

### Research (Team tier)

```javascript
const results = await client.runResearch(
  'Analyze the state of AI safety research in 2025'
);
```

### Subscriptions

```javascript
// Get available tiers
const { tiers } = await client.getTiers();

// Create Stripe checkout session
const checkout = await client.createCheckout({
  priceId: 'price_xxx',
  successUrl: 'https://yourapp.com/success',
  cancelUrl: 'https://yourapp.com/pricing'
});
console.log(checkout.checkoutUrl);

// Open Stripe customer portal (manage subscription, update payment)
const portal = await client.createPortal({
  returnUrl: 'https://yourapp.com/dashboard'
});
console.log(portal.portalUrl);

// Get current subscription status
const status = await client.getSubscriptionStatus();
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

```javascript
const { RairosClient, AuthenticationError, RateLimitError, RairosError } = require('rairos');

try {
  const client = new RairosClient('invalid_key');
  const results = await client.searchPapers({ query: 'test' });
} catch (error) {
  if (error instanceof AuthenticationError) {
    console.log('Invalid API key');
  } else if (error instanceof RateLimitError) {
    console.log(`Rate limit exceeded - retry after ${error.resetAt}`);
  } else if (error instanceof RairosError) {
    console.log(`API error: ${error.message}`);
  }
}
```

## License

MIT License
