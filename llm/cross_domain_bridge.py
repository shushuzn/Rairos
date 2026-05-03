"""Cross-Domain Gap Bridge — suggest Gene Pool connections between distant research domains.

Identifies capsules whose trigger_keywords overlap with methods from unrelated categories,
e.g. physics techniques applied to ML, biology → AI, etc.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

CAPSULES_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"

# Method keywords that bridge multiple domains
BRIDGE_METHODS = {
    "optimization", "gradient", "entropy", "diffusion", "probability",
    "manifold", "latent", "embedding", "attention", "gradient descent",
    "bayesian", "sampling", "markov", "neural network", "transformer",
    "adversarial", "reinforcement learning", "supervised", "unsupervised",
    "semi-supervised", "few-shot", "zero-shot", "transfer learning",
    "causal", "counterfactual", "variational", "information theory",
}

# Domain pairs that rarely interact but have bridging potential
DOMAIN_PAIRS = [
    ("physics", "cs.AI"), ("biology", "cs.LG"), ("chemistry", "cs.CL"),
    ("math", "cs.CV"), ("economics", "cs.AI"), ("neuroscience", "cs.NE"),
]


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULES_PATH.exists():
        return []
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8")).get("capsules", [])


def _jaccard(a: List[str], b: List[str]) -> float:
    s_a, s_b = set(a), set(b)
    if not s_a or not s_b:
        return 0.0
    return len(s_a & s_b) / len(s_a | s_b)


def _keyword_overlap(kw1: List[str], kw2: List[str]) -> float:
    s1 = {k.lower() for k in kw1}
    s2 = {k.lower() for k in kw2}
    if not s1 or not s2:
        return 0.0
    return len(s1 & s2) / min(len(s1), len(s2))


def find_cross_domain_bridges() -> List[Dict[str, Any]]:
    """Find capsules from different domains whose keywords share bridging methods."""
    capsules = _load_capsules()
    results: List[Dict[str, Any]] = []

    for i, cap_a in enumerate(capsules):
        if cap_a.get("status") not in ("", "active"):
            continue
        kw_a = cap_a.get("trigger_keywords", [])
        cat_a = cap_a.get("source_category", "")

        for cap_b in capsules[i+1:]:
            if cap_b.get("status") not in ("", "active"):
                continue
            kw_b = cap_b.get("trigger_keywords", [])
            cat_b = cap_b.get("source_category", "")

            # Skip if same category cluster
            if cat_a and cat_b:
                clusters = {
                    "cs.AI|cs.LG|cs.CL|cs.CV": "AI/ML",
                    "physics": "physics",
                    "biology|medicine": "bio",
                    "math|stats": "math",
                }
                for cluster, label in clusters.items():
                    if cat_a in cluster and cat_b in cluster:
                        break
                else:
                    # Different clusters — check for bridging
                    overlap = _keyword_overlap(kw_a, kw_b)
                    if overlap >= 0.15:
                        bridge_kw = set(k.lower() for k in kw_a) & set(k.lower() for k in kw_b)
                        results.append({
                            "capsule_a": cap_a.get("action_gap_title", ""),
                            "capsule_b": cap_b.get("action_gap_title", ""),
                            "cap_a_id": cap_a.get("capsule_id", ""),
                            "cap_b_id": cap_b.get("capsule_id", ""),
                            "category_a": cat_a,
                            "category_b": cat_b,
                            "bridge_keywords": list(bridge_kw)[:5],
                            "overlap_score": round(overlap, 3),
                        })

    results.sort(key=lambda x: -x["overlap_score"])
    return results[:20]


def render_cross_domain_html(bridges: Optional[List[Dict[str, Any]]] = None) -> str:
    if bridges is None:
        bridges = find_cross_domain_bridges()

    lines = ['<div class="cross-domain">']
    lines.append("<h3>🔀 Cross-Domain Gap Bridge</h3>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
                "Capsules from different research domains connected by shared method keywords. "
                "These bridges suggest opportunities for cross-pollination.</p>")

    if not bridges:
        lines.append("<p style='color:#A89E8C;font-size:13px'>No strong cross-domain bridges yet. "
                    "Extract gaps from papers across multiple fields to discover connections.</p>")
    else:
        for b in bridges:
            lines.append(f"""
<div style='display:flex;gap:12px;margin-bottom:14px;padding:12px;background:#f8f4ef;border-radius:6px'>
  <div style='flex:1'>
    <div style='font-size:11px;color:#6B8FB5;font-weight:700;margin-bottom:2px'>{b['category_a'] or 'unknown'}</div>
    <div style='font-size:13px;font-weight:600;color:#2a2a2a'>{b['capsule_a'][:60]}</div>
  </div>
  <div style='display:flex;flex-direction:column;align-items:center;justify-content:center;width:80px'>
    <div style='font-size:18px'>⟷</div>
    <div style='font-size:10px;color:#A89E8C'>overlap={b['overlap_score']:.0%}</div>
  </div>
  <div style='flex:1'>
    <div style='font-size:11px;color:#C4706A;font-weight:700;margin-bottom:2px;text-align:right'>{b['category_b'] or 'unknown'}</div>
    <div style='font-size:13px;font-weight:600;color:#2a2a2a'>{b['capsule_b'][:60]}</div>
  </div>
</div>
<div style='margin-left:90px;margin-bottom:14px;font-size:11px;color:#A89E8C'>
  Bridge: {', '.join(f'<code>{k}</code>' for k in b['bridge_keywords'])}
</div>""")

    lines.append("<style>.cross-domain { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
