"""Autonomous Research Orchestrator — closed-loop research agent.

Watches arXiv via subscriptions, triggers deep gap analysis on new papers,
scores results against Gene Pool preferences, and notifies when high-value
research opportunities are found.

Closed loop:
    arXiv subscription watch → new paper detected
        → DeepResearchAgent (iterative gap analysis)
            → GapAnalyzerV2 (gap detection)
                → Gene Pool scoring (preference-aware ranking)
                    → webhook notification (Discord/Feishu)
                    → encode into Gene Pool
                    → report to Claude Code (via MCP)
"""

from __future__ import annotations

import json
import logging
import threading
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Any, Dict, List, Optional
from core.observability import emit_research_event, EventType, get_trace_id

from rairos_survey_generator_py import generate_survey_py as generate_survey

if TYPE_CHECKING:
    from llm.insight.tracker import EvolutionTracker
    from llm.subscription_monitor import SubscriptionMonitor
    from llm.subscription_scorer import SubscriptionScorer
    from llm.research.gap_analyzer import GapAnalyzerV2
    from research_loop.deep_research import DeepResearchAgent

logger = logging.getLogger(__name__)

# ─── State ───────────────────────────────────────────────────────────────────


def _get_state_path() -> Path:
    path = Path.home() / ".ai_research_os" / "autonomous"
    path.mkdir(parents=True, exist_ok=True)
    return path / "orchestrator_state.json"


def _load_state() -> dict:
    path = _get_state_path()
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
        except Exception:
            pass
    return {
        "running": False,
        "interval_minutes": 30,
        "last_check": "",
        "sessions": [],  # session_ids of completed deep-research runs
        "alerts": [],  # recent high-value alerts
    }


def _save_state(state: dict) -> None:
    path = _get_state_path()
    path.write_text(json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8")


# ─── Data classes ─────────────────────────────────────────────────────────────


@dataclass
class ResearchAlert:
    """A high-value research opportunity discovered by the agent."""

    alert_id: str
    session_id: str
    topic: str
    triggered_by: str  # arxiv_id of the paper that triggered this
    trigger_title: str
    gaps_found: int
    top_gap_title: str
    top_gap_type: str
    severity: str  # HIGH / MEDIUM / LOW
    gene_pool_score: float  # 0.0–1.0
    preference_boost: bool
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "ResearchAlert":
        d.pop("alert_id", None)
        return cls(alert_id=str(uuid.uuid4())[:8], **d)  # type: ignore[return-value]


@dataclass
class OrchestratorConfig:
    """Configuration for the autonomous orchestrator."""

    interval_minutes: int = 30
    min_gap_severity_for_alert: str = "MEDIUM"  # alert only for HIGH/MEDIUM gaps
    min_gene_pool_score_for_alert: float = 0.3  # must match Gene Pool somewhat
    min_papers_for_deep_analysis: int = 3
    max_alerts_stored: int = 50


# ─── Core orchestrator ────────────────────────────────────────────────────────


class AutonomousOrchestrator:
    """Autonomous research orchestrator with closed-loop gap discovery.

    Workflow per subscription cycle:
        1. Check all arXiv subscriptions for new papers (SubscriptionMonitor)
        2. For each subscription with new papers:
            a. Run DeepResearchAgent on the topic + new papers
            b. Collect gaps from GapAnalyzerV2
            c. Score each gap against Gene Pool (preference-aware)
            d. Generate ResearchAlert for high-scoring gaps
            e. Send webhook notification
            f. Record to state
    """

    def __init__(
        self,
        config: Optional[OrchestratorConfig] = None,
        webhook_enabled: bool = True,
    ):
        self.config = config or OrchestratorConfig()
        self.webhook_enabled = webhook_enabled
        self._stop_event: Optional[threading.Event] = None
        self._watch_thread: Optional[threading.Thread] = None

        # Lazy-loaded components
        self._monitor: Optional["SubscriptionMonitor"] = None
        self._scorer: Optional["SubscriptionScorer"] = None
        self._tracker: Optional["EvolutionTracker"] = None
        self._DeepResearchAgent: Optional[type["DeepResearchAgent"]] = None
        self._GapAnalyzerV2: Optional[type["GapAnalyzerV2"]] = None

    # ── Component lazy-init ────────────────────────────────────────────────────

    def _init_components(self):
        """Initialize heavy components on first use."""
        if self._monitor is not None:
            return

        from llm.subscription_monitor import SubscriptionMonitor
        from llm.subscription_scorer import SubscriptionScorer
        from llm.insight.tracker import EvolutionTracker
        from research_loop.deep_research import DeepResearchAgent
        from llm.research.gap_analyzer import GapAnalyzerV2

        from db.database import Database

        db = Database()
        db.init()

        self._db = db
        self._monitor = SubscriptionMonitor(db, SubscriptionScorer(db))
        self._scorer = SubscriptionScorer(db)
        self._tracker = EvolutionTracker()
        self._DeepResearchAgent = DeepResearchAgent
        self._GapAnalyzerV2 = GapAnalyzerV2

    # ── Subscription watch ────────────────────────────────────────────────────

    def check_subscriptions(self) -> Dict[str, List[Dict[str, Any]]]:
        """Check all subscriptions for new papers. Returns sub_id -> new papers."""
        self._init_components()
        return self._monitor.check_all()  # type: ignore[union-attr]

    # ── Deep research on a topic ─────────────────────────────────────────────

    def run_deep_research(self, topic: str, new_papers: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Run deep research loop for a topic.

        Args:
            topic: Research topic
            new_papers: List of new paper dicts from subscription check

        Returns:
            Dict with keys: gaps (list), papers_analyzed (int), session_id (str)
        """
        trace_id = get_trace_id() or ""
        emit_research_event(
            EventType.SESSION_START,
            topic=topic,
            n_new_papers=len(new_papers),
            trace_id=trace_id,
        )
        self._init_components()

        # Build query from the new papers' titles + abstracts
        query = topic

        # Initialize DeepResearchAgent
        agent = self._DeepResearchAgent(  # type: ignore[misc]
            query=query,
            max_iterations=2,
            max_papers_per_iteration=len(new_papers) + 3,
            verbose=False,
        )
        session = agent.start()

        # Add discovered papers to session
        for p in new_papers:
            from research_loop.snapstate import PaperSnapshot

            snapshot = PaperSnapshot(
                arxiv_id=p.get("arxiv_id", ""),
                title=p.get("title", ""),
                abstract=p.get("abstract", ""),
                url=p.get("pdf_url", ""),
            )
            session.papers.append(snapshot)
            agent.db.upsert_paper(  # type: ignore[union-attr]
                paper_id=p.get("arxiv_id", ""),
                source="arxiv",
                title=p.get("title", ""),
                abstract=p.get("abstract", ""),
                categories=p.get("categories", ""),
            )

        # Run the agent loop (shortened)
        try:
            result = agent.run()
        except Exception as e:
            logger.error(f"DeepResearchAgent failed: {e}")
            emit_research_event(
                EventType.SESSION_END,
                topic=topic,
                trace_id=trace_id,
                papers_analyzed=len(new_papers),
                error=str(e),
            )
            return {
                "gaps": [],
                "papers_analyzed": len(new_papers),
                "session_id": session.session_id,
                "error": str(e),
            }

        emit_research_event(
            EventType.SESSION_END,
            topic=topic,
            trace_id=trace_id,
            papers_analyzed=len(new_papers),
            iterations=result.iterations if hasattr(result, "iterations") else 0,
        )
        return {
            "gaps": result.gaps if hasattr(result, "gaps") else [],
            "papers_analyzed": len(new_papers),
            "session_id": session.session_id,
            "iterations": result.iterations if hasattr(result, "iterations") else 0,
        }

    # ── Gene Pool scoring ────────────────────────────────────────────────────

    def score_gaps_against_gene_pool(
        self,
        gaps: List[Any],
        topic: str,
    ) -> List[Dict[str, Any]]:
        """Score gaps against Gene Pool for preference-aware ranking.

        Returns list of scored gap dicts with gene_pool_score and preference_boost.
        """
        self._init_components()

        scored = []
        for gap in gaps:
            gap_type_name = (
                gap.gap_type.value if hasattr(gap.gap_type, "value") else str(gap.gap_type)
            )

            # Find matching capsule in Gene Pool
            capsules = self._tracker.find_capsule(  # type: ignore[union-attr]
                topic=topic,
                gap_type=gap_type_name,
                keywords=[],
            )

            gene_pool_score = 0.0
            preference_boost = False
            capsule = capsules[0] if capsules else None
            if capsule:
                gene_pool_score = getattr(capsule, "outcome_success_score", 0.0)
                preference_boost = gene_pool_score >= 0.5

            scored.append(
                {
                    "gap": gap,
                    "gap_type": gap_type_name,
                    "title": gap.title if hasattr(gap, "title") else str(gap),
                    "description": gap.description if hasattr(gap, "description") else "",
                    "severity": gap.severity.value if hasattr(gap.severity, "value") else "MEDIUM",
                    "gene_pool_score": gene_pool_score,
                    "preference_boost": preference_boost,
                }
            )

        return scored

    # ── Alert generation ────────────────────────────────────────────────────

    def generate_alerts(
        self,
        scored_gaps: List[Dict[str, Any]],
        session_id: str,
        topic: str,
        trigger_paper: Dict[str, Any],
    ) -> List[ResearchAlert]:
        """Generate ResearchAlert objects for high-value gaps."""
        severity_rank = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
        min_sev = severity_rank.get(self.config.min_gap_severity_for_alert, 1)

        alerts = []
        for sg in scored_gaps:
            sev_rank = severity_rank.get(sg["severity"], 2)
            if sev_rank > min_sev:
                continue
            if sg["gene_pool_score"] < self.config.min_gene_pool_score_for_alert:
                continue

            alert = ResearchAlert(
                alert_id=str(uuid.uuid4())[:8],
                session_id=session_id,
                topic=topic,
                triggered_by=trigger_paper.get("arxiv_id", ""),
                trigger_title=trigger_paper.get("title", "")[:80],
                gaps_found=1,
                top_gap_title=sg["title"][:80],
                top_gap_type=sg["gap_type"],
                severity=sg["severity"],
                gene_pool_score=sg["gene_pool_score"],
                preference_boost=sg["preference_boost"],
            )
            alerts.append(alert)

        return alerts

    # ── Webhook notification ────────────────────────────────────────────────

    def _send_webhook(self, alert: ResearchAlert) -> None:
        """Send Discord/Feishu webhook notification for an alert."""
        if not self.webhook_enabled:
            return
        try:
            from core.notifications import get_webhook_notifier

            webhook = get_webhook_notifier()
            if webhook:
                # Build a summary for the webhook
                paper_link = f"https://arxiv.org/abs/{alert.triggered_by}"
                msg = (
                    f"🔬 **Research Opportunity Alert**\n\n"
                    f"**Topic:** {alert.topic}\n"
                    f"**Trigger:** [{alert.trigger_title}]({paper_link})\n\n"
                    f"**Top Gap:** [{alert.top_gap_title}]({paper_link})\n"
                    f"**Type:** {alert.top_gap_type} | **Severity:** {alert.severity}\n"
                    f"**Gene Pool Match:** {alert.gene_pool_score:.2f}\n"
                )
                if alert.preference_boost:
                    msg += "\n✅ Matches your research preferences!"
                webhook.notify_custom(msg)  # type: ignore[attr-defined]
        except Exception as e:
            logger.warning(f"Webhook notification failed: {e}")

    # ── Main cycle ───────────────────────────────────────────────────────────

    def run_cycle(self) -> List[ResearchAlert]:
        """Run one complete orchestrator cycle.

        Returns list of ResearchAlerts generated in this cycle.
        """
        self._init_components()
        all_alerts: List[ResearchAlert] = []

        logger.info("[Orchestrator] Starting cycle...")
        sub_results = self.check_subscriptions()

        for sub_id, new_papers in sub_results.items():
            if not new_papers:
                continue

            topic = sub_id  # topic is used as sub_id in the DB

            logger.info(f"[Orchestrator] {len(new_papers)} new papers for subscription '{topic}'")

            # Run deep research
            try:
                research_result = self.run_deep_research(topic, new_papers)
            except Exception as e:
                logger.error(f"Deep research failed for '{topic}': {e}")
                continue

            gaps = research_result.get("gaps", [])
            session_id = research_result.get("session_id", "")
            if not gaps:
                logger.info(f"[Orchestrator] No gaps found for '{topic}'")
                continue

            # ── Incremental filtering: suppress already-seen gaps ─────────────
            gaps, filter_stats = self._db.filter_new_gaps(topic, gaps)
            seen_count = filter_stats["seen"]
            suppressed = filter_stats["suppressed"]
            if suppressed > 0:
                logger.info(
                    f"[Orchestrator] Suppressed {suppressed} already-seen gaps (total seen: {seen_count})"
                )
            if not gaps:
                logger.info(f"[Orchestrator] All gaps already known for '{topic}' — skipping")
                continue

            # Score against Gene Pool
            scored = self.score_gaps_against_gene_pool(gaps, topic)

            # Generate alerts
            trigger = new_papers[0]  # use first new paper as trigger
            alerts = self.generate_alerts(
                scored,
                research_result["session_id"],
                topic,
                trigger,
            )

            for alert in alerts:
                self._send_webhook(alert)
                all_alerts.append(alert)

                # Encode into Gene Pool
                try:
                    self._tracker.record_gap_accept(  # type: ignore[union-attr]
                        topic=alert.topic,
                        gap_type=alert.top_gap_type,
                        gap_title=alert.top_gap_title,
                        gap_description="",
                    )
                except Exception as e:
                    logger.warning(f"Gene Pool encode failed: {e}")

            # Record filtered gaps to gap_history for future incremental reporting
            self._db.record_gap_history(topic, session_id, gaps)

            # ── Generate research survey ─────────────────────────────────────────
            if gaps:
                try:
                    survey_path = generate_survey(
                        topic=topic,
                        scored_gaps_json=json.dumps(scored),
                        papers_analyzed=research_result.get("papers_analyzed", 0),
                        session_id=session_id,
                        iterations=research_result.get("iterations", 0),
                        gap_history_stats_json=json.dumps(filter_stats) if filter_stats else None,
                    )
                    logger.info(f"[Orchestrator] Survey generated: {survey_path}")
                except Exception as e:
                    logger.warning(f"[Orchestrator] Survey generation failed: {e}")

            logger.info(f"[Orchestrator] Generated {len(alerts)} alerts for '{topic}'")

        # Update state
        state = _load_state()
        for alert in all_alerts:
            state["alerts"].insert(0, alert.to_dict())
        state["alerts"] = state["alerts"][: self.config.max_alerts_stored]
        state["last_check"] = time.strftime("%Y-%m-%dT%H:%M:%S")
        _save_state(state)

        logger.info(f"[Orchestrator] Cycle complete: {len(all_alerts)} alerts")
        return all_alerts

    # ── Background watch ──────────────────────────────────────────────────────

    def start_watch(self, interval_minutes: int = 30) -> None:
        """Start background watch loop in a daemon thread."""
        if self._watch_thread and self._watch_thread.is_alive():
            logger.warning("Watch already running")
            return

        self._stop_event = threading.Event()
        self._watch_thread = threading.Thread(
            target=self._watch_loop,
            args=(interval_minutes, self._stop_event),
            name="autonomous-orchestrator",
            daemon=True,
        )
        self._watch_thread.start()

        state = _load_state()
        state["running"] = True
        state["interval_minutes"] = interval_minutes
        _save_state(state)

        logger.info(f"[Orchestrator] Watch started (interval={interval_minutes}min)")

    def stop_watch(self) -> None:
        """Stop the background watch loop."""
        if self._stop_event:
            self._stop_event.set()

        state = _load_state()
        state["running"] = False
        _save_state(state)

        logger.info("[Orchestrator] Watch stopped")

    def _watch_loop(self, interval_minutes: int, stop_event: threading.Event) -> None:
        """Internal watch loop runner. Runs orchestrator cycle + evolution."""
        evolution_counter = 0
        while not stop_event.is_set():
            try:
                self.run_cycle()
            except Exception as e:
                logger.error(f"[Orchestrator] Cycle error: {e}")

            # Regenerate situation report
            try:
                from llm.report import save as _save_report

                _save_report()
            except Exception:
                pass

            # Run evolution every 3 cycles (or ~90min at 30min intervals)
            evolution_counter += 1
            if evolution_counter >= 3:
                evolution_counter = 0
                try:
                    result = self.run_evolution_cycle()
                    logger.info(f"[Orchestrator] Evolution: {result}")
                except Exception as e:
                    logger.error(f"[Orchestrator] Evolution error: {e}")

            stop_event.wait(timeout=interval_minutes * 60)

    # ── State access ─────────────────────────────────────────────────────────

    def get_recent_alerts(self, limit: int = 20) -> List[ResearchAlert]:
        """Get recent research alerts."""
        state = _load_state()
        alerts = []
        for d in state.get("alerts", [])[:limit]:
            try:
                alerts.append(ResearchAlert(**d))
            except Exception:
                pass
        return alerts

    # ── Evolution cycle ───────────────────────────────────────────────────

    def run_evolution_cycle(self, topic: str = "") -> Dict[str, Any]:
        """Run one InsightEvolution cycle on the Gene Pool."""
        try:
            self._init_components()
            from llm.insight.evolution import InsightEvolution

            evolver = InsightEvolution(tracker=self._tracker)
            evo_topic = topic or self._get_best_evolution_topic()
            result = evolver.evolve(topic=evo_topic)
            return result
        except Exception as e:
            logger.error(f"Evolution cycle failed: {e}")
            return {"error": str(e)}

    def _get_best_evolution_topic(self) -> str:
        """Pick the best topic for evolution from user history."""
        try:
            self._init_components()
            profile = self._tracker.get_profile()  # type: ignore[union-attr]
            topics = list(profile.topic_frequency.keys())
            if topics:
                return max(topics, key=lambda t: profile.topic_frequency[t])  # type: ignore[no-any-return]
        except Exception:
            pass
        return "machine learning"

    def generate_credibility_report(self) -> str:
        """Generate a credibility report for the Web UI."""
        try:
            self._init_components()
            from llm.insight.evolution import InsightEvolution

            evolver = InsightEvolution(tracker=self._tracker)
            return evolver.credibility_report()
        except Exception as e:
            logger.error(f"Credibility report failed: {e}")
            return f"<p>Error generating report: {e}</p>"

    def get_status(self) -> Dict[str, Any]:
        """Get orchestrator status with evolution stats."""
        state = _load_state()
        self._init_components()

        # Gene Pool stats
        pool_stats: Dict[str, Any] = {}
        try:
            pool_stats = self._tracker.get_gene_pool_stats()  # type: ignore[union-attr]
        except Exception:
            pass

        return {
            "running": state.get("running", False),
            "interval_minutes": state.get("interval_minutes", 30),
            "last_check": state.get("last_check", ""),
            "alerts_count": len(state.get("alerts", [])),
            "gene_pool": {
                "total_capsules": pool_stats.get("total", 0),
                "avg_score": pool_stats.get("avg_score", 0.0),
                "by_gap_type": pool_stats.get("by_gap_type", {}),
            },
            "evolution": {
                "available": True,
            },
        }
