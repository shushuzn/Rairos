"""
Gap Detection example for Rairos Python SDK.

This example demonstrates:
- Detecting research gaps for a query
- Analyzing gap detection results

Note: Gap detection requires Pro tier or higher.
"""

import os
from rairos import RairosClient, RairosError, ForbiddenError


def main():
    api_key = os.environ.get("RAIROS_API_KEY")
    if not api_key:
        print("Error: RAIROS_API_KEY environment variable not set")
        return

    client = RairosClient(api_key=api_key)

    # Check tier before attempting gap detection
    print("=" * 50)
    print("Checking subscription tier...")
    print("=" * 50)

    try:
        status = client.get_subscription_status()
        tier = status.get('tier', 'free')
        print(f"Current tier: {tier}")

        if tier == 'free':
            print("\nGap detection requires Pro tier or higher.")
            print("Upgrade at https://rairos.ai/pricing")
            return

    except RairosError as e:
        print(f"Failed to check tier: {e}")
        return

    # Detect research gaps
    print("\n" + "=" * 50)
    print("Detecting research gaps...")
    print("=" * 50)

    queries = [
        "neural network architecture optimization",
        "transformer model efficiency",
        "few-shot learning limitations",
    ]

    for query in queries:
        print(f"\nQuery: {query}")
        print("-" * 40)

        try:
            gaps = client.detect_gap(
                query=query,
                categories=["cs.AI", "cs.LG"]
            )

            if gaps.get('gaps'):
                print(f"Found {len(gaps['gaps'])} potential gaps:")
                for i, gap in enumerate(gaps['gaps'][:3], 1):
                    print(f"  {i}. {gap.get('description', 'N/A')}")
                    if gap.get('confidence'):
                        print(f"     Confidence: {gap['confidence']:.2f}")
            else:
                print("No significant gaps found.")

        except ForbiddenError:
            print("Insufficient permissions for gap detection")
            break
        except RairosError as e:
            print(f"Gap detection failed: {e}")


if __name__ == "__main__":
    main()
