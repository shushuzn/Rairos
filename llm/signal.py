"""Pattern-based signal system: match live events against historical Gene Pool patterns."""

from __future__ import annotations

from datetime import datetime
from typing import Any, Dict, List

from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker


def signal(event_keyword: str) -> Dict[str, Any]:
    """Analyze a live event against historical Gene Pool patterns.

    Returns a signal report: what happened before, what to watch, estimated impact.
    """
    client = Jin10Client()
    client.ensure_init()
    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()

    # 1. Get live event context
    raw: List[Any] = client.search_flash(event_keyword)  # type: ignore[assignment]
    live_items = list(raw)[:5]

    # 2. Match against historical Gene Pool
    matches: List[Dict[str, Any]] = []
    for c in capsules:
        score = c.trigger_match(event_keyword, c.trigger_gap_type, c.trigger_keywords)
        if score > 0.2:
            kw_overlap = sum(1 for kw in c.trigger_keywords if kw.lower() in event_keyword.lower())
            text_match = kw_overlap * 0.15
            total = score + text_match
            matches.append(
                {
                    "capsule_id": c.capsule_id,
                    "title": c.action_gap_title,
                    "type": c.action_gap_type,
                    "score": c.outcome_success_score,
                    "credibility": c.credibility_badge,
                    "match": round(total, 3),
                }
            )

    matches.sort(key=lambda x: x["match"], reverse=True)

    # 3. Get current market state
    quotes = {}
    for sym in ["USOIL", "XAUUSD", "EURUSD", "USDCNH"]:
        try:
            raw_q = client.get_quote(sym)
            q = raw_q.get("data", raw_q)
            quotes[sym] = {
                "price": q.get("close", "?"),
                "change": q.get("ups_percent", "?") if q.get("ups_percent") else "?",
            }
        except Exception:
            pass

    # 4. Generate signal assessment
    high_matches = [m for m in matches if m["match"] >= 0.5]
    signal_level = "HIGH" if len(high_matches) >= 2 else "MEDIUM" if high_matches else "LOW"

    # 5. Build impact estimate from capsule scores
    impact_sectors = []
    for m in matches[:3]:
        if "oil" in m["title"].lower() or "石油" in m["title"] or "Hormuz" in m["title"]:
            impact_sectors.append("energy")
        if "military" in m["title"].lower() or "ceasefire" in m["title"] or "导弹" in m["title"]:
            impact_sectors.append("defense")
        if "inflation" in m["title"].lower() or "rate" in m["title"] or "加息" in m["title"]:
            impact_sectors.append("finance")

    return {
        "event": event_keyword,
        "timestamp": datetime.now().isoformat()[:16],
        "signal": signal_level,
        "capsule_matches": matches[:5],
        "markets": quotes,
        "impact_sectors": list(set(impact_sectors)),
        "news_count": len(live_items),
        "recommendation": _recommendation(signal_level, matches[:3] if matches else []),
    }


def _recommendation(level: str, top_matches: List) -> str:
    if level == "HIGH":
        topics = [m["title"][:40] for m in top_matches[:2]]
        return f"Watchlist: {'; '.join(topics)}. Historical patterns suggest market impact."
    if level == "MEDIUM":
        return f"Related patterns found ({top_matches[0]['title'][:40]}). Monitor for escalation."
    return "No significant pattern match. Low priority event."


def render_signal(result: Dict[str, Any]) -> str:
    """Render signal report."""
    from cli._shared import Colors as C

    _RED = getattr(C, "FAIL", C.END)
    _GREEN = getattr(C, "GREEN", C.END)
    _YELLOW = getattr(C, "WARNING", C.END)

    sig = result.get("signal", "LOW")
    sig_color = _RED if sig == "HIGH" else _YELLOW if sig == "MEDIUM" else _GREEN

    lines = [
        f"\n  {C.CYAN}═══ Signal Analysis ═══{C.END}",
        f"  Event: {result.get('event', '?')}",
        f"  Signal: {sig_color}{sig}{C.END}  |  {result.get('timestamp', '')}",
        "",
    ]

    if result.get("capsule_matches"):
        lines.append(f"  {_YELLOW}Historical Pattern Matches{C.END}")
        for m in result["capsule_matches"][:3]:
            lines.append(f"  match={m['match']:.2f} [{m['credibility'].upper()}] {m['title'][:55]}")
        lines.append("")

    if result.get("markets"):
        lines.append(f"  {_YELLOW}Current Markets{C.END}")
        for k, v in result["markets"].items():
            lines.append(f"  {k:<8} {str(v.get('price', '')):>8}  {v.get('change', '?')}")
        lines.append("")

    if result.get("impact_sectors"):
        lines.append(f"  {_YELLOW}Impact Sectors{C.END}")
        lines.append(f"  {', '.join(result['impact_sectors'])}")
        lines.append("")

    lines.append(f"  {_YELLOW}Recommendation{C.END}")
    lines.append(f"  {result.get('recommendation', '')}")
    lines.append(f"  {C.CYAN}═══════════════════════════════════{C.END}")
    return "\n".join(lines)
