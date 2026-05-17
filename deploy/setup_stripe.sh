#!/bin/bash
# Stripe Setup Script for Rairos API
# Usage: ./setup_stripe.sh <stripe_api_key>
#
# This script creates Stripe products and prices for all tiers.
# Run this once to set up your Stripe account.

set -e

STRIPE_KEY="${1:-${STRIPE_API_KEY}}"

if [ -z "$STRIPE_KEY" ]; then
    echo "Error: Stripe API key required"
    echo "Usage: $0 <stripe_api_key>"
    exit 1
fi

echo "Creating Rairos products and prices in Stripe..."
echo ""

# Function to extract price ID from response
get_price_id() {
    echo "$1" | grep -o '"id":"price_[^"]*"' | head -1 | cut -d'"' -f4
}

# Create Pro product
echo "Creating Pro tier..."
PRO_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Pro" \
  -d "description=Pro tier - 10,000 requests/day" \
  -d "metadata[tier]=pro" \
  -d "metadata[requests]=10000")

PRO_PRICE_ID=$(get_price_id "$PRO_PRODUCT")

if [ -z "$PRO_PRICE_ID" ]; then
    echo "Error: Failed to create Pro product"
    exit 1
fi

echo "  Product ID: $PRO_PRODUCT_ID"

echo "Creating Pro monthly price..."
PRO_MONTHLY=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$PRO_PRICE_ID" \
  -d "unit_amount=2900" \
  -d "currency=usd" \
  -d "recurring[interval]=month")
PRO_MONTHLY_ID=$(get_price_id "$PRO_MONTHLY")

echo "Creating Pro annual price (2 months free)..."
PRO_ANNUAL=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$PRO_PRICE_ID" \
  -d "unit_amount=26100" \
  -d "currency=usd" \
  -d "recurring[interval]=year")
PRO_ANNUAL_ID=$(get_price_id "$PRO_ANNUAL")

# Create Team product
echo ""
echo "Creating Team tier..."
TEAM_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Team" \
  -d "description=Team tier - 100,000 requests/day" \
  -d "metadata[tier]=team" \
  -d "metadata[requests]=100000")

TEAM_PRICE_ID=$(get_price_id "$TEAM_PRODUCT")

if [ -z "$TEAM_PRICE_ID" ]; then
    echo "Error: Failed to create Team product"
    exit 1
fi

echo "Creating Team monthly price..."
TEAM_MONTHLY=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$TEAM_PRICE_ID" \
  -d "unit_amount=9900" \
  -d "currency=usd" \
  -d "recurring[interval]=month")
TEAM_MONTHLY_ID=$(get_price_id "$TEAM_MONTHLY")

echo "Creating Team annual price..."
TEAM_ANNUAL=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$TEAM_PRICE_ID" \
  -d "unit_amount=89100" \
  -d "currency=usd" \
  -d "recurring[interval]=year")
TEAM_ANNUAL_ID=$(get_price_id "$TEAM_ANNUAL")

# Create Enterprise product
echo ""
echo "Creating Enterprise tier..."
ENTERPRISE_PRODUCT=$(curl -s -X POST "https://api.stripe.com/v1/products" \
  -u "$STRIPE_KEY" \
  -d "name=Rairos Enterprise" \
  -d "description=Enterprise tier - Unlimited requests" \
  -d "metadata[tier]=enterprise" \
  -d "metadata[requests]=unlimited")

ENTERPRISE_PRICE_ID=$(get_price_id "$ENTERPRISE_PRODUCT")

if [ -z "$ENTERPRISE_PRICE_ID" ]; then
    echo "Error: Failed to create Enterprise product"
    exit 1
fi

echo "Creating Enterprise monthly price..."
ENTERPRISE_MONTHLY=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$ENTERPRISE_PRICE_ID" \
  -d "unit_amount=49900" \
  -d "currency=usd" \
  -d "recurring[interval]=month")
ENTERPRISE_MONTHLY_ID=$(get_price_id "$ENTERPRISE_MONTHLY")

echo "Creating Enterprise annual price..."
ENTERPRISE_ANNUAL=$(curl -s -X POST "https://api.stripe.com/v1/prices" \
  -u "$STRIPE_KEY" \
  -d "product=$ENTERPRISE_PRICE_ID" \
  -d "unit_amount=449100" \
  -d "currency=usd" \
  -d "recurring[interval]=year")
ENTERPRISE_ANNUAL_ID=$(get_price_id "$ENTERPRISE_ANNUAL")

echo ""
echo "============================================================"
echo "Stripe setup complete!"
echo "============================================================"
echo ""
echo "Add these to your .env file:"
echo ""
echo "# Stripe Price IDs"
echo "STRIPE_PRICE_PRO_MONTHLY=$PRO_MONTHLY_ID"
echo "STRIPE_PRICE_PRO_ANNUAL=$PRO_ANNUAL_ID"
echo "STRIPE_PRICE_TEAM_MONTHLY=$TEAM_MONTHLY_ID"
echo "STRIPE_PRICE_TEAM_ANNUAL=$TEAM_ANNUAL_ID"
echo "STRIPE_PRICE_ENTERPRISE_MONTHLY=$ENTERPRISE_MONTHLY_ID"
echo "STRIPE_PRICE_ENTERPRISE_ANNUAL=$ENTERPRISE_ANNUAL_ID"
echo ""
echo "Next steps:"
echo "1. Copy the price IDs above to your .env file"
echo "2. Set up your Stripe webhook at:"
echo "   https://dashboard.stripe.com/webhooks"
echo "3. Add endpoint: https://your-domain.com/subscription/webhook"
echo "4. Enable these events:"
echo "   - checkout.session.completed"
echo "   - customer.subscription.created"
echo "   - customer.subscription.updated"
echo "   - customer.subscription.deleted"
echo "   - invoice.payment_failed"
