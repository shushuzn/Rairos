"""Parse vcvarsall output and build with proper env."""
import subprocess
import os
import json
import sys

vcvars = r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
workdir = r"D:\OpenClaw\workspace\80-PROJECTS\ai_research_os"

# Create a helper script to dump env
env_script = os.path.join(workdir, "_dump_env.py")
with open(env_script, "w") as f:
    f.write("import os, json; print('ENVJSON' + json.dumps(dict(os.environ)))")

# Run vcvarsall then our script
cmd = [vcvars, "x64", "&&", "python", env_script]
full_cmd = '"' + vcvars + '" x64 && python "' + env_script + '"'

result = subprocess.run(full_cmd, shell=True, capture_output=True, text=True, timeout=30, cwd=workdir)

# Parse env from output
env = None
for line in result.stdout.splitlines():
    if line.startswith('ENVJSON'):
        env = json.loads(line[7:])
        break

if not env:
    print("Failed to get vcvars env")
    print("STDOUT:", result.stdout[:1000])
    print("STDERR:", result.stderr[:500])
    sys.exit(1)

# Print key paths
for k in ['INCLUDE', 'LIB', 'PATH']:
    if k in env:
        print(f"{k}={env[k][:300]}")

# Now run cargo with this environment
env_clean = {}
for k in ['INCLUDE', 'LIB', 'PATH', 'SYSTEMROOT', 'TEMP', 'TMP', 'USERPROFILE']:
    if k in env:
        env_clean[k] = env[k]

# Add original env for other vars
for k, v in os.environ.items():
    if k not in env_clean:
        env_clean[k] = v

print("\n=== Running cargo build ===")
cargo_result = subprocess.run(
    ['cargo', 'build', '--package', 'rairos-core'],
    cwd=workdir,
    env=env_clean,
    timeout=600
)

# Clean up
try:
    os.remove(env_script)
except:
    pass

print("CARGO RC:", cargo_result.returncode)
if cargo_result.stdout:
    print("STDOUT:", cargo_result.stdout[-3000:])
if cargo_result.stderr:
    print("STDERR:", cargo_result.stderr[-3000:])
