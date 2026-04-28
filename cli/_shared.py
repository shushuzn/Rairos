"""AI Research OS CLI shared utilities."""
from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Optional

from notes.keyword_tags import infer_tags_if_empty

# ─── Environment Loading ──────────────────────────────────────────────────────
# Load .env from current working directory for packaged installs.
# Only loads if variables are not already set (preserves existing env).


def load_dotenv() -> None:
    """Load .env from current working directory if present.

    This ensures CLI commands can find API keys when running from
    any directory, even after packaging.
    """
    env_file = Path.cwd() / ".env"
    if env_file.exists():
        with open(env_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, _, value = line.partition("=")
                    os.environ.setdefault(key.strip(), value.strip())


# Lazy Database accessor — resolved at call time, not import time.
# This ensures mocks (patch('cli.Database')) work correctly.


def get_db():
    """Get a Database instance via the cli namespace (patchable via patch('cli.Database'))."""
    import cli
    return cli.Database()


class Colors:
    """ANSI color codes for terminal output."""
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


def colored(text: str, color: str) -> str:
    """Return text with ANSI color code."""
    if not sys.stdout.isatty():
        return text
    return f"{color}{text}{Colors.ENDC}"


def print_success(message: str) -> None:
    """Print success message in green."""
    print(colored(message, Colors.OKGREEN))


def print_error(message: str) -> None:
    """Print error message in red."""
    print(colored(message, Colors.FAIL), file=sys.stderr)


def print_warning(message: str) -> None:
    """Print warning message in yellow."""
    print(colored(message, Colors.WARNING))


def print_info(message: str) -> None:
    """Print info message in blue."""
    print(colored(message, Colors.OKBLUE))


def print_header(message: str) -> None:
    """Print header message in purple."""
    print(colored(message, Colors.HEADER))


def cmd_infer_tags_if_empty(tags: list, paper) -> list:
    """Wrapper for infer_tags_if_empty to avoid import cycle."""
    return infer_tags_if_empty(tags, paper)
