"""Rairos demo automation script — automates browser for screen recording.

Usage:
    # Start the web UI first: python -m uvicorn web.app_new:app --port 8765
    # Then run this script:
    python tools/demo_script.py --url http://localhost:8765 --output demo_steps.json

This script navigates the Rairos web UI and outputs step timestamps.
Use with OBS or tools.demo_recorder.py for final video/GIF output.
"""

import argparse
import json
import time
from pathlib import Path

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    print("Error: playwright not installed. Run: pip install playwright && playwright install chromium")
    raise SystemExit(1) from None

DEMO_STEPS = [
    {"name": "homepage", "url": "/", "wait": 2, "desc": "Rairos Dashboard"},
    {"name": "papers", "url": "/papers", "wait": 2, "desc": "Paper Library"},
    {"name": "gene_pool", "url": "/gene-pool", "wait": 2, "desc": "Gene Pool Overview"},
    {"name": "credibility", "url": "/gene-pool/credibility", "wait": 2, "desc": "Credibility Filter"},
    {"name": "research_briefing", "url": "/briefing", "wait": 2, "desc": "Research Briefings"},
    {"name": "chat", "url": "/chat", "wait": 3, "desc": "Research Chat"},
    {"name": "citation_pathfinder", "url": "/citation-pathfinder", "wait": 2, "desc": "Citation Pathfinder"},
    {"name": "channels", "url": "/arxiv-channels", "wait": 2, "desc": "arXiv Alert Channels"},
    {"name": "climate", "url": "/climate-monitor", "wait": 2, "desc": "Climate AI Monitor"},
    {"name": "voice_capsule", "url": "/voice-capsule", "wait": 2, "desc": "Voice to Capsule"},
]


def run_demo(url: str, output: str, headless: bool = True):
    """Navigate through demo steps and save timestamps."""
    results = []
    start_time = time.time()

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=headless)
        page = browser.new_page(viewport={"width": 1400, "height": 900})

        for step in DEMO_STEPS:
            step_url = url.rstrip("/") + step["url"]
            ts = time.time() - start_time
            print(f"[{ts:.1f}s] → {step['name']}: {step_url}")
            try:
                page.goto(step_url, wait_until="networkidle", timeout=10000)
                page.wait_for_timeout(step["wait"])
                results.append({
                    "name": step["name"],
                    "desc": step["desc"],
                    "url": step_url,
                    "timestamp": round(ts, 1),
                    "status": "ok",
                })
            except Exception as e:
                results.append({
                    "name": step["name"],
                    "desc": step["desc"],
                    "url": step_url,
                    "timestamp": round(ts, 1),
                    "status": f"error: {e}",
                })
                print(f"  ERROR: {e}")

        browser.close()

    output_path = Path(output)
    output_path.write_text(json.dumps(results, indent=2), encoding="utf-8")
    total = time.time() - start_time
    print(f"\nDemo complete in {total:.1f}s. Steps saved to {output_path}")

    ok = sum(1 for r in results if r["status"] == "ok")
    print(f"OK: {ok}/{len(results)} steps loaded successfully.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Automate Rairos demo navigation")
    parser.add_argument("--url", default="http://localhost:8765", help="Base URL of Rairos web UI")
    parser.add_argument("--output", "-o", default="demo_steps.json", help="Output JSON with timestamps")
    parser.add_argument("--visible", action="store_true", help="Show browser window during recording")
    args = parser.parse_args()

    run_demo(args.url, args.output, headless=not args.visible)
