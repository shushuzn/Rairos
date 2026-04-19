#!/usr/bin/env python3
"""Upload coverage artifact from test-pytest job via GitHub API.
Runs in the same step as pytest so .coverage file is guaranteed to exist.
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
PV = os.environ.get("PV", "")  # python version, e.g. "3.11"

coverage_path = pathlib.Path(".coverage")
if not coverage_path.exists():
    print(f"[ERROR] .coverage not found at {coverage_path.absolute()}")
    sys.exit(1)

print(f"Found .coverage: {coverage_path.stat().st_size} bytes")

# Create a zip containing just the .coverage file
zip_path = pathlib.Path(f"coverage-data-{PV}.zip")
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.write(coverage_path, arcname=f".coverage.{PV}")

print(f"Created {zip_path} ({zip_path.stat().st_size} bytes)")

# Upload via GitHub API
# Step 1: Create the artifact
create_url = f"https://api.github.com/repos/{REPO}/actions/artifacts"
zip_size = zip_path.stat().st_size
create_data = json.dumps({
    "name": f"coverage-data-{PV}",
    "size_bytes": zip_size,
    "content_type": "application/zip",
}).encode()

create_req = urllib.request.Request(
    create_url,
    data=create_data,
    headers={
        "Authorization": f"Bearer {GH_TOKEN}",
        "Accept": "application/vnd.github.v3+json",
        "Content-Type": "application/json",
    },
    method="POST",
)
with urllib.request.urlopen(create_req) as resp:
    result = json.load(resp)
    upload_url = result.get("upload_url", "")
    artifact_id = result.get("id", "")
    print(f"Created artifact id={artifact_id}")

if not upload_url:
    print(f"[ERROR] No upload_url in response: {result}")
    sys.exit(1)

# Step 2: Upload the zip content
# Clean up the upload_url template (remove {?name,label} suffix)
upload_url = upload_url.split("{")[0]

zip_data = zip_path.read_bytes()
upload_req = urllib.request.Request(
    upload_url,
    data=zip_data,
    headers={
        "Authorization": f"Bearer {GH_TOKEN}",
        "Content-Type": "application/zip",
        "Content-Length": str(len(zip_data)),
    },
    method="POST",
)
with urllib.request.urlopen(upload_req) as resp:
    result = json.load(resp)
    print(f"Uploaded artifact: {result.get('name')} id={result.get('id')}")

zip_path.unlink()
print("Done")
