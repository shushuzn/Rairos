"""Theme-based research reports — written analysis, not lists."""

from __future__ import annotations
from datetime import datetime
from typing import Dict, List, Any

from llm.insight.tracker import EvolutionTracker

# ── Helpers ──────────────────────────────────────────────────────────────

def _caps_by_theme(caps: list, keywords: list, exclude: set = None) -> list:
    exclude = exclude or set()
    result = []
    for c in caps:
        if c.capsule_id in exclude:
            continue
        text = (c.action_gap_title + " " + c.trigger_topic).lower()
        if any(kw in text for kw in keywords):
            result.append(c)
    return result

THEMES = [
    ("vla", "VLA / Robotics", ["lapo", "vla", "libero", "diffusion policy", "octo", "gr-2", "robot", "embodied"]),
    ("geopolitics", "Geopolitics / Energy", ["iran", "hormuz", "oil", "uae", "military", "ceasefire", "drone", "missile", "能源", "石油"]),
    ("economy", "Economy / Markets", ["treasury", "debt", "borrowing", "fed", "inflation", "财政部", "国债"]),
    ("theory", "Theory / Representation", ["latent", "reasoning", "attention", "representation", "interpretability", "contradiction", "visual"]),
    ("safety", "Safety / Incidents", ["fireworks", "explosion", "earthquake", "safety", "injured", "爆炸", "地震", "事故"]),
]


def load_data() -> tuple:
    """Load capsules and organize by theme. Returns (caps, themedict)."""
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()
    used = set()
    theme_map = {}
    for slug, name, kws in THEMES:
        matched = _caps_by_theme(caps, kws, used)
        theme_map[slug] = {"name": name, "capsules": matched}
        for c in matched:
            used.add(c.capsule_id)
    theme_map["_uncategorized"] = [c for c in caps if c.capsule_id not in used]
    return caps, theme_map


def report_index() -> str:
    """Generate the index page with summary of all themes."""
    caps, theme_map = load_data()
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []
    def w(s=""): lines.append(s)

    w("RESEARCH REPORTS")
    w(now)
    w(f"Total: {len(caps)} capsules")
    w("")

    for slug, name, _ in THEMES:
        info = theme_map[slug]
        clist = info["capsules"]
        types = {}
        for c in clist:
            types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1
        avg = sum(c.outcome_success_score for c in clist) / len(clist) if clist else 0
        w(f"  {name}")
        w(f"  {len(clist)} capsules | {', '.join(f'{t}={n}' for t,n in types.items())} | avg {avg:.2f}")
        w(f"  /report/{slug}")
        w("")

    uncat = theme_map.get("_uncategorized", [])
    if uncat:
        w(f"  Other ({len(uncat)} unthemed)")

    w("")
    w("─" * 50)
    return "\n".join(lines)


def report_vla() -> str:
    """VLA / Robotics analysis."""
    _, theme_map = load_data()
    caps = theme_map["vla"]["capsules"]
    types = {}
    for c in caps:
        types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1

    lines = []
    def w(s=""): lines.append(s)
    w("VLA / ROBOTICS")
    w(f"{len(caps)} capsules | {types}")
    w("")
    w("  This theme covers vision-language-action models for robotic manipulation.")
    w("  The central question is how to combine reasoning, representation learning,")
    w("  and reinforcement learning into a unified policy.")
    w("")
    w("  Key findings:")
    w("")
    w("  \u2022 LAPO vs PPO convergence: no systematic comparison exists with matched")
    w("    compute budgets. LAPO jointly optimizes latent reasoning and action,")
    w("    but whether this outperforms action-only PPO is unverified.")
    w("")
    w("  \u2022 LIBERO benchmark covers 110 single-arm tasks but excludes deformable")
    w("    objects, dual-arm coordination, and failure recovery. Real-world")
    w("    deployment gaps are significant.")
    w("")
    w("  \u2022 Diffusion vs token-based action representations: the trade-off between")
    w("    continuous (diffusion) and discrete (token) action spaces is unresolved,")
    w("    especially for high-frequency control tasks.")
    w("")
    w("  \u2022 Warm-up strategy: no controlled ablation exists comparing one-shot")
    w("    warm-up (LaST-R1 approach) vs diverse multi-demonstration warm-up.")
    w("")
    w("  \u2022 Generalist vs specialist policies: generalist VLAs underperform")
    w("    specialists on specific embodiments, but provide better RL fine-tuning")
    w("    starting points.")
    w("")
    return "\n".join(lines)


def report_geopolitics() -> str:
    """Geopolitics / Energy analysis."""
    _, theme_map = load_data()
    caps = theme_map["geopolitics"]["capsules"]
    types = {}
    for c in caps:
        types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1

    lines = []
    def w(s=""): lines.append(s)
    w("GEOPOLITICS / ENERGY")
    w(f"{len(caps)} capsules | {types}")
    w("")
    w("  The system tracks 10 capsules related to Middle East geopolitics and")
    w("  energy security, centered on the Strait of Hormuz chokepoint.")
    w("")
    w("  Key findings:")
    w("")
    w("  \u2022 Strait of Hormuz handles ~20% of global oil/LNG transit. Any disruption")
    w("    directly impacts global energy markets.")
    w("")
    w("  \u2022 Iran-US military escalation directly affects oil prices. Chevron's CEO")
    w("    confirmed supply tightening and inventory drawdowns.")
    w("")
    w("  \u2022 UAE infrastructure is vulnerable to asymmetric threats (drones, missiles).")
    w("    Latest reports show 19 projectiles intercepted in a single wave.")
    w("")
    w("  \u2022 The Iran-US ceasefire is fragile. One month passed between the ceasefire")
    w("    and the first missile alert, suggesting diplomatic efforts are failing.")
    w("")
    w("  \u2022 Energy supply chain security models need to incorporate real-time")
    w("    geopolitical event monitoring to be effective.")
    w("")
    return "\n".join(lines)


def report_economy() -> str:
    """Economy / Markets analysis."""
    _, theme_map = load_data()
    caps = theme_map["economy"]["capsules"]
    types = {}
    for c in caps:
        types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1

    lines = []
    def w(s=""): lines.append(s)
    w("ECONOMY / MARKETS")
    w(f"{len(caps)} capsules | {types}")
    w("")
    w("  One capsule tracking US Treasury borrowing data.")
    w("")
    w("  Key findings:")
    w("")
    w("  \u2022 US Treasury borrowed $577B in Q1, projects $189B in Q2 and $671B in Q3.")
    w("    The Q3 figure is 3.5x Q2, indicating a significant fiscal ramp-up.")
    w("")
    w("  \u2022 Cash balance targets: $893B (Q1 end), $900B (Q2 end), $950B (Sep end).")
    w("")
    w("  More economic data capsules needed for deeper analysis.")
    w("")
    return "\n".join(lines)


def report_theory() -> str:
    """Theory / Representation analysis."""
    _, theme_map = load_data()
    caps = theme_map["theory"]["capsules"]
    types = {}
    for c in caps:
        types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1

    lines = []
    def w(s=""): lines.append(s)
    w("THEORY / REPRESENTATION")
    w(f"{len(caps)} capsules | {types}")
    w("")
    w("  This theme covers theoretical aspects of latent reasoning, attention")
    w("  mechanisms, and representation learning in VLA models.")
    w("")
    w("  Key findings:")
    w("")
    w("  \u2022 Latent reasoning chain length has diminishing returns. LaST-R1's adaptive")
    w("    CoT mechanism proves that longer chains are not always better.")
    w("")
    w("  \u2022 Optimal reasoning frequency should be learned per-task, not fixed.")
    w("    Adaptive approaches outperform fixed schedules.")
    w("")
    w("  \u2022 No latent-space probing methods exist for VLA models. Visual vs physical")
    w("    attention patterns cannot currently be analyzed.")
    w("")
    w("  \u2022 Diffusion vs token-based representations: a fundamental trade-off between")
    w("    continuous expressiveness and discrete interpretability.")
    w("")
    return "\n".join(lines)


def report_safety() -> str:
    """Safety / Incidents analysis."""
    _, theme_map = load_data()
    caps = theme_map["safety"]["capsules"]
    types = {}
    for c in caps:
        types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1

    lines = []
    def w(s=""): lines.append(s)
    w("SAFETY / INCIDENTS")
    w(f"{len(caps)} capsules | {types}")
    w("")
    w("  Three incident capsules: Mexico earthquake, Liuyang fireworks factory")
    w("  explosion, and UAE Fujairah drone attack.")
    w("")
    w("  \u2022 Liuyang fireworks factory explosion in Hunan, China drew high-level")
    w("    political response from Xi Jinping and Li Qiang, indicating systemic")
    w("    workplace safety concerns.")
    w("")
    w("  \u2022 Mexico 5.5 earthquake in Oaxaca state was a moderate event with no")
    w("    immediate economic impact signals.")
    w("")
    w("  \u2022 UAE Fujairah drone attack caused 3 injuries at oil facilities.")
    w("")
    return "\n".join(lines)


REPORT_FUNCS = {
    "vla": report_vla,
    "geopolitics": report_geopolitics,
    "economy": report_economy,
    "theory": report_theory,
    "safety": report_safety,
}


def get_report(slug: str) -> str:
    """Get a specific theme report by slug."""
    func = REPORT_FUNCS.get(slug)
    if func:
        return func()
    return None
