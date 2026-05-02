#!/usr/bin/env python3
"""Upload coverage artifact from test-pytest job via GitHub API (uses curl).

Rationale: actions/upload-artifact@v4 doesn't persist files across steps.
This script runs in the same step as pytest, so .coverage is guaranteed to exist.
Uses the GitHub API to create + upload the artifact directly via curl.
"""
import json
import os
import subprocess
import sys
import zipfile
import pathlib

GH_TOKEN = os.environ.get("GH_TOKEN", "")
REPO = os.environ.get("REPO", "")
PV = os.environ.get("PV", "")

coverage_path = pathlib.Path(".coverage")
if not coverage_path.exists():
    print("[ERROR] .coverage not found")
    sys.exit(0)  # Don't fail the step

print(f"Found .coverage: {coverage_path.stat().st_size} bytes")

# Create zip
zip_path = pathlib.Path(f"coverage-data-{PV}.zip")
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.write(coverage_path, arcname=f".coverage.{PV}")
print(f"Created {zip_path} ({zip_path.stat().st_size} bytes)")

# Create artifact via GitHub API
create_url = f"https://api.github.com/repos/{REPO}/actions/artifacts"
create_data = json.dumps({
    "name": f"coverage-data-{PV}",
    "size": zip_path.stat().st_size,
    "content_type": "application/zip",
}).encode()

result = subprocess.run(
    ["curl", "-s", "-S", "-X", "POST",
     "-H", f"Authorization: Bearer {GH_TOKEN}",
     "-H", "Accept: application/vnd.github.v3+json",
     "-H", "Content-Type: application/json",
     "-d", "@-",  # read body from stdin
     create_url],
    input=create_data,
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
)
try:
    create_resp = json.loads(result.stdout.decode())
except Exception:
    print(f"[ERROR] Failed to parse create response: {result.stdout.decode()[:500]}")
    print(f"[ERROR] stderr: {result.stderr.decode()[:500]}")
    sys.exit(0)

upload_url = create_resp.get("upload_url", "")
artifact_id = create_resp.get("id", "")
print(f"Artifact id={artifact_id}, upload_url={upload_url[:80] if upload_url else 'MISSING'}")

if not upload_url:
    print(f"[ERROR] No upload_url in response: {create_resp}")
    sys.exit(0)

# Upload via curl (simpler than raw HTTP for this)
upload_url_clean = upload_url.split("{")[0]
zip_data = zip_path.read_bytes()

upload_result = subprocess.run(
    ["curl", "-s", "-S", "-X", "POST",
     "-H", f"Authorization: Bearer {GH_TOKEN}",
     "-H", "Content-Type: application/zip",
     "-H", f"Content-Length: {len(zip_data)}",
     "--data-binary", "@-",  # read binary from stdin
     upload_url_clean],
    input=zip_data,
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
)
try:
    upload_resp = json.loads(upload_result.stdout.decode())
    print(f"Uploaded: {upload_resp.get('name', 'unknown')} id={upload_resp.get('id', 'unknown')}")
except Exception:
    import traceback
    traceback.print_exc()
    if upload_result.stdout:
        print(f"Upload response: {upload_result.stdout.decode()[:500]}")
    if upload_result.stderr:
        print(f"Upload stderr: {upload_result.stderr.decode()[:500]}")

zip_path.unlink()
print("Done")
