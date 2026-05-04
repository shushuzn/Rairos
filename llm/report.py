"""Research report — written analysis, not capsule dump."""

from __future__ import annotations
from datetime import datetime

from llm.insight.tracker import EvolutionTracker


def generate() -> str:
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()
    research = [c for c in caps if getattr(c, "source_arxiv_category", "") not in ("cs.GL", "")]
    events = [c for c in caps if getattr(c, "source_arxiv_category", "") == "cs.GL"]

    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []
    def w(s=""): lines.append(s)

    w("RAIROS RESEARCH REPORT")
    w(now)
    w("")
    w("This report summarizes the current state of the system's Gene Pool,")
    w("organized by research themes and live events.")
    w("")

    w("─" * 60)
    w("")

    # Research section
    w("RESEARCH")
    w("")
    w(f"The system is tracking {len(research)} research capsules across VLA")
    w("robotics, representation learning, and evaluation methodology.")
    w("")
    w("VLA & ROBOTICS")
    w(f"  {len([c for c in research if 'lapo' in c.action_gap_title.lower() or 'vla' in c.action_gap_title.lower() or 'libero' in c.action_gap_title.lower() or 'robot' in c.action_gap_title.lower() or 'diffusion' in c.action_gap_title.lower()])} capsules")
    w("  Covers LAPO vs PPO convergence, LIBERO benchmark analysis,")
    w("  diffusion policy representations, and generalist vs specialist")
    w("  policy trade-offs.")
    w("")

    w("REPRESENTATION & THEORY")
    w(f"  {len([c for c in research if 'latent' in c.action_gap_title.lower() or 'reasoning' in c.action_gap_title.lower() or 'attention' in c.action_gap_title.lower() or 'representation' in c.action_gap_title.lower()])} capsules")
    w("  Latent reasoning chain length saturation, visual vs physical")
    w("  attention, diffusion vs token-based representations.")
    w("")

    w("EVALUATION & BENCHMARKING")
    w(f"  {len([c for c in research if 'benchmark' in c.action_gap_title.lower() or 'evaluation' in c.action_gap_title.lower() or 'liberoo' in c.action_gap_title.lower()])} capsules")
    w("  LIBERO coverage gaps, missing zero-shot evaluation protocols,")
    w("  and the need for standardized VLA manipulation benchmarks.")
    w("")

    # Events section
    w("LIVE EVENTS")
    w("")
    w(f"The system is monitoring {len(events)} real-world events.")
    w("")

    geo = [c for c in events if 'iran' in c.action_gap_title.lower() or 'hormuz' in c.action_gap_title.lower() or 'oil' in c.action_gap_title.lower() or 'military' in c.action_gap_title.lower()]
    econ = [c for c in events if 'treasury' in c.action_gap_title.lower() or 'debt' in c.action_gap_title.lower()]
    safety = [c for c in events if 'fireworks' in c.action_gap_title.lower() or 'explosion' in c.action_gap_title.lower() or 'earthquake' in c.action_gap_title.lower()]

    if geo:
        w("GEOPOLITICAL")
        w(f"  {len(geo)} capsules")
        for c in sorted(geo, key=lambda x: x.outcome_success_score, reverse=True)[:3]:
            w(f"  \u2022 {c.action_gap_title[:60]}")
        w("")

    if econ:
        w("ECONOMIC")
        w(f"  {len(econ)} capsules")
        for c in econ:
            w(f"  \u2022 {c.action_gap_title[:60]}")
        w("")

    if safety:
        w("SAFETY")
        w(f"  {len(safety)} capsules")
        for c in safety:
            w(f"  \u2022 {c.action_gap_title[:60]}")
        w("")

    # Stats
    w("STATS")
    w("")
    w(f"  Total capsules: {len(caps)}")
    w(f"  Research: {len(research)}")
    w(f"  Events: {len(events)}")
    w(f"  Gap types: {len(set(c.action_gap_type for c in caps))}")
    w(f"  Avg score: {sum(c.outcome_success_score for c in caps)/len(caps):.2f}")
    w(f"  High credibility: {sum(1 for c in caps if c.credibility_badge == 'high')}")
    w("")

    w("─" * 60)
    w("End")

    return "\n".join(lines)


def save() -> str:
    r = generate()
    with open("SITUATION_REPORT.md", "w", encoding="utf-8") as f:
        f.write(r)
    return "SITUATION_REPORT.md"
