"""
CapsuleGene Crossover — genetic algorithm on Gene Pool archetypes.

闭环:
  Gene Pool (active capsules) → selection → crossover → V3 capsule
  → encode to Gene Pool → benchmark_runner scores it → impact decay evaluates fitness

Algorithm:
  1. Selection: top-k capsules by fitness = success_score × log(feedback_count+1)
  2. Crossover: single-point swap of archetype dict between two parent capsules
  3. Mutation: random perturbation of keywords, algorithm_fingerprint, paper_section_refs
  4. V3 capsule: parent_a_id + parent_b_id in archetype, evolved_generation = max(parents)+1
  5. Only V3 if: both parents credibility_badge != "low" AND fitness > threshold

MCP tool 'crossover':
  evolve    — run one crossover generation (N pairs → N V3 capsules)
  rank_v3   — list all V3 capsules in Gene Pool
  mutate    — apply random mutation to one capsule
  best      — top crossover candidates (highest fitness parents)
"""

from __future__ import annotations

import copy
import random
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import cast, Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"


# ─── Configuration ─────────────────────────────────────────────────────────────

DEFAULT_POPULATION_SIZE = 20  # top-k capsules considered for crossover
DEFAULT_OFFSPRING_COUNT = 5  # how many V3 capsules to create per evolve call
MIN_FITNESS_THRESHOLD = 0.3  # minimum fitness to be a crossover parent
MUTATION_RATE = 0.15  # per-field mutation probability


# ─── Fitness & Selection ──────────────────────────────────────────────────────


def compute_fitness(capsule: Any) -> float:
    """Fitness = success_score × log(feedback_count+1)."""
    import math

    score = capsule.outcome_success_score
    fb = capsule.feedback_count
    return score * math.log(fb + 1)  # type: ignore[no-any-return]


def compute_trust(capsule: Any, inbound_citations: int = 0) -> float:
    """CapsuleTrust = impact × citation_boost × badge_multiplier."""
    from llm.gene_pool_decay import get_capsule_trust, compute_impact_score

    impact, _ = compute_impact_score(
        success_score=capsule.outcome_success_score,
        created_at=capsule.created_at,
        feedback_count=capsule.feedback_count,
        inbound_citations=inbound_citations,
    )
    badge = getattr(capsule, "credibility_badge", "medium")
    return get_capsule_trust(impact, inbound_citations, badge)


def select_parents(
    capsules: List[Any],
    k: int = DEFAULT_POPULATION_SIZE,
    use_trust: bool = False,
) -> List[Any]:
    """Select top-k capsules by fitness, filtered by credibility and min fitness.

    If use_trust=True, sort by CapsuleTrust instead of raw fitness.
    CapsuleTrust = impact × citation_boost × badge_multiplier.
    """
    candidates = [
        c
        for c in capsules
        if c.status == "active"
        and c.credibility_badge != "low"
        and compute_fitness(c) >= MIN_FITNESS_THRESHOLD
    ]
    if use_trust:
        candidates.sort(key=lambda c: compute_trust(c), reverse=True)
    else:
        candidates.sort(key=compute_fitness, reverse=True)
    return candidates[:k]


# ─── Crossover ────────────────────────────────────────────────────────────────


def crossover(parent_a: Any, parent_b: Any) -> Dict[str, Any]:
    """Single-point archetype crossover between two capsules.

    Returns a dict with 'archetype', 'parent_a_id', 'parent_b_id', 'parent_generations'.
    Does NOT create a CapsuleGene — caller decides whether to encode it.
    """
    arch_a = copy.deepcopy(parent_a.archetype) or {}
    arch_b = copy.deepcopy(parent_b.archetype) or {}

    # Single-point crossover: operate only on shared keys
    shared_keys = [k for k in arch_a if k in arch_b]
    private_a = {k: arch_a[k] for k in arch_a if k not in arch_b}
    private_b = {k: arch_b[k] for k in arch_b if k not in arch_a}

    if not shared_keys:
        # No shared keys: shallow merge
        merged = {**arch_a, **arch_b}
    else:
        # Pick crossover point in shared keys (need at least 2 to swap)
        point = random.randint(1, max(1, len(shared_keys) - 1))
        swapped_keys = set(shared_keys[:point])
        merged = {}

        for k in shared_keys:
            merged[k] = arch_a[k] if k not in swapped_keys else arch_b[k]

        merged.update(private_a)
        merged.update(private_b)

    return {
        "archetype": merged,
        "parent_a_id": parent_a.capsule_id,
        "parent_b_id": parent_b.capsule_id,
        "parent_generations": max(parent_a.evolved_generation, parent_b.evolved_generation) + 1,
        "parent_fitness_a": compute_fitness(parent_a),
        "parent_fitness_b": compute_fitness(parent_b),
    }


def mutate_archetype(archetype: Dict[str, Any]) -> Dict[str, Any]:
    """Random mutation of archetype fields.

    Mutation types:
      - trigger_keywords: shuffle + inject noise
      - algorithm_fingerprint: regenerate (random suffix)
      - paper_section_refs: randomly drop 20%
      - gap_type: no change (too structural)
    """
    import random as _random

    arch = copy.deepcopy(archetype)

    # Mutate trigger_keywords
    if "trigger_keywords" in arch and isinstance(arch["trigger_keywords"], list):
        kw = arch["trigger_keywords"]
        if _random.random() < MUTATION_RATE and kw:
            # Drop 20% of keywords
            kept = kw[: int(len(kw) * 0.8)]
            arch["trigger_keywords"] = kept

    # Mutate algorithm_fingerprint
    if _random.random() < MUTATION_RATE and "algorithm_fingerprint" in arch:
        fp = arch["algorithm_fingerprint"]
        if isinstance(fp, str) and len(fp) > 4:
            # Flip 2 random chars in the fingerprint
            chars = list(fp)
            for _ in range(2):
                if chars:
                    idx = _random.randint(0, len(chars) - 1)
                    chars[idx] = _random.choice("0123456789abcdef")
            arch["algorithm_fingerprint"] = "".join(chars)

    # Mutate paper_section_refs
    if _random.random() < MUTATION_RATE and "paper_section_refs" in arch:
        refs = arch["paper_section_refs"]
        if isinstance(refs, list) and refs:
            # Drop 20% of refs
            arch["paper_section_refs"] = refs[: int(len(refs) * 0.8)]

    # Mutate title_embedding (if present)
    if _random.random() < MUTATION_RATE and "title_embedding" in arch:
        emb = arch["title_embedding"]
        if isinstance(emb, list) and emb:
            # Small noise injection
            noise_idx = _random.randint(0, len(emb) - 1)
            emb[noise_idx] += (_random.random() - 0.5) * 0.1
            arch["title_embedding"] = emb

    return arch


# ─── V3 Encoding ─────────────────────────────────────────────────────────────


def encode_v3_capsule(
    crossover_result: Dict[str, Any],
    gap_title: str,
    gap_type: str,
    trigger_topic: str,
    source_paper_ids: Optional[List[str]] = None,
) -> Optional[str]:
    """Encode a V3 capsule directly into DB + JSONL, bypassing encode_capsule."""
    import json
    from llm.insight.gene import CapsuleGene
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    archetype = _sanitize_archetype(crossover_result["archetype"])
    archetype["parent_capsule_id"] = crossover_result["parent_a_id"]
    archetype["parent_capsule_id_b"] = crossover_result["parent_b_id"]
    archetype["crossover_generation"] = crossover_result["parent_generations"]
    if source_paper_ids:
        archetype["source_paper_ids"] = source_paper_ids

    capsule_id = uuid.uuid4().hex[:12]
    cap = CapsuleGene(
        capsule_id=capsule_id,
        created_at=datetime.utcnow().isoformat(),
        trigger_topic=trigger_topic or "crossover",
        trigger_gap_type=gap_type or "method_gap",
        trigger_keywords=[],
        action_gap_type=gap_type or "method_gap",
        action_gap_title=gap_title,
        outcome_success_score=0.5,
        feedback_count=1,
        evolved_generation=crossover_result["parent_generations"],
        archetype=archetype,
        status="active",
    )

    # Write to JSONL (safe, JSON-serializable only)
    try:
        jsonl_path = tracker.data_dir / "gene_pool.jsonl"
        with open(jsonl_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(cap.to_dict(), ensure_ascii=False) + "\n")
    except Exception:
        pass

    # Write to SQLite
    try:
        from llm.insight.storage import _capsule_to_row

        conn = tracker._ensure_db()
        row = _capsule_to_row(cap)
        conn.execute(
            """INSERT OR REPLACE INTO capsules (
                capsule_id, created_at, trigger_topic, trigger_gap_type,
                trigger_keywords, action_gap_type, action_gap_title,
                outcome_success_score, feedback_count, evolved_generation,
                archetype, status, low_score_streak,
                credibility_score, trendslop, trendslop_reason,
                credibility_badge, source_arxiv_category, title_embedding
            ) VALUES (
                :capsule_id, :created_at, :trigger_topic, :trigger_gap_type,
                :trigger_keywords, :action_gap_type, :action_gap_title,
                :outcome_success_score, :feedback_count, :evolved_generation,
                :archetype, :status, :low_score_streak,
                :credibility_score, :trendslop, :trendslop_reason,
                :credibility_badge, :source_arxiv_category, :title_embedding
            )""",
            row,
        )
        conn.commit()
        return capsule_id
    except Exception as e:
        import logging

        logging.getLogger("crossover").warning(f"DB insert failed: {e}")
        return capsule_id  # Still return ID since JSONL succeeded


def _sanitize_archetype(archetype: Dict[str, Any]) -> Dict[str, Any]:
    """Remove non-serializable fields from archetype before DB/JSON storage."""
    import json

    cleaned = {}
    for k, v in archetype.items():
        if k == "title_embedding":
            continue
        try:
            json.dumps(v)
            cleaned[k] = v
        except (TypeError, ValueError):
            pass
    return cleaned


# ─── Evolution run ─────────────────────────────────────────────────────────────


def run_evolution(
    offspring_count: int = DEFAULT_OFFSPRING_COUNT,
    population_size: int = DEFAULT_POPULATION_SIZE,
) -> Dict[str, Any]:
    """Run one crossover generation.

    Selects top capsules, performs crossover + mutation, encodes V3 capsules.
    Returns summary of what was created.
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    parents = select_parents(capsules, k=population_size, use_trust=True)

    if len(parents) < 2:
        return {
            "error": f"Need at least 2 eligible parents, got {len(parents)}",
            "created": [],
        }

    # Ensure we always pair distinct parents
    created: List[Dict[str, Any]] = []
    used_pairs: set = set()

    for _ in range(offspring_count):
        # Pick two distinct random parents weighted by fitness
        p_a, p_b = random.sample(parents, 2)
        pair_key = tuple(sorted([p_a.capsule_id, p_b.capsule_id]))
        if pair_key in used_pairs and len(parents) >= 3:
            continue  # avoid duplicate pairs
        used_pairs.add(pair_key)

        # Crossover
        xo = crossover(p_a, p_b)

        # Mutation
        xo["archetype"] = mutate_archetype(xo["archetype"])

        # Generate V3 title
        title_a = p_a.action_gap_title[:30]
        title_b = p_b.action_gap_title[:30]
        v3_title = f"V3:{title_a} × {title_b}"

        # Encode
        capsule_id = encode_v3_capsule(
            crossover_result=xo,
            gap_title=v3_title,
            gap_type=p_a.action_gap_type or p_b.action_gap_type,
            trigger_topic=p_a.trigger_topic or p_b.trigger_topic,
            source_paper_ids=[p_a.trigger_topic, p_b.trigger_topic],
        )

        created.append(
            {
                "capsule_id": capsule_id,
                "parent_a_id": xo["parent_a_id"],
                "parent_b_id": xo["parent_b_id"],
                "generation": xo["parent_generations"],
                "fitness_a": round(xo["parent_fitness_a"], 3),
                "fitness_b": round(xo["parent_fitness_b"], 3),
            }
        )

    return {
        "parents_considered": len(parents),
        "pairs_tried": len(used_pairs),
        "created": created,
        "generation": max((c["generation"] for c in created), default=0),
    }


def get_v3_capsules() -> List[Dict[str, Any]]:
    """Return all V3 capsules (evolved_generation >= 1) from Gene Pool."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    v3 = [c for c in capsules if c.evolved_generation >= 1 and c.status == "active"]
    v3.sort(key=lambda c: compute_fitness(c), reverse=True)
    return [
        {
            "capsule_id": c.capsule_id,
            "action_gap_title": c.action_gap_title,
            "evolved_generation": c.evolved_generation,
            "success_score": c.outcome_success_score,
            "feedback_count": c.feedback_count,
            "fitness": round(compute_fitness(c), 3),
            "parent_ids": [
                c.archetype.get("parent_capsule_id", ""),
                c.archetype.get("parent_capsule_id_b", ""),
            ],
            "created_at": c.created_at,
        }
        for c in v3
    ]


def get_top_candidates(
    limit: int = 10,
) -> List[Dict[str, Any]]:
    """Return top crossover candidates (highest CapsuleTrust active capsules)."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    active = [c for c in capsules if c.status == "active" and c.credibility_badge != "low"]
    active.sort(key=lambda c: compute_trust(c), reverse=True)
    return [
        {
            "capsule_id": c.capsule_id,
            "action_gap_title": c.action_gap_title[:60],
            "evolved_generation": c.evolved_generation,
            "success_score": round(c.outcome_success_score, 3),
            "feedback_count": c.feedback_count,
            "fitness": round(compute_fitness(c), 3),
            "capsule_trust": round(compute_trust(c), 4),
            "credibility_badge": c.credibility_badge,
        }
        for c in active[:limit]
    ]


def mutate_single(capsule_id: str) -> Optional[str]:
    """Apply mutation to a single capsule's archetype in-place."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    cap = next((c for c in capsules if c.capsule_id == capsule_id), None)
    if not cap:
        return None

    original_arch = copy.deepcopy(cap.archetype) or {}
    mutated_arch = mutate_archetype(original_arch)
    cap.archetype = mutated_arch
    cap.evolved_generation = max(cap.evolved_generation, 0) + 1
    tracker.update_capsule(cap)
    return cap.capsule_id


# ─── Benchmark → V3 score update ───────────────────────────────────────────────


def update_v3_scores_from_benchmark(
    arxiv_id: str,
    pass_rate: float,
    coverage_ratio: float = 0.0,
) -> int:
    """Update success_score for all V3 capsules that cite this arXiv paper.

    When a paper is benchmarked, any V3 capsule whose source_paper_ids
    include this arXiv ID gets its success_score updated to reflect the
    real benchmark performance. This closes the feedback loop:
    benchmark → V3 capsule fitness update.

    Returns the number of V3 capsules updated.
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()

    # Combined score mirrors leaderboard: pass_rate × 0.7 + coverage × 0.3
    combined = round(pass_rate * 0.7 + coverage_ratio * 0.3, 4)

    updated = 0
    for cap in capsules:
        if cap.evolved_generation < 1:
            continue
        if cap.status != "active":
            continue
        source_ids = cap.archetype.get("source_paper_ids", []) if cap.archetype else []
        if arxiv_id not in source_ids:
            continue

        # Update: weighted blend with previous score (60% new, 40% existing)
        new_fitness = combined
        old_fitness = cap.outcome_success_score
        cap.outcome_success_score = round(old_fitness * 0.4 + new_fitness * 0.6, 4)
        cap.feedback_count += 1
        tracker.update_capsule(cap)
        updated += 1

    return updated


# ─── Debate Protocol ─────────────────────────────────────────────────────────────


DEBATE_STATE_FILE = GP_DIR / "debate_state.json"


@dataclass
class DebateEntry:
    """Record of a single debate between two capsules on the same gap_type."""

    debate_id: str
    capsule_a_id: str
    capsule_b_id: str
    gap_type: str
    winner_id: str  # capsule_id of the winner
    loser_id: str  # capsule_id of the loser
    score_a: float
    score_b: float
    judged_at: str  # ISO timestamp


def _load_debate_state() -> List[DebateEntry]:
    if not DEBATE_STATE_FILE.exists():
        return []
    try:
        import json

        data = json.loads(DEBATE_STATE_FILE.read_text(encoding="utf-8"))
        return [DebateEntry(**d) for d in data]
    except Exception:
        return []


def _save_debate_state(debates: List[DebateEntry]) -> None:
    import json

    GP_DIR.mkdir(parents=True, exist_ok=True)
    DEBATE_STATE_FILE.write_text(
        json.dumps([deb.__dict__ for deb in debates], indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def _now_iso() -> str:
    from datetime import datetime

    return datetime.utcnow().isoformat()


def _score_argument(capsule: Any, inbound_citations: int = 0) -> float:
    """Score a capsule's argument strength for debate adjudication.

    Composite: success_score × capsule_trust × log(feedback_count + 1)
    Higher = stronger argument, more credible, better cited.
    """
    import math

    success = capsule.outcome_success_score
    fb = capsule.feedback_count
    fb_bonus = math.log(fb + 1)
    # Citation bonus
    citation_bonus = 1.0 + 0.05 * inbound_citations
    return round(success * fb_bonus * citation_bonus, 4)  # type: ignore[no-any-return]


def debate_capsules(capsule_a_id: str, capsule_b_id: str, gap_type: str) -> Optional[DebateEntry]:
    """Run a debate between two capsules on the same gap_type.

    Returns DebateEntry with winner/loser, or None if either capsule not found.
    The loser is marked as 'challenged' status in the Gene Pool.
    """
    import uuid

    from llm.insight.tracker import EvolutionTracker
    from llm.gene_pool_decay import get_inbound_citations

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    capsule_map = {c.capsule_id: c for c in capsules}

    cap_a = capsule_map.get(capsule_a_id)
    cap_b = capsule_map.get(capsule_b_id)
    if not cap_a or not cap_b:
        return None

    inbound_a = get_inbound_citations(cap_a.trigger_topic)
    inbound_b = get_inbound_citations(cap_b.trigger_topic)

    score_a = _score_argument(cap_a, inbound_a)
    score_b = _score_argument(cap_b, inbound_b)

    if score_a >= score_b:
        winner_id, loser_id = capsule_a_id, capsule_b_id
        loser_cap = cap_b
    else:
        winner_id, loser_id = capsule_b_id, capsule_a_id
        loser_cap = cap_a

    # Mark loser as challenged (best-effort — DB schema may not have all columns)
    try:
        loser_cap.status = "challenged"
        loser_cap.low_score_streak = max(loser_cap.low_score_streak, 2)
        tracker.update_capsule(loser_cap)
    except Exception:
        pass  # non-critical: debate record is the primary output

    entry = DebateEntry(
        debate_id=uuid.uuid4().hex[:8],
        capsule_a_id=capsule_a_id,
        capsule_b_id=capsule_b_id,
        gap_type=gap_type,
        winner_id=winner_id,
        loser_id=loser_id,
        score_a=score_a,
        score_b=score_b,
        judged_at=_now_iso(),
    )

    debates = _load_debate_state()
    debates.append(entry)
    _save_debate_state(debates)

    return entry


def get_debate_history(limit: int = 20) -> List[Dict[str, Any]]:
    """Return recent debate results."""
    debates = _load_debate_state()
    debates.sort(key=lambda d: d.judged_at, reverse=True)
    return [
        {
            "debate_id": d.debate_id,
            "capsule_a_id": d.capsule_a_id,
            "capsule_b_id": d.capsule_b_id,
            "gap_type": d.gap_type,
            "winner_id": d.winner_id,
            "loser_id": d.loser_id,
            "score_a": d.score_a,
            "score_b": d.score_b,
            "judged_at": d.judged_at,
        }
        for d in debates[:limit]
    ]


def get_debate_candidates(gap_type: str, limit: int = 5) -> List[Dict[str, Any]]:
    """Return top capsule candidates eligible for debate on a gap_type."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()

    candidates = [
        c
        for c in capsules
        if c.status == "active" and c.action_gap_type == gap_type and c.credibility_badge != "low"
    ]
    candidates.sort(key=lambda c: c.outcome_success_score, reverse=True)
    return [
        {
            "capsule_id": c.capsule_id,
            "action_gap_title": c.action_gap_title[:60],
            "success_score": c.outcome_success_score,
            "feedback_count": c.feedback_count,
        }
        for c in candidates[:limit]
    ]


def crossover_action(
    action: str = "evolve",
    offspring_count: int = DEFAULT_OFFSPRING_COUNT,
    capsule_id: Optional[str] = None,
    capsule_id_b: Optional[str] = None,
    gap_type: Optional[str] = None,
) -> dict:
    """MCP tool dispatcher for CapsuleGene Crossover.

    Actions:
      evolve      — run one crossover generation, create V3 capsules
      rank_v3    — list all V3 capsules sorted by fitness
      mutate     — mutate a single capsule's archetype
      best       — top crossover candidates by fitness
      lineage    — render capsule lineage tree
      descendants — find V3 capsules citing this as ancestor
      debate     — run debate between two capsules on same gap_type
      debate_history — list past debate results
      debate_candidates — list capsules eligible for debate on a gap_type
    """
    if action == "evolve":
        result = run_evolution(offspring_count=offspring_count)
        return result

    elif action == "rank_v3":
        v3 = get_v3_capsules()
        return {
            "v3_capsules": v3,
            "total_v3": len(v3),
        }

    elif action == "mutate":
        if not capsule_id:
            return {"error": "capsule_id required for mutate action"}
        mutate_result: dict[str, Any] = (
            cast(dict[str, Any], mutate_single(capsule_id))
            if isinstance(mutate_single(capsule_id), dict)
            else {}
        )
        if mutate_result:
            return {"mutated": mutate_result}
        return {"error": f"Capsule {capsule_id} not found"}

    elif action == "best":
        candidates = get_top_candidates()
        return {
            "candidates": candidates,
            "total": len(candidates),
        }

    elif action == "lineage":
        if not capsule_id:
            return {"error": "capsule_id required for lineage action"}
        tree = get_lineage(capsule_id)
        if not tree:
            return {"error": f"Capsule {capsule_id} not found"}
        ascii_tree = render_lineage_tree(capsule_id)
        roots = get_root_ancestors(capsule_id)
        return {
            "capsule_id": capsule_id,
            "lineage_tree": tree,
            "ascii_tree": ascii_tree,
            "root_ancestors": roots,
        }

    elif action == "descendants":
        if not capsule_id:
            return {"error": "capsule_id required for descendants action"}
        desc = get_descendants(capsule_id)
        return {
            "ancestor_id": capsule_id,
            "descendants": desc,
            "count": len(desc),
        }

    elif action == "debate":
        if not capsule_id or not capsule_id_b:
            return {"error": "capsule_id and capsule_id_b required for debate"}
        if not gap_type:
            return {"error": "gap_type required for debate"}
        entry = debate_capsules(capsule_id, capsule_id_b, gap_type)
        if not entry:
            return {"error": "One or both capsules not found"}
        return {
            "debate_id": entry.debate_id,
            "winner_id": entry.winner_id,
            "loser_id": entry.loser_id,
            "score_a": entry.score_a,
            "score_b": entry.score_b,
            "gap_type": entry.gap_type,
            "judged_at": entry.judged_at,
        }

    elif action == "debate_history":
        history = get_debate_history()
        return {
            "debates": history,
            "total": len(history),
        }

    elif action == "debate_candidates":
        if not capsule_id:
            return {"error": "gap_type required for debate_candidates"}
        candidates = get_debate_candidates(capsule_id)
        return {
            "gap_type": capsule_id,
            "candidates": candidates,
        }

    else:
        return {"error": f"Unknown action: {action}"}


# ─── Genealogy / Lineage ─────────────────────────────────────────────────────────


def get_lineage(capsule_id: str, max_depth: int = 5) -> Optional[Dict[str, Any]]:
    """Trace the full lineage tree for a capsule.

    For V3 capsules: recursively finds parent capsules (parent_capsule_id, parent_capsule_id_b).
    Returns dict with capsule data and children, or None if not found.
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()

    capsule_map = {c.capsule_id: c for c in capsules}

    def build_tree(cid: str, depth: int) -> Optional[Dict[str, Any]]:
        if depth > max_depth:
            return None
        cap = capsule_map.get(cid)
        if not cap:
            return None

        arch = cap.archetype or {}
        parent_a = arch.get("parent_capsule_id", "")
        parent_b = arch.get("parent_capsule_id_b", "")
        generation = arch.get("crossover_generation", 0)

        node = {
            "capsule_id": cid,
            "action_gap_title": cap.action_gap_title[:60] if cap.action_gap_title else "",
            "evolved_generation": generation,
            "success_score": cap.outcome_success_score,
            "feedback_count": cap.feedback_count,
            "parent_a_id": parent_a,
            "parent_b_id": parent_b,
            "children": [],
        }

        if parent_a and parent_a != cid:
            child = build_tree(parent_a, depth + 1)
            if child:
                node["children"].append(child)
        if parent_b and parent_b != cid and parent_b != parent_a:
            child = build_tree(parent_b, depth + 1)
            if child:
                node["children"].append(child)

        return node

    return build_tree(capsule_id, depth=0)


def render_lineage_tree(capsule_id: str, max_depth: int = 5) -> str:
    """Render a capsule's lineage as an ASCII tree.

    Returns a string like:
    ┌── capsule_id [V3] (score=0.85)
    │   ┌── parent_a [V2] (score=0.72)
    │   │   └── grandparent_1 [V1] (score=0.60)
    │   └── parent_b [V1] (score=0.65)
    """
    tree = get_lineage(capsule_id, max_depth=max_depth)
    if not tree:
        return f"Capsule {capsule_id} not found"

    lines: List[str] = []

    def render_node(node: Dict[str, Any], prefix: str = "", is_last: bool = True) -> None:
        gen = node.get("evolved_generation", 0)
        gen_label = f"[V{gen}]" if gen > 0 else "[V0]"
        score = node.get("success_score", 0)
        title = node.get("action_gap_title", "") or node["capsule_id"]
        short_id = node["capsule_id"][:8]

        connector = "└── " if is_last else "├── "
        lines.append(f"{prefix}{connector}{short_id} {gen_label} score={score:.3f} {title[:40]}")

        children = node.get("children", [])
        child_prefix = prefix + ("    " if is_last else "│   ")
        for i, child in enumerate(children):
            render_node(child, child_prefix, i == len(children) - 1)

    lines.append(f"Capsule {capsule_id} [V{tree.get('evolved_generation', 0)}] — Lineage Tree")
    lines.append("─" * 60)
    for i, child in enumerate(tree.get("children", [])):
        render_node(child, "", i == len(tree["children"]) - 1)

    return "\n".join(lines)


def get_root_ancestors(capsule_id: str, max_depth: int = 10) -> List[Dict[str, Any]]:
    """Find all root ancestors (V0 capsules with no parents) in a capsule's lineage."""
    lineage = get_lineage(capsule_id, max_depth=max_depth)
    if not lineage:
        return []

    roots: List[Dict[str, Any]] = []

    def collect_roots(node: Dict[str, Any]) -> None:
        children = node.get("children", [])
        if not children:
            roots.append(
                {
                    "capsule_id": node["capsule_id"],
                    "action_gap_title": node.get("action_gap_title", ""),
                    "evolved_generation": node.get("evolved_generation", 0),
                    "success_score": node.get("success_score", 0),
                }
            )
        for child in children:
            collect_roots(child)

    for child in lineage.get("children", []):
        collect_roots(child)

    return roots


def get_descendants(capsule_id: str) -> List[Dict[str, Any]]:
    """Find all V3 capsules that have this capsule as an ancestor in their lineage."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()

    descendants: List[Dict[str, Any]] = []
    for cap in capsules:
        if cap.evolved_generation < 1:
            continue
        arch = cap.archetype or {}
        parent_a = arch.get("parent_capsule_id", "")
        parent_b = arch.get("parent_capsule_id_b", "")
        if capsule_id in (parent_a, parent_b):
            descendants.append(
                {
                    "capsule_id": cap.capsule_id,
                    "action_gap_title": cap.action_gap_title[:60],
                    "evolved_generation": cap.evolved_generation,
                    "success_score": cap.outcome_success_score,
                }
            )

    return descendants
