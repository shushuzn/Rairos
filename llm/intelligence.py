"""Rairos Intelligence — unified situation report from all data sources."""

from __future__ import annotations


from datetime import datetime
from typing import Any, Dict, List, Optional

from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker
from llm.scout import scout
from llm.watch import WatchDaemon


def intelligence(topic: str = "", verbose: bool = False) -> Dict[str, Any]:
    """Generate a unified situation report from all data sources.

    Args:
        topic: Optional focus topic. If empty, uses Gene Pool's top interests.
        verbose: Include detailed breakdowns.

    Returns:
        Dict with sections: geopolitics, markets, gene_pool, watch, papers.
    """
    report: Dict[str, Any] = {
        "generated_at": datetime.now().isoformat()[:16],
        "topic": topic or "global",
    }

    # 1. Geopolitical situation from Jin10 flash
    try:
        client = Jin10Client()
        client.ensure_init()

        flash_topics = [topic] if topic else ["伊朗", "霍尔木兹", "石油", "美联储"]
        flash_items = []
        for t in flash_topics[:4]:
            raw = client.search_flash(t)  # type: ignore[assignment]
            items: List[Any] = raw if isinstance(raw, list) else []
            for item in items[:3]:
                if isinstance(item, dict):
                    flash_items.append(
                        {
                            "time": str(item.get("time", ""))[:16],
                            "content": str(item.get("content", ""))[:120],
                            "topic": t,
                        }
                    )
        report["flash_news"] = flash_items[:10]
    except Exception as e:
        report["flash_news"] = []
        if verbose:
            report["flash_error"] = str(e)

    # 2. Key market quotes
    try:
        symbols = ["XAUUSD", "USOIL", "EURUSD", "USDCNH"] if not topic else ["XAUUSD", "USOIL"]
        quotes = []
        for sym in symbols:
            try:
                raw = client.get_quote(sym)  # type: ignore[assignment]
                q = raw.get("data", raw)  # type: ignore[attr-defined]
                quotes.append(
                    {
                        "code": q.get("code", sym),
                        "name": q.get("name", sym),
                        "price": q.get("close", "?"),
                        "change": q.get("ups_percent", "?"),
                    }
                )
            except Exception:
                pass
        report["markets"] = quotes
    except Exception as e:
        report["markets"] = []
        if verbose:
            report["markets_error"] = str(e)

    # 3. Gene Pool state
    try:
        tracker = EvolutionTracker()
        capsules = tracker._load_capsules()
        by_type: Dict[str, int] = {}
        total_score = 0.0
        high_cred = 0
        for c in capsules:
            by_type[c.action_gap_type] = by_type.get(c.action_gap_type, 0) + 1
            total_score += c.outcome_success_score
            if c.credibility_badge == "high":
                high_cred += 1

        report["gene_pool"] = {
            "total": len(capsules),
            "avg_score": round(total_score / len(capsules), 3) if capsules else 0,
            "by_type": by_type,
            "high_credibility": high_cred,
        }

        # Top N capsules by score
        sorted_caps = sorted(capsules, key=lambda c: c.outcome_success_score, reverse=True)
        report["top_capsules"] = [
            {
                "title": c.action_gap_title[:70],
                "score": c.outcome_success_score,
                "badge": c.credibility_badge,
                "type": c.action_gap_type,
            }
            for c in sorted_caps[:5]
        ]
    except Exception:
        report["gene_pool"] = {}

    # 4. Watch daemon status
    try:
        watch = WatchDaemon()
        ws = watch.get_status()
        report["watch"] = {
            "running": ws.get("running", False),
            "last_check": str(ws.get("last_check", ""))[:16],
            "events_monitored": ws.get("total_events", 0),
        }
    except Exception:
        report["watch"] = {}

    # 5. Related academic papers (scout)
    try:
        search_topic = topic or "geopolitical risk energy security"
        papers = scout(topic=search_topic, sources="arxiv", max_results=3, min_match_score=0.05)
        report["papers"] = [
            {"title": p.title[:70], "score": p.match_score, "capsule": p.matched_gap_title[:50]}
            for p in papers[:3]
        ]
    except Exception:
        report["papers"] = []

    return report


def render_report(report: Dict[str, Any]) -> str:
    """Render intelligence report as human-readable text."""
    from cli._shared import Colors as C

    # Map colors to available names
    _RED = C.FAIL if hasattr(C, "FAIL") else C.END
    _GREEN = C.GREEN if hasattr(C, "GREEN") else C.END
    _YELLOW = C.WARNING if hasattr(C, "WARNING") else C.END

    lines = [
        f"\n  {C.CYAN}═══ Rairos Intelligence ═══{C.END}",
        f"  {report.get('generated_at', '?')}  |  Topic: {report.get('topic', 'global')}",
        "",
    ]

    flash = report.get("flash_news", [])
    if flash:
        lines.append(f"  {_YELLOW}Geopolitical Flash{C.END}")
        for item in flash[:5]:
            lines.append(f"  [{item.get('time', '')}] {item.get('content', '')[:80]}")
        if len(flash) > 5:
            lines.append(f"  ... and {len(flash) - 5} more")
        lines.append("")

    markets = report.get("markets", [])
    if markets:
        lines.append(f"  {_YELLOW}Markets{C.END}")
        for q in markets:
            ch = q.get("change", "?")
            clr = (
                _GREEN
                if isinstance(ch, str) and ch.startswith("-")
                else _RED
                if isinstance(ch, str)
                else C.END
            )
            lines.append(
                f"  {q.get('code', '?'):<8} {str(q.get('price', '?')):>8}  {clr}{ch}{C.END}"
            )
        lines.append("")

    gp = report.get("gene_pool", {})
    if gp:
        lines.append(f"  {_YELLOW}Gene Pool{C.END}")
        lines.append(
            f"  {gp.get('total', 0)} capsules, avg score {gp.get('avg_score', 0)}, "
            f"{gp.get('high_credibility', 0)} high credibility"
        )
        lines.append(f"  Types: {gp.get('by_type', {})}")
        lines.append("")

    top = report.get("top_capsules", [])
    if top:
        lines.append(f"  {_YELLOW}Top Capsules{C.END}")
        for c in top:
            badge = f"[{c.get('badge', '?').upper()}]" if c.get("badge") else ""
            lines.append(f"  {badge} {c.get('title', '')[:60]} (score={c.get('score', 0)})")
        lines.append("")

    watch = report.get("watch", {})
    if watch:
        status = f"{_GREEN}RUNNING{C.END}" if watch.get("running") else f"{_RED}STOPPED{C.END}"
        lines.append(f"  {_YELLOW}Watch Daemon{C.END}")
        lines.append(
            f"  {status}  |  Last: {watch.get('last_check', 'never')}  |  "
            f"Events: {watch.get('events_monitored', 0)}"
        )
        lines.append("")

    papers = report.get("papers", [])
    if papers:
        lines.append(f"  {_YELLOW}Related Papers{C.END}")
        for p in papers:
            lines.append(f"  {p.get('title', '')[:60]} (match={p.get('score', 0):.2f})")
            lines.append(f"  → {p.get('capsule', '')[:50]}")
        lines.append("")

    lines.append(f"  {C.CYAN}═══════════════════════════════════{C.END}")
    return "\n".join(lines)
