"""Report organized by research themes/topics."""

from __future__ import annotations
from datetime import datetime

from llm.insight.tracker import EvolutionTracker


def _theme(capsules, keywords) -> list:
    """Filter capsules matching any of the given keywords."""
    result = []
    for c in capsules:
        text = (c.action_gap_title + " " + c.trigger_topic).lower()
        if any(kw in text for kw in keywords):
            result.append(c)
    return result


def generate() -> str:
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()

    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []
    def w(s=""): lines.append(s)

    w("=" * 60)
    w("RAIROS REPORT")
    w(now)
    w("=" * 60)
    w(f"Total: {len(caps)} capsules")
    w()

    # Theme 1: VLA / Robotics
    vla = _theme(caps, ["lapo", "vla", "liberoo", "diffusion policy", "octo", "gr-2",
                        "robot", "manipulation", "embodied"])
    if vla:
        w("1. VLA / ROBOTICS")
        w("-" * 60)
        for c in sorted(vla, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Theme 2: Geopolitics / Energy
    geo = _theme(caps, ["iran", "hormuz", "oil", "uae", "military", "ceasefire",
                        "能源", "drone", "missile"])
    if geo:
        w("2. GEOPOLITICS / ENERGY")
        w("-" * 60)
        for c in sorted(geo, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Theme 3: Economy / Markets (not in VLA or Geo)
    taken = set(c.capsule_id for c in vla + geo)
    econ_raw = _theme(caps, ["treasury", "debt", "borrowing", "fed", "inflation"])
    econ = [c for c in econ_raw if c.capsule_id not in taken]
    if econ:
        w("3. ECONOMY / MARKETS")
        w("-" * 60)
        for c in sorted(econ, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Theme 4: Theory / Methods (only if not already in VLA)
    all_vla = set(c.capsule_id for c in vla)
    theory_raw = _theme(caps, ["latent", "reasoning", "attention", "representation",
                                "interpretability", "method", "theoretical"])
    theory = [c for c in theory_raw if c.capsule_id not in all_vla]
    if theory:
        w("4. THEORY / METHODS")
        w("-" * 60)
        for c in sorted(theory, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Theme 5: Safety / Incidents (not in any previous theme)
    taken = set(c.capsule_id for c in vla + geo + econ + theory)
    safety_raw = _theme(caps, ["fireworks", "explosion", "earthquake", "safety", "injured"])
    safety = [c for c in safety_raw if c.capsule_id not in taken]
    if safety:
        w("5. SAFETY / INCIDENTS")
        w("-" * 60)
        for c in sorted(safety, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Theme 6: Other
    all_tagged = set()
    for group in [vla, geo, econ, theory, safety]:
        for c in group:
            all_tagged.add(c.capsule_id)
    other = [c for c in caps if c.capsule_id not in all_tagged]
    if other:
        w("6. OTHER")
        w("-" * 60)
        for c in sorted(other, key=lambda x: x.outcome_success_score, reverse=True)[:5]:
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:70]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        if len(other) > 5:
            w(f"  ... and {len(other)-5} more")
        w()

    # Stats
    w("STATS")
    w("-" * 60)
    by_type = {}
    for c in caps:
        by_type[c.action_gap_type] = by_type.get(c.action_gap_type, 0) + 1
    w(f"  Gap types: {by_type}")
    w(f"  Avg score: {sum(c.outcome_success_score for c in caps)/len(caps):.2f}")
    w(f"  High credibility: {sum(1 for c in caps if c.credibility_badge == 'high')}")
    w()

    w("=" * 60)
    return "\n".join(lines)


def save() -> str:
    r = generate()
    path = "SITUATION_REPORT.md"
    with open(path, "w", encoding="utf-8") as f:
        f.write(r)
    return path
