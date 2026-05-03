"""Gene Pool Import/Export — backup/restore pool as JSON; share across machines."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict

GP_DIR = Path.home() / ".ai_research_os" / "gene_pool"


def export_pool() -> Dict[str, Any]:
    """Export the full Gene Pool as a JSON dict."""
    capsules_path = GP_DIR / "capsules.json"
    jsonl_path = GP_DIR / "gene_pool.jsonl"

    result: Dict[str, Any] = {"version": "1.0", "exported_at": str(__import__("datetime").datetime.now().isoformat())}

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

    if not merge and capsules_path.exists():
        capsules_path.unlink()
    if not merge and jsonl_path.exists():
        jsonl_path.unlink()

    capsules = data.get("capsules", {})
    if capsules:
        existing: Dict[str, Any] = {}
        if capsules_path.exists():
            existing = json.loads(capsules_path.read_text(encoding="utf-8"))
        existing_caps = existing.get("capsules", [])
        existing_ids = {c["capsule_id"] for c in existing_caps}
        new_caps = [c for c in capsules.get("capsules", []) if c.get("capsule_id") not in existing_ids]
        merged = {"version": "1.0", "capsules": existing_caps + new_caps}
        capsules_path.write_text(json.dumps(merged, indent=2, ensure_ascii=False), encoding="utf-8")
        stats["capsules_imported"] = len(new_caps)

    genes = data.get("genes", [])
    if genes:
        existing_ids: set = set()
        if jsonl_path.exists():
            for line in jsonl_path.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if line:
                    existing_ids.add(json.loads(line).get("gene_id", ""))
        new_genes = [g for g in genes if g.get("gene_id", "") not in existing_ids]
        with jsonl_path.open("a", encoding="utf-8") as f:
            for g in new_genes:
                f.write(json.dumps(g, ensure_ascii=False) + "\n")
        stats["genes_imported"] = len(new_genes)

    return stats


def render_io_html() -> str:
    lines = ['<div class="pool-io">']
    lines.append("<h3>📦 Gene Pool Import / Export</h3>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
                 "Export your Gene Pool as JSON, or import from a previously exported file.</p>")

    lines.append("<div style='display:flex;gap:12px;margin-bottom:20px'>")
    lines.append("<button id='exportBtn'>⬇ Export JSON</button>")
    lines.append("</div>")

    lines.append("<div style='border:2px dashed #ccc;border-radius:6px;padding:20px;text-align:center;margin-bottom:16px'>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:10px'>Drop a Gene Pool JSON export here to import</p>")
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

    lines.append("<style>.pool-io { font-family: Georgia, serif; } button { background:#6B8FB5; color:white; border:none; border-radius:4px; padding:8px 18px; cursor:pointer; font-size:13px; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
