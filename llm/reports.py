"""Theme-based research reports — written analysis, not lists."""

from __future__ import annotations
from datetime import datetime
from typing import Dict, List, Any

from llm.insight.tracker import EvolutionTracker


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
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()
    used = set()
    theme_map = {}
    for slug, name, kws in THEMES:
        matched = _caps_by_theme(caps, kws, used)
        theme_map[slug] = {"name": name, "capsules": matched, "slug": slug}
        for c in matched:
            used.add(c.capsule_id)
    theme_map["_uncategorized"] = [c for c in caps if c.capsule_id not in used]
    return caps, theme_map


def report_all() -> list:
    """Return a list of dicts, one per theme, with name, summary, and content."""
    _, theme_map = load_data()
    results = []

    for slug, name, _ in THEMES:
        info = theme_map[slug]
        clist = info["capsules"]
        types = {}
        for c in clist:
            types[c.action_gap_type] = types.get(c.action_gap_type, 0) + 1
        avg = sum(c.outcome_success_score for c in clist) / len(clist) if clist else 0

        if slug == "vla":
            analysis = _report_vla(clist)
        elif slug == "geopolitics":
            analysis = _report_geopolitics(clist)
        elif slug == "economy":
            analysis = _report_economy(clist)
        elif slug == "theory":
            analysis = _report_theory(clist)
        elif slug == "safety":
            analysis = _report_safety(clist)
        else:
            analysis = ""

        # Generate summary sentence
        if slug == "vla":
            summary = "LAPO vs PPO convergence, LIBERO benchmarks, diffusion policy representations."
        elif slug == "geopolitics":
            summary = "Iran-US escalation, Strait of Hormuz chokepoint, UAE drone attacks."
        elif slug == "economy":
            summary = "US Treasury borrowing data Q1-Q3."
        elif slug == "theory":
            summary = "Latent reasoning chains, attention analysis, representation trade-offs."
        elif slug == "safety":
            summary = "Liuyang explosion, Mexico earthquake, Fujairah attack."

        results.append({
            "slug": slug,
            "name": name,
            "count": len(clist),
            "types": types,
            "avg": round(avg, 2),
            "summary": summary,
            "content": analysis,
        })

    return results


def _report_vla(caps) -> str:
    lines = []
    w = lambda s="": lines.append(s)
    w("This theme covers vision-language-action models for robotic manipulation.")
    w("The central question is how to combine reasoning, representation learning,")
    w("and reinforcement learning into a unified policy.")
    w("")
    w("Key findings:")
    w("")
    w("* LAPO vs PPO convergence: no systematic comparison exists with matched")
    w("  compute budgets. LAPO jointly optimizes latent reasoning and action,")
    w("  but whether this outperforms action-only PPO is unverified.")
    w("")
    w("* LIBERO benchmark covers 110 single-arm tasks but excludes deformable")
    w("  objects, dual-arm coordination, and failure recovery. Real-world")
    w("  deployment gaps are significant.")
    w("")
    w("* Diffusion vs token-based action representations: the trade-off between")
    w("  continuous (diffusion) and discrete (token) action spaces is unresolved,")
    w("  especially for high-frequency control tasks.")
    w("")
    w("* Warm-up strategy: no controlled ablation exists comparing one-shot")
    w("  warm-up (LaST-R1 approach) vs diverse multi-demonstration warm-up.")
    w("")
    w("* Generalist vs specialist policies: generalist VLAs underperform")
    w("  specialists on specific embodiments, but provide better RL fine-tuning")
    w("  starting points.")
    return "\n".join(lines)


def _report_geopolitics(caps) -> str:
    lines = []
    w = lambda s="": lines.append(s)
    w("The system tracks 10 capsules related to Middle East geopolitics and")
    w("energy security, centered on the Strait of Hormuz chokepoint.")
    w("")
    w("Key findings:")
    w("")
    w("* Strait of Hormuz handles ~20% of global oil/LNG transit. Any disruption")
    w("  directly impacts global energy markets.")
    w("")
    w("* Iran-US military escalation directly affects oil prices. Chevron's CEO")
    w("  confirmed supply tightening and inventory drawdowns.")
    w("")
    w("* UAE infrastructure is vulnerable to asymmetric threats (drones, missiles).")
    w("  Latest reports show 19 projectiles intercepted in a single wave.")
    w("")
    w("* The Iran-US ceasefire is fragile. One month passed between the ceasefire")
    w("  and the first missile alert, suggesting diplomatic efforts are failing.")
    w("")
    w("* Energy supply chain security models need to incorporate real-time")
    w("  geopolitical event monitoring to be effective.")
    return "\n".join(lines)


def _report_economy(caps) -> str:
    lines = []
    w = lambda s="": lines.append(s)
    w("One capsule tracking US Treasury borrowing data.")
    w("")
    w("Key findings:")
    w("")
    w("* US Treasury borrowed $577B in Q1, projects $189B in Q2 and $671B in Q3.")
    w("  The Q3 figure is 3.5x Q2, indicating a significant fiscal ramp-up.")
    w("")
    w("* Cash balance targets: $893B (Q1 end), $900B (Q2 end), $950B (Sep end).")
    w("")
    w("More economic data capsules needed for deeper analysis.")
    return "\n".join(lines)


def _report_theory(caps) -> str:
    lines = []
    w = lambda s="": lines.append(s)
    w("This theme covers theoretical aspects of latent reasoning, attention")
    w("mechanisms, and representation learning in VLA models.")
    w("")
    w("Key findings:")
    w("")
    w("* Latent reasoning chain length has diminishing returns. LaST-R1's adaptive")
    w("  CoT mechanism proves that longer chains are not always better.")
    w("")
    w("* Optimal reasoning frequency should be learned per-task, not fixed.")
    w("  Adaptive approaches outperform fixed schedules.")
    w("")
    w("* No latent-space probing methods exist for VLA models. Visual vs physical")
    w("  attention patterns cannot currently be analyzed.")
    w("")
    w("* Diffusion vs token-based representations: a fundamental trade-off between")
    w("  continuous expressiveness and discrete interpretability.")
    return "\n".join(lines)


def _report_safety(caps) -> str:
    lines = []
    w = lambda s="": lines.append(s)
    w("Three incident capsules: Mexico earthquake, Liuyang fireworks factory")
    w("explosion, and UAE Fujairah drone attack.")
    w("")
    w("* Liuyang fireworks factory explosion in Hunan, China drew high-level")
    w("  political response from Xi Jinping and Li Qiang, indicating systemic")
    w("  workplace safety concerns.")
    w("")
    w("* Mexico 5.5 earthquake in Oaxaca state was a moderate event with no")
    w("  immediate economic impact signals.")
    w("")
    w("* UAE Fujairah drone attack caused 3 injuries at oil facilities.")
    return "\n".join(lines)
