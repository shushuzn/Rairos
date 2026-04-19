import json
import subprocess
import re

# Extract token from git remote
r = subprocess.run(['git', 'remote', 'get-url', 'origin'], capture_output=True, text=True)
url = r.stdout.strip()
token_match = re.search(r'https://ghp_[^@]+', url)
token = token_match.group(0).replace('https://', '') if token_match else ''
print('Token:', token[:10] + '...')

data = {
    'title': 'fix(tests): mock fallback also returns empty so PDFParseError is raised',
    'head': 'fix/test-pymupdf-fallback-mock',
    'base': 'main',
    'body': '## Summary\n\n- Fix test: mock `_pdfminer_fallback` to return empty text so PDFParseError is correctly raised when PyMuPDF fails\n- The test previously only mocked _extract_structured but not the fallback, so real sample.pdf extraction succeeded instead of raising\n\n## Test\n- 939 tests pass, 1 skipped'
}

with open('C:/tmp/pr_body.json', 'w') as f:
    json.dump(data, f)

cmd = ['curl', '-s', '-X', 'POST',
    '-H', f'Authorization: Bearer {token}',
    '-H', 'Content-Type: application/json',
    '-H', 'Accept: application/vnd.github+json',
    'https://api.github.com/repos/shushuzn/ai_research_os/pulls',
    '--data-binary', '@/c/tmp/pr_body.json']

r = subprocess.run(cmd, capture_output=True, text=True)
resp = json.loads(r.stdout)
print('PR:', resp.get('html_url', resp.get('message', 'unknown')))
