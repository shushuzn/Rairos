"""Gene Pool Backup Scheduler — daily snapshots with 30-version retention.

Backs up:
  - gene_pool.jsonl (EvolutionTracker raw genes)
  - capsules.json (capsule metadata)
"""

from __future__ import annotations

import json
import shutil
from datetime import datetime
from pathlib import Path
from typing import List, Optional

BACKUP_DIR = Path.home() / ".ai_research_os" / "backups"
GP_DIR = Path.home() / ".ai_research_os" / "gene_pool"
MAX_BACKUPS = 30
TIMESTAMP_FORMAT = "%Y%m%d"


def _backup_name(stamp: str) -> str:
    return f"gene_pool_{stamp}.tar.gz"


def _list_backups() -> List[str]:
    if not BACKUP_DIR.exists():
        return []
    names = [p.stem.replace("gene_pool_", "") for p in BACKUP_DIR.glob("gene_pool_*.tar.gz")]
    names.sort(reverse=True)
    return names


def create_backup() -> str:
    """Create a timestamped backup of both Gene Pool stores."""
    BACKUP_DIR.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime(TIMESTAMP_FORMAT)

    import tarfile
    import tempfile
    import os

    with tempfile.TemporaryDirectory() as tmpdir:
        tar_path = os.path.join(tmpdir, f"gene_pool_{stamp}.tar.gz")
        with tarfile.open(tar_path, "w:gz") as tar:
            for fname in ["gene_pool.jsonl", "capsules.json"]:
                src = GP_DIR / fname
                if src.exists():
                    tar.add(src, arcname=fname)
        shutil.copy2(tar_path, BACKUP_DIR / f"gene_pool_{stamp}.tar.gz")

    _prune_old()
    return stamp


def _prune_old() -> None:
    """Remove backups beyond MAX_BACKUPS, keeping newest."""
    backups = _list_backups()
    for old in backups[MAX_BACKUPS:]:
        for p in BACKUP_DIR.glob(f"gene_pool_{old}.tar.gz"):
            p.unlink()


def restore_backup(stamp: str) -> bool:
    """Restore Gene Pool from a specific backup stamp."""
    import tarfile

    backup_file = BACKUP_DIR / f"gene_pool_{stamp}.tar.gz"
    if not backup_file.exists():
        return False

    import tempfile
    import os

    with tempfile.TemporaryDirectory() as tmpdir:
        with tarfile.open(backup_file, "r:gz") as tar:
            tar.extractall(tmpdir)
        for fname in ["gene_pool.jsonl", "capsules.json"]:
            src = Path(tmpdir) / fname
            if src.exists():
                shutil.copy2(src, GP_DIR / fname)
    return True


def get_backup_info() -> dict:
    """Return info about available backups."""
    backups = _list_backups()
    total_size = sum(
        (BACKUP_DIR / f"gene_pool_{b}.tar.gz").stat().st_size
        for b in backups
        if (BACKUP_DIR / f"gene_pool_{b}.tar.gz").exists()
    )
    return {
        "available": len(backups),
        "stamps": backups[:10],
        "total_size_mb": round(total_size / 1024 / 1024, 2),
        "max_backups": MAX_BACKUPS,
    }


def render_backup_html(info: Optional[dict] = None) -> str:
    if info is None:
        info = get_backup_info()

    lines = ['<div class="backup-panel">']
    lines.append("<h3>💾 Gene Pool Backup</h3>")
    lines.append(
        f"<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
        f"<b>{info['available']}</b> backups · "
        f"{info['total_size_mb']}MB total · "
        f"max {info['max_backups']} versions retained</p>"
    )

    lines.append("<div style='margin-bottom:16px'>")
    lines.append(
        "<button onclick='triggerBackup()' style='background:#6B8FB5;color:white;border:none;"
        "border-radius:4px;padding:8px 16px;cursor:pointer;font-size:13px'>"
        "☁️ Take Backup Now</button>"
    )
    lines.append("</div>")

    if info["stamps"]:
        lines.append("<table style='width:100%;border-collapse:collapse;font-size:13px'>")
        lines.append(
            "<tr style='border-bottom:1px solid #e0dbd4'><th style='text-align:left;padding:6px 8px'>Date</th>"
            "<th style='text-align:right;padding:6px 8px'>Action</th></tr>"
        )
        for stamp in info["stamps"]:
            yr, mo, day = stamp[:4], stamp[4:6], stamp[6:8]
            lines.append(
                f"<tr style='border-bottom:1px solid #f0ebe5'>"
                f"<td style='padding:6px 8px'>{yr}-{mo}-{day}</td>"
                f"<td style='text-align:right;padding:6px 8px'>"
                f"<button onclick='restoreBackup(\"{stamp}\")' style='font-size:11px;padding:2px 8px;"
                f"cursor:pointer;background:transparent;border:1px solid #ccc;border-radius:3px'>Restore</button>"
                f"</td></tr>"
            )
        lines.append("</table>")
    else:
        lines.append(
            "<p style='color:#A89E8C;font-size:13px'>No backups yet. Click 'Take Backup Now' to create your first snapshot.</p>"
        )

    lines.append("""
<script>
function triggerBackup() {
    fetch('/gene-pool/backup/create', {method:'POST'})
      .then(r => r.json())
      .then(d => { alert('Backup created: ' + d.stamp); location.reload(); });
}
function restoreBackup(stamp) {
    if (!confirm('Restore backup from ' + stamp + '? Current Gene Pool will be overwritten.')) return;
    fetch('/gene-pool/backup/restore/' + stamp, {method:'POST'})
      .then(r => r.json())
      .then(d => { alert(d.message); location.reload(); });
}
</script>""")

    lines.append("<style>.backup-panel { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
