"""Autonomous pattern discovery — cross-reference events, market data, and Gene Pool capsules to find hidden correlations."""

from __future__ import annotations

import json
import time
from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker
from llm.insight.gene import CapsuleGene

PATTERNS_FILE = Path.home() / ".ai_research_os" / "patterns.json"


def _load_patterns() -> Dict:
    if PATTERNS_FILE.exists():
        try:
            return json.loads(PATTERNS_FILE.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
        except Exception:
            pass
    return {"correlations": [], "discovered_at": []}


def _save_patterns(data: Dict) -> None:
    PATTERNS_FILE.parent.mkdir(parents=True, exist_ok=True)
    PATTERNS_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def discover(force: bool = False) -> Dict[str, Any]:
    """Run pattern discovery: find correlations between events and markets.

    Looks at:
    - Gene Pool capsules with source_arxiv_category='cs.GL' (geopolitical events)
    - Current market quotes
    - Timestamps: when events were created vs when markets moved

    Returns discovered patterns.
    """
    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()
    client = Jin10Client()
    client.ensure_init()

    # 1. Classify capsules into event types
    event_caps: List[Dict] = []
    research_caps: List[Dict] = []
    for c in capsules:
        entry = {
            "id": c.capsule_id,
            "title": c.action_gap_title,
            "type": c.action_gap_type,
            "score": c.outcome_success_score,
            "created": c.created_at,
            "keywords": c.trigger_keywords,
        }
        if getattr(c, "source_arxiv_category", "") == "cs.GL" or any(
            kw in str(c.action_gap_title).lower()
            for kw in ["oil", "military", "ceasefire", "hormuz", "drone", "地震", "导弹"]
        ):
            event_caps.append(entry)
        else:
            research_caps.append(entry)

    # 2. Get current market state for correlation
    markets = {}
    for sym in ["USOIL", "XAUUSD", "EURUSD", "USDCNH", "UKOIL", "COPPER"]:
        try:
            raw = client.get_quote(sym)
            q = raw.get("data", raw)
            markets[sym] = {
                "price": q.get("close", "0"),
                "change_pct": q.get("ups_percent", "0"),
                "change_val": q.get("ups_price", "0"),
            }
        except Exception:
            pass

    # 3. Discover patterns: event type → market impact
    correlations = _load_patterns()
    new_patterns = []

    # Pattern 1: Hormuz-related events → oil price impact
    hormuz_caps = [
        c for c in event_caps if "hormuz" in str(c["title"]).lower() or "石油" in str(c["title"])
    ]
    if hormuz_caps and "USOIL" in markets:
        oil_change = float(markets["USOIL"].get("change_pct", "0"))
        if abs(oil_change) > 2:
            pattern = {
                "type": "hormuz_oil_correlation",
                "event_count": len(hormuz_caps),
                "avg_event_score": sum(c["score"] for c in hormuz_caps) / len(hormuz_caps),
                "current_oil_change_pct": oil_change,
                "signal": "oil_volatility" if abs(oil_change) > 3 else "oil_watch",
                "last_event": max(c["created"] for c in hormuz_caps),
                "discovered_at": datetime.now().isoformat(),
            }
            new_patterns.append(pattern)

    # Pattern 2: Military escalation → safe haven (gold)
    military_caps = [
        c
        for c in event_caps
        if "military" in str(c["title"]).lower()
        or "导弹" in str(c["title"])
        or "ceasefire" in str(c["title"]).lower()
    ]
    if military_caps and "XAUUSD" in markets:
        gold_change = float(markets["XAUUSD"].get("change_pct", "0"))
        if abs(gold_change) > 1:
            direction = "up" if gold_change > 0 else "down"
            pattern = {
                "type": "military_gold_safe_haven",
                "event_count": len(military_caps),
                "current_gold_change_pct": gold_change,
                "direction": direction,
                "note": f"Gold moving {direction} {abs(gold_change):.1f}% during military escalation events",
                "discovered_at": datetime.now().isoformat(),
            }
            new_patterns.append(pattern)

    # Pattern 3: Overall Gene Pool health → research confidence
    total_caps = len(capsules)
    avg_score = sum(c.outcome_success_score for c in capsules) / total_caps if total_caps else 0
    event_ratio = len(event_caps) / total_caps if total_caps else 0
    pattern = {
        "type": "gene_pool_composition",
        "total_capsules": total_caps,
        "event_vs_research_ratio": round(event_ratio, 3),
        "avg_score": round(avg_score, 3),
        "note": f"Gene Pool: {total_caps} capsules, {len(event_caps)} events, {len(research_caps)} research, avg {avg_score:.2f}",
        "discovered_at": datetime.now().isoformat(),
    }
    new_patterns.append(pattern)

    # Store discovered patterns
    if new_patterns:
        all_p = correlations.get("correlations", [])
        for np_ in new_patterns:
            # Deduplicate by type
            existing = [p for p in all_p if p["type"] == np_["type"]]
            if existing:
                existing[0].update(np_)
            else:
                all_p.append(np_)
        correlations["correlations"] = all_p[-50:]  # keep last 50
        correlations["last_discovery"] = datetime.now().isoformat()
        _save_patterns(correlations)

    return {
        "patterns_discovered": len(new_patterns),
        "total_patterns": len(correlations.get("correlations", [])),
        "event_capsules": len(event_caps),
        "research_capsules": len(research_caps),
        "new_patterns": new_patterns,
        "markets": markets,
    }


def render_discovery(result: Dict[str, Any]) -> str:
    """Render discovery results."""
    from cli._shared import Colors as C

    _YELLOW = getattr(C, "WARNING", C.END)
    _GREEN = getattr(C, "GREEN", C.END)

    lines = [
        f"\n  {C.CYAN}═══ Pattern Discovery ═══{C.END}",
    ]
    if result.get("new_patterns"):
        lines.append(f"  {_GREEN}{result['patterns_discovered']} new patterns discovered{C.END}")
        lines.append(f"  Total stored: {result['total_patterns']}")
        lines.append("")
        for p in result["new_patterns"]:
            lines.append(f"  {_YELLOW}[{p['type']}]{C.END}")
            for k, v in p.items():
                if k != "type" and k != "discovered_at" and v is not None:
                    lines.append(f"    {k}: {v}")
            lines.append("")
    else:
        lines.append("  No new patterns discovered this cycle.")

    lines.append(
        f"  Gene Pool: {result.get('event_capsules', 0)} event capsules, {result.get('research_capsules', 0)} research"
    )
    if result.get("markets"):
        lines.append(f"  Markets tracked: {list(result['markets'].keys())}")
    lines.append(f"  {C.CYAN}═══════════════════════════════════{C.END}")
    return "\n".join(lines)
