"""Report organized by research themes — research and news separated."""

from __future__ import annotations
from datetime import datetime

from llm.insight.tracker import EvolutionTracker


def _find(caps, keywords, exclude=None):
    """Filter capsules by keyword match, excluding already-listed IDs."""
    exclude = exclude or set()
    result = []
    for c in caps:
        if c.capsule_id in exclude:
            continue
        text = (c.action_gap_title + " " + c.trigger_topic).lower()
        if any(kw in text for kw in keywords):
            result.append(c)
    return result


def generate() -> str:
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()

    # Separate research (cs.RO, cs.LG, etc.) from event (cs.GL) capsules
    research = [c for c in caps if getattr(c, "source_arxiv_category", "") not in ("cs.GL", "")]
    events = [c for c in caps if getattr(c, "source_arxiv_category", "") == "cs.GL"]

    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []

    def w(s=""):
        lines.append(s)

    w("=" * 60)
    w("RAIROS RESEARCH REPORT")
    w(now)
    w("=" * 60)
    w()

    used = set()

    # Theme 1: VLA / Robotics
    vla = _find(
        research,
        ["lapo", "vla", "liberoo", "diffusion policy", "octo", "gr-2", "robot", "embodied"],
    )
    if vla:
        w("VLA / ROBOTICS")
        w("-" * 60)
        for c in sorted(vla, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
            used.add(c.capsule_id)
        w()

    # Theme 2: Learning / Optimization
    learn = _find(
        research,
        ["rl", "reinforcement", "fine-tuning", "warmup", "warm-up", "sample", "convergence"],
        used,
    )
    if learn:
        w("LEARNING / OPTIMIZATION")
        w("-" * 60)
        for c in sorted(learn, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
            used.add(c.capsule_id)
        w()

    # Theme 3: Representation / Theory
    theory = _find(
        research,
        ["latent", "reasoning", "attention", "representation", "interpretability", "contradiction"],
        used,
    )
    if theory:
        w("REPRESENTATION / THEORY")
        w("-" * 60)
        for c in sorted(theory, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
            used.add(c.capsule_id)
        w()

    # Theme 4: Benchmark / Evaluation
    bench = _find(
        research,
        ["benchmark", "libero", "evaluation", "gap", "scalability", "generalization"],
        used,
    )
    if bench:
        w("BENCHMARK / EVALUATION")
        w("-" * 60)
        for c in sorted(bench, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
            used.add(c.capsule_id)
        w()

    # Remaining research
    remaining = [c for c in research if c.capsule_id not in used]
    if remaining:
        w("OTHER RESEARCH")
        w("-" * 60)
        for c in sorted(remaining, key=lambda x: x.outcome_success_score, reverse=True):
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
            w(f"  score={c.outcome_success_score:.2f} | {c.action_gap_type}")
        w()

    # Events section (deduplicated, condensed)
    if events:
        seen = set()
        w("LIVE EVENTS")
        w("-" * 60)
        for c in sorted(events, key=lambda x: x.outcome_success_score, reverse=True):
            key = c.action_gap_title[:50]
            if key in seen:
                continue
            seen.add(key)
            w(f"  [{c.credibility_badge.upper()}] {c.action_gap_title[:65]}")
        w()

    # Stats
    w("STATS")
    w("-" * 60)
    by_type = {}
    for c in caps:
        by_type[c.action_gap_type] = by_type.get(c.action_gap_type, 0) + 1
    w(f"  Total: {len(caps)} capsules ({len(research)} research, {len(events)} events)")
    w(f"  Gap types: {by_type}")
    w(f"  Avg score: {sum(c.outcome_success_score for c in caps) / len(caps):.2f}")
    w(f"  High credibility: {sum(1 for c in caps if c.credibility_badge == 'high')}")
    w()

    w("=" * 60)
    return "\n".join(lines)


def save() -> str:
    r = generate()
    with open("SITUATION_REPORT.md", "w", encoding="utf-8") as f:
        f.write(r)
    return "SITUATION_REPORT.md"
