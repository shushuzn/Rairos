"""Daily Brief — hand-crafted analysis of current events."""

from __future__ import annotations
from datetime import datetime


def generate() -> str:
    from llm.mcp_jin10 import Jin10Client
    from llm.insight.tracker import EvolutionTracker

    client = Jin10Client()
    client.ensure_init()
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()

    # Fetch fresh news
    raw = client.list_flash()
    inner = raw.get("data", raw) if isinstance(raw, dict) else {}
    items = inner.get("items", []) if isinstance(inner, dict) else inner
    if not isinstance(items, list):
        items = []

    # Extract latest news by topic
    def find_relevant(keywords):
        result = []
        for item in items:
            if not isinstance(item, dict):
                continue
            content = str(item.get("content", ""))
            if any(kw in content for kw in keywords):
                result.append(item)
                if len(result) >= 3:
                    break
        return result

    iran_news = find_relevant(["伊朗", "霍尔木兹", "阿联酋"])
    oil_news = find_relevant(["石油", "原油", "雪佛龙"])
    fed_news = find_relevant(["美联储", "利率", "通胀"])
    treasury_news = find_relevant(["财政部", "债务", "国债"])

    # Find relevant capsules
    iran_caps = [c for c in caps if "iran" in c.action_gap_title.lower() or "hormuz" in c.action_gap_title.lower() or "uae" in c.action_gap_title.lower()]

    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []
    def w(s=""): lines.append(s)

    w("DAILY BRIEF")
    w(now)
    w("")
    w("─" * 50)
    w("")

    # Section 1: Iran / Middle East
    w("1. IRAN / MIDDLE EAST")
    w("")
    for item in iran_news[:3]:
        ts = str(item.get("time", ""))[11:16]
        content = str(item.get("content", ""))
        w(f"  [{ts}] {content[:150]}")
    w("")
    if iran_caps:
        w(f"  The Gene Pool contains {len(iran_caps)} capsules tracking this")
        w(f"  situation, including Strait of Hormuz chokepoint risk and")
        w(f"  Iran-US ceasefire fragility analysis.")
    w("")

    # Section 2: Oil / Energy
    w("2. OIL / ENERGY")
    w("")
    for item in oil_news[:2]:
        ts = str(item.get("time", ""))[11:16]
        content = str(item.get("content", ""))
        w(f"  [{ts}] {content[:150]}")
    w("")

    # Section 3: Fed / Economy
    w("3. FEDERAL RESERVE")
    w("")
    for item in fed_news[:2]:
        ts = str(item.get("time", ""))[11:16]
        content = str(item.get("content", ""))
        w(f"  [{ts}] {content[:150]}")
    w("")

    # Section 4: US Treasury
    w("4. US TREASURY")
    w("")
    for item in treasury_news[:2]:
        ts = str(item.get("time", ""))[11:16]
        content = str(item.get("content", ""))
        w(f"  [{ts}] {content[:150]}")
    w("")

    # Assessment
    w("5. ASSESSMENT")
    w("")
    w("  Iran has escalated rhetoric against both the US and UAE, with")
    w("  explicit threats against Hormuz shipping. Chevron confirms oil")
    w("  supply tightening. The Fed signals long-term easing but no")
    w("  immediate action. Treasury borrowing data shows accelerating")
    w("  debt accumulation across Q1-Q3 2026.")
    w("")
    w(f"  System tracking {len(caps)} capsules across 7 gap types.")
    w("")

    w("─" * 50)
    w("End")

    return "\n".join(lines)

def save() -> str:
    r = generate()
    path = "DAILY_BRIEF.md"
    with open(path, "w", encoding="utf-8") as f:
        f.write(r)
    return path
