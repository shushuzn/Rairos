# Rairos JavaScript SDK

JavaScript/TypeScript SDK for the Rairos API platform.

## Installation

```bash
npm install rairos
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

// Create checkout session
const checkout = await client.createCheckout({
  priceId: 'price_pro_monthly',
  successUrl: 'https://yourapp.com/success',
  cancelUrl: 'https://yourapp.com/pricing'
});
console.log(checkout.checkoutUrl);  // Redirect user to this URL

// Get subscription status
const status = await client.getSubscriptionStatus();
```

## Error Handling

```javascript
const { RairosClient, AuthenticationError, RateLimitError } = require('rairos');

try {
  const client = new RairosClient('invalid_key');
  const results = await client.searchPapers({ query: 'test' });
} catch (error) {
  if (error instanceof AuthenticationError) {
    console.log('Invalid API key');
  } else if (error instanceof RateLimitError) {
    console.log('Rate limit exceeded - wait and retry');
  } else {
    console.log(`API error: ${error.message}`);
  }
}
```

## License

MIT License
