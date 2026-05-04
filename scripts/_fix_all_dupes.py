"""Remove all duplicate route fallbacks - find and fix systematically."""
with open("web/routes_misc.py", encoding="utf-8") as f:
    lines = f.readlines()

import re

# Find all route paths and their line numbers
all_routes = []
for i, line in enumerate(lines, 1):
    m = re.match(r'@router\.(get|post)\(["\']([^"\']+)["\']\)', line.strip())
    if m:
        all_routes.append((i, m.group(2)))

# Count duplicates
from collections import Counter
counts = Counter(path for _, path in all_routes)

# For each duplicate, find which ones are fallbacks (have 'loading' in them)
removed = 0
for path, count in counts.items():
    if count <= 1:
        continue
    
    print(f"\n{path}: {count} registrations")
    
    # Find the line numbers for this path
    positions = [(idx, line) for idx, (lineno, p) in enumerate(all_routes) if p == path]
    
    for pos_idx, (lineno, _) in positions:
        # Check if this is a fallback (has 'loading' in the function body)
        # Search the function body for 'loading'
        start = lineno - 1
        end = start + 1
        while end < len(lines) and not lines[end].strip().startswith("@router.") and end - start < 30:
            end += 1
        
        body = "".join(lines[start:end])
        if "loading" in body.lower() and len(body) < 200:
            # This is a fallback - remove it
            del lines[start:end]
            print(f"  Removed fallback at line {lineno}")
            removed += 1
            break

print(f"\nRemoved {removed} fallback routes")

with open("web/routes_misc.py", "w", encoding="utf-8") as f:
    f.writelines(lines)

import py_compile
py_compile.compile("web/routes_misc.py", doraise=True)
print("Compiles OK")
