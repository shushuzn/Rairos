"""Research Memory + Anomaly Detector.

Tracks your research stance log over time — what you concluded, rejected, deferred.
Watches new papers and flags when they directly contradict your prior decisions.
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


class StanceType(Enum):
    SUPPORTED = "supported"
    REJECTED = "rejected"
    DEFERRED = "deferred"
    QUALIFIED = "qualified"  # supported with caveats


class AnomalySeverity(Enum):
    HIGH = "high"      # direct contradiction
    MEDIUM = "medium"  # challenges evidence
    LOW = "low"        # tangential challenge


@dataclass
class ResearchStance:
    """A research decision you made — stance on a claim, method, or hypothesis."""
    stance_id: str
    topic: str                       # e.g. "RAG vs fine-tuning for knowledge tasks"
    claim: str                       # the specific claim you took a stance on
    stance: StanceType
    evidence_refs: List[str]          # arxiv_ids that support this stance
    reasoning: str                    # why you took this stance
    confidence: float = 0.5           # 0.0–1.0 how certain you were
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    tags: List[str] = field(default_factory=list)
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["stance"] = self.stance.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "ResearchStance":
        d = dict(d)
        d["stance"] = StanceType(d.pop("stance", "supported"))
        return cls(**d)


@dataclass
class AnomalyAlert:
    """A new paper contradicts or challenges a prior research stance."""
    anomaly_id: str
    stance_id: str
    topic: str
    stance_claim: str
    paper_title: str
    paper_arxiv_id: str
    anomaly_type: str               # "contradiction", "counterevidence", "challenge"
    severity: AnomalySeverity
    description: str
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["severity"] = self.severity.value
        return d


# ─── Storage ─────────────────────────────────────────────────────────


def _get_memory_path() -> Path:
    path = Path.home() / ".ai_research_os" / "research_memory"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _get_stance_path() -> Path:
    return _get_memory_path() / "stances.json"


def _get_anomaly_path() -> Path:
    return _get_memory_path() / "anomalies.json"


def _load_stances() -> List[Dict[str, Any]]:
    path = _get_stance_path()
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return []


def _save_stances(stances: List[Dict[str, Any]]) -> None:
    path = _get_stance_path()
    path.write_text(json.dumps(stances, indent=2, ensure_ascii=False), encoding="utf-8")


def _load_anomalies() -> List[Dict[str, Any]]:
    path = _get_anomaly_path()
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return []


def _save_anomalies(anomalies: List[Dict[str, Any]]) -> None:
    path = _get_anomaly_path()
    path.write_text(json.dumps(anomalies, indent=2, ensure_ascii=False), encoding="utf-8")


# ─── Core API ────────────────────────────────────────────────────────


class ResearchMemory:
    """Personal research stance log with anomaly detection."""

    def __init__(self):
        self._stances: List[ResearchStance] = []
        self._anomalies: List[AnomalyAlert] = []
        self._load()

    def _load(self) -> None:
        raw = _load_stances()
        self._stances = [ResearchStance.from_dict(d) for d in raw]
        raw_anom = _load_anomalies()
        self._anomalies = []
        for d in raw_anom:
            d_copy = dict(d)
            d_copy["severity"] = AnomalySeverity(d_copy.get("severity", "medium"))
            self._anomalies.append(AnomalyAlert(**d_copy))

    def _persist(self) -> None:
        _save_stances([s.to_dict() for s in self._stances])
        _save_anomalies([a.to_dict() for a in self._anomalies])

    # ── Stance CRUD ────────────────────────────────────────────────────

    def add_stance(
        self,
        topic: str,
        claim: str,
        stance: StanceType,
        evidence_refs: Optional[List[str]] = None,
        reasoning: str = "",
        confidence: float = 0.5,
        tags: Optional[List[str]] = None,
        notes: str = "",
    ) -> ResearchStance:
        """Record a research stance."""
        s = ResearchStance(
            stance_id=str(uuid.uuid4())[:8],
            topic=topic,
            claim=claim,
            stance=stance,
            evidence_refs=evidence_refs or [],
            reasoning=reasoning,
            confidence=confidence,
            tags=tags or [],
            notes=notes,
        )
        self._stances.append(s)
        self._persist()
        return s

    def update_stance(self, stance_id: str, **kwargs) -> Optional[ResearchStance]:
        """Update an existing stance."""
        for s in self._stances:
            if s.stance_id == stance_id:
                if "claim" in kwargs:
                    s.claim = kwargs["claim"]
                if "stance" in kwargs:
                    s.stance = StanceType(kwargs["stance"]) if isinstance(kwargs["stance"], str) else kwargs["stance"]
                if "reasoning" in kwargs:
                    s.reasoning = kwargs["reasoning"]
                if "confidence" in kwargs:
                    s.confidence = float(kwargs["confidence"])
                if "notes" in kwargs:
                    s.notes = kwargs["notes"]
                if "tags" in kwargs:
                    s.tags = kwargs["tags"]
                s.updated_at = time.time()
                self._persist()
                return s
        return None

    def get_stances(self, topic: Optional[str] = None, stance_type: Optional[StanceType] = None) -> List[ResearchStance]:
        """Query stances by topic or type."""
        results = self._stances
        if topic:
            results = [s for s in results if topic.lower() in s.topic.lower()]
        if stance_type:
            results = [s for s in results if s.stance == stance_type]
        return sorted(results, key=lambda s: s.created_at, reverse=True)

    def get_stance(self, stance_id: str) -> Optional[ResearchStance]:
        for s in self._stances:
            if s.stance_id == stance_id:
                return s
        return None

    # ── Anomaly Detection ─────────────────────────────────────────────

    def check_paper_against_stances(
        self,
        paper: Dict[str, Any],
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
    ) -> List[AnomalyAlert]:
        """Check a new paper against all prior stances. Returns anomalies."""
        anomalies: List[AnomalyAlert] = []

        for stance in self._stances:
            if use_llm:
                anomaly = self._llm_check_against_stance(paper, stance, api_key, base_url, model)
            else:
                anomaly = self._keyword_check_against_stance(paper, stance)

            if anomaly:
                anomalies.append(anomaly)

        # Persist new anomalies
        for a in anomalies:
            existing = [x for x in self._anomalies if x.paper_arxiv_id == a.paper_arxiv_id and x.stance_id == a.stance_id]
            if not existing:
                self._anomalies.append(a)

        if anomalies:
            self._persist()

        return anomalies

    def _keyword_check_against_stance(
        self,
        paper: Dict[str, Any],
        stance: ResearchStance,
    ) -> Optional[AnomalyAlert]:
        """Fast keyword-based contradiction check."""
        paper_text = (
            paper.get("title", "") + " " + paper.get("abstract", "")
        ).lower()
        _stance_text = stance.claim.lower()

        # Check for negation patterns indicating contradiction
        contradiction_signals = ["fail to", "does not", "cannot", "ineffective", "worse than", "no evidence", "contrary to"]
        for signal in contradiction_signals:
            if signal in paper_text and any(w in paper_text for w in stance.claim.lower().split()[:5]):
                return AnomalyAlert(
                    anomaly_id=str(uuid.uuid4())[:8],
                    stance_id=stance.stance_id,
                    topic=stance.topic,
                    stance_claim=stance.claim,
                    paper_title=paper.get("title", "")[:120],
                    paper_arxiv_id=paper.get("arxiv_id", ""),
                    anomaly_type="challenge",
                    severity=AnomalySeverity.MEDIUM,
                    description="Paper discusses limitations that challenge the claimed stance",
                )
        return None

    def _llm_check_against_stance(
        self,
        paper: Dict[str, Any],
        stance: ResearchStance,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> Optional[AnomalyAlert]:
        """LLM-powered deep contradiction check."""
        import os

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        prompt = f"""You are a research anomaly detector. Given a prior research stance and a new paper, determine if the paper contradicts or challenges that stance.

PRIOR STANCE:
- Topic: {stance.topic}
- Claim: {stance.claim}
- Your stance: {stance.stance.value} (confidence: {stance.confidence})
- Reasoning: {stance.reasoning[:200]}

NEW PAPER:
- Title: {paper.get('title', 'Unknown')}
- Abstract: {paper.get('abstract', 'N/A')[:500]}

TASK: Respond with ONLY a JSON object (no markdown, no explanation):
{{
  "anomaly_type": "contradiction" | "counterevidence" | "challenge" | "none",
  "severity": "high" | "medium" | "low",
  "description": "2-sentence explanation of how this paper challenges your prior stance"
}}

If the paper does NOT contradict or challenge the stance, respond with: {{"anomaly_type": "none", "severity": "low", "description": ""}}"""

        try:
            response = call_llm_chat_completions(
                base_url=base_url or os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
                api_key=api_key or os.getenv("OPENAI_API_KEY", ""),
                model=model or os.getenv("LLM_MODEL", "") or LLM_MODEL,
                system_prompt="You are a precise research anomaly detector. Respond with valid JSON only.",
                user_prompt=prompt,
            )

            parsed = json.loads(response.strip())
            if parsed.get("anomaly_type", "none") == "none":
                return None

            severity_map = {"high": AnomalySeverity.HIGH, "medium": AnomalySeverity.MEDIUM, "low": AnomalySeverity.LOW}
            return AnomalyAlert(
                anomaly_id=str(uuid.uuid4())[:8],
                stance_id=stance.stance_id,
                topic=stance.topic,
                stance_claim=stance.claim,
                paper_title=paper.get("title", "")[:120],
                paper_arxiv_id=paper.get("arxiv_id", ""),
                anomaly_type=parsed["anomaly_type"],
                severity=severity_map.get(parsed.get("severity", "medium"), AnomalySeverity.MEDIUM),
                description=parsed.get("description", ""),
            )
        except Exception:
            return None

    # ── Batch check from subscription ────────────────────────────────

    def check_papers_batch(
        self,
        papers: List[Dict[str, Any]],
        use_llm: bool = True,
        api_key: Optional[str] = None,
    ) -> List[AnomalyAlert]:
        """Check multiple papers for anomalies against all stances."""
        all_anomalies: List[AnomalyAlert] = []
        for paper in papers:
            anomalies = self.check_paper_against_stances(paper, use_llm=use_llm, api_key=api_key)
            all_anomalies.extend(anomalies)
        return all_anomalies

    # ── Anomaly access ────────────────────────────────────────────────

    def get_recent_anomalies(self, limit: int = 20) -> List[AnomalyAlert]:
        """Get most recent anomaly alerts."""
        sorted_anomalies = sorted(self._anomalies, key=lambda a: a.created_at, reverse=True)
        return sorted_anomalies[:limit]

    def get_anomalies_by_stance(self, stance_id: str) -> List[AnomalyAlert]:
        return [a for a in self._anomalies if a.stance_id == stance_id]

    def dismiss_anomaly(self, anomaly_id: str) -> None:
        self._anomalies = [a for a in self._anomalies if a.anomaly_id != anomaly_id]
        self._persist()

    # ── Summary ───────────────────────────────────────────────────────

    def get_summary(self) -> Dict[str, Any]:
        """Get memory summary stats."""
        stance_counts = {}
        for s in self._stances:
            stance_counts[s.stance.value] = stance_counts.get(s.stance.value, 0) + 1
        return {
            "total_stances": len(self._stances),
            "stance_breakdown": stance_counts,
            "total_anomalies": len(self._anomalies),
            "recent_anomalies": len([a for a in self._anomalies if time.time() - a.created_at < 86400]),
        }
