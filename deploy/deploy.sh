#!/bin/bash
# Rairos Deployment Script
# Usage: ./deploy.sh [environment]
#
# environments: local, staging, production

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ENV="${1:-local}"

echo "========================================="
echo "Rairos Deployment - $ENV"
echo "========================================="

cd "$PROJECT_ROOT"

# Load environment variables
if [ -f ".env" ]; then
    echo "Loading .env file..."
    export $(grep -v '^#' .env | xargs)
fi

# Check required variables
check_required() {
    local var_name="$1"
    local var_value="${!var_name}"
    if [ -z "$var_value" ]; then
        echo "Error: $var_name is not set"
        exit 1
    fi
}

echo "Checking required environment variables..."
check_required "DATABASE_URL"
check_required "STRIPE_SECRET_KEY"

# Build the API Gateway
echo ""
echo "Building API Gateway..."
docker build \
    -f deploy/Dockerfile \
    -t rairos-api-gateway:latest \
    --build-arg BUILDKIT_INLINE_CACHE=1 \
    .

# For production, tag with commit hash
if [ "$ENV" = "production" ]; then
    COMMIT_HASH=$(git rev-parse --short HEAD)
    docker tag rairos-api-gateway:latest "rairos-api-gateway:$COMMIT_HASH"
fi

echo ""
echo "Building complete!"

# Show container status
echo ""
echo "Starting containers..."
case "$ENV" in
    local)
        docker compose -f deploy/docker-compose.yml up -d
        ;;
    staging|production)
        docker compose -f deploy/docker-compose.prod.yml up -d
        ;;
esac

echo ""
echo "Waiting for services to be healthy..."
sleep 5

# Check health
echo ""
echo "Checking API health..."
for i in {1..30}; do
    if curl -sf http://localhost:8081/health > /dev/null 2>&1; then
        echo "API Gateway is healthy!"
        break
    fi
    echo "Waiting... ($i/30)"
    sleep 2
done

# Show status
echo ""
echo "Container Status:"
docker compose -f deploy/docker-compose.yml ps 2>/dev/null || docker compose -f deploy/docker-compose.prod.yml ps

echo ""
echo "========================================="
echo "Deployment complete!"
echo "========================================="
echo ""
echo "Endpoints:"
echo "  API Gateway: http://localhost:8081"
echo "  Swagger UI:  http://localhost:8081/docs"
echo "  Metrics:     http://localhost:8081/metrics"
echo ""
echo "Optional:"
[ -n "$SLACK_WEBHOOK_URL" ] && echo "  Prometheus:  http://localhost:9090"
[ -n "$SLACK_WEBHOOK_URL" ] && echo "  Grafana:     http://localhost:3000"
