"""Gene Pool Watcher — auto-discover and fill diversity gaps.

Monitors underrepresented algorithm families in the Gene Pool and automatically
creates ArXiv subscriptions to fill those gaps. Closes the self-evolution loop:
Gene Pool gap → auto-subscribe → paper2code → Gene Pool encode → diversity re评估.
"""

from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"


# ─── Family → ArXiv category + keyword mapping ──────────────────────────────

FAMILY_ARXIV_CONFIG: Dict[str, Dict[str, Any]] = {
    "attention": {
        "keywords": ["transformer", "self-attention", "multi-head attention", "vision transformer", "ViT"],
        "category": "cs.CL",  # Computation and Language
    },
    "reinforcement": {
        "keywords": ["reinforcement learning", "policy gradient", "DQN", "PPO", "A3C", "reward"],
        "category": "cs.LG",  # Machine Learning
    },
    "language_model": {
        "keywords": ["language model", "LLM", "GPT", "BERT", "decoder", "autoregressive", " Transformer"],
        "category": "cs.CL",
    },
    "vision": {
        "keywords": ["CNN", "image classification", "object detection", "segmentation", "ViT", "vision transformer"],
        "category": "cs.CV",
    },
    "optimization": {
        "keywords": ["optimizer", "Adam", "SGD", "gradient descent", "loss landscape", "training dynamics"],
        "category": "cs.LG",
    },
    "graph": {
        "keywords": ["graph neural network", "GNN", "message passing", "node classification", "graph convolution"],
        "category": "cs.SD",  # Social and Information Networks / cs.LG
    },
    "reasoning": {
        "keywords": ["chain-of-thought", "reasoning", "logical inference", "planning", "theorem proving"],
        "category": "cs.AI",
    },
    "embodied": {
        "keywords": ["robotics", "embodied", "navigation", "control", "motor", "reinforcement learning robot"],
        "category": "cs.RO",  # Robotics
    },
    "other": {
        "keywords": ["neural network", "deep learning", "training", "representation learning"],
        "category": "cs.LG",
    },
}


@dataclass
class GapSubscription:
    """An automatically created subscription targeting a underrepresented family."""

    family: str
    keywords: List[str]
    arxiv_category: str
    enabled: bool = True
    last_checked: str = ""  # ISO timestamp


@dataclass
class WatcherState:
    """Persistent state for the GenePoolWatcher."""

    gap_subscriptions: List[GapSubscription] = field(default_factory=list)
    last_diversity_check: str = ""  # ISO timestamp
    underrepresented_families: List[str] = field(default_factory=list)
    diversity_score: float = 0.0


def _load_watcher_state() -> WatcherState:
    """Load watcher state from disk, or return empty state."""
    from llm.gene_pool_io import get_gene_pool_diversity

    state = WatcherState()
    try:
        diversity = get_gene_pool_diversity()
        state.underrepresented_families = diversity.get("underrepresented_families", [])
        state.diversity_score = diversity.get("diversity_score", 0.0)
        state.last_diversity_check = _now_iso()
    except Exception:
        pass

    # Load gap subscriptions from file
    gap_sub_path = GP_DIR / "gap_subscriptions.json"
    if gap_sub_path.exists():
        try:
            import json
            data = json.loads(gap_sub_path.read_text(encoding="utf-8"))
            state.gap_subscriptions = [
                GapSubscription(
                    family=s.get("family", "other"),
                    keywords=s.get("keywords", []),
                    arxiv_category=s.get("arxiv_category", "cs.LG"),
                    enabled=s.get("enabled", True),
                    last_checked=s.get("last_checked", ""),
                )
                for s in data.get("gap_subscriptions", [])
            ]
        except Exception:
            pass
    return state


def _save_watcher_state(state: WatcherState) -> None:
    """Persist watcher state to disk."""
    import json

    GP_DIR.mkdir(parents=True, exist_ok=True)
    gap_sub_path = GP_DIR / "gap_subscriptions.json"
    data = {
        "gap_subscriptions": [
            {
                "family": s.family,
                "keywords": s.keywords,
                "arxiv_category": s.arxiv_category,
                "enabled": s.enabled,
                "last_checked": s.last_checked,
            }
            for s in state.gap_subscriptions
        ],
        "last_diversity_check": state.last_diversity_check,
        "underrepresented_families": state.underrepresented_families,
        "diversity_score": state.diversity_score,
    }
    gap_sub_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def _now_iso() -> str:
    from datetime import datetime
    return datetime.utcnow().isoformat()


# ─── Gap detection and subscription creation ───────────────────────────────────


def _build_gap_subscriptions(underrep_families: List[str]) -> List[GapSubscription]:
    """Build GapSubscription list for underrepresented families."""
    subs = []
    for fam in underrep_families:
        config = FAMILY_ARXIV_CONFIG.get(fam, FAMILY_ARXIV_CONFIG["other"])
        subs.append(
            GapSubscription(
                family=fam,
                keywords=config["keywords"],
                arxiv_category=config["category"],
            )
        )
    return subs


def _diff_subscriptions(
    existing: List[GapSubscription], new: List[GapSubscription]
) -> tuple[List[GapSubscription], List[str]]:
    """Diff existing vs new gap subscriptions.

    Returns:
        - gap_subscriptions_to_add: new GapSubscriptions not in existing
        - gap_families_to_remove: families in existing but not in new (no longer underrepresented)
    """
    existing_families = {s.family for s in existing}
    new_families = {s.family for s in new}

    to_add = [s for s in new if s.family not in existing_families]
    to_remove = [f for f in existing_families if f not in new_families]

    return to_add, to_remove


# ─── ArXiv subscription registration ────────────────────────────────────────────


def _register_gap_subscription(sub: GapSubscription) -> Optional[str]:
    """Register a gap subscription in the database.

    Returns subscription_id if successful, None otherwise.
    """
    try:
        from db.database import Database
        from llm.subscription_monitor import SubscriptionMonitor

        db = Database()
        db.init()

        # Build topic from family name
        topic = f"[AUTO-GAP] {sub.family} — diversity fill"
        keywords_str = ",".join(sub.keywords[:5])

        # Insert into subscriptions table
        cur = db.conn.cursor()
        cur.execute(
            """
            INSERT INTO subscriptions (topic, keywords, category, enabled, auto_generated, created_at, updated_at, last_check_id, last_checked_at)
            VALUES (?, ?, ?, ?, 1, datetime('now'), datetime('now'), NULL, NULL)
            """,
            (topic, keywords_str, sub.arxiv_category, 1),
        )
        db.conn.commit()
        sub_id = str(cur.lastrowid)
        return sub_id
    except Exception as e:
        import logging
        logging.getLogger("gene_pool_watcher").warning(f"Failed to register gap subscription: {e}")
        return None


def _disable_subscription(sub_id: str) -> bool:
    """Disable a subscription by ID."""
    try:
        from db.database import Database

        db = Database()
        db.init()
        db.conn.execute(
            "UPDATE subscriptions SET enabled = 0 WHERE id = ?",
            (sub_id,),
        )
        db.conn.commit()
        return True
    except Exception:
        return False


# ─── Main watcher class ──────────────────────────────────────────────────────


class GenePoolWatcher:
    """Periodically checks Gene Pool diversity and auto-creates gap subscriptions.

    Usage:
        watcher = GenePoolWatcher(interval_minutes=60)
        watcher.start()  # starts background thread
        watcher.stop()   # stops background thread
    """

    def __init__(
        self,
        interval_minutes: int = 60,
        min_diversity_score: float = 50.0,
        enabled: bool = True,
    ):
        """
        Args:
            interval_minutes: how often to check diversity (default 60 min)
            min_diversity_score: trigger gap-filling only if diversity_score falls below this
            enabled: if False, watcher only monitors without acting
        """
        self.interval_seconds = interval_minutes * 60
        self.min_diversity_score = min_diversity_score
        self.enabled = enabled
        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None
        self.state = _load_watcher_state()

    def start(self) -> None:
        """Start the watcher background thread."""
        if self._thread is not None and self._thread.is_alive():
            return
        self._stop_event.clear()
        self._thread = threading.Thread(
            target=self._run_loop,
            daemon=True,
            name="GenePoolWatcher",
        )
        self._thread.start()

    def stop(self) -> None:
        """Stop the watcher background thread."""
        self._stop_event.set()
        if self._thread is not None:
            self._thread.join(timeout=5)
            self._thread = None

    def _run_loop(self) -> None:
        """Main watch loop — runs until stop()."""
        while not self._stop_event.is_set():
            try:
                self._check_and_update()
            except Exception as e:
                import logging
                logging.getLogger("gene_pool_watcher").warning(f"Watch cycle error: {e}")
            self._stop_event.wait(timeout=self.interval_seconds)

    def _check_and_update(self) -> Dict[str, Any]:
        """Check diversity and update gap subscriptions.

        Returns a summary dict of what was done.
        """
        from llm.gene_pool_io import get_gene_pool_diversity

        diversity = get_gene_pool_diversity()
        underrep = diversity.get("underrepresented_families", [])
        diversity_score = diversity.get("diversity_score", 0.0)
        total_capsules = diversity.get("capsule_count", 0)

        self.state.underrepresented_families = underrep
        self.state.diversity_score = diversity_score
        self.state.last_diversity_check = _now_iso()

        summary = {
            "diversity_score": diversity_score,
            "total_capsules": total_capsules,
            "underrepresented_families": underrep,
            "gap_subscriptions_added": [],
            "gap_subscriptions_removed": [],
            "triggered": diversity_score < self.min_diversity_score,
        }

        if not self.enabled:
            _save_watcher_state(self.state)
            return summary

        # Build target gap subscriptions for current underrepresented families
        target_subs = _build_gap_subscriptions(underrep)

        # Diff against existing gap subscriptions
        to_add, to_remove = _diff_subscriptions(self.state.gap_subscriptions, target_subs)

        # Add new subscriptions for newly underrepresented families
        for sub in to_add:
            sub_id = _register_gap_subscription(sub)
            if sub_id:
                self.state.gap_subscriptions.append(sub)
                summary["gap_subscriptions_added"].append(sub.family)

        # Disable subscriptions for families no longer underrepresented
        for fam in to_remove:
            for i, gs in enumerate(self.state.gap_subscriptions):
                if gs.family == fam:
                    gs.enabled = False
                    summary["gap_subscriptions_removed"].append(fam)

        _save_watcher_state(self.state)
        return summary

    def trigger_now(self) -> Dict[str, Any]:
        """Manually trigger a diversity check and subscription update."""
        return self._check_and_update()

    def get_state(self) -> WatcherState:
        """Return current watcher state."""
        return self.state


# ─── CLI integration helpers ──────────────────────────────────────────────────


def render_watcher_status_html(state: Optional[WatcherState] = None) -> str:
    """Render watcher status for web UI."""
    if state is None:
        state = _load_watcher_state()

    lines = ['<div class="watcher-panel">']
    lines.append("<h3>🧬 Gene Pool Gap Watcher</h3>")

    if state.underrepresented_families:
        lines.append(
            f"<p style='font-size:13px;color:#A89E8C;margin-bottom:12px'>"
            f"Underrepresented families detected: <b>{', '.join(state.underrepresented_families)}</b><br>"
            f"Diversity score: <b>{state.diversity_score}</b> | Total capsules: <b>{state.diversity_score}</b></p>"
        )
    else:
        lines.append(
            f"<p style='font-size:13px;color:#A89E8C;margin-bottom:12px'>"
            f"Gene Pool is well-diversified. Diversity score: <b>{state.diversity_score}</b></p>"
        )

    if state.gap_subscriptions:
        lines.append("<h4>Auto-Gap Subscriptions</h4>")
        lines.append("<ul style='font-size:13px'>")
        for gs in state.gap_subscriptions:
            status = "✓" if gs.enabled else "✗"
            lines.append(f"<li>{status} <b>{gs.family}</b> — {', '.join(gs.keywords[:2])}</li>")
        lines.append("</ul>")
    else:
        lines.append(
            "<p style='font-size:13px;color:#A89E8C'>No gap subscriptions active.</p>"
        )

    lines.append(
        f"<p style='font-size:12px;color:#888;margin-top:12px'>"
        f"Last checked: {state.last_diversity_check or 'never'}</p>"
    )
    lines.append("</div>")
    return "\n".join(lines)
