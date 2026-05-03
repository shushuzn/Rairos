"""Web Authentication — optional API key management + session persistence.

Simple session-based auth:
  - Optional: if no sessions exist, the web UI is fully open (single-user mode)
  - When a username/password is set, login is required
  - Sessions stored in ~/.ai_research_os/sessions.json
"""

from __future__ import annotations

import hashlib
import json
import secrets
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional

AUTH_FILE = Path.home() / ".ai_research_os" / "auth.json"
SESSIONS_FILE = Path.home() / ".ai_research_os" / "sessions.json"
SESSION_TTL = 86400 * 7  # 7 days


def _hash_password(password: str, salt: str) -> str:
    return hashlib.pbkdf2_hmac("sha256", password.encode(), salt.encode(), 100_000).hex()


def _generate_salt() -> str:
    return secrets.token_hex(16)


@dataclass
class User:
    username: str
    salt: str
    password_hash: str
    created_at: str


def _load_auth() -> Dict[str, Any]:
    if not AUTH_FILE.exists():
        return {"users": {}, "setup_complete": False}
    return json.loads(AUTH_FILE.read_text(encoding="utf-8"))


def _save_auth(data: Dict[str, Any]) -> None:
    AUTH_FILE.parent.mkdir(parents=True, exist_ok=True)
    AUTH_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def _load_sessions() -> Dict[str, Any]:
    if not SESSIONS_FILE.exists():
        return {}
    return json.loads(SESSIONS_FILE.read_text(encoding="utf-8"))


def _save_sessions(sessions: Dict[str, Any]) -> None:
    SESSIONS_FILE.parent.mkdir(parents=True, exist_ok=True)
    SESSIONS_FILE.write_text(json.dumps(sessions, indent=2, ensure_ascii=False), encoding="utf-8")


def is_auth_enabled() -> bool:
    auth = _load_auth()
    return auth.get("setup_complete", False) is True


def setup_admin(username: str, password: str) -> bool:
    """Set up the first admin user. Must be called before any auth is enabled."""
    auth = _load_auth()
    if auth.get("setup_complete"):
        return False
    salt = _generate_salt()
    auth["users"][username] = {
        "salt": salt,
        "password_hash": _hash_password(password, salt),
        "created_at": str(__import__("datetime").datetime.now().isoformat()),
    }
    auth["setup_complete"] = True
    _save_auth(auth)
    return True


def verify_login(username: str, password: str) -> bool:
    auth = _load_auth()
    user = auth.get("users", {}).get(username)
    if not user:
        return False
    return user["password_hash"] == _hash_password(password, user["salt"])


def create_session(username: str) -> str:
    """Create a new session and return the session token."""
    token = secrets.token_hex(32)
    sessions = _load_sessions()
    sessions[token] = {
        "username": username,
        "created_at": time.time(),
        "expires_at": time.time() + SESSION_TTL,
    }
    _save_sessions(sessions)
    return token


def validate_session(token: str) -> Optional[str]:
    """Return username if valid session, None otherwise."""
    sessions = _load_sessions()
    sess = sessions.get(token)
    if not sess:
        return None
    if time.time() > sess.get("expires_at", 0):
        del sessions[token]
        _save_sessions(sessions)
        return None
    return sess.get("username")


def revoke_session(token: str) -> None:
    sessions = _load_sessions()
    sessions.pop(token, None)
    _save_sessions(sessions)
