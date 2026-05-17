# SDK Error Reference

Complete error code reference for Rairos Python and JavaScript SDKs.

## Error Classes

### RairosError

Base exception for all SDK errors.

**Attributes:**
- `message`: Human-readable error message
- `code`: Error code string (e.g., 'RATE_LIMITED')
- `status_code`: HTTP status code
- `details`: Additional error details

### AuthenticationError

Raised when authentication fails (invalid or expired API key).

| Property | Value |
|----------|-------|
| HTTP Status | 401 |
| Error Codes | AUTH, INVALID_API_KEY, UNAUTHORIZED |
| Common Causes | Invalid API key, expired key, revoked key |

**Python Example:**
```python
from rairos import AuthenticationError

try:
    client = RairosClient(api_key="invalid_key")
except AuthenticationError as e:
    print(f"Auth failed: {e.message}")
```

**JavaScript Example:**
```javascript
const { AuthenticationError } = require('rairos');

try {
    const client = new RairosClient('invalid_key');
} catch (error) {
    if (error instanceof AuthenticationError) {
        console.log(`Auth failed: ${error.message}`);
    }
}
```

---

### RateLimitError

Raised when rate limit is exceeded.

| Property | Value |
|----------|-------|
| HTTP Status | 429 |
| Error Code | RATE_LIMITED |

**Attributes:**
- `limit`: The rate limit that was exceeded
- `reset_at`: When the rate limit will reset (datetime/timestamp)

**Python Example:**
```python
from rairos import RateLimitError

try:
    results = client.search_papers(query="test")
except RateLimitError as e:
    print(f"Rate limited! Reset at: {e.reset_at}")
    print(f"Limit: {e.limit}")
```

**JavaScript Example:**
```javascript
const { RateLimitError } = require('rairos');

try {
    const results = await client.searchPapers({ query: 'test' });
} catch (error) {
    if (error instanceof RateLimitError) {
        console.log(`Rate limited! Reset at: ${error.resetAt}`);
        console.log(`Limit: ${error.limit}`);
    }
}
```

---

### ValidationError

Raised when request validation fails.

| Property | Value |
|----------|-------|
| HTTP Status | 400 |
| Error Codes | VALIDATION_ERROR, BAD_REQUEST |
| Common Causes | Missing required parameters, invalid format |

**Python Example:**
```python
from rairos import ValidationError

try:
    client.search_papers()  # Missing query
except ValidationError as e:
    print(f"Invalid request: {e.message}")
```

---

### NotFoundError

Raised when a resource is not found.

| Property | Value |
|----------|-------|
| HTTP Status | 404 |
| Error Codes | NOT_FOUND |
| Common Causes | Invalid paper ID, expired API key |

---

### ForbiddenError

Raised when access to a resource is forbidden (insufficient tier).

| Property | Value |
|----------|-------|
| HTTP Status | 403 |
| Error Codes | FORBIDDEN, INSUFFICIENT_PERMISSIONS |
| Common Causes | Pro feature used on Free tier, expired subscription |

**Python Example:**
```python
from rairos import ForbiddenError

try:
    gaps = client.detect_gap(query="test")
except ForbiddenError as e:
    print("Gap detection requires Pro tier")
```

---

### PaymentError

Raised when a payment-related error occurs.

| Property | Value |
|----------|-------|
| HTTP Status | 402 |
| Error Code | PAYMENT_ERROR |
| Common Causes | Card declined, insufficient funds, Stripe error |

---

### ServerError

Raised when an internal server error occurs.

| Property | Value |
|----------|-------|
| HTTP Status | 500+ |
| Error Codes | INTERNAL_ERROR, DATABASE_ERROR, SERVER_ERROR |
| Common Causes | Rairos server issues (usually temporary) |

---

## Error Code Reference Table

| Error | HTTP Status | Code | Description |
|-------|-------------|------|-------------|
| AuthenticationError | 401 | AUTH | Invalid or expired API key |
| RateLimitError | 429 | RATE_LIMITED | Rate limit exceeded |
| ValidationError | 400 | VALIDATION_ERROR | Invalid request parameters |
| NotFoundError | 404 | NOT_FOUND | Resource not found |
| ForbiddenError | 403 | FORBIDDEN | Insufficient permissions |
| PaymentError | 402 | PAYMENT_ERROR | Payment failed |
| ServerError | 500 | SERVER_ERROR | Internal server error |

---

## Troubleshooting

### Authentication Errors

1. **Check API key format** - Should be a valid UUID or key string
2. **Verify environment variable** - `RAIROS_API_KEY` is set correctly
3. **Check key status** - Key may be revoked in dashboard

### Rate Limit Errors

1. **Wait for reset** - Check `reset_at` timestamp
2. **Upgrade tier** - Higher tiers have higher limits
3. **Implement backoff** - SDK retries automatically with `max_retries`

### Payment Errors

1. **Check Stripe status** - Visit Stripe Dashboard
2. **Verify price ID** - Ensure price ID is correct
3. **Check card validity** - Card may be expired

### Server Errors

1. **Check status page** - Visit rairos.ai status
2. **Retry later** - Usually temporary
3. **Contact support** - If persists

---

## Error Handling Best Practices

### Python

```python
from rairos import (
    RairosClient,
    RairosError,
    AuthenticationError,
    RateLimitError,
    ForbiddenError,
)

client = RairosClient(api_key="...")

try:
    results = client.search_papers(query="...")
except AuthenticationError:
    # Handle auth errors (re-authenticate, check key)
    print("Please check your API key")
except RateLimitError as e:
    # Handle rate limits (wait, upgrade, implement backoff)
    print(f"Rate limited until {e.reset_at}")
except ForbiddenError:
    # Handle permission errors (upgrade tier)
    print("Please upgrade your subscription")
except RairosError as e:
    # Handle all other errors
    print(f"API error: {e.message}")
    print(f"Details: {e.details}")
```

### JavaScript

```javascript
const {
    RairosClient,
    RairosError,
    AuthenticationError,
    RateLimitError,
    ForbiddenError
} = require('rairos');

const client = new RairosClient('...');

try {
    const results = await client.searchPapers({ query: '...' });
} catch (error) {
    if (error instanceof AuthenticationError) {
        console.log('Please check your API key');
    } else if (error instanceof RateLimitError) {
        console.log(`Rate limited until ${error.resetAt}`);
    } else if (error instanceof ForbiddenError) {
        console.log('Please upgrade your subscription');
    } else if (error instanceof RairosError) {
        console.log(`API error: ${error.message}`);
        console.log(`Details: ${JSON.stringify(error.details)}`);
    }
}
```
