"""Build rairos-core with proper MSVC environment."""
import subprocess
import os
import sys

# Path to vcvarsall
vcvars = r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"

# Get the environment after vcvarsall runs
# Use setlocal and echo to capture env vars
cmd = f'"{vcvars}" x64 && python -c "import os; print(dict(os.environ))"'
result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)

print("STDOUT:", result.stdout[:500] if result.stdout else "none")
print("STDERR:", result.stderr[:500] if result.stderr else "none")
print("RC:", result.returncode)
