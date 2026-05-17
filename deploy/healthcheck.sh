#!/bin/bash
# Health Check Script for Rairos API Gateway
# Usage: ./deploy/healthcheck.sh [url]

set -e

URL="${1:-http://localhost:8081/health}"
TIMEOUT=5

echo "Checking API health: $URL"

# Check health endpoint
response=$(curl -sf --max-time "$TIMEOUT" "$URL" 2>/dev/null || echo "")

if [ -z "$response" ]; then
    echo "FAIL: API is not responding"
    exit 1
fi

# Parse response
status=$(echo "$response" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)

if [ "$status" = "ok" ]; then
    echo "PASS: API is healthy"
    exit 0
else
    echo "WARN: Unexpected response: $response"
    exit 1
fi
