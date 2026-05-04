"""Remove ALL duplicate labor-displacement routes, keep only the real one."""
with open("web/routes_misc.py", encoding="utf-8") as f:
    lines = f.readlines()

# Find all labor-displacement route registrations
line_nums = []
for i, line in enumerate(lines, 1):
    if '@router.get("/labor-displacement")' in line:
        line_nums.append(i)

print(f"Found {len(line_nums)} registrations at lines: {line_nums[:5]}")

if len(line_nums) <= 1:
    print("Already clean")
else:
    # Keep the LAST one (the real implementation), remove the rest
    keep = line_nums[-1]
    remove = line_nums[:-1]
    
    # Remove from last to first to preserve indices
    for r in reversed(remove):
        idx = r - 1
        # Remove from this line until we hit the next route or empty line
        end = idx + 1
        while end < len(lines) and not lines[end].strip().startswith("@router."):
            end += 1
        # Actually we need to be more precise - remove the decorator + function body
        del lines[idx:end]
    
    with open("web/routes_misc.py", "w", encoding="utf-8") as f:
        f.writelines(lines)

    # Verify
    count = sum(1 for l in lines if '@router.get("/labor-displacement")' in l)
    print(f"After cleanup: {count} registration(s)")

import py_compile
py_compile.compile("web/routes_misc.py", doraise=True)
print("Compiles OK")
