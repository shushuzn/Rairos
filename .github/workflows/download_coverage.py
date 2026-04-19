#!/usr/bin/env python3
"""Download coverage artifacts from pytest matrix jobs via GitHub API."""
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

out_dir = pathlib.Path(f"cc-{pv}")
out_dir.mkdir(exist_ok=True)

# Step 1: Find the test-pytest job for this Python version
jobs_url = f"https://api.github.com/repos/{REPO}/actions/runs/{RUN_ID}/jobs"
req = urllib.request.Request(jobs_url, headers={"Authorization": f"Bearer {GH_TOKEN}"})
with urllib.request.urlopen(req) as resp:
    jobs_data = json.load(resp)

job_id = None
for job in jobs_data.get("jobs", []):
    if job["name"] == f"test-pytest ({pv})":
        job_id = job["id"]
        break

if not job_id:
    print(f"[ERROR] No test-pytest job found for Python {pv}")
    sys.exit(1)

print(f"Found job_id={job_id}")

# Step 2: Get artifact info for this job
artifacts_url = f"https://api.github.com/repos/{REPO}/actions/jobs/{job_id}/artifacts"
req = urllib.request.Request(artifacts_url, headers={"Authorization": f"Bearer {GH_TOKEN}"})
with urllib.request.urlopen(req) as resp:
    artifacts_data = json.load(resp)

archive_url = None
for artifact in artifacts_data.get("artifacts", []):
    if artifact["name"] == "coverage-data":
        archive_url = artifact["archive_url"]
        break

if not archive_url:
    print(f"[ERROR] No coverage-data artifact found for Python {pv}")
    sys.exit(1)

print(f"Found archive_url")

# Step 3: Download the artifact zip
zip_path = out_dir / "coverage.zip"
req = urllib.request.Request(archive_url, headers={"Authorization": f"Bearer {GH_TOKEN}"})
with urllib.request.urlopen(req) as resp:
    with open(zip_path, "wb") as f:
        f.write(resp.read())

print(f"Downloaded {zip_path.stat().st_size} bytes")

# Step 4: Extract .coverage.* from zip
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
