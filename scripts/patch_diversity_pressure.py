"""Patch: add DiversityPressureEvaluator to gene_pool_watcher.py"""
import re

with open('llm/gene_pool_watcher.py', 'r', encoding='utf-8') as f:
    content = f.read()

evaluator_code = '''

# ─── Diversity Pressure Evaluator ───────────────────────────────────────────────


@dataclass
class DiversityPressureResult:
    """Result of a diversity pressure evaluation."""

    triggered: bool
    pressure_level: float          # 0.0–1.0 (how much pressure)
    overrepresented_families: List[str]
    underrepresented_families: List[str]
    eviction_candidates: List[Dict]  # [{capsule_id, family, score, reason}]
    archived_capsule_ids: List[str]
    diversity_score: float
    saturation: float


class DiversityPressureEvaluator:
    """Actively rebalances GenePool diversity via forced eviction.

    When the GenePool is highly saturated AND poorly diversified, this evaluator
    identifies low-score capsules in over-represented families as eviction candidates.
    Unlike passive time-decay (which archives by age/impact), this is DIVERSITY-PRESSURE
    eviction — it explicitly punishes homogeneous concentration.

    Trigger conditions (both must hold):
      - saturation >= saturation_threshold  (default 0.80)
      - diversity_score < diversity_threshold  (default 40)

    Eviction logic:
      - Compute per-family capsule counts and identify over-represented families
      - Within each over-represented family, rank capsules by outcome_success_score
      - Evict the bottom N% (eviction_rate) of capsules in those families
      - Only evict if there are underrepresented families to fill the gap
    """

    # ── Configuration ─────────────────────────────────────────────────────────
    SATURATION_THRESHOLD = 0.80     # trigger at 80% gene pool saturation
    DIVERSITY_THRESHOLD = 40        # trigger when diversity_score < 40
    EVICTION_RATE = 0.20            # evict bottom 20% of over-represented family capsules
    MIN_CAPSULES_TO_EVICT = 1      # minimum capsules to actually evict
    CAPACITY_MARGIN = 0.90         # target capacity after eviction

    # Family keyword map (mirrors gene_pool_io.py for consistency)
    FAMILY_KEYWORDS = {
        "attention": ["attention", "transformer", "multi-head", "self-attention", "cross-attention"],
        "reinforcement": ["rl", "reinforcement", "policy", "reward", "agent", "DQN", "PPO", "A3C"],
        "language_model": ["LM", "language model", "decoder", "autoregressive", "LLM", "GPT", "BERT"],
        "vision": ["CNN", "convolution", "resnet", "image", "vision", "ViT", "classification"],
        "optimization": ["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
        "graph": ["GNN", "graph", "node", "edge", "message passing"],
        "reasoning": ["reasoning", "chain-of-thought", "logical", "inference", "planning"],
        "embodied": ["embodied", "robotics", "navigation", "control", "motor"],
    }

    def __init__(self, capacity: int = 50):
        self.capacity = capacity

    def _family_of_keywords(self, keywords: List[str]) -> str:
        """Infer algorithm family from trigger keywords."""
        kw_set = {k.lower() for k in keywords}
        for fam, fam_kws in self.FAMILY_KEYWORDS.items():
            if any(fk in kw_set for fk in fam_kws):
                return fam
        return "other"

    def _family_of_capsule(self, capsule: Any) -> str:
        """Infer algorithm family from a capsule dict/object."""
        kws = capsule.get("trigger_keywords", []) if hasattr(capsule, "get") else getattr(capsule, "trigger_keywords", [])
        return self._family_of_keywords(kws)

    def _compute_saturation(self, total: int) -> float:
        """Gene pool saturation: total / capacity."""
        return min(1.0, total / self.capacity)

    def evaluate(
        self,
        capsules: List[Any],
        gap_subscriptions: Optional[List["GapSubscription"]] = None,
    ) -> DiversityPressureResult:
        """Evaluate diversity pressure and return eviction candidates.

        Args:
            capsules: list of active GenePool capsules
            gap_subscriptions: current GapSubscription list (used to check underrep families)

        Returns:
            DiversityPressureResult with eviction candidates
        """
        from llm.gene_pool_io import get_gene_pool_diversity

        total = len(capsules)
        saturation = self._compute_saturation(total)

        # ── Load diversity metrics ─────────────────────────────────────────────
        diversity = get_gene_pool_diversity()
        diversity_score = diversity.get("diversity_score", 0.0)
        overrep = diversity.get("overrepresented_families", [])
        underrep = diversity.get("underrepresented_families", [])

        # ── Compute pressure level ────────────────────────────────────────────
        sat_pressure = max(0.0, (saturation - self.SATURATION_THRESHOLD) / (1.0 - self.SATURATION_THRESHOLD))
        div_pressure = max(0.0, (self.DIVERSITY_THRESHOLD - diversity_score) / self.DIVERSITY_THRESHOLD)
        pressure_level = sat_pressure * 0.5 + div_pressure * 0.5  # combined 0–1

        triggered = (
            saturation >= self.SATURATION_THRESHOLD
            and diversity_score < self.DIVERSITY_THRESHOLD
        )

        if not triggered:
            return DiversityPressureResult(
                triggered=False,
                pressure_level=round(pressure_level, 3),
                overrepresented_families=overrep,
                underrepresented_families=underrep,
                eviction_candidates=[],
                archived_capsule_ids=[],
                diversity_score=diversity_score,
                saturation=round(saturation, 3),
            )

        # ── Identify eviction candidates in over-represented families ─────────
        # Only evict if there are underrepresented families that could use the slots
        if not underrep or not overrep:
            return DiversityPressureResult(
                triggered=False,
                pressure_level=round(pressure_level, 3),
                overrepresented_families=overrep,
                underrepresented_families=underrep,
                eviction_candidates=[],
                archived_capsule_ids=[],
                diversity_score=diversity_score,
                saturation=round(saturation, 3),
            )

        eviction_candidates: List[Dict] = []

        # Group capsules by family
        family_capsules: Dict[str, List[Any]] = {}
        for cap in capsules:
            fam = self._family_of_capsule(cap)
            family_capsules.setdefault(fam, []).append(cap)

        for fam in overrep:
            fam_caps = family_capsules.get(fam, [])
            if not fam_caps:
                continue

            # Sort by outcome_success_score ascending (lowest first)
            scored = [
                (c, c.get("outcome_success_score", 0.0) if hasattr(c, "get") else getattr(c, "outcome_success_score", 0.0))
                for c in fam_caps
            ]
            scored.sort(key=lambda x: x[1])

            n_evict = max(self.MIN_CAPSULES_TO_EVICT, int(len(scored) * self.EVICTION_RATE))
            for cap, score in scored[:n_evict]:
                cid = cap.get("capsule_id", "") if hasattr(cap, "get") else getattr(cap, "capsule_id", "")
                eviction_candidates.append({
                    "capsule_id": cid,
                    "family": fam,
                    "score": round(score, 4),
                    "reason": f"diversity_pressure: {fam} is over-represented (pressure={pressure_level:.2f})",
                    "gap_type": cap.get("action_gap_type", "unknown") if hasattr(cap, "get") else getattr(cap, "action_gap_type", "unknown"),
                })

        return DiversityPressureResult(
            triggered=True,
            pressure_level=round(pressure_level, 3),
            overrepresented_families=overrep,
            underrepresented_families=underrep,
            eviction_candidates=eviction_candidates,
            archived_capsule_ids=[],
            diversity_score=diversity_score,
            saturation=round(saturation, 3),
        )

    def execute_evictions(
        self,
        result: DiversityPressureResult,
    ) -> DiversityPressureResult:
        """Actually archive the eviction candidates from the GenePool.

        Returns updated result with archived_capsule_ids filled in.
        """
        if not result.eviction_candidates:
            return result

        from llm.insight.tracker import EvolutionTracker

        tracker = EvolutionTracker()
        archived: List[str] = []

        for candidate in result.eviction_candidates:
            cid = candidate["capsule_id"]
            try:
                capsules = tracker._load_capsules()
                cap = next((c for c in capsules if c.capsule_id == cid), None)
                if cap and cap.status == "active":
                    cap.status = "archived"
                    tracker.update_capsule(cap)
                    archived.append(cid)
            except Exception:
                pass

        result.archived_capsule_ids = archived
        return result


'''

if 'class DiversityPressureEvaluator' in content:
    print('DiversityPressureEvaluator already exists')
else:
    # Add import for Optional if not present
    if 'from typing import' in content and 'Optional' not in content.split('from typing import')[1].split('\n')[0]:
        content = re.sub(
            r'(from typing import [^\n]+)',
            lambda m: m.group(1) + '\nfrom typing import Optional' if 'Optional' not in m.group(1) else m.group(1),
            content,
            count=1
        )
    content += evaluator_code
    with open('llm/gene_pool_watcher.py', 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)
    print(f'Added DiversityPressureEvaluator, file size: {len(content)}')
