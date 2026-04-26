"""AI Research OS CLI shared utilities."""
from __future__ import annotations

import sys
from typing import Optional

from notes.keyword_tags import infer_tags_if_empty

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
