"""
Subscription Management example for Rairos Python SDK.

This example demonstrates:
- Getting available subscription tiers
- Getting current subscription status
- Creating a checkout session (requires Stripe price ID)
"""

import os
from rairos import RairosClient, RairosError


def main():
    api_key = os.environ.get("RAIROS_API_KEY")
    if not api_key:
        print("Error: RAIROS_API_KEY environment variable not set")
        return

    client = RairosClient(api_key=api_key)

    # Get available tiers
    print("=" * 50)
    print("Available Subscription Tiers:")
    print("=" * 50)

    try:
        tiers = client.get_tiers()
        for tier in tiers:
            print(f"\n{tier.get('name', 'N/A')}")
            print(f"  Price: ${tier.get('price', 0)/100:.2f}/{tier.get('interval', 'month')}")
            print(f"  Daily requests: {tier.get('requests_limit', 'unlimited')}")
            print(f"  Features: {', '.join(tier.get('features', []))}")

    except RairosError as e:
        print(f"Failed to get tiers: {e}")

    # Get current subscription status
    print("\n" + "=" * 50)
    print("Current Subscription Status:")
    print("=" * 50)

    try:
        status = client.get_subscription_status()
        print(f"Tier: {status.get('tier', 'N/A')}")
        print(f"Subscription active: {status.get('subscription_active', False)}")

        if status.get('stripe_customer_id'):
            print(f"Stripe Customer ID: {status.get('stripe_customer_id')}")

    except RairosError as e:
        print(f"Failed to get subscription status: {e}")

    # Note: Checkout requires actual Stripe price IDs
    print("\n" + "=" * 50)
    print("Creating Checkout Session:")
    print("=" * 50)
    print("To create a checkout session, you need:")
    print("  1. Stripe account configured")
    print("  2. Valid Stripe Price IDs")
    print("  3. Replace 'price_xxx' with your actual price ID")
    print("\nExample:")
    print("""
    checkout = client.create_checkout(
        price_id="price_xxx",
        success_url="https://yourapp.com/success",
        cancel_url="https://yourapp.com/pricing"
    )
    print(checkout['checkout_url'])
    """)


if __name__ == "__main__":
    main()
