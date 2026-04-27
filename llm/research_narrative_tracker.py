"""Research Narrative Tracker: Unified view connecting gaps, hypotheses, experiments, and insights.

Aggregates state from all existing trackers to provide a "research thread" view:
- Exploration → Hypothesis → Validation → Publication phase tracking
- Publication readiness scoring
- Next-step recommendations

This is a VIEW layer — it reads from all trackers, never writes back to them.
"""
from __future__ import annotations

import logging
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from llm.tracker_base import JsonFileStore

logger = logging.getLogger(__name__)


# ─── Phase ────────────────────────────────────────────────────────────────────


class NarrativePhase(Enum):
    """Phase of a research thread."""
    EXPLORATION = "exploration"   # gaps + questions identified
    HYPOTHESIS  = "hypothesis"   # hypotheses generated
    VALIDATION  = "validation"   # experiments running/completed
    PUBLICATION = "publication"   # ready to write / published


# ─── Data model ────────────────────────────────────────────────────────────────


@dataclass
class ResearchThread:
    """A research topic trajectory — unified view across all trackers."""
    id: str
    topic: str
    phase: NarrativePhase
    phase_updated_at: str = ""

    # Aggregated IDs from existing trackers (not duplicated data)
    paper_ids: List[str] = field(default_factory=list)
    question_ids: List[str] = field(default_factory=list)
    hypothesis_ids: List[str] = field(default_factory=list)
    experiment_ids: List[str] = field(default_factory=list)
    insight_card_ids: List[str] = field(default_factory=list)

    # Counts
    gap_count: int = 0
    hypothesis_count: int = 0
    validated_count: int = 0   # experiments with status=completed
    rejected_count: int = 0   # experiments with status=failed
    running_count: int = 0     # experiments with status=running

    # Computed readiness scores (0.0–1.0)
    contribution_score: float = 0.0
    experiment_score: float = 0.0
    narrative_score: float = 0.0

    # User-facing narrative
    notes: str = ""
    created_at: str = ""
    updated_at: str = ""

    def __post_init__(self):
        now = datetime.now().isoformat()
        if not self.created_at:
            self.created_at = now
        if not self.updated_at:
            self.updated_at = now
        if not self.phase_updated_at:
            self.phase_updated_at = now

    def to_dict(self) -> dict:
        d = asdict(self)
        d["phase"] = self.phase.value  # serialize enum as string
        return d

    @classmethod
    def from_dict(cls, data: dict) -> "ResearchThread":
        data = data.copy()
        if isinstance(data.get("phase"), str):
            data["phase"] = NarrativePhase(data["phase"])
        return cls(**data)


# ─── Persistence ────────────────────────────────────────────────────────────────


class ResearchNarrativeTracker(JsonFileStore):
    """Persist research threads to ~/.ai_research_os/narrative/threads.json."""

    def __init__(self, data_dir: Optional[Path] = None):
        p = Path(data_dir or Path.home() / ".ai_research_os" / "narrative")
        p.mkdir(parents=True, exist_ok=True)
        self.data_file = p / "threads.json"

    def _post_load(self, raw: List[dict]) -> List[ResearchThread]:
        return [ResearchThread.from_dict(r) for r in raw]

    def _pre_save(self, threads: List[ResearchThread]) -> List[dict]:
        return [t.to_dict() for t in threads]

    # ─── CRUD ────────────────────────────────────────────────────────────────

    def list_threads(self) -> List[ResearchThread]:
        return self._load()

    def get_thread(self, thread_id: str) -> Optional[ResearchThread]:
        for t in self._load():
            if t.id == thread_id:
                return t
        return None

    def get_by_topic(self, topic: str) -> Optional[ResearchThread]:
        for t in self._load():
            if t.topic.lower() == topic.lower():
                return t
        return None

    def upsert(self, thread: ResearchThread) -> None:
        threads = self._load()
        for i, t in enumerate(threads):
            if t.id == thread.id:
                threads[i] = thread
                break
        else:
            threads.append(thread)
        thread.updated_at = datetime.now().isoformat()
        self._save(threads)

    def delete(self, thread_id: str) -> bool:
        threads = self._load()
        original = len(threads)
        threads = [t for t in threads if t.id != thread_id]
        if len(threads) < original:
            self._save(threads)
            return True
        return False


# ─── Aggregation service ────────────────────────────────────────────────────────


class ResearchNarrativeService:
    """
    Aggregate state from all existing trackers into a ResearchThread.

    Read-only — never writes to EvolutionTracker, ExperimentTracker, etc.
    """

    def __init__(self, tracker: Optional[ResearchNarrativeTracker] = None):
        self.tracker = tracker or ResearchNarrativeTracker()

    # ─── Main entry ─────────────────────────────────────────────────────────

    def aggregate(self, topic: str) -> ResearchThread:
        """
        Build a ResearchThread for the given topic by reading all trackers.
        """
        existing = self.tracker.get_by_topic(topic)
        thread = ResearchThread(
            id=existing.id if existing else str(uuid.uuid4())[:8],
            topic=topic,
            phase=NarrativePhase.EXPLORATION,
            phase_updated_at=datetime.now().isoformat(),
        )

        # Aggregate from each tracker
        thread.question_ids = self._from_question_tracker(topic)
        thread.hypothesis_ids = self._from_evolution_tracker(topic)
        thread.experiment_ids, thread.validated_count, thread.rejected_count, thread.running_count = (
            self._from_experiment_tracker(thread.hypothesis_ids)
        )
        thread.insight_card_ids, thread.gap_count = self._from_insight_manager(topic)
        thread.paper_ids = self._from_papers_db(topic)

        # Counts
        thread.hypothesis_count = len(thread.hypothesis_ids)

        # Phase + scores
        thread.phase = self._compute_phase(thread)
        thread.contribution_score, thread.experiment_score, thread.narrative_score = (
            self._compute_readiness(thread)
        )

        # Preserve user notes
        if existing:
            thread.notes = existing.notes
            if existing.phase != thread.phase:
                thread.phase_updated_at = datetime.now().isoformat()

        thread.updated_at = datetime.now().isoformat()
        return thread

    def save(self, thread: ResearchThread) -> None:
        self.tracker.upsert(thread)

    # ─── Source aggregators ─────────────────────────────────────────────────

    def _from_question_tracker(self, topic: str) -> List[str]:
        """Pull question IDs matching this topic."""
        try:
            from llm.question_tracker import QuestionTracker
            qt = QuestionTracker()
            qs = qt.list_questions(topic=topic)
            return [q.id for q in qs]
        except Exception as e:
            logger.debug("QuestionTracker unavailable: %s", e)
            return []

    def _from_evolution_tracker(self, topic: str) -> List[str]:
        """Pull hypothesis_ids from hypothesized events."""
        try:
            from llm.insight_evolution import EvolutionTracker, ExplorationAction
            ev = EvolutionTracker()
            events = ev.get_recent_events(limit=10000)
            hypothesis_ids = []
            for e in events:
                if e.hypothesis_id:
                    # Match topic in event topic or gap_title
                    if (e.topic and topic.lower() in e.topic.lower()) or (
                        e.gap_title and topic.lower() in e.gap_title.lower()
                    ):
                        if e.hypothesis_id not in hypothesis_ids:
                            hypothesis_ids.append(e.hypothesis_id)
            return hypothesis_ids
        except Exception as e:
            logger.debug("EvolutionTracker unavailable: %s", e)
            return []

    def _from_experiment_tracker(
        self, hypothesis_ids: List[str]
    ) -> Tuple[List[str], int, int, int]:
        """Pull experiments linked to these hypotheses."""
        if not hypothesis_ids:
            return [], 0, 0, 0
        try:
            from llm.experiment_tracker import ExperimentTracker
            et = ExperimentTracker()
            all_exps = et.list_experiments()
            linked = [e for e in all_exps if e.hypothesis_id in hypothesis_ids]
            ids = [e.id for e in linked]
            validated = sum(1 for e in linked if e.status == "completed")
            rejected = sum(1 for e in linked if e.status == "failed")
            running = sum(1 for e in linked if e.status == "running")
            return ids, validated, rejected, running
        except Exception as e:
            logger.debug("ExperimentTracker unavailable: %s", e)
            return [], 0, 0, 0

    def _from_insight_manager(self, topic: str) -> Tuple[List[str], int]:
        """Search insight cards for this topic."""
        try:
            from llm.insight_cards import InsightManager, InsightCard
            im = InsightManager()
            cards = im.search_cards(query=topic)
            ids = []
            for c in cards:
                if isinstance(c, InsightCard):
                    cid = c.card_id
                elif isinstance(c, dict):
                    cid = c.get("card_id", "")
                else:
                    cid = getattr(c, "card_id", "")
                if cid and cid not in ids:
                    ids.append(cid)
            return ids, len(ids)
        except Exception as e:
            logger.debug("InsightManager unavailable: %s", e)
            return [], 0

    def _from_papers_db(self, topic: str) -> List[str]:
        """Find paper IDs matching this topic."""
        try:
            from db.database import Database
            db = Database()
            db.init()
            results, _ = db.search_papers(topic, limit=20)
            return [r.paper_id for r in results]
        except Exception as e:
            logger.debug("Database unavailable: %s", e)
            return []

    # ─── Phase & score computation ─────────────────────────────────────────

    def _compute_phase(self, thread: ResearchThread) -> NarrativePhase:
        """Determine current narrative phase."""
        if thread.experiment_score >= 0.8 and thread.contribution_score >= 0.7:
            return NarrativePhase.PUBLICATION
        if thread.hypothesis_ids:
            return NarrativePhase.VALIDATION
        if thread.question_ids:
            return NarrativePhase.HYPOTHESIS
        return NarrativePhase.EXPLORATION

    def _compute_readiness(
        self, thread: ResearchThread
    ) -> Tuple[float, float, float]:
        """Compute contribution, experiment, and narrative scores (0.0–1.0)."""
        contrib = self._contribution_score(thread)
        exp = self._experiment_score(thread)
        narr = self._narrative_score(thread, contrib, exp)
        return contrib, exp, narr

    def _contribution_score(self, thread: ResearchThread) -> float:
        score = 0.0
        # Has gap + hypothesis: +0.3
        if thread.gap_count > 0 and thread.hypothesis_count > 0:
            score += 0.3
        # Multiple hypotheses (>1): +0.2
        if thread.hypothesis_count > 1:
            score += 0.2
        # Has insight cards: +0.2
        if thread.insight_card_ids:
            score += 0.2
        # Multiple paper IDs (active research): +0.2
        if len(thread.paper_ids) >= 3:
            score += 0.2
        # Has questions driving exploration: +0.1
        if thread.question_ids:
            score += 0.1
        return min(score + 1e-9, 1.0)

    def _experiment_score(self, thread: ResearchThread) -> float:
        score = 0.0
        total = thread.validated_count + thread.rejected_count + thread.running_count
        if total == 0:
            return 0.0
        # ≥3 validated: +0.4
        if thread.validated_count >= 3:
            score += 0.4
        elif thread.validated_count >= 1:
            score += 0.2
        # Both validated AND rejected (bidirectional exploration): +0.2
        if thread.validated_count >= 1 and thread.rejected_count >= 1:
            score += 0.2
        # Has running experiments: +0.1
        if thread.running_count >= 1:
            score += 0.1
        # Multiple hypotheses being validated: +0.1
        if thread.hypothesis_count >= 2:
            score += 0.1
        # Has paper IDs for experimental basis: +0.2
        if thread.paper_ids:
            score += 0.2
        return min(score, 1.0)

    def _narrative_score(
        self, thread: ResearchThread, contrib: float, exp: float
    ) -> float:
        """How coherent is the research story."""
        score = 0.5  # baseline
        # Coherent flow: questions → hypotheses → experiments: +0.3
        if thread.question_ids and thread.hypothesis_ids:
            score += 0.15
        if thread.hypothesis_ids and thread.experiment_ids:
            score += 0.15
        # Experiment results back up contribution: +0.1
        if thread.validated_count >= 1 and contrib >= 0.4:
            score += 0.1
        # Insight cards provide supporting narrative: +0.1
        if len(thread.insight_card_ids) >= 3:
            score += 0.1
        return min(score, 1.0)

    # ─── Recommendations ───────────────────────────────────────────────────

    def generate_next_steps(self, thread: ResearchThread) -> List[Dict[str, str]]:
        """
        Generate concrete next-step recommendations based on current state.
        Returns list of {action, reason} dicts.
        """
        steps = []

        if thread.phase == NarrativePhase.EXPLORATION:
            if thread.gap_count == 0:
                steps.append({
                    "action": f"Run gap analysis on '{thread.topic}'",
                    "reason": "No gaps identified yet — start with landscape analysis",
                })
            elif thread.question_ids == 0:
                steps.append({
                    "action": "Generate research questions from gaps",
                    "reason": f"{thread.gap_count} gaps found — convert to actionable questions",
                })
            else:
                steps.append({
                    "action": "Generate hypotheses from questions",
                    "reason": f"{len(thread.question_ids)} questions ready — move to hypothesis phase",
                })

        elif thread.phase == NarrativePhase.HYPOTHESIS:
            steps.append({
                "action": "Run `airos hypothesize '{topic}'` with gap context".format(topic=thread.topic),
                "reason": f"{thread.hypothesis_count} hypotheses generated — design experiments",
            })
            if thread.insight_card_ids == 0:
                steps.append({
                    "action": "Extract insight cards from related papers",
                    "reason": "No insight cards linked — build supporting evidence",
                })

        elif thread.phase == NarrativePhase.VALIDATION:
            if thread.validated_count == 0:
                steps.append({
                    "action": "Design first experiment for hypothesis " + (thread.hypothesis_ids[0] if thread.hypothesis_ids else ""),
                    "reason": "Hypotheses exist but no experiments completed yet",
                })
            if thread.running_count == 0:
                steps.append({
                    "action": "Run `airos experiment --hypothesis-id <id>`",
                    "reason": "No experiments in progress — start validation",
                })
            if thread.experiment_score < 0.5:
                steps.append({
                    "action": "Expand benchmark coverage (need ≥3 benchmarks)",
                    "reason": f"Experiment score {thread.experiment_score:.0%} — add more benchmarks",
                })
            if thread.contribution_score < 0.5:
                steps.append({
                    "action": "Collect more insight cards to strengthen narrative",
                    "reason": f"Contribution score {thread.contribution_score:.0%} — build supporting evidence",
                })

        elif thread.phase == NarrativePhase.PUBLICATION:
            steps.append({
                "action": "Draft paper structure using `airos story '{topic}'`".format(topic=thread.topic),
                "reason": "Research is publication-ready — start writing",
            })
            if thread.narrative_score < 0.8:
                steps.append({
                    "action": "Strengthen narrative coherence (add more insight cards)",
                    "reason": f"Narrative score {thread.narrative_score:.0%} — polish story",
                })

        return steps


# ─── Renderer ────────────────────────────────────────────────────────────────


def _phase_icon(phase: NarrativePhase) -> str:
    return {
        NarrativePhase.EXPLORATION: "🔍 EXPLORATION",
        NarrativePhase.HYPOTHESIS:  "💡 HYPOTHESIS",
        NarrativePhase.VALIDATION: "🔬 VALIDATION",
        NarrativePhase.PUBLICATION: "📄 PUBLICATION",
    }.get(phase, "??")


def _score_bar(score: float, width: int = 8) -> str:
    filled = int(round(score * width))
    return "█" * filled + "░" * (width - filled)


def render_thread(thread: ResearchThread, service: ResearchNarrativeService) -> str:
    """Render a ResearchThread as human-readable text."""
    lines = [
        "═" * 60,
        f"📊 Research Narrative: {thread.topic}",
        "═" * 60,
        f"Phase: {_phase_icon(thread.phase)}   Updated: {thread.phase_updated_at[:10]}",
        "",
    ]

    # Gap summary
    if thread.gap_count > 0:
        lines.append(f"GAP ANALYSIS ({thread.gap_count} gaps identified)")
        lines.append(f"  Insight cards: {len(thread.insight_card_ids)} linked")
    else:
        lines.append("GAP ANALYSIS — No gaps identified yet. Run `airos gap <topic>` first.")

    # Question summary
    if thread.question_ids:
        lines.append(f"QUESTIONS: {len(thread.question_ids)} tracked")
    else:
        lines.append("QUESTIONS: None tracked yet")

    lines.append("")

    # Hypothesis status
    if thread.hypothesis_ids:
        lines.append(f"HYPOTHESIS STATUS ({thread.hypothesis_count} generated)")
        try:
            from llm.insight_evolution import EvolutionTracker
            ev = EvolutionTracker()
            for hid in thread.hypothesis_ids[:5]:
                events = ev.get_hypothesis_events(hid)
                verdict, detail = _compute_verdict(events)
                icon = {"VALIDATED": "✅", "REJECTED": "❌", "MIXED": "⚠", "INCONCLUSIVE": "○"}.get(verdict, "?")
                lines.append(f"  {icon} [{hid}] {verdict} — {detail}")
        except Exception:
            for hid in thread.hypothesis_ids[:5]:
                lines.append(f"  • [{hid}]")
        if len(thread.hypothesis_ids) > 5:
            lines.append(f"  ... +{len(thread.hypothesis_ids) - 5} more")
    else:
        lines.append("HYPOTHESIS STATUS — None generated yet")

    lines.append("")

    # Experiment summary
    if thread.experiment_ids:
        lines.append(f"EXPERIMENTS: {len(thread.experiment_ids)} total")
        lines.append(
            f"  ✅ {thread.validated_count} validated  "
            f"❌ {thread.rejected_count} rejected  "
            f"⚡ {thread.running_count} running"
        )
    else:
        lines.append("EXPERIMENTS: None yet")

    lines.append("")

    # Publication readiness
    lines.append("PUBLICATION READINESS")
    lines.append(
        f"├─ Theoretical contribution  {_score_bar(thread.contribution_score)}  "
        f"{thread.contribution_score:.0%}"
    )
    lines.append(
        f"├─ Experimental support      {_score_bar(thread.experiment_score)}  "
        f"{thread.experiment_score:.0%}"
    )
    lines.append(
        f"└─ Narrative coherence       {_score_bar(thread.narrative_score)}  "
        f"{thread.narrative_score:.0%}"
    )

    # Next steps
    steps = service.generate_next_steps(thread)
    if steps:
        lines.append("")
        lines.append("NEXT RECOMMENDED STEPS")
        for i, s in enumerate(steps[:5], 1):
            lines.append(f"  {i}. {s['action']}")
            lines.append(f"     理由: {s['reason']}")

    # User notes
    if thread.notes:
        lines.append("")
        lines.append("NARRATIVE NOTES")
        lines.append(f"  {thread.notes}")

    lines.append("")
    lines.append("═" * 60)
    lines.append(f"Thread ID: {thread.id}  |  Created: {thread.created_at[:10]}")
    return "\n".join(lines)


def _compute_verdict(events) -> Tuple[str, str]:
    """Match the logic from cli/cmd/hypothesize.py."""
    if not events:
        return "INCONCLUSIVE", "no experiments recorded"
    action_vals = set()
    for e in events:
        val = getattr(e.action, "value", None) or str(getattr(e, "action", ""))
        action_vals.add(val)
    has_completed = "validated" in action_vals or "completed" in action_vals
    has_failed = "rejected" in action_vals or "failed" in action_vals
    if has_completed and has_failed:
        return "MIXED", "both validated and rejected exist"
    if has_completed:
        return "VALIDATED", "experiments succeeded"
    if has_failed:
        return "REJECTED", "experiments failed"
    return "INCONCLUSIVE", "no completed experiments yet"


def render_dashboard(threads: List[ResearchThread]) -> str:
    """Render a table overview of all threads."""
    if not threads:
        return "No research threads yet. Run `airos narrative track <topic>` to start."

    phase_col = {
        NarrativePhase.EXPLORATION: "🔍 EXPL",
        NarrativePhase.HYPOTHESIS:  "💡 HYP",
        NarrativePhase.VALIDATION:  "🔬 VAL",
        NarrativePhase.PUBLICATION:  "📄 PUB",
    }
    lines = [
        "═" * 70,
        "📊 Research Narrative Dashboard",
        "═" * 70,
        f"  {'Topic':<22} {'Phase':<10} {'Gaps':>4} {'Hyps':>4} "
        f"{'Exp':>4} {'Val':>4} {'Contrib':>7} {'ExpScore':>8} {'Narr':>5}",
        f"  {'-'*22} {'-'*10} {'-'*4} {'-'*4} {'-'*4} {'-'*4} {'-'*7} {'-'*8} {'-'*5}",
    ]
    for t in sorted(threads, key=lambda x: x.phase.value):
        phase_str = phase_col.get(t.phase, "??")
        lines.append(
            f"  {t.topic[:22]:<22} {phase_str:<10} "
            f"{t.gap_count:>4} {t.hypothesis_count:>4} "
            f"{len(t.experiment_ids):>4} {t.validated_count:>4} "
            f"{t.contribution_score:>7.0%} {t.experiment_score:>8.0%} "
            f"{t.narrative_score:>5.0%}"
        )
    lines.append("═" * 70)
    return "\n".join(lines)
