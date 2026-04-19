#!/usr/bin/env -u PYTHONHOME python3
"""Post-bump hook: sync pyproject.toml [project].version with cz bump result."""

import os
import re
import sys
from pathlib import Path

NEW_VERSION = os.environ.get("CZ_POST_CURRENT_VERSION")
if not NEW_VERSION:
    sys.exit(0)

pyproject = Path("pyproject.toml")
if not pyproject.exists():
    sys.exit(0)

text = pyproject.read_text(encoding="utf-8")

# Replace "version = X" inside [project] section only.
# We find [project] then scan until next [...] header.
lines = text.splitlines(keepends=True)
in_project = False
new_lines = []
updated = False

for line in lines:
    stripped = line.strip()
    if stripped == "[project]":
        in_project = True
        new_lines.append(line)
    elif stripped.startswith("[") and stripped != "[project]":
        in_project = False
        new_lines.append(line)
    elif in_project and re.match(r'^version\s*=\s*"[^"]+"', stripped):
        # Replace version = "X.Y.Z" with new version
        new_line = re.sub(r'^version\s*=\s*"[^"]+"', f'version = "{NEW_VERSION}"', line, count=1)
        new_lines.append(new_line)
        updated = True
    else:
        new_lines.append(line)

if updated:
    pyproject.write_text("".join(new_lines), encoding="utf-8")
    print(f"[post-bump] pyproject.toml [project].version → {NEW_VERSION}")
