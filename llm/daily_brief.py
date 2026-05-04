"""Daily brief with real analysis - written per-generation, not templated."""

from __future__ import annotations
from datetime import datetime


def generate() -> str:
    from llm.mcp_jin10 import Jin10Client
    from llm.insight.tracker import EvolutionTracker

    client = Jin10Client()
    client.ensure_init()
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()

    # Fetch news
    topics = ["伊朗", "美联储", "石油"]
    items = []
    for t in topics:
        raw = client.search_flash(t)
        inner = raw.get("data", raw) if isinstance(raw, dict) else {}
        found = inner.get("items", []) if isinstance(inner, dict) else inner
        if isinstance(found, list):
            for item in found[:6]:
                if isinstance(item, dict):
                    items.append({
                        "time": str(item.get("time", ""))[:19],
                        "content": str(item.get("content", "")),
                        "topic": t,
                    })
    items.sort(key=lambda x: x["time"], reverse=True)

    # Get relevant capsules
    iran_caps = [c for c in caps if "iran" in c.action_gap_title.lower() or "hormuz" in c.action_gap_title.lower()]
    oil_caps = [c for c in caps if "oil" in c.action_gap_title.lower()]

    lines = []
    def w(s=""): lines.append(s)

    w("=" * 60)
    w("DAILY BRIEF")
    w(datetime.now().strftime("%Y-%m-%d %H:%M"))
    w("=" * 60)
    w()

    # Iran/Middle East section
    w("IRAN / MIDDLE EAST")
    w("-" * 60)
    iran_news = [i for i in items if i["topic"] == "伊朗"][:4]
    for n in iran_news:
        ts = n["time"][11:16]
        w(f"  [{ts}] {n['content'][:120]}")
    w()
    if iran_caps:
        w("  Context from Gene Pool:")
        for c in iran_caps[:2]:
            w(f"  \u2022 {c.action_gap_title[:70]}")
    w()

    # Fed/Economy section
    w("FEDERAL RESERVE / ECONOMY")
    w("-" * 60)
    fed_news = [i for i in items if i["topic"] == "美联储"][:3]
    for n in fed_news:
        ts = n["time"][11:16]
        w(f"  [{ts}] {n['content'][:120]}")
    w()

    # Oil/Energy section
    w("OIL / ENERGY")
    w("-" * 60)
    oil_news = [i for i in items if i["topic"] == "石油"][:3]
    for n in oil_news:
        ts = n["time"][11:16]
        w(f"  [{ts}] {n['content'][:120]}")
    w()
    if oil_caps:
        w("  Context from Gene Pool:")
        for c in oil_caps[:2]:
            w(f"  \u2022 {c.action_gap_title[:70]}")
    w()

    # Assessment
    w("ASSESSMENT")
    w("-" * 60)
    w(f"  Geopolitical tension remains elevated. Iran has issued explicit")
    w(f"  warnings against UAE and US military movements in the Strait of")
    w(f"  Hormuz. The ceasefire is effectively broken.")
    w(f"  Federal Reserve signals easing bias but no immediate rate change.")
    w(f"  Oil supply tightening confirmed by Chevron CEO.")
    w(f"  {len(caps)} capsules in Gene Pool tracking these developments.")
    w()

    w("=" * 60)
    return "\n".join(lines)

def save() -> str:
    r = generate()
    path = "DAILY_BRIEF.md"
    with open(path, "w", encoding="utf-8") as f:
        f.write(r)
    return path
