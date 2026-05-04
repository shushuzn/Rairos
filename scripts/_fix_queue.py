"""Find and remove remaining insights/queue fallback."""
with open("web/routes_misc.py", encoding="utf-8") as f:
    c = f.read()

import re

# Find all insights/queue routes
for m in re.finditer(r'@router\.(?:get|post)\("/insights/queue[^"]*"\)', c):
    line = c[:m.start()].count("\n") + 1
    end = m.end()
    # Check if this route has "loading" in its body
    body = c[end:end+500]
    if "loading" in body.lower():
        print(f"Line {line}: FALLBACK - {body[:60]}")
        # Remove this entire function (decorator + body)
        func_end = end
        while func_end < len(c) and not c[func_end:func_end+1].startswith("@") and func_end - end < 2000:
            func_end += 1
        # Go back to find function start
        c = c[:m.start()] + c[func_end:]
        print("  Removed!")
        break

with open("web/routes_misc.py", "w", encoding="utf-8") as f:
    f.write(c)

# Verify
remaining = len(re.findall(r'@router\.(?:get|post)\("/insights/queue[^"]*"\)', c))
print(f"Remaining routes: {remaining}")

import py_compile
py_compile.compile("web/routes_misc.py", doraise=True)
print("Compiles OK")
