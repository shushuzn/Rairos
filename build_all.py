"""Build all rairos crates with proper MSVC environment."""
import subprocess
import os
import json

vcvars = r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
workdir = r"D:\OpenClaw\workspace\80-PROJECTS\ai_research_os"

# Create helper script to dump env
env_script = os.path.join(workdir, "_dump_env.py")
with open(env_script, "w") as f:
    f.write("import os, json; print('ENVJSON' + json.dumps(dict(os.environ)))")

# Run vcvarsall then our script
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
    exit(1)

# Build clean env with MSVC paths
env_clean = {}
for k in ['INCLUDE', 'LIB', 'PATH', 'SYSTEMROOT', 'TEMP', 'TMP', 'USERPROFILE']:
    if k in env:
        env_clean[k] = env[k]

for k, v in os.environ.items():
    if k not in env_clean:
        env_clean[k] = v

# Build all crates
print("=== Building all crates ===")
crates = ['rairos-core', 'rairos-parser', 'rairos-llm', 'rairos-cli',
          'rairos-research', 'rairos-web', 'rairos-kg']

for crate in crates:
    print(f"\n--- Building {crate} ---")
    r = subprocess.run(
        ['cargo', 'build', '--package', crate],
        cwd=workdir,
        env=env_clean,
        timeout=600
    )
    if r.returncode == 0:
        print(f"  {crate}: OK")
    else:
        print(f"  {crate}: FAILED (rc={r.returncode})")
        print("  STDERR:", r.stderr[-1000:])
        break

# Clean up
try:
    os.remove(env_script)
except Exception:
    pass

print("\n=== Done ===")
