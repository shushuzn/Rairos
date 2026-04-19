#!/usr/bin/env python3
"""Generate coverage badge SVG from coverage.json."""
import json
import sys

try:
    with open("coverage.json") as f:
        data = json.load(f)
    pct = round(data["totals"]["percent_covered"], 1)
except (FileNotFoundError, KeyError, json.JSONDecodeError) as e:
    print(f"Error reading coverage: {e}", file=sys.stderr)
    sys.exit(1)

svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="110" height="20">  # noqa: E501
<linearGradient id="b" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
<linearGradient id="a" x1="0" y1="0" x2="0" y2="100%"><stop stop-color="#4c1"/><stop offset="1" stop-color="#4c1"/></linearGradient>
<g ry="3">
<text x="5" y="15" fill="#fff" font-family="Verdana" font-size="11" font-weight="bold">coverage</text>
<text x="65" y="15" fill="#fff" font-family="Verdana" font-size="11">{pct}%</text>
<rect width="110" height="20" fill="url(#a)"/>
<rect width="110" height="20" fill="url(#b)"/>
</g>
</svg>'''

with open("coverage-badge.svg", "w") as f:
    f.write(svg)

print(f"Badge generated: {pct}%")
