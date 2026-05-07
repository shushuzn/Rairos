"""Remove duplicate labor-displacement route fallbacks."""

with open("web/routes_misc.py", encoding="utf-8") as f:
    c = f.read()

# Remove first fallback
old1 = '@router.get("/labor-displacement")\ndef _labor_fallback(request: Request):\n    return templates.TemplateResponse(request, "generic.html", {"page": "labor-displacement", "title": "Labor Track", "content": "<p>Labor displacement module loading...</p>"})\n\n\n'
c = c.replace(old1, "")

# Remove second fallback
old2 = '@router.get("/labor-displacement")\nasync def _labor_fb(request: Request):\n    return templates.TemplateResponse(request, "generic.html", {"page": "labor-displacement", "title": "Labor Displacement Tracker", "content": "<p>Labor displacement module loading...</p>"})\n\n\n'
c = c.replace(old2, "")

with open("web/routes_misc.py", "w", encoding="utf-8") as f:
    f.write(c)

import py_compile

py_compile.compile("web/routes_misc.py", doraise=True)
print("OK - fallbacks removed")

# Count remaining labor routes
count = c.count('@router.get("/labor-displacement")')
print(f"Remaining labor routes: {count}")
if count == 1:
    print("CLEAN - only one route handler for /labor-displacement")
