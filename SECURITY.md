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

## API Key Protection

Rairos uses environment variables for sensitive credentials. **Never commit API keys to git.**

### Required Setup

1. **Copy the example file:**
   ```bash
   cp .env.example .env
   ```

2. **Edit `.env` with your actual keys:**
   ```bash
   nano .env
   ```

3. **Ensure `.env` is ignored by git:**
   ```bash
   # This should already be in .gitignore:
   # .env
   # .env.*
   # !.env.example
   ```

### Key Files

| File | Purpose | In Git? |
|------|---------|---------|
| `.env` | Actual API keys | ❌ Never |
| `.env.example` | Template (empty values) | ✅ Yes |
| `~/.ai_research_os/` | Data directory | ❌ No |

### Permissions

```bash
# Restrict permissions on sensitive files
chmod 600 .env
chmod 600 ~/.ai_research_os/secrets
```

### Pre-commit Hook

Rairos includes a pre-commit hook that scans for secrets:

```bash
# Install pre-commit (optional, for local secret scanning)
pip install pre-commit
pre-commit install

# Or use the built-in hook (already in .git/hooks/pre-commit)
```

The hook blocks commits containing detected API keys.

### If You Accidentally Commit a Key

1. **Immediately rotate the key** at the provider (OpenAI, GitHub, etc.)
2. **Remove from git history:**
   ```bash
   git filter-branch --force --index-filter \
     'git rm --cached --ignore-unmatch .env' \
     --prune-empty --tag-name-filter cat -- --all
   ```
3. **Push cleaned history:**
   ```bash
   git push origin --force --all
   ```
4. **Add `.env` to `.gitignore` if not present**
