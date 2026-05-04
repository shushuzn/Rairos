"""Remove all fallback routes that block real implementations."""
with open("web/routes_misc.py", encoding="utf-8") as f:
    c = f.read()

# Find all fallback functions (short functions with 'loading' in body)
import re

# Pattern: @router.get("/path") followed by a short function with "loading"
# These are the duplicate fallbacks we added earlier

fallback_patterns = [
    (r'@router\.get\("/citation-chain"\)[^@]*?loading[^@]*?(?=@router)', "citation-chain"),
    (r'@router\.get\("/insights/queue"\)[^@]*?loading[^@]*?(?=@router)', "insights/queue"),
    (r'@router\.get\("/voice-capsule"\)[^@]*?loading[^@]*?(?=@router)', "voice-capsule"),
    (r'@router\.get\("/policy-impact"\)[^@]*?loading[^@]*?(?=@router)', "policy-impact"),
    (r'@router\.get\("/labor-displacement"\)[^@]*?loading[^@]*?(?=@router)', "labor-displacement"),
    (r'@router\.get\("/researchers"\)[^@]*?loading[^@]*?(?=@router)', "researchers"),
    (r'@router\.get\("/arxiv-channels"\)[^@]*?loading[^@]*?(?=@router)', "arxiv-channels"),
]

for pattern, name in fallback_patterns:
    match = re.search(pattern, c, re.DOTALL)
    if match:
        c = c[:match.start()] + c[match.end():]
        print(f"Removed fallback: {name}")
    else:
        print(f"No fallback found: {name}")

with open("web/routes_misc.py", "w", encoding="utf-8") as f:
    f.write(c)

import py_compile
py_compile.compile("web/routes_misc.py", doraise=True)
print("\nCompiles OK")
