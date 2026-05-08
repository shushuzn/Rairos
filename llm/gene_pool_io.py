"""Gene Pool I/O — unified read/write for capsules.json + import/export."""

from __future__ import annotations

import json
import tarfile
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"  # unified with EvolutionTracker
CAPSULE_PATH = GP_DIR / "capsules.json"


# =============================================================================
# Unified capsule read API (replaces scattered _read_capsules_json / _load_capsules)
# =============================================================================


def load_capsules(
    gap_type: Optional[str] = None,
    status: Optional[str] = None,
    source_paper_id: Optional[str] = None,
) -> List[Dict[str, Any]]:
    """Load capsules with optional filtering by gap_type, status, source_paper_id.

    gene_pool.jsonl is the authoritative store. After reading, this function
    syncs capsules.json so the web UI cache stays fresh.
    """
    if not GP_DIR.exists():
        return []
    try:
        text = (GP_DIR / "gene_pool.jsonl").read_text(encoding="utf-8").strip()
        capsules = [json.loads(l) for l in text.split("\n") if l.strip()]
    except Exception:
        return []

    # Sync capsules.json for backward compat (web UI, evolution.py reads)
    _sync_capsules_json(capsules)

    if gap_type is not None:
        capsules = [c for c in capsules if c.get("action_gap_type") == gap_type]
    if status is not None:
        capsules = [c for c in capsules if c.get("status") == status]
    if source_paper_id is not None:
        capsules = [
            c for c in capsules if c.get("archetype", {}).get("source_paper_id") == source_paper_id
        ]
    return capsules


def _sync_capsules_json(capsules: List[Dict[str, Any]]) -> None:
    """Rebuild capsules.json from gene_pool.jsonl data (one-way sync)."""
    try:
        cpath = GP_DIR / "capsules.json"
        GP_DIR.mkdir(parents=True, exist_ok=True)
        cpath.write_text(
            json.dumps({"version": "1.0", "capsules": capsules}, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
    except Exception:
        pass


def get_capsule_by_paper(paper_id: str, gap_type: Optional[str] = None) -> Optional[Dict[str, Any]]:
    """Get the most recent active capsule for a given paper."""
    capsules = load_capsules(gap_type=gap_type, status="active")
    candidates = [c for c in capsules if c.get("archetype", {}).get("source_paper_id") == paper_id]
    if not candidates:
        return None
    return max(candidates, key=lambda c: c.get("created_at", ""))


def paper_exists_in_pool(paper_id: str, gap_type: Optional[str] = None) -> bool:
    """Check if a paper already has a capsule entry (for deduplication)."""
    return get_capsule_by_paper(paper_id, gap_type) is not None


def fingerprint_exists_in_pool(fingerprint: str, gap_type: Optional[str] = None) -> bool:
    """Check if a capsule with this algorithm fingerprint already exists.

    Enables cross-paper dedup: two different papers implementing the same
    algorithm (same fingerprint) should not both be encoded.
    """
    capsules = load_capsules(gap_type=gap_type, status="active")
    for c in capsules:
        if c.get("archetype", {}).get("algorithm_fingerprint") == fingerprint:
            return True
    return False


def get_gene_pool_diversity() -> Dict[str, Any]:
    """Return diversity metrics for the Gene Pool.

    Metrics:
    - shannon_index: Shannon entropy of algorithm-family distribution (higher = more diverse)
    - capsule_count: total active capsules
    - family_counts: capsule count per algorithm family (from trigger_keywords)
    - gap_type_counts: capsule count per gap_type
    - diversity_score: 0-100 normalized score (100 = perfectly balanced)
    - underrepresented_families: families with < 10% of median representation
    - overrepresented_families: families with > 2x median representation
    """
    capsules = load_capsules(status="active")
    if not capsules:
        return {
            "shannon_index": 0.0,
            "capsule_count": 0,
            "family_counts": {},
            "gap_type_counts": {},
            "diversity_score": 0,
            "underrepresented_families": [],
            "overrepresented_families": [],
        }

    import math

    # ─── Algorithm family from trigger_keywords ───────────────────────────────────
    FAMILY_KEYWORDS = {
        "attention": ["attention", "transformer", "multi-head", "self-attention", "cross-attention"],
        "reinforcement": ["rl", "reinforcement", "policy", "reward", "agent", "DQN", "PPO", "A3C"],
        "language_model": ["LM", "language model", "decoder", "autoregressive", "LLM", "GPT", "BERT"],
        "vision": ["CNN", "convolution", "resnet", "image", "vision", "ViT", "classification"],
        "optimization": ["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
        "graph": ["GNN", "graph", "node", "edge", "message passing"],
        "reasoning": ["reasoning", "chain-of-thought", "logical", "inference", "planning"],
        "embodied": ["embodied", "robotics", "navigation", "control", "motor"],
    }

    def family_of(keywords: List[str]) -> str:
        kw_set = {k.lower() for k in keywords}
        for fam, fam_kws in FAMILY_KEYWORDS.items():
            if any(fk in kw_set for fk in fam_kws):
                return fam
        return "other"

    family_counts: Dict[str, int] = {}
    gap_type_counts: Dict[str, int] = {}
    for cap in capsules:
        kws = cap.get("trigger_keywords", [])
        fam = family_of(kws) if kws else "other"
        family_counts[fam] = family_counts.get(fam, 0) + 1
        gt = cap.get("action_gap_type", "unknown")
        gap_type_counts[gt] = gap_type_counts.get(gt, 0) + 1

    # ─── Shannon entropy of family distribution ─────────────────────────────────
    total = len(capsules)
    shannon = 0.0
    for count in family_counts.values():
        p = count / total
        if p > 0:
            shannon -= p * math.log(p)
    max_entropy = math.log(len(family_counts)) if family_counts else 1.0
    normalized_shannon = shannon / max_entropy if max_entropy > 0 else 0.0

    # ─── Diversity score (0-100) ───────────────────────────────────────────────
    # Penalize both imbalance (low shannon) and low coverage (few families)
    family_coverage = len(family_counts) / len(FAMILY_KEYWORDS)
    diversity_score = int(normalized_shannon * 0.6 * 100 + family_coverage * 0.4 * 100)

    # ─── Under/over-represented families ──────────────────────────────────────
    median_count = sorted(family_counts.values())[len(family_counts) // 2] if family_counts else 1
    underrep = [f for f, c in family_counts.items() if c < median_count * 0.1]
    overrep = [f for f, c in family_counts.items() if c > median_count * 2.0]

    return {
        "shannon_index": round(shannon, 4),
        "shannon_normalized": round(normalized_shannon, 4),
        "capsule_count": total,
        "family_counts": dict(sorted(family_counts.items(), key=lambda x: -x[1])),
        "gap_type_counts": dict(sorted(gap_type_counts.items(), key=lambda x: -x[1])),
        "diversity_score": diversity_score,
        "underrepresented_families": sorted(underrep),
        "overrepresented_families": sorted(overrep),
        "median_family_count": median_count,
        "family_coverage": round(family_coverage, 4),
    }


def export_pool() -> Dict[str, Any]:
    """Export the full Gene Pool as a JSON dict."""
    capsules_path = GP_DIR / "capsules.json"
    jsonl_path = GP_DIR / "gene_pool.jsonl"

    result: Dict[str, Any] = {
        "version": "1.0",
        "exported_at": str(__import__("datetime").datetime.now().isoformat()),
    }

    if capsules_path.exists():
        result["capsules"] = json.loads(capsules_path.read_text(encoding="utf-8"))

    if jsonl_path.exists():
        genes = []
        for line in jsonl_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line:
                genes.append(json.loads(line))
        result["genes"] = genes

    return result


def import_pool(data: Dict[str, Any], merge: bool = True) -> Dict[str, int]:
    """Import Gene Pool from JSON dict. merge=True appends; False replaces."""
    capsules_path = GP_DIR / "capsules.json"
    jsonl_path = GP_DIR / "gene_pool.jsonl"
    GP_DIR.mkdir(parents=True, exist_ok=True)

    stats = {"capsules_imported": 0, "genes_imported": 0}

    capsules = data.get("capsules", [])
    if capsules:
        existing: Dict[str, Any] = {}
        if capsules_path.exists():
            existing = json.loads(capsules_path.read_text(encoding="utf-8"))
        existing_caps = existing.get("capsules", [])
        existing_ids = {c["capsule_id"] for c in existing_caps}
        if merge:
            new_caps = [c for c in capsules if c.get("capsule_id") not in existing_ids]
        else:
            # merge=False: replace entire pool, no deduplication needed
            new_caps = capsules
            existing_caps = []
        merged = {"version": "1.0", "capsules": existing_caps + new_caps}
        capsules_path.write_text(json.dumps(merged, indent=2, ensure_ascii=False), encoding="utf-8")
        stats["capsules_imported"] = len(new_caps)

    genes = data.get("genes", [])
    if genes:
        existing_gene_ids: set = set()
        if merge and jsonl_path.exists():
            for line in jsonl_path.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if line:
                    existing_gene_ids.add(json.loads(line).get("gene_id", ""))
        new_genes = [g for g in genes if g.get("gene_id", "") not in existing_gene_ids]
        # Use "a" for merge (append), "w" for replace
        mode = "a" if merge else "w"
        with jsonl_path.open(mode, encoding="utf-8") as f:
            for g in new_genes:
                f.write(json.dumps(g, ensure_ascii=False) + "\n")
        stats["genes_imported"] = len(new_genes)

    return stats


# =============================================================================
# Gene Pool Backup — daily snapshots with 30-version retention.
# Backs up: gene_pool.jsonl + capsules.json as a tar.gz archive.
# =============================================================================

import shutil
from datetime import datetime
from pathlib import Path

BACKUP_DIR = Path.home() / ".ai_research_os" / "backups"
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
    import tempfile
    import os

    BACKUP_DIR.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime(TIMESTAMP_FORMAT)

    with tempfile.TemporaryDirectory() as tmpdir:
        tar_path = os.path.join(tmpdir, f"gene_pool_{stamp}.tar.gz")
        with tarfile.open(tar_path, "w:gz") as tar:
            for fname in ["gene_pool.jsonl", "capsules.json"]:
                src = GP_DIR / fname
                if src.exists():
                    tar.add(src, arcname=fname)
        shutil.copy2(tar_path, BACKUP_DIR / f"gene_pool_{stamp}.tar.gz")

    _prune_old_backups()
    return stamp


def _prune_old_backups() -> None:
    """Remove backups beyond MAX_BACKUPS, keeping newest."""
    backups = _list_backups()
    for old in backups[MAX_BACKUPS:]:
        for p in BACKUP_DIR.glob(f"gene_pool_{old}.tar.gz"):
            p.unlink()


def restore_backup(stamp: str) -> bool:
    """Restore Gene Pool from a specific backup stamp."""
    backup_file = BACKUP_DIR / f"gene_pool_{stamp}.tar.gz"
    if not backup_file.exists():
        return False

    import tempfile

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


def render_backup_html(info: dict | None = None) -> str:
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


def render_io_html() -> str:
    lines = ['<div class="pool-io">']
    lines.append("<h3>📦 Gene Pool Import / Export</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
        "Export your Gene Pool as JSON, or import from a previously exported file.</p>"
    )

    lines.append("<div style='display:flex;gap:12px;margin-bottom:20px'>")
    lines.append("<button id='exportBtn'>⬇ Export JSON</button>")
    lines.append("</div>")

    lines.append(
        "<div style='border:2px dashed #ccc;border-radius:6px;padding:20px;text-align:center;margin-bottom:16px'>"
    )
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:10px'>Drop a Gene Pool JSON export here to import</p>"
    )
    lines.append("<input type='file' id='importFile' accept='.json' style='font-size:12px'>")
    lines.append("</div>")

    lines.append("<div id='io-status' style='font-size:13px;margin-top:10px'></div>")

    lines.append("""
<script>
document.getElementById('exportBtn').addEventListener('click', function() {
    fetch('/gene-pool/io/export')
      .then(function(r) { return r.json(); })
      .then(function(d) {
          var blob = new Blob([JSON.stringify(d, null, 2)], {type: 'application/json'});
          var url = URL.createObjectURL(blob);
          var a = document.createElement('a'); a.href = url;
          a.download = 'gene_pool_export_' + new Date().toISOString().slice(0,10) + '.json';
          document.body.appendChild(a); a.click(); document.body.removeChild(a);
          URL.revokeObjectURL(url);
      });
});
document.getElementById('importFile').addEventListener('change', function(el) {
    var file = el.target.files[0]; if (!file) return;
    var reader = new FileReader();
    reader.onload = function(e) {
        var statusEl = document.getElementById('io-status');
        try {
            var data = JSON.parse(e.target.result);
            fetch('/gene-pool/io/import', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(data)
            }).then(function(r) { return r.json(); }).then(function(d) {
                var msg = '✅ Imported: ' + d.capsules_imported + ' capsules, ' + d.genes_imported + ' genes';
                statusEl.textContent = msg;
            }).catch(function(err) {
                statusEl.textContent = '❌ Import failed: ' + err.message;
            });
        } catch(err) {
            statusEl.textContent = '❌ Invalid file: ' + err.message;
        }
    };
    reader.readAsText(file);
});
</script>""")

    lines.append(
        "<style>.pool-io { font-family: Georgia, serif; } button { background:#6B8FB5; color:white; border:none; border-radius:4px; padding:8px 18px; cursor:pointer; font-size:13px; }</style>"
    )
    lines.append("</div>")
    return "\n".join(lines)
