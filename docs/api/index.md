# Rairos API Documentation

REST API for the Rairos research platform.

## Base URL

```
https://api.rairos.ai/api/v1
```

## Authentication

All API requests require authentication via Bearer token in the Authorization header:

```
Authorization: Bearer <your_api_key>
```

## Rate Limits

| Tier | Daily Limit | Minute Limit |
|------|-------------|--------------|
| Free | 100 | 10 |
| Pro | 10,000 | 1,000 |
| Team | 100,000 | 10,000 |
| Enterprise | Unlimited | Unlimited |

## Endpoints

### Authentication

#### POST /auth/register

Register a new user account.

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "user_id": "uuid",
  "email": "user@example.com",
  "api_key": "64_character_key",
  "tier": "free"
}
```

#### POST /auth/login

Login and get a new API key.

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "user_id": "uuid",
  "email": "user@example.com",
  "api_key": "64_character_key",
  "tier": "free"
}
```

### API Keys

#### GET /keys

List all API keys for the authenticated user.

**Response:**
```json
[
  {
    "id": "uuid",
    "name": "Production Key",
    "tier": "pro",
    "requests_used": 5432,
    "requests_limit": 10000,
    "created_at": "2025-05-01T00:00:00Z",
    "expires_at": null
  }
]
```

#### POST /keys

Create a new API key.

**Request Body:**
```json
{
  "name": "Production Key"
}
```

**Response:**
```json
{
  "id": "uuid",
  "api_key": "64_character_key",
  "name": "Production Key",
  "tier": "pro"
}
```

### Usage

#### GET /usage

Get current API usage statistics.

**Response:**
```json
{
  "tier": "pro",
  "requests_used": 5432,
  "requests_limit": 10000,
  "requests_remaining": 4568,
  "reset_at": "2025-05-18T00:00:00Z"
}
```

### Papers

#### GET /papers/search

Search papers.

**Query Parameters:**
| Parameter | Type | Default | Description |
|----------|------|---------|-------------|
| q | string | - | Search query |
| page | int | 1 | Page number |
| per_page | int | 20 | Results per page (max 100) |

**Response:**
```json
{
  "papers": [
    {
      "id": "uuid",
      "title": "Attention Is All You Need",
      "abstract": "...",
      "authors": "Vaswani et al.",
      "categories": "cs.CL,cs.AI",
      "published": "2017-06-12T00:00:00Z"
    }
  ],
  "page": 1,
  "per_page": 20
}
```

#### GET /papers/{id}

Get a specific paper by ID.

**Response:**
```json
{
  "id": "uuid",
  "title": "Attention Is All You Need",
  "abstract": "...",
  "authors": "Vaswani et al.",
  "categories": "cs.CL,cs.AI",
  "published": "2017-06-12T00:00:00Z"
}
```

### Gap Detection (Pro+)

#### POST /gap/detect

Detect research gaps for a query. **Requires Pro tier or higher.**

**Request Body:**
```json
{
  "query": "What are the gaps in transformer architecture research?"
}
```

**Response:**
```json
{
  "status": "placeholder",
  "message": "Gap detection requires rairos-research integration"
}
```

### Research (Team+)

#### POST /research/run

Run automated research. **Requires Team tier or higher.**

**Request Body:**
```json
{
  "query": "Analyze the state of AI safety research in 2025"
}
```

**Response:**
```json
{
  "status": "placeholder",
  "message": "Research execution requires rairos-research integration"
}
```

### Subscriptions

#### GET /tiers

Get available subscription tiers.

**Response:**
```json
{
  "tiers": [
    {
      "name": "free",
      "price_id": "",
      "price_monthly": 0,
      "requests_limit": 100
    },
    {
      "name": "pro",
      "price_id": "price_pro_monthly",
      "price_monthly": 2900,
      "requests_limit": 10000
    },
    {
      "name": "team",
      "price_id": "price_team_monthly",
      "price_monthly": 9900,
      "requests_limit": 100000
    },
    {
      "name": "enterprise",
      "price_id": "price_enterprise_monthly",
      "price_monthly": 49900,
      "requests_limit": 9223372036854775807
    }
  ]
}
```

#### POST /subscription/checkout

Create a Stripe Checkout session for subscription upgrade.

**Request Body:**
```json
{
  "price_id": "price_pro_monthly",
  "success_url": "https://rairos.ai/success",
  "cancel_url": "https://rairos.ai/pricing"
}
```

**Response:**
```json
{
  "checkout_url": "https://checkout.stripe.com/...",
  "session_id": "uuid"
}
```

#### POST /subscription/portal

Create a Stripe Customer Portal session for managing subscription.

**Request Body:**
```json
{
  "return_url": "https://rairos.ai/dashboard"
}
```

**Response:**
```json
{
  "portal_url": "https://billing.stripe.com/..."
}
```

#### GET /subscription/status

Get current subscription status.

**Response:**
```json
{
  "tier": "pro",
  "stripe_customer_id": "cus_xxx",
  "subscription_active": true
}
```

### Webhooks

#### POST /subscription/webhook

Stripe webhook endpoint for subscription events.

**Events:**
- `checkout.session.completed`
- `customer.subscription.created`
- `customer.subscription.updated`
- `customer.subscription.deleted`
- `invoice.payment_failed`

## Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Daily request limit exceeded",
    "limit": 100,
    "reset_at": "2025-05-18T00:00:00Z"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|------------|-------------|
| UNAUTHORIZED | 401 | Authentication required |
| INVALID_API_KEY | 401 | Invalid API key |
| RATE_LIMITED | 429 | Rate limit exceeded |
| FORBIDDEN | 403 | Tier requirement not met |
| NOT_FOUND | 404 | Resource not found |
| VALIDATION_ERROR | 400 | Invalid request |
| DATABASE_ERROR | 500 | Database error |
| REDIS_ERROR | 500 | Cache/error service error |
| PAYMENT_ERROR | 402 | Payment failed |
| INTERNAL_ERROR | 500 | Internal server error |

## SDKs

Official SDKs available:

- [Python SDK](../sdks/python/README.md)
- [JavaScript/TypeScript SDK](../sdks/js/README.md)
