"""
Usage Dashboard example for Rairos Python SDK.

This example demonstrates:
- Getting current usage summary
- Getting detailed usage dashboard with breakdowns
"""

import os
from rairos import RairosClient, RairosError


def print_usage_summary(usage):
    """Print a formatted usage summary."""
    print(f"Tier: {usage.get('tier', 'N/A')}")
    print(f"Requests used: {usage.get('requests_used', 0)}")
    print(f"Requests limit: {usage.get('requests_limit', 'unlimited')}")
    print(f"Rate limit per minute: {usage.get('rate_limit_per_minute', 'N/A')}")

    # Calculate percentage if limit exists
    if usage.get('requests_limit') and usage.get('requests_limit') != 'unlimited':
        used = usage.get('requests_used', 0)
        limit = usage.get('requests_limit', 1)
        pct = (used / limit) * 100
        print(f"Usage: {pct:.1f}%")


def print_dashboard_details(dashboard):
    """Print detailed dashboard information."""
    print("\n" + "=" * 50)
    print("Detailed Dashboard:")
    print("=" * 50)

    # Endpoint breakdown
    if dashboard.get('endpoints'):
        print("\nUsage by Endpoint:")
        for endpoint, count in dashboard['endpoints'].items():
            print(f"  {endpoint}: {count}")

    # Trends
    if dashboard.get('trends'):
        print("\nUsage Trends:")
        for trend in dashboard['trends'][-5:]:  # Last 5 data points
            print(f"  {trend.get('date', 'N/A')}: {trend.get('requests', 0)} requests")

    # Daily breakdown
    if dashboard.get('daily'):
        print("\nDaily Usage (last 7 days):")
        for day in dashboard['daily'][-7:]:
            date = day.get('date', 'N/A')
            count = day.get('requests', 0)
            bar = '█' * min(int(count / 100), 50)
            print(f"  {date}: {count:>6} {bar}")


def main():
    api_key = os.environ.get("RAIROS_API_KEY")
    if not api_key:
        print("Error: RAIROS_API_KEY environment variable not set")
        return

    client = RairosClient(api_key=api_key)

    # Get basic usage
    print("=" * 50)
    print("Current Usage Summary:")
    print("=" * 50)

    try:
        usage = client.get_usage()
        print_usage_summary(usage)

    except RairosError as e:
        print(f"Failed to get usage: {e}")
        return

    # Get detailed dashboard
    print("\n" + "=" * 50)
    print("Fetching detailed dashboard...")
    print("=" * 50)

    try:
        dashboard = client.get_usage_dashboard()
        print_dashboard_details(dashboard)

    except RairosError as e:
        print(f"Failed to get dashboard: {e}")


if __name__ == "__main__":
    main()
