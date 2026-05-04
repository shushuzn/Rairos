"""Capsule storage mixin for EvolutionTracker — gene_pool.jsonl I/O."""

from __future__ import annotations

import json
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from llm.insight.gene import CapsuleGene


class CapsuleStorageMixin:
    """Mixin that provides Gene Pool capsule storage to EvolutionTracker.

    Expects the host class to provide:
        self.data_dir: Path
        self.get_archetype() -> dict
        self._extract_keywords(text) -> list[str]
        self._get_timestamp() -> str
        self.record_capsule_lifecycle_event(...)
    """

    @property
    def _gene_pool_file(self) -> Path:
        return self.data_dir / "gene_pool.jsonl"

    def encode_capsule(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        gap_description: str = "",
        success_score: float = 0.8,
        status: str = "active",
        source_paper_id: str = "",
    ) -> CapsuleGene:
        archetype = self.get_archetype()
        if source_paper_id:
            archetype["source_paper_id"] = source_paper_id
        capsule = CapsuleGene(
            capsule_id=uuid.uuid4().hex[:12],
            created_at=self._get_timestamp(),
            trigger_topic=topic,
            trigger_gap_type=gap_type,
            trigger_keywords=self._extract_keywords(gap_title),
            action_gap_type=gap_type,
            action_gap_title=gap_title,
            outcome_success_score=success_score,
            feedback_count=1,
            evolved_generation=0,
            archetype=archetype,
            status=status,
        )

        with open(self._gene_pool_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(capsule.to_dict(), ensure_ascii=False) + "\n")

        self.record_capsule_lifecycle_event(
            capsule_id=capsule.capsule_id,
            action="created",
            gap_title=capsule.action_gap_title,
            gap_type=capsule.action_gap_type,
        )

        return capsule

    def find_capsule(
        self,
        topic: str,
        gap_type: str,
        keywords: Optional[List[str]] = None,
        min_score: float = 0.2,
    ) -> List[CapsuleGene]:
        if not self._gene_pool_file.exists():
            return []

        keywords = keywords or []
        scored: List[Tuple[CapsuleGene, float]] = []

        with open(self._gene_pool_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    capsule = CapsuleGene.from_dict(json.loads(line))
                except Exception:
                    continue

                if capsule.status == "archived":
                    continue
                match_score = capsule.trigger_match(topic, gap_type, keywords)
                if match_score >= min_score:
                    scored.append((capsule, match_score))

        scored.sort(key=lambda x: x[1], reverse=True)
        return [capsule for capsule, _ in scored]

    def archive_capsule(self, capsule_id: str) -> bool:
        archived = False
        archived_title = ""
        archived_gap_type = ""

        capsules = self._load_capsules()
        for c in capsules:
            if c.capsule_id == capsule_id:
                c.status = "archived"
                archived = True
                archived_title = c.action_gap_title
                archived_gap_type = c.action_gap_type
                break
        if archived:
            self._save_capsules(capsules)
            self.record_capsule_lifecycle_event(
                capsule_id=capsule_id,
                action="archived",
                gap_title=archived_title,
                gap_type=archived_gap_type,
            )

        return archived

    def _load_capsules(self) -> List[CapsuleGene]:
        capsules = []
        if self._gene_pool_file.exists():
            with open(self._gene_pool_file, encoding="utf-8") as f:
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
        with open(self._gene_pool_file, "w", encoding="utf-8") as f:
            for c in capsules:
                f.write(json.dumps(c.to_dict(), ensure_ascii=False) + "\n")

    def get_gene_pool_stats(self) -> Dict[str, Any]:
        capsules = []
        if self._gene_pool_file.exists():
            with open(self._gene_pool_file, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        capsules.append(CapsuleGene.from_dict(json.loads(line)))
                    except Exception:
                        continue

        if not capsules:
            return {"total": 0, "avg_score": 0.0, "by_gap_type": {}}

        by_type: Dict[str, int] = {}
        total_score = 0.0
        for c in capsules:
            by_type[c.action_gap_type] = by_type.get(c.action_gap_type, 0) + 1
            total_score += c.outcome_success_score

        return {
            "total": len(capsules),
            "avg_score": total_score / len(capsules),
            "by_gap_type": by_type,
            "generations": sorted(set(c.evolved_generation for c in capsules)),
        }
