import re, pathlib
c = pathlib.Path("rairos_mcp.py").read_text("utf-8")
tools = re.findall(r'"name": "(\w+)"', c)
for i, t in enumerate(tools, 1):
    print(f"{i:3d}. {t}")
print(f"Total: {len(tools)} tools")
