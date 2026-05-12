# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v1.7.x  | :white_check_mark: |
| < v1.7  | :x:                |

## Reporting a Vulnerability

Please report security vulnerabilities via [GitHub Security Advisories](https://github.com/shushuzn/Rairos/security/advisories) rather than public issues.

## Known Vulnerabilities

### urllib3 CVEs (GHSA-mf9v-mfxr-j63j, GHSA-qccp-gfcp-xxvc)

**Affected:** Python dependencies (boto3, aiohttp, etc.)
**Status:** Acknowledged. No patched version available yet. Upgrade to urllib3 >= 2.7.1 when released.
**Impact:** Requires accepting HTTP responses from untrusted servers. Mitigated by not processing untrusted HTTP content.
**Tracking:** [Dependabot alerts](https://github.com/shushuzn/Rairos/security/dependabot)

### GitPython CVE (GHSA-mv93-w799-cj2w)

**Affected:** Direct dependency (none — GitPython is not a direct dependency of Rairos)
**Status:** Not applicable. GitPython is not used by Rairos.
**Note:** Dependabot detected it as a transitive dependency of a development tool.

## Dependency Management

- **Python:** Managed via `pyproject.toml` + `uv`. Run `uv pip tree` to audit.
- **Rust:** Managed via `Cargo.toml`. Run `cargo audit` in CI.
- **Auto-updates:** Renovate bot handles dependency PRs automatically.

## Security Tools

| Tool | Purpose | Run |
|------|---------|-----|
| `cargo audit` | Rust vulnerability scanning | CI (rust.yml) |
| `bandit` | Python security linting | CI (ci.yml) |
| `ruff` | Python linting | CI (ci.yml) |
| `clippy` | Rust linting | CI (rust.yml) |
| Renovate | Auto dependency updates | GitHub App |

## Hardened Dependencies

For production deployments, prefer `--no-deps` installs and pin exact versions:

```bash
uv pip install --no-deps -r requirements.txt
```
