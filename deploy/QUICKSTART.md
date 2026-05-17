# Rairos API Gateway - Quick Start

Docker Compose based local development and testing environment.

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+

## Quick Start

```bash
# Clone the repository
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# Start all services
docker compose -f deploy/docker-compose.prod.yml up -d

# Check health
curl http://localhost:8081/health

# View Swagger docs
open http://localhost:8081/docs
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| API Gateway | 8081 | Main REST API |
| PostgreSQL | 5432 | User & subscription data |
| Redis | 6379 | Rate limiting cache |
| Prometheus | 9090 | Metrics collection |
| Grafana | 3000 | Dashboards |

## Environment Variables

Copy and configure:

```bash
cp deploy/.env.example .env
# Edit .env with your values
```

Required variables:
- `DATABASE_URL` - PostgreSQL connection string
- `STRIPE_SECRET_KEY` - Stripe API key
- `STRIPE_PRICE_*` - Price IDs from Stripe Dashboard

## Common Commands

```bash
# View logs
docker compose -f deploy/docker-compose.prod.yml logs -f api-gateway

# Restart API gateway
docker compose -f deploy/docker-compose.prod.yml restart api-gateway

# Stop all services
docker compose -f deploy/docker-compose.prod.yml down

# Rebuild after code changes
docker compose -f deploy/docker-compose.prod.yml up -d --build
```

## Production Deployment

For production, use the full deployment checklist:

```bash
# See deploy/CHECKLIST.md for production setup
cat deploy/CHECKLIST.md
```

## Testing

```bash
# Run API tests
cargo test -p rairos-api-gateway

# Build Docker image
docker build -f deploy/Dockerfile -t rairos-api:latest .
```
