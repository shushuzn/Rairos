"""
API Key Management example for Rairos Python SDK.

This example demonstrates:
- Listing all API keys
- Creating a new API key
- Rotating an existing key
"""

import os
from rairos import RairosClient, RairosError


def main():
    api_key = os.environ.get("RAIROS_API_KEY")
    if not api_key:
        print("Error: RAIROS_API_KEY environment variable not set")
        return

    client = RairosClient(api_key=api_key)

    # List all API keys
    print("=" * 50)
    print("Your API Keys:")
    print("=" * 50)

    try:
        keys = client.list_keys()
        for key in keys:
            print(f"  ID: {key.get('id', 'N/A')}")
            print(f"  Name: {key.get('name', 'unnamed')}")
            print(f"  Created: {key.get('created_at', 'N/A')}")
            print(f"  Requests used: {key.get('requests_used', 0)}")
            print()

    except RairosError as e:
        print(f"Failed to list keys: {e}")
        return

    # Create a new API key
    print("=" * 50)
    print("Creating a new API key...")
    print("=" * 50)

    try:
        new_key = client.create_key(name="example-key")
        print(f"New key created!")
        print(f"  ID: {new_key.get('id', 'N/A')}")
        print(f"  Key: {new_key.get('api_key', 'N/A')}")
        print("\n  IMPORTANT: Save this key now! It won't be shown again.")

    except RairosError as e:
        print(f"Failed to create key: {e}")
        return

    # Demonstrate key rotation (with a new key for safety)
    print("\n" + "=" * 50)
    print("Rotating the new key...")
    print("=" * 50)

    try:
        rotated = client.rotate_key(
            key_id=new_key.get('id'),
            grace_period_hours=1  # Short grace period for demo
        )
        print(f"Key rotated!")
        print(f"  New key: {rotated.get('api_key', 'N/A')}")
        print(f"  Old key expires: {rotated.get('old_key_expires_at', 'N/A')}")

    except RairosError as e:
        print(f"Failed to rotate key: {e}")


if __name__ == "__main__":
    main()
