# Monitoring & Alerting

This directory contains the monitoring stack for Rairos API Gateway.

## Services

| Service | Port | Description |
|---------|------|-------------|
| API Gateway | 8081 | Main application |
| Prometheus | 9090 | Metrics collection |
| AlertManager | 9093 | Alert routing |
| Grafana | 3000 | Dashboards |
| PostgreSQL | 5432 | Database |
| Redis | 6379 | Cache/Rate limiting |

## Quick Start

1. Copy environment file:
```bash
cp .env.example .env
# Edit .env with your values
```

2. Start monitoring stack:
```bash
docker compose up -d prometheus alertmanager grafana
```

3. Access dashboards:
- Grafana: http://localhost:3000 (admin/${GRAFANA_PASSWORD})
- Prometheus: http://localhost:9090
- AlertManager: http://localhost:9093

## Configuration

### Prometheus
- Config: `prometheus.yml`
- Rules: `alerts.yml`
- Retention: 15 days

### AlertManager
- Config: `alertmanager.yml`
- Slack webhook: `${SLACK_WEBHOOK_URL}`

### Grafana
- Dashboards: `grafana/dashboards/`
- Provisioning: `grafana/provisioning/`

## Alert Rules

| Alert | Severity | Condition |
|-------|----------|-----------|
| HighErrorRate | critical | 5xx > 5% for 5m |
| RateLimitNearCapacity | warning | Usage > 80% |
| DatabaseConnectionFailure | critical | DB connections = 0 |
| RedisConnectionFailure | warning | Redis connections = 0 |
| HighAuthFailureRate | warning | 401 > 10% for 5m |
| APIServerDown | critical | API unreachable |

## Metrics

Available metrics at `/metrics`:

- `http_requests_total` - Total HTTP requests
- `http_request_duration_seconds` - Request latency histogram
- `rate_limit_used` / `rate_limit_total` - Rate limit usage
- `active_api_keys` - Number of active API keys
- `subscription_tiers` - Users per subscription tier
- `webhook_events_total` - Webhook events received
- `db_connections_active` - Active database connections
- `redis_connections_active` - Active Redis connections

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| POSTGRES_PASSWORD | Yes | PostgreSQL password |
| GRAFANA_PASSWORD | No | Grafana admin password (default: admin) |
| SLACK_WEBHOOK_URL | No | Slack webhook for alerts |
| STRIPE_WEBHOOK_SECRET | No | Stripe webhook verification |
