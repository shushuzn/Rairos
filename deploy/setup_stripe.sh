#!/bin/bash
# Stripe Setup Script for Rairos API
# Usage: ./setup_stripe.sh <stripe_api_key>

set -e

STRIPE_KEY="${1:-${STRIPE_API_KEY}}"

if [ -z "$STRIPE_KEY" ]; then
    echo "Error: Stripe API key required"
    echo "Usage: $0 <stripe_api_key>"
    exit 1
fi

echo "Creating Rairos products and prices in Stripe..."

# Create Pro product
echo "Creating Pro tier..."
PRO_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Pro" \
  -d "description=Pro tier - 10,000 requests/day" \
  -d "metadata[tier]=pro")

PRO_PRICE_ID=$(echo "$PRO_PRODUCT" | grep -o '"id":"price_[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$PRO_PRICE_ID" ]; then
    echo "Error: Failed to create Pro product"
    exit 1
fi

echo "Creating Pro monthly price..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$PRO_PRICE_ID" \
  -d "unit_amount=2900" \
  -d "currency=usd" \
  -d "recurring[interval]=month"

echo "Creating Pro annual price (2 months free)..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$PRO_PRICE_ID" \
  -d "unit_amount=26100" \
  -d "currency=usd" \
  -d "recurring[interval]=year"

# Create Team product
echo "Creating Team tier..."
TEAM_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Team" \
  -d "description=Team tier - 100,000 requests/day" \
  -d "metadata[tier]=team")

TEAM_PRICE_ID=$(echo "$TEAM_PRODUCT" | grep -o '"id":"price_[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$TEAM_PRICE_ID" ]; then
    echo "Error: Failed to create Team product"
    exit 1
fi

echo "Creating Team monthly price..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$TEAM_PRICE_ID" \
  -d "unit_amount=9900" \
  -d "currency=usd" \
  -d "recurring[interval]=month"

echo "Creating Team annual price..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$TEAM_PRICE_ID" \
  -d "unit_amount=89100" \
  -d "currency=usd" \
  -d "recurring[interval]=year"

# Create Enterprise product
echo "Creating Enterprise tier..."
ENTERPRISE_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Enterprise" \
  -d "description=Enterprise tier - Unlimited requests" \
  -d "metadata[tier]=enterprise")

ENTERPRISE_PRICE_ID=$(echo "$ENTERPRISE_PRODUCT" | grep -o '"id":"price_[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$ENTERPRISE_PRICE_ID" ]; then
    echo "Error: Failed to create Enterprise product"
    exit 1
fi

echo "Creating Enterprise monthly price..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$ENTERPRISE_PRICE_ID" \
  -d "unit_amount=49900" \
  -d "currency=usd" \
  -d "recurring[interval]=month"

echo "Creating Enterprise annual price..."
curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$ENTERPRISE_PRICE_ID" \
  -d "unit_amount=449100" \
  -d "currency=usd" \
  -d "recurring[interval]=year"

echo ""
echo "Stripe setup complete!"
echo ""
echo "Update your stripe.rs with these price IDs:"
echo "PRO_PRICE_ID=$PRO_PRICE_ID"
echo "TEAM_PRICE_ID=$TEAM_PRICE_ID"
echo "ENTERPRISE_PRICE_ID=$ENTERPRISE_PRICE_ID"
