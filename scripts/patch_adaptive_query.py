"""Patch deep_research.py: add adaptive query strategy."""
import re

with open('research_loop/deep_research.py', 'r', encoding='utf-8') as f:
    content = f.read()

# Check if already patched
if '_AdaptiveQueryStrategy' in content:
    print('Already has _AdaptiveQueryStrategy')
    exit(0)

# ── 1. Add _AdaptiveQueryStrategy class before DeepResearchAgent ──────────────
adaptive_class = '''

class _AdaptiveQueryStrategy:
    """Adaptive query planning: evolve search strategy based on gap coverage.

    Tracks gap_type coverage across iterations and adjusts query to explore
    under-represented gap types. Also tracks query→gaps success rate to
    weight future query construction.
    """

    ALL_GAP_TYPES = frozenset([
        "capability", "improvement", "contradiction",
        "assumption", "extension", "baseline_gap",
        "evaluation_gap", "reproducibility_gap", "embodied_planning",
        "rl_pretraining", "scaling_laws", "reasoning",
    ])

    def __init__(self, topic: str):
        self.topic = topic
        # query → list of gap_type found
        self._query_gap_types: dict[str, list[str]] = {}
        # gap_type → how many times it appeared
        self._gap_type_counts: dict[str, int] = {}
        self._total_gaps = 0

    def record_search_result(self, query: str, gaps: list) -> None:
        """Record gap types found from a search result."""
        if not gaps:
            return
        found_types = set()
        for g in gaps:
            gt = g.gap_type if isinstance(g.gap_type, str) else str(getattr(g.gap_type, 'value', g.gap_type))
            found_types.add(gt)
            self._gap_type_counts[gt] = self._gap_type_counts.get(gt, 0) + 1
            self._total_gaps += 1
        self._query_gap_types[query] = list(found_types)

    def gap_type_coverage(self) -> dict[str, float]:
        """Return coverage ratio for each gap type (0.0–1.0)."""
        if self._total_gaps == 0:
            return {gt: 0.0 for gt in self.ALL_GAP_TYPES}
        return {
            gt: self._gap_type_counts.get(gt, 0) / self._total_gaps
            for gt in self.ALL_GAP_TYPES
        }

    def under_represented_types(self, threshold: float = 0.15) -> list[str]:
        """Return gap types that appear in < threshold of all gaps."""
        coverage = self.gap_type_coverage()
        return [gt for gt, ratio in coverage.items() if ratio > 0 and ratio < threshold]

    def most_productive_queries(self, top_k: int = 3) -> list[str]:
        """Return queries that produced the most diverse gap types."""
        scored = []
        for q, types in self._query_gap_types.items():
            scored.append((q, len(set(types))))
        scored.sort(key=lambda x: x[1], reverse=True)
        return [q for q, _ in scored[:top_k]]

    def build_adaptive_query(
        self,
        iteration: int,
        latest_gap_title: str = "",
        latest_gap_type: str = "",
        gene_pool_hint: str = "",
        confidence: float = 0.0,
    ) -> str:
        """Build next search query adaptively.

        Strategy:
        - iter 0: use topic directly
        - under-represented gap types exist: target them explicitly
        - high confidence GenePool hint: blend with gap context
        - otherwise: expand topic with gap type direction
        """
        under_rep = self.under_represented_types()

        if iteration == 0:
            return self.topic

        # Case 1: have under-represented types → target them
        if under_rep:
            target = under_rep[0]
            # Pick a productive query from history and extend with target type
            productive = self.most_productive_queries(1)
            if productive:
                base = productive[0]
            else:
                base = self.topic
            return f"{base} {target}"

        # Case 2: high-confidence GenePool hint
        if gene_pool_hint and confidence >= 0.4:
            if latest_gap_title:
                return f"{gene_pool_hint} {latest_gap_title}"
            return gene_pool_hint

        # Case 3: latest gap context
        if latest_gap_type == "Contradiction":
            return f"{self.topic} {latest_gap_title} disagreement"
        elif latest_gap_type == "improvement":
            return f"{self.topic} {latest_gap_title} improvement"
        elif latest_gap_title:
            return f"{self.topic} {latest_gap_title}"

        return self.topic

    def query_similarity(self, q1: str, q2: str) -> float:
        """Simple word-overlap similarity between two queries (0.0–1.0)."""
        words1 = set(q1.lower().split())
        words2 = set(q2.lower().split())
        if not words1 or not words2:
            return 0.0
        intersection = words1 & words2
        union = words1 | words2
        return len(intersection) / len(union) if union else 0.0


'''

old_agent = '\nclass DeepResearchAgent:'
if old_agent in content:
    content = content.replace(old_agent, adaptive_class + old_agent, 1)
    print('Added _AdaptiveQueryStrategy class')
else:
    print('WARNING: DeepResearchAgent marker not found')
    exit(1)

# ── 2. Add _adaptive_strategy to __init__ ────────────────────────────────────
old_init = '        self.tracker = get_evolution_tracker()'
new_init = '''        self.tracker = get_evolution_tracker()
        self._adaptive_strategy = _AdaptiveQueryStrategy(topic=query)'''
if new_init not in content:
    content = content.replace(old_init, new_init, 1)
    print('Added _adaptive_strategy to __init__')
else:
    print('_adaptive_strategy already in __init__')

# ── 3. Record search results in _adaptive_strategy after gap analysis ────────
# Find where gaps are analyzed and record in adaptive strategy
# This should happen after gap_analyzer.run() returns gaps
# The best place is after _analyze_gaps() call in the main loop
# Look for: "gap_snapshots = self._analyze_gaps"
old_analyze = '            gap_snapshots = self._analyze_gaps(snapshots, iteration)\n            self._progress["gaps_found"] += len(gap_snapshots)\n            self._record_thought(\n                "analyzer",\n                f"Found {len(gap_snapshots)} gaps in {len(snapshots)} papers",\n                iteration,\n            )\n\n            # ── Encode accepted gaps into Gene Pool'
new_analyze = '''            gap_snapshots = self._analyze_gaps(snapshots, iteration)
            self._progress["gaps_found"] += len(gap_snapshots)
            self._record_thought(
                "analyzer",
                f"Found {len(gap_snapshots)} gaps in {len(snapshots)} papers",
                iteration,
            )

            # ── Adaptive query strategy: record gap types ─────────────────────
            self._adaptive_strategy.record_search_result(search_query, gap_snapshots)

            # ── Encode accepted gaps into Gene Pool'''
if 'Adaptive query strategy' not in content:
    content = content.replace(old_analyze, new_analyze, 1)
    print('Added adaptive strategy recording')
else:
    print('Adaptive strategy recording already present')

# ── 4. Modify _plan_next_search to use _adaptive_strategy ───────────────────
old_plan = '''    def _plan_next_search(self, iteration: int) -> str:
        """PLANNER: decide next search query based on session state + GenePool history."""

        gaps = self.session.gaps if self.session else []

        search_history = self.session.search_history if self.session else []

        if iteration == 0:
            planned = self.query

        elif gaps:
            latest_gap = gaps[-1] if gaps else None

            if latest_gap:
                # Ask GenePool for successful search strategies on this gap type/topic

                hint, confidence = self._get_search_guidance(
                    topic=self.query,
                    gap_type=latest_gap.gap_type,
                    gap_title=latest_gap.title,
                )

                if hint and confidence >= 0.3:
                    # GenePool has a successful pattern — incorporate it

                    planned = f"{hint} {latest_gap.title}"

                    self._record_thought(
                        "planner",
                        f"GenePool-guided search (confidence={confidence:.2f}): {planned}",
                        iteration,
                    )

                elif latest_gap.gap_type == "Contradiction":
                    planned = f"{self.query} {latest_gap.title} disagreement"

                else:
                    planned = f"{self.query} {latest_gap.title} improvement"

            else:
                planned = self.query

        else:
            planned = self.query

        # Avoid duplicate searches

        if planned in search_history:
            planned = f"{self.query} {iteration}"

        self._record_thought("planner", f"Planned search: {planned}", iteration)

        return planned'''

new_plan = '''    def _plan_next_search(self, iteration: int) -> str:
        """PLANNER: decide next search query using adaptive strategy + GenePool."""

        gaps = self.session.gaps if self.session else []
        search_history = self.session.search_history if self.session else []

        if iteration == 0:
            planned = self.query
        elif gaps:
            latest_gap = gaps[-1]

            # Get GenePool guidance
            hint, confidence = self._get_search_guidance(
                topic=self.query,
                gap_type=latest_gap.gap_type if latest_gap else "",
                gap_title=latest_gap.title if latest_gap else "",
            )

            # Use adaptive strategy to build query
            planned = self._adaptive_strategy.build_adaptive_query(
                iteration=iteration,
                latest_gap_title=latest_gap.title if latest_gap else "",
                latest_gap_type=latest_gap.gap_type if latest_gap else "",
                gene_pool_hint=hint or "",
                confidence=confidence,
            )

            # Semantic deduplication: avoid near-duplicate queries
            for prev_q in search_history:
                sim = self._adaptive_strategy.query_similarity(planned, prev_q)
                if sim > 0.75:
                    planned = f"{planned} variant{iteration}"
                    break

            # Log decision
            if hint and confidence >= 0.3:
                self._record_thought(
                    "planner",
                    f"GenePool-guided (conf={confidence:.2f}): {planned}",
                    iteration,
                )
            else:
                coverage = self._adaptive_strategy.gap_type_coverage()
                under_rep = self._adaptive_strategy.under_represented_types()
                self._record_thought(
                    "planner",
                    f"Adaptive search: {planned} | coverage={coverage} under_rep={under_rep[:2]}",
                    iteration,
                )
        else:
            planned = self.query

        # Fallback duplicate guard
        if planned in search_history:
            planned = f"{self.query} exploration{iteration}"

        self._record_thought("planner", f"Planned search: {planned}", iteration)
        return planned'''

if '_AdaptiveQueryStrategy' in content and 'build_adaptive_query' not in content:
    content = content.replace(old_plan, new_plan, 1)
    print('Patched _plan_next_search')
else:
    print('WARNING: plan already patched or strategy missing')

with open('research_loop/deep_research.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'File size: {len(content)}')
