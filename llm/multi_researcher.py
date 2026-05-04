"""Multi-Researcher Support — shared Gene Pool with source_user tags; collaborative gap tracking.

Each capsule can be tagged with a source_user so different researchers can have
their own views of the pool, or share selectively.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "gene_pool"
CAPSULES_PATH = GP_DIR / "capsules.json"
USERS_FILE = GP_DIR / "users.json"

DEFAULT_USER = "default"


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULES_PATH.exists():
        return []
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8")).get("capsules", [])


def _load_users() -> Dict[str, Any]:
    if not USERS_FILE.exists():
        return {}
    return json.loads(USERS_FILE.read_text(encoding="utf-8"))


def _save_users(users: Dict[str, Any]) -> None:
    USERS_FILE.parent.mkdir(parents=True, exist_ok=True)
    USERS_FILE.write_text(json.dumps(users, indent=2, ensure_ascii=False), encoding="utf-8")


def get_researchers() -> List[Dict[str, Any]]:
    """Return list of all researchers who have contributed capsules."""
    capsules = _load_capsules()
    user_counts: Dict[str, int] = {}
    for cap in capsules:
        uid = cap.get("source_user", DEFAULT_USER) or DEFAULT_USER
        user_counts[uid] = user_counts.get(uid, 0) + 1
    return [
        {"user_id": uid, "capsule_count": count}
        for uid, count in sorted(user_counts.items(), key=lambda x: -x[1])
    ]


def get_capsules_for_user(user_id: str) -> List[Dict[str, Any]]:
    """Return capsules visible to a specific user (own + shared)."""
    capsules = _load_capsules()
    if user_id == "all":
        return capsules
    return [
        c
        for c in capsules
        if c.get("source_user", DEFAULT_USER) == user_id
        or c.get("visibility", "shared") == "shared"
    ]


def add_researcher(user_id: str, name: str = "", email: str = "") -> bool:
    users = _load_users()
    if user_id in users:
        return False
    users[user_id] = {
        "name": name or user_id,
        "email": email,
        "joined_at": str(__import__("datetime").datetime.now().isoformat()),
    }
    _save_users(users)
    return True


def render_multi_researcher_html() -> str:
    researchers = get_researchers()
    capsules = _load_capsules()
    users = _load_users()

    lines = ['<div class="multi-researcher">']
    lines.append("<h3>👥 Multi-Researcher Support</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
        "Collaborative Gene Pool. Each researcher is tagged on their capsules.</p>"
    )

    # Researcher list
    lines.append(
        "<h4 style='font-size:13px;font-weight:700;color:#333;margin-bottom:8px'>"
        f"Researchers ({len(researchers)})</h4>"
    )

    if not researchers:
        lines.append(
            "<p style='color:#A89E8C;font-size:13px'>No researchers yet. "
            "Add a researcher ID to start collaborating.</p>"
        )
    else:
        for r in researchers:
            uid = r["user_id"]
            info = users.get(uid, {})
            lines.append(f"""
<div style='display:flex;justify-content:space-between;align-items:center;padding:10px 12px;background:#f8f4ef;border-radius:6px;margin-bottom:8px'>
  <div>
    <div style='font-weight:600;font-size:13px'>{info.get("name", uid)}</div>
    <div style='font-size:11px;color:#A89E8C'>{uid} · {r["capsule_count"]} capsules</div>
  </div>
  <button onclick="switchView('{uid}')" style="font-size:11px;padding:3px 10px;cursor:pointer;border-radius:3px;border:1px solid #ccc;background:transparent">
    View
  </button>
</div>""")

    # Add researcher
    lines.append("<div style='margin-top:16px;padding:14px;background:#f8f4ef;border-radius:6px'>")
    lines.append(
        "<h4 style='font-size:13px;font-weight:700;color:#333;margin-bottom:8px'>Add Researcher</h4>"
    )
    lines.append(
        "<input type='text' id='newUserId' placeholder='User ID' style='font-size:12px;padding:5px 8px;border-radius:4px;border:1px solid #ccc;margin-right:6px;width:120px'>"
    )
    lines.append(
        "<input type='text' id='newUserName' placeholder='Name' style='font-size:12px;padding:5px 8px;border-radius:4px;border:1px solid #ccc;margin-right:6px;width:140px'>"
    )
    lines.append(
        "<button onclick='addResearcher()' style='background:#6B8FB5;color:white;border:none;border-radius:4px;padding:5px 14px;cursor:pointer;font-size:12px'>Add</button>"
    )
    lines.append("</div>")

    # Shared vs private toggle per capsule (info)
    total = len(capsules)
    shared = sum(1 for c in capsules if c.get("visibility", "shared") == "shared")
    lines.append(
        f"<div style='margin-top:16px;font-size:12px;color:#7a7570'>{shared}/{total} capsules shared · visibility is set per capsule via the capsule editor.</div>"
    )

    lines.append("""
<script>
function addResearcher() {
    var uid = document.getElementById('newUserId').value.trim();
    var name = document.getElementById('newUserName').value.trim();
    if (!uid) { alert('User ID required'); return; }
    fetch('/researchers/add', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({user_id: uid, name: name})
    }).then(function(r) { return r.json(); })
      .then(function(d) {
          if (d.success) location.reload();
          else alert('Error: ' + (d.error || 'unknown'));
      });
}
function switchView(uid) {
    fetch('/researchers/capsules/' + uid)
      .then(function(r) { return r.json(); })
      .then(function(d) {
          alert('User ' + uid + ' has ' + d.count + ' visible capsules');
      });
}
</script>""")

    lines.append("<style>.multi-researcher { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
