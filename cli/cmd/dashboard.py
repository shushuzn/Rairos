"""CLI command: dashboard — start web UI and open browser.

Usage:
    rairos dashboard     Start web UI and open default browser
    rairos dashboard --port 8888
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import webbrowser
from cli._shared import print_success, print_info, print_error


def _build_web_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "dashboard",
        help="Start Rairos Web UI and open in browser",
        description="Launch the FastAPI web interface and open your default browser.",
    )
    p.add_argument("--port", "-p", type=int, default=8501, help="Port (default: 8501)")
    p.add_argument("--host", type=str, default="127.0.0.1", help="Host (default: 127.0.0.1)")
    p.add_argument("--no-browser", action="store_true", help="Don't open browser")
    p.set_defaults(func=_run_web)
    return p


def _run_web(args) -> None:
    port = getattr(args, "port", 8501)
    host = getattr(args, "host", "127.0.0.1")
    no_browser = getattr(args, "no_browser", False)

    url = f"http://{host}:{port}"

    # Start uvicorn in background
    try:
        proc = subprocess.Popen(
            [sys.executable, "-m", "uvicorn", "web.app:app", "--host", host, "--port", str(port)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        print_success(f"Rairos Web UI started on {url}")
        print_info(f"PID: {proc.pid}  |  Stop: Ctrl+C or taskkill /f /pid {proc.pid}")

        if not no_browser:
            import time

            time.sleep(2)
            webbrowser.open(url)
            print_info("Browser opened.")
    except Exception as e:
        print_error(f"Failed: {e}")
        sys.exit(1)
