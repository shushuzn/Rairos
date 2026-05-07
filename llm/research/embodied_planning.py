"""Embodied planning analysis — specialized analysis + dashboard rendering."""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from llm.research.gap_configs import GAP_ANALYZER_CONFIGS
from llm.research.gene_pool_writer import save_gap_to_gene_pool
from llm.research.contradiction_detector import detect_field_contradiction


def track_embodied_evolution(
    paper_id: str,
    title: str,
    representation_type: str,
    confidence: float,
    gap_title: str,
) -> None:
    """Track embodied planning evolution over time."""
    try:
        save_gap_to_gene_pool(
            paper_id=paper_id,
            title=title,
            gap_type="embodied_planning",
            gap_title=gap_title,
            keywords=["embodied", "planning", representation_type],
            summary=f"Embodied planning analysis: {representation_type} representation",
            polarity="open",
            extra_fields={
                "representation_type": representation_type,
                "confidence": confidence,
            },
        )
    except Exception:
        pass


def render_embodied_planning_dashboard(type_counts: Dict[str, int], papers: List[Dict]) -> str:
    """Render embodied planning dashboard with type distribution."""
    total = sum(type_counts.values())
    if total == 0:
        return _empty_dashboard("No papers analyzed yet")

    bars = []
    for rep_type in ["discrete", "continuous", "hybrid"]:
        count = type_counts.get(rep_type, 0)
        pct = count / total * 100
        bar_len = int(pct / 5)
        bars.append(f"  {rep_type:<12} {count:>4} ({pct:5.1f}%) {'█' * bar_len}")

    papers_section = ""
    if papers:
        rows = []
        for p in papers[:10]:
            rt = p.get("representation_type", "?")
            rt_icon = {"discrete": "◼", "continuous": "◯", "hybrid": "◈"}.get(rt, "?")
            rows.append(
                f"  {rt_icon} {p.get('title', '?')[:60]:<60} conf={p.get('confidence', 0):.2f}"
            )
        papers_section = "\n".join(rows)

    return f"""
╔══════════════════════════════════════════════════════════════╗
║        EMBODIED PLANNING — LATENT REPRESENTATION SURVEY    ║
╠══════════════════════════════════════════════════════════════╣
║  Type Distribution (N={total})                                ║
║{"║".join(bars)}║
╠══════════════════════════════════════════════════════════════╣
║  Top Papers                                                   ║
{papers_section if papers_section else "║  (run analysis to see papers)"}
╚══════════════════════════════════════════════════════════════╝"""


def _empty_dashboard(msg: str) -> str:
    return f"[ Embodied Planning ] {msg}"


def render_embodied_planning_graph(type_counts: Dict[str, int]) -> str:
    """Render ASCII graph of representation type distribution."""
    total = sum(type_counts.values())
    if total == 0:
        return "No data"

    _max_count = max(type_counts.values()) if type_counts else 1
    lines = ["# Embodied Planning — Representation Types", ""]
    for rep_type in ["discrete", "continuous", "hybrid"]:
        count = type_counts.get(rep_type, 0)
        pct = count / total * 100
        bar = "▓" * int(pct / 5)
        lines.append(f"  {rep_type:<12}: {bar} {count:>4} ({pct:5.1f}%)")
    return "\n".join(lines)


def render_compare_view(paper_ids: List[str], db=None) -> str:
    """Render comparison view for multiple papers."""
    if not paper_ids:
        return _empty_dashboard("No papers selected")

    if db is None:
        from db.database import Database

        db = Database()

    lines = ["# Compare View", ""]
    for pid in paper_ids:
        paper = db.get_paper(pid)
        if not paper:
            continue
        lines.append(f"## {paper.title[:80]}")
        lines.append(f"Authors: {paper.authors or 'Unknown'}")
        abstract = (paper.abstract or "")[:300]
        lines.append(f"Abstract: {abstract}...")
        lines.append("")

    return "\n".join(lines)


def render_evolution_timeline() -> str:
    """Render timeline of embodied planning evolution."""
    try:
        from llm.gene_pool_io import load_capsules

        capsules = load_capsules(gap_type="embodied_planning", status="active")
    except Exception:
        return _empty_dashboard("Could not load Gene Pool")

    if not capsules:
        return _empty_dashboard("No evolution data yet")

    lines = ["# Embodied Planning Evolution Timeline", ""]
    for c in capsules[:20]:
        arch = c.get("archetype", {})
        rep = arch.get("representation_type", "?")
        conf = arch.get("confidence", 0)
        title = c.get("trigger_topic", "?")[:50]
        lines.append(f"  [{rep}] {conf:.2f} — {title}")

    return "\n".join(lines)


def render_confidence_calibration() -> str:
    """Render confidence calibration across all gap analyses."""
    try:
        from llm.gene_pool_io import load_capsules

        capsules = load_capsules(status="active")
    except Exception:
        return _empty_dashboard("Could not load Gene Pool")

    if not capsules:
        return _empty_dashboard("No calibration data")

    bins = {"0.0-0.2": 0, "0.2-0.4": 0, "0.4-0.6": 0, "0.6-0.8": 0, "0.8-1.0": 0}
    for c in capsules:
        conf = c.get("outcome_success_score", 0.5)
        if conf < 0.2:
            bins["0.0-0.2"] += 1
        elif conf < 0.4:
            bins["0.2-0.4"] += 1
        elif conf < 0.6:
            bins["0.4-0.6"] += 1
        elif conf < 0.8:
            bins["0.6-0.8"] += 1
        else:
            bins["0.8-1.0"] += 1

    lines = ["# Confidence Calibration", ""]
    for bin_label, count in bins.items():
        bar = "█" * count
        lines.append(f"  {bin_label}: {bar} ({count})")

    return "\n".join(lines)
