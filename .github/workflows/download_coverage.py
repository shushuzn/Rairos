#!/usr/bin/env python3
"""Download coverage artifacts from pytest matrix jobs via GitHub API.

Uses the run-level artifacts endpoint (works reliably) instead of job-level endpoint (404s).
"""
import json
import os
import sys
import urllib.request
import zipfile
import pathlib

GH_TOKEN = os.environ.get("GH_TOKEN", "")
REPO = os.environ.get("REPO", "")
RUN_ID = os.environ.get("RUN_ID", "")

pv = sys.argv[1] if len(sys.argv) > 1 else None
if not pv:
    print("[ERROR] Python version required as argument")
    sys.exit(1)

artifact_name = f"pytest-output-{pv}"
out_dir = pathlib.Path(f"cc-{pv}")
out_dir.mkdir(exist_ok=True)
zip_path = out_dir / "coverage.zip"

# Use run-level artifacts endpoint (works reliably unlike job-level endpoint)
artifacts_url = f"https://api.github.com/repos/{REPO}/actions/runs/{RUN_ID}/artifacts"
req = urllib.request.Request(
    artifacts_url,
    headers={
        "Authorization": f"Bearer {GH_TOKEN}",
        "Accept": "application/vnd.github.v3+json",
    },
)
with urllib.request.urlopen(req) as resp:
    artifacts_data = json.load(resp)

archive_url = None
for artifact in artifacts_data.get("artifacts", []):
    if artifact["name"] == artifact_name:
        archive_url = artifact["archive_download_url"]
        print(f"Found artifact: {artifact['name']} (id={artifact['id']})")
        break

if not archive_url:
    available = [a["name"] for a in artifacts_data.get("artifacts", [])]
    print(f"[ERROR] No {artifact_name} artifact found. Available: {available}")
    sys.exit(1)

# Download the artifact zip
print(f"Downloading {archive_url[:80]}...")
req = urllib.request.Request(
    archive_url,
    headers={"Authorization": f"Bearer {GH_TOKEN}"},
)
with urllib.request.urlopen(req) as resp:
    with open(zip_path, "wb") as f:
        f.write(resp.read())

print(f"Downloaded {zip_path.stat().st_size} bytes")

# Extract .coverage.* from zip
with zipfile.ZipFile(zip_path, "r") as zf:
    names = zf.namelist()
    for name in names:
        if name.endswith(f".coverage.{pv}"):
            zf.extract(name, out_dir)
            src = out_dir / name
            dst = out_dir / f".coverage.{pv}"
            src.rename(dst)
            print(f"Extracted {dst} ({dst.stat().st_size} bytes)")
            break
    else:
        print(f"[ERROR] .coverage.{pv} not found in zip")
        print(f"Files: {names}")
        sys.exit(1)

zip_path.unlink()
print(f"Done: cc-{pv}/.coverage.{pv}")
