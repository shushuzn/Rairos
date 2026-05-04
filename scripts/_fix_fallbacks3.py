"""Remove all fallback route stubs from routes_misc.py."""
import re

with open("web/routes_misc.py", encoding="utf-8") as f:
    c = f.read()

# Remove all _fb fallback functions
# Pattern: @router.get("/path")\nasync def name_fb(request):\n    return templates.TemplateResponse(...)
c = re.sub(
    r'@router\.get\("[^"]+"\)\nasync def \w+_fb\(request: Request\):\n    return templates\.TemplateResponse\(request, "generic\.html", \{[^}]+\}\)\n?\n?',
    "",
    c,
)

with open("web/routes_misc.py", "w", encoding="utf-8") as f:
    f.write(c)

# Count remaining routes
remaining = len(re.findall(r"@router\.(?:get|post)\(", c))
print(f"Remaining routes: {remaining}")

import py_compile
py_compile.compile("web/routes_misc.py", doraise=True)
print("Compiles OK")
