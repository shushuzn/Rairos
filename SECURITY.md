# Security Policy

## Supported Versions

Rairos is under active development. Security patches are provided for the latest release.

| Version | Supported |
|---------|-----------|
| latest  | ✅ |

## Reporting a Vulnerability

Open a [security advisory](https://github.com/shushuzn/Rairos/security/advisories/new) or email the maintainers directly.

We aim to acknowledge receipt within 48 hours and provide a timeline for the fix.

## Known Vulnerabilities

### OpenSSL / ring (transitive)

Rairos uses `rusqlite` and `reqwest`, which depend on `openssl` and `ring` respectively.
These are system-level dependencies managed by Cargo. Run `cargo audit` to check for known vulnerabilities.

### Dependabot Alerts

Rust crate vulnerabilities are tracked via Dependabot. Check [GitHub Security](https://github.com/shushuzn/Rairos/security/dependabot) for active alerts.

## Security Best Practices

- Use `cargo audit` to scan for known vulnerabilities
- Keep Rust toolchain up to date: `rustup update`
- Run with `RAIROS_DB` pointing to an isolated database path for sandboxed testing
