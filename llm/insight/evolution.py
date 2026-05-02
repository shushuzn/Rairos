"""
InsightEvolution — feedback-descent闭环优化insight capsule质量。

闭环流程:
    audit()     → 评估现有capsule质量，打分
    propose()   → 生成V2改进候选
    evaluate()  → pairwise LLM比较
    apply()     → 淘汰低质量capsule，采纳高分候选

论文thesis: Self-Evolving Gene/Capsule System
- Gene: 触发器-动作-结果编码的research action pattern
- Capsule: 成功的gene实例（被用户ACCEPT的事件编码）
- feedback-descent: 用户行为 → capsule质量评分 → 淘汰/进化
"""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from llm.insight.gene import CapsuleGene
from llm.insight.preferences import ExplorationAction
from llm.insight.tracker import EvolutionTracker


# ─── Quality Score ──────────────────────────────────────────────────────────

@dataclass
class CapsuleQuality:
    """Quality score for a single capsule."""
    capsule_id: str
    quality_score: float          # 0.0–1.0, composite
    novelty: float               # 0.0–1.0, how unique is this pattern
    utility: float               # 0.0–1.0, how often referenced/used
    freshness: float             # 0.0–1.0, recency of use
    overall: float               # 0.0–1.0, weighted composite


@dataclass
class AuditResult:
    """Result of auditing the gene pool."""
    total_capsules: int
    avg_quality: float
    high_quality: List[CapsuleQuality]   # score > 0.7
    low_quality: List[CapsuleQuality]     # score < 0.3
    candidate_ids: List[str]             # worth evolving
    retire_ids: List[str]                # should be removed


# ─── V2 Candidate ───────────────────────────────────────────────────────────

@dataclass
class CapsuleCandidate:
    """A proposed V2 improvement to a capsule."""
    original_id: str
    candidate_id: str
    trigger_topic: str
    trigger_gap_type: str
    trigger_keywords: List[str]
    action_gap_type: str
    action_gap_title: str
    mutation_description: str   # what changed and why
    confidence: float          # 0.0–1.0, how confident in this mutation
    source: str                # 'trigger_refine' | 'keyword_expand' | 'gap_type_transfer' | 'llm_suggested'


@dataclass
class EvaluationResult:
    """Result of pairwise LLM evaluation between candidates."""
    winner_id: str
    loser_id: str
    reasoning: str
    confidence: float           # how decisive the win was


# ─── InsightEvolution ────────────────────────────────────────────────────────

class InsightEvolution:
    """
    Feedback-descent闭环: 讓capsule gene pool质量持续提升。

    核心思想:
    1. 用户每次ACCEPT/REJECT gap，都是一次feedback
    2. capsule = 成功的pattern encoding
    3. 质量差的capsule降低权重，屡次差的淘汰
    4. 好的capsule衍生突变，产生新候选
    5. pairwise LLM评估决定哪个候选更优
    6. 采纳高分候选，回输到Gene Pool

    跟EvoSkill的区别:
    - EvoSkill: benchmark-driven skill optimization (task/answer)
    - InsightEvolution: behavior-driven capsule optimization (pattern/quality)
    """

    # Quality score thresholds
    HIGH_QUALITY_THRESHOLD = 0.70
    LOW_QUALITY_THRESHOLD = 0.30
    RETIRE_COUNT_THRESHOLD = 3   # retire if score < 0.3 this many times

    # Evolution params
    MAX_CANDIDATES_PER_EVOLVE = 5
    MAX_GENE_POOL_SIZE = 500    # cap to prevent bloat

    def __init__(self, tracker: Optional[EvolutionTracker] = None):
        self.tracker = tracker or EvolutionTracker()
        self._llm_client = None   # lazily initialized

    # ─── LLM Client (lazy) ─────────────────────────────────────────────────

    def _get_llm_client(self):
        """Lazy LLM client init. Returns a client with .generate(prompt) method."""
        if self._llm_client is None:
            try:
                from llm.client import get_client
                self._llm_client = get_client()
            except Exception:
                self._llm_client = None
        return self._llm_client

    # ─── Step 1: Audit ──────────────────────────────────────────────────────

    def audit(self, min_capsules: int = 3) -> AuditResult:
        """
        Audit all capsules in the gene pool.

        Quality = f(novelty, utility, freshness, feedback_score)

        Args:
            min_capsules: minimum capsules needed for meaningful evolution

        Returns:
            AuditResult with quality scores and action recommendations
        """
        capsules = self._load_capsules()
        if len(capsules) < min_capsules:
            return AuditResult(
                total_capsules=len(capsules),
                avg_quality=0.0,
                high_quality=[],
                low_quality=[],
                candidate_ids=[],
                retire_ids=[],
            )

        scored = []
        for capsule in capsules:
            q = self._score_capsule(capsule)
            scored.append(q)

        avg_q = sum(q.overall for q in scored) / len(scored)

        high_q = [q for q in scored if q.overall >= self.HIGH_QUALITY_THRESHOLD]
        low_q = [q for q in scored if q.overall < self.LOW_QUALITY_THRESHOLD]
        candidates = [q.capsule_id for q in scored if q.overall >= 0.5]
        # Retire: low quality AND low novelty AND old
        retire = [q.capsule_id for q in low_q
                  if q.novelty < 0.3 and q.freshness < 0.3]

        return AuditResult(
            total_capsules=len(capsules),
            avg_quality=avg_q,
            high_quality=high_q,
            low_quality=low_q,
            candidate_ids=candidates,
            retire_ids=retire,
        )

    def _score_capsule(self, capsule: CapsuleGene) -> CapsuleQuality:
        """Compute multi-dimensional quality score for a capsule."""
        # Novelty: feedback_count low relative to age (if rarely used, may be too specific)
        novelty = min(capsule.feedback_count / 10.0, 1.0) if capsule.feedback_count > 0 else 0.5

        # Utility: outcome_success_score is primary signal
        utility = capsule.outcome_success_score

        # Freshness: time since creation (newer = better for evolution)
        try:
            age_days = (datetime.now() - datetime.fromisoformat(capsule.created_at)).days
            freshness = max(0.0, 1.0 - age_days / 365.0)
        except Exception:
            freshness = 0.5

        # Composite: utility weighted heavily
        overall = 0.5 * utility + 0.2 * novelty + 0.2 * freshness + 0.1 * capsule.outcome_success_score

        return CapsuleQuality(
            capsule_id=capsule.capsule_id,
            quality_score=overall,
            novelty=novelty,
            utility=utility,
            freshness=freshness,
            overall=overall,
        )

    # ─── Step 2: Propose ───────────────────────────────────────────────────

    def propose(
        self,
        topic: str,
        gap_type: Optional[str] = None,
        limit: int = 5,
    ) -> List[CapsuleCandidate]:
        """
        Propose V2 improvements for capsules matching the topic.

        Mutation strategies:
        1. trigger_refine: same pattern, broader/narrower topic keywords
        2. keyword_expand: same trigger, extract additional keywords
        3. gap_type_transfer: same topic, different gap_type (cross-domain)
        4. llm_suggested: LLM proposes modifications based on usage patterns

        Args:
            topic: research topic
            gap_type: optional gap type filter
            limit: max candidates to generate

        Returns:
            List of CapsuleCandidate V2 improvements
        """
        candidates: List[CapsuleCandidate] = []
        capsules = self.tracker.find_capsule(topic, gap_type or "", min_score=0.1)

        for capsule in capsules[:self.MAX_CANDIDATES_PER_EVOLVE]:
            # Strategy 1: trigger_refine — broaden topic
            c1 = self._mutate_trigger_broaden(capsule, topic)
            if c1:
                candidates.append(c1)

            # Strategy 2: gap_type_transfer — cross gap types
            c2 = self._mutate_gap_type_transfer(capsule, topic)
            if c2:
                candidates.append(c2)

            # Strategy 3: keyword_expand
            c3 = self._mutate_keyword_expand(capsule, topic)
            if c3:
                candidates.append(c3)

        # Strategy 4: LLM-proposed improvements for top capsules
        if len(candidates) < limit:
            llm_candidates = self._propose_llm(top_capsules=capsules[:3], topic=topic)
            candidates.extend(llm_candidates)

        return candidates[:limit]

    def _mutate_trigger_broaden(
        self, capsule: CapsuleGene, topic: str
    ) -> Optional[CapsuleCandidate]:
        """Broaden trigger topic to be more general."""
        if not capsule.trigger_topic:
            return None
        broader = capsule.trigger_topic
        # Add parent topic words if topic has hyphen/slash
        if "-" in topic or "/" in topic:
            broader = topic.replace("-", " ").replace("/", " ")
        elif capsule.trigger_topic.lower() not in topic.lower():
            broader = topic

        return CapsuleCandidate(
            original_id=capsule.capsule_id,
            candidate_id=str(uuid.uuid4())[:8],
            trigger_topic=broader,
            trigger_gap_type=capsule.trigger_gap_type,
            trigger_keywords=capsule.trigger_keywords,
            action_gap_type=capsule.action_gap_type,
            action_gap_title=capsule.action_gap_title,
            mutation_description=f"trigger_refine: broadened from '{capsule.trigger_topic}' to '{broader}'",
            confidence=0.7,
            source="trigger_refine",
        )

    def _mutate_gap_type_transfer(
        self, capsule: CapsuleGene, topic: str
    ) -> Optional[CapsuleCandidate]:
        """Transfer successful pattern to different gap type."""
        from llm.gap_analyzer import GapType

        # List all gap types
        all_types = [gt.value for gt in GapType]
        if capsule.trigger_gap_type in all_types:
            idx = all_types.index(capsule.trigger_gap_type)
            # Try adjacent gap type
            new_type = all_types[(idx + 1) % len(all_types)]
        else:
            new_type = all_types[0]

        return CapsuleCandidate(
            original_id=capsule.capsule_id,
            candidate_id=str(uuid.uuid4())[:8],
            trigger_topic=capsule.trigger_topic,
            trigger_gap_type=new_type,
            trigger_keywords=capsule.trigger_keywords,
            action_gap_type=capsule.action_gap_type,
            action_gap_title=capsule.action_gap_title,
            mutation_description=f"gap_type_transfer: {capsule.trigger_gap_type} → {new_type}",
            confidence=0.5,
            source="gap_type_transfer",
        )

    def _mutate_keyword_expand(
        self, capsule: CapsuleGene, topic: str
    ) -> Optional[CapsuleCandidate]:
        """Expand keyword set with topic words."""
        topic_words = [w.strip() for w in topic.split() if len(w) > 3]
        existing = set(k.lower() for k in capsule.trigger_keywords)
        new_kws = [w for w in topic_words if w.lower() not in existing]

        if not new_kws:
            return None

        expanded_kws = capsule.trigger_keywords + new_kws[:3]

        return CapsuleCandidate(
            original_id=capsule.capsule_id,
            candidate_id=str(uuid.uuid4())[:8],
            trigger_topic=capsule.trigger_topic,
            trigger_gap_type=capsule.trigger_gap_type,
            trigger_keywords=expanded_kws,
            action_gap_type=capsule.action_gap_type,
            action_gap_title=capsule.action_gap_title,
            mutation_description=f"keyword_expand: added {new_kws[:3]} to keywords",
            confidence=0.6,
            source="keyword_expand",
        )

    def _propose_llm(
        self,
        top_capsules: List[CapsuleGene],
        topic: str,
    ) -> List[CapsuleCandidate]:
        """Use LLM to propose V2 improvements for top capsules."""
        if not top_capsules:
            return []

        client = self._get_llm_client()
        if not client:
            return []

        try:
            capsules_json = json.dumps([c.to_dict() for c in top_capsules[:3]], indent=2, ensure_ascii=False)
        except Exception:
            return []

        prompt = f"""Given these successful research capsule patterns, propose 1-2 improvements.

Topic: {topic}

Capsules:
{capsules_json}

For each proposed improvement, respond with:
- original_id: which capsule to improve
- mutation_description: what to change and why
- suggested_trigger_topic: new trigger topic (or same)
- suggested_trigger_gap_type: new gap type (or same)
- suggested_keywords: additional keywords to add (or empty list)

Respond as JSON array of objects with keys: original_id, mutation_description, suggested_trigger_topic, suggested_trigger_gap_type, suggested_keywords"""

        try:
            response = client.generate(prompt)
            proposals = json.loads(response)
            if not isinstance(proposals, list):
                return []

            candidates = []
            for p in proposals[:2]:
                original = next((c for c in top_capsules if c.capsule_id == p.get("original_id")), top_capsules[0])
                candidates.append(CapsuleCandidate(
                    original_id=original.capsule_id,
                    candidate_id=str(uuid.uuid4())[:8],
                    trigger_topic=p.get("suggested_trigger_topic", original.trigger_topic),
                    trigger_gap_type=p.get("suggested_trigger_gap_type", original.trigger_gap_type),
                    trigger_keywords=p.get("suggested_keywords", original.trigger_keywords),
                    action_gap_type=original.action_gap_type,
                    action_gap_title=original.action_gap_title,
                    mutation_description=p.get("mutation_description", "llm_suggested"),
                    confidence=0.6,
                    source="llm_suggested",
                ))
            return candidates
        except Exception:
            return []

    # ─── Step 3: Evaluate ─────────────────────────────────────────────────

    def evaluate(
        self,
        candidates: List[CapsuleCandidate],
    ) -> List[EvaluationResult]:
        """
        Pairwise LLM comparison: which candidate is better?

        Evaluates every pair and returns ranked results.

        Args:
            candidates: list of V2 candidates to compare

        Returns:
            List of EvaluationResult for each pair (sorted by confidence desc)
        """
        if len(candidates) < 2:
            return []

        results: List[EvaluationResult] = []
        client = self._get_llm_client()

        for i in range(len(candidates)):
            for j in range(i + 1, len(candidates)):
                a, b = candidates[i], candidates[j]

                if client:
                    try:
                        result = self._llm_compare(a, b, client)
                        results.append(result)
                    except Exception:
                        # Fallback: confidence=0.5
                        results.append(EvaluationResult(
                            winner_id=a.candidate_id,
                            loser_id=b.candidate_id,
                            reasoning="llm_unavailable",
                            confidence=0.5,
                        ))
                else:
                    # No LLM: use confidence scores
                    winner = a if a.confidence >= b.confidence else b
                    loser = b if winner is a else a
                    results.append(EvaluationResult(
                        winner_id=winner.candidate_id,
                        loser_id=loser.candidate_id,
                        reasoning=f"fallback: confidence {winner.confidence} vs {loser.confidence}",
                        confidence=abs(winner.confidence - loser.confidence),
                    ))

        # Sort by confidence desc
        results.sort(key=lambda r: r.confidence, reverse=True)
        return results

    def _llm_compare(
        self,
        a: CapsuleCandidate,
        b: CapsuleCandidate,
        client,
    ) -> EvaluationResult:
        """Use LLM to compare two candidates."""
        prompt = f"""Compare two research capsule improvements. Which is more likely to be useful?

Capsule A:
  trigger_topic: {a.trigger_topic}
  trigger_gap_type: {a.trigger_gap_type}
  trigger_keywords: {a.trigger_keywords}
  action_gap_title: {a.action_gap_title}
  mutation: {a.mutation_description}

Capsule B:
  trigger_topic: {b.trigger_topic}
  trigger_gap_type: {b.trigger_gap_type}
  trigger_keywords: {b.trigger_keywords}
  action_gap_title: {b.action_gap_title}
  mutation: {b.mutation_description}

Respond with JSON:
{{"winner": "A" or "B", "reasoning": "why", "confidence": 0.0-1.0}}"""

        try:
            response = client.generate(prompt)
            parsed = json.loads(response)
            winner_id = a.candidate_id if parsed.get("winner") == "A" else b.candidate_id
            loser_id = b.candidate_id if winner_id == a.candidate_id else a.candidate_id
            return EvaluationResult(
                winner_id=winner_id,
                loser_id=loser_id,
                reasoning=parsed.get("reasoning", ""),
                confidence=float(parsed.get("confidence", 0.5)),
            )
        except Exception:
            # Fallback: pick higher confidence
            winner = a if a.confidence >= b.confidence else b
            return EvaluationResult(
                winner_id=winner.candidate_id,
                loser_id=b.candidate_id if winner is a else a.candidate_id,
                reasoning="parse_error_fallback",
                confidence=0.5,
            )

    # ─── Step 4: Apply ─────────────────────────────────────────────────────

    def apply(
        self,
        candidates: List[CapsuleCandidate],
        evaluations: List[EvaluationResult],
        audit: AuditResult,
    ) -> Dict[str, Any]:
        """
        Apply evolution: retire low-quality capsules, add winning candidates.

        Args:
            candidates: all proposed candidates
            evaluations: pairwise evaluation results
            audit: audit result with retire list

        Returns:
            dict with apply summary
        """
        added = 0
        retired = 0

        # Retire low-quality capsules
        for cid in audit.retire_ids:
            if self._retire_capsule(cid):
                retired += 1

        # If gene pool is oversized, retire oldest low-quality first
        if self._gene_pool_size() > self.MAX_GENE_POOL_SIZE:
            excess = self._gene_pool_size() - self.MAX_GENE_POOL_SIZE
            oldest_low = self._get_oldest_low_quality(excess)
            for cid in oldest_low:
                if self._retire_capsule(cid):
                    retired += 1

        # Merge overlapping capsules (same gap_type + >80% keyword overlap)
        merged = self._merge_capsules()
        retired += merged

        # Auto-archive: update low_score_streak, archive if streak >= 3
        auto_archived = self._auto_archive_low_score()
        retired += auto_archived

        # Add winning candidates (winners from evaluations)
        if evaluations:
            winner_ids = set(e.winner_id for e in evaluations)
            winners = [c for c in candidates if c.candidate_id in winner_ids]
            for c in winners[:3]:  # max 3 per evolution cycle
                if self._add_candidate(c):
                    added += 1

        return {
            "added": added,
            "retired": retired,
            "total_capsules": self._gene_pool_size(),
            "avg_quality": audit.avg_quality,
        }

    def _retire_capsule(self, capsule_id: str) -> bool:
        """Mark a capsule as retired (append to retire log, remove from both stores)."""
        capsules = self._load_capsules()
        updated = [c for c in capsules if c.capsule_id != capsule_id]

        if len(updated) == len(capsules):
            return False  # not found

        # Rewrite gene pool (remove retired)
        self._save_capsules(updated)

        # Also remove from capsules.json (web UI store)
        capsules_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if capsules_path.exists():
            try:
                data = json.loads(capsules_path.read_text(encoding="utf-8"))
                raw = data.get("capsules", []) if isinstance(data, dict) else data
                raw = [c for c in raw if c.get("capsule_id", "") != capsule_id]
                data["capsules"] = raw
                capsules_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            except Exception:
                pass

        # Log retirement
        retire_file = self.tracker.data_dir / "retired.jsonl"
        with open(retire_file, "a", encoding="utf-8") as f:
            f.write(json.dumps({
                "capsule_id": capsule_id,
                "retired_at": datetime.now().isoformat(),
            }, ensure_ascii=False) + "\n")

        return True

    def _add_candidate(self, candidate: CapsuleCandidate) -> bool:
        """Add a winning candidate as a new capsule to the gene pool."""
        capsules = self._load_capsules()

        # Check for near-duplicate
        for c in capsules:
            if (c.trigger_topic == candidate.trigger_topic
                    and c.trigger_gap_type == candidate.trigger_gap_type):
                return False  # duplicate, skip

        new_capsule = CapsuleGene(
            capsule_id=candidate.candidate_id,
            created_at=datetime.now().isoformat(),
            trigger_topic=candidate.trigger_topic,
            trigger_gap_type=candidate.trigger_gap_type,
            trigger_keywords=candidate.trigger_keywords,
            action_gap_type=candidate.action_gap_type,
            action_gap_title=candidate.action_gap_title,
            outcome_success_score=candidate.confidence * 0.7,  # initial score from confidence
            feedback_count=0,
            evolved_generation=1,  # V2 generation
            archetype={},
            status="active",
        )

        capsules.append(new_capsule)
        self._save_capsules(capsules)
        return True

    # ─── Full evolution cycle ───────────────────────────────────────────────

    def evolve(
        self,
        topic: str,
        gap_type: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        Run one complete feedback-descent evolution cycle.

        Args:
            topic: research topic
            gap_type: optional gap type filter

        Returns:
            dict with cycle summary
        """
        # 1. Audit
        audit = self.audit()

        # 2. Propose
        candidates = self.propose(topic, gap_type)

        # 3. Evaluate
        evaluations = self.evaluate(candidates)

        # 4. Apply
        result = self.apply(candidates, evaluations, audit)

        return {
            "audit": {
                "total": audit.total_capsules,
                "avg_quality": round(audit.avg_quality, 3),
                "candidates": len(audit.candidate_ids),
                "to_retire": len(audit.retire_ids),
            },
            "proposed": len(candidates),
            "evaluations": len(evaluations),
            "result": result,
        }

    # ─── Helpers ────────────────────────────────────────────────────────────

    def _load_capsules(self) -> List[CapsuleGene]:
        """Load all capsules from gene pool."""
        gene_file = self.tracker.data_dir / "gene_pool.jsonl"
        if not gene_file.exists():
            return []

        capsules = []
        with open(gene_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    capsules.append(CapsuleGene.from_dict(json.loads(line)))
                except Exception:
                    continue
        return capsules

    def _save_capsules(self, capsules: List[CapsuleGene]) -> None:
        """Rewrite gene pool file."""
        gene_file = self.tracker.data_dir / "gene_pool.jsonl"
        with open(gene_file, "w", encoding="utf-8") as f:
            for c in capsules:
                f.write(json.dumps(c.to_dict(), ensure_ascii=False) + "\n")

    def _gene_pool_size(self) -> int:
        """Return current gene pool size."""
        return len(self._load_capsules())

    def _get_oldest_low_quality(self, limit: int) -> List[str]:
        """Get IDs of oldest low-quality capsules."""
        capsules = self._load_capsules()
        scored = [(c, self._score_capsule(c)) for c in capsules]
        low = [c for c, q in scored if q.overall < 0.5]
        # Sort by creation date, oldest first
        try:
            low.sort(key=lambda c: c.created_at)
        except Exception:
            pass
        return [c.capsule_id for c in low[:limit]]

    OVERLAP_THRESHOLD = 0.80  # merge if keyword Jaccard > 80%

    def _merge_capsules(self) -> int:
        """Find capsule pairs with same gap_type + >80% keyword overlap; merge into higher-score, archive lower-score.

        Returns the number of capsules archived.
        """
        capsules = [c for c in self._load_capsules() if c.status == "active"]
        merged_count = 0
        to_archive: set[str] = set()

        for i, a in enumerate(capsules):
            if a.capsule_id in to_archive:
                continue
            for b in capsules[i + 1:]:
                if b.capsule_id in to_archive:
                    continue
                if a.trigger_gap_type != b.trigger_gap_type:
                    continue

                # Keyword Jaccard similarity
                set_a = set(k.lower() for k in a.trigger_keywords)
                set_b = set(k.lower() for k in b.trigger_keywords)
                if not set_a or not set_b:
                    continue
                intersection = len(set_a & set_b)
                union = len(set_a | set_b)
                jaccard = intersection / union if union > 0 else 0.0

                if jaccard >= self.OVERLAP_THRESHOLD:
                    # Keep the one with higher score, archive the other
                    loser = b if a.outcome_success_score >= b.outcome_success_score else a
                    winner = a if loser is b else b

                    # Merge keywords into winner (union, dedup preserving order)
                    existing = set(k.lower() for k in winner.trigger_keywords)
                    merged_kws = list(winner.trigger_keywords)
                    for kw in loser.trigger_keywords:
                        if kw.lower() not in existing:
                            merged_kws.append(kw)
                    winner.trigger_keywords = merged_kws[:20]
                    winner.feedback_count += loser.feedback_count
                    to_archive.add(loser.capsule_id)
                    merged_count += 1

        if not to_archive:
            return 0

        # Archive losers and save updated winners
        remaining = [c for c in capsules if c.capsule_id not in to_archive]
        # Add back winners (with merged keywords) + any non-active capsules
        all_capsules = self._load_capsules()
        winners_updated = {
            c.capsule_id: c for c in capsules
            if c.capsule_id not in to_archive
        }
        result = []
        for c in all_capsules:
            if c.capsule_id in to_archive:
                c.status = "archived"
                result.append(c)
            elif c.capsule_id in winners_updated:
                result.append(winners_updated[c.capsule_id])
            else:
                result.append(c)

        self._save_capsules(result)

        # Also clean capsules.json
        capsules_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if capsules_path.exists():
            try:
                data = json.loads(capsules_path.read_text(encoding="utf-8"))
                raw = data.get("capsules", []) if isinstance(data, dict) else data
                raw = [c for c in raw if c.get("capsule_id", "") not in to_archive]
                data["capsules"] = raw
                capsules_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            except Exception:
                pass

        return len(to_archive)

    LOW_SCORE_THRESHOLD = 0.30
    STREAK_THRESHOLD = 3  # archive after 3 consecutive low-score cycles

    def _auto_archive_low_score(self) -> int:
        """Update low_score_streak for all active capsules; archive those with streak >= 3.

        Each evolution cycle:
        - If capsule score < 0.3: increment streak
        - If capsule score >= 0.3: reset streak to 0
        - If streak >= 3: mark archived

        Returns the number of capsules auto-archived.
        """
        capsules = self._load_capsules()
        to_archive: set[str] = set()
        updated = []

        for c in capsules:
            if c.status != "active":
                updated.append(c)
                continue

            if c.outcome_success_score < self.LOW_SCORE_THRESHOLD:
                c.low_score_streak += 1
            else:
                c.low_score_streak = 0

            if c.low_score_streak >= self.STREAK_THRESHOLD:
                c.status = "archived"
                to_archive.add(c.capsule_id)

            updated.append(c)

        if not to_archive:
            self._save_capsules(updated)
            return 0

        self._save_capsules(updated)

        # Also update capsules.json
        capsules_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if capsules_path.exists():
            try:
                data = json.loads(capsules_path.read_text(encoding="utf-8"))
                raw = data.get("capsules", []) if isinstance(data, dict) else data
                archived_ids = set(to_archive)
                for c in raw:
                    if c.get("capsule_id", "") in archived_ids:
                        c["status"] = "archived"
                data["capsules"] = raw
                capsules_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            except Exception:
                pass

        return len(to_archive)

    # ─── CLI entry point ───────────────────────────────────────────────────

    def render_summary(self, result: Dict[str, Any]) -> str:
        """Render evolution result as human-readable string."""
        audit = result.get("audit", {})
        res = result.get("result", {})
        lines = [
            "=== Insight Evolution Cycle ===",
            f"Capsules:    {audit.get('total', 0)} total, avg quality {audit.get('avg_quality', 0):.3f}",
            f"Proposed:    {result.get('proposed', 0)} candidates",
            f"Evaluated:   {result.get('evaluations', 0)} pairs",
            f"Added:       {res.get('added', 0)} new capsules",
            f"Retired:     {res.get('retired', 0)} capsules",
            f"Pool size:   {res.get('total_capsules', 0)}",
        ]
        return "\n".join(lines)
