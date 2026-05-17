"""
Basic usage example for Rairos Python SDK.

This example demonstrates:
- Initializing the client
- Searching for papers
- Checking usage statistics
"""

import os
from rairos import RairosClient, RairosError


def main():
    # Initialize client with API key from environment
    api_key = os.environ.get("RAIROS_API_KEY")
    if not api_key:
        print("Error: RAIROS_API_KEY environment variable not set")
        print("Set it with: export RAIROS_API_KEY=your_key_here")
        return

    client = RairosClient(api_key=api_key)

    # Search for papers
    print("=" * 50)
    print("Searching for papers about machine learning...")
    print("=" * 50)

    try:
        results = client.search_papers(query="machine learning", page=1, per_page=5)
        print(f"Found {results.get('total', 0)} papers\n")

        for i, paper in enumerate(results.get('papers', [])[:5], 1):
            print(f"{i}. {paper.get('title', 'N/A')}")
            authors = paper.get('authors', [])
            if authors:
                print(f"   Authors: {', '.join(authors[:3])}{'...' if len(authors) > 3 else ''}")
            print(f"   Score: {paper.get('score', 'N/A')}")
            print()

    except RairosError as e:
        print(f"Search failed: {e}")

    # Check usage
    print("=" * 50)
    print("Checking API usage...")
    print("=" * 50)

    try:
        usage = client.get_usage()
        print(f"Tier: {usage.get('tier', 'N/A')}")
        print(f"Requests used: {usage.get('requests_used', 0)}")
        print(f"Requests limit: {usage.get('requests_limit', 'unlimited')}")
        print(f"Rate limit per minute: {usage.get('rate_limit_per_minute', 'N/A')}")

    except RairosError as e:
        print(f"Usage check failed: {e}")


if __name__ == "__main__":
    main()
