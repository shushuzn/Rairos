"""Auto Router — model and effort selection based on task complexity.

Inspired by DeepSeek-TUI's auto mode: a small flash routing call determines
which model and iteration count to use per query, without user specification.

Rairos uses a rule-based heuristic (no extra LLM call needed) that maps
query complexity to a recommended model tier and effort level.

Usage:
    from llm.auto_router import AutoRouter
    router = AutoRouter()
    plan = router.route("RLHF alignment theory")
    print(plan.model, plan.iterations)  # e.g. "gpt-4o-mini", 2
"""

from __future__ import annotations

import os
from dataclasses import dataclass


# Model tier definitions
MODEL_TIER = {
    "fast": "gpt-4o-mini",  # simple, fast, cheap
    "balanced": "gpt-4o",  # medium complexity
    "powerful": "claude-3-5-sonnet",  # high complexity, reasoning-heavy
    "ultra": "o1-preview",  # theoretical/mathematical proofs
}


@dataclass
class RoutePlan:
    """Output of auto-router: recommended model + iterations for a query."""

    model: str
    iterations: int  # recommended max_iterations
    effort: str  # "low" | "medium" | "high"
    complexity: int  # 1-5 scale
    reasoning: str  # one-line explanation of routing choice


class AutoRouter:
    """Route queries to appropriate model tier and effort level.

    Uses heuristic complexity scoring (no extra LLM call) to decide:
    - Model: fast → balanced → powerful → ultra
    - Iterations: 1-5 based on complexity
    - Effort: low/medium/high

    Model selection is guided by:
    - Query domain (theory → more reasoning power needed)
    - Query specificity (broad → higher iterations)
    - Cost sensitivity (env: AIROS_ROUTER_COST_SENSITIVE=1)
    """

    # Complexity thresholds for model tier selection
    CROSS_DOMAIN_KEYWORDS = [
        "comparison",
        " vs ",
        " versus ",
        "combine",
        "hybrid",
        "cross-domain",
        "transfer",
        "integration",
    ]
    THEORY_KEYWORDS = [
        "theory",
        "framework",
        "principle",
        "analysis",
        "understanding",
        "proof",
        "theorem",
        "convergence",
        "optimal",
        "guarantee",
    ]
    SPECIFIC_MODEL_KEYWORDS = [
        "transformer",
        "llm",
        "gpt",
        "bert",
        "rlhf",
        "ppo",
        "lora",
        "resnet",
        "lstm",
        "gan",
        "vae",
        "diffusion",
    ]
    PRACTICAL_KEYWORDS = [
        "implementation",
        "benchmark",
        "evaluation",
        "dataset",
        "code",
        "build",
        "run",
        "experiment",
    ]

    def __init__(self, cost_sensitive: bool = False):
        self.cost_sensitive = cost_sensitive or os.getenv("AIROS_ROUTER_COST_SENSITIVE") == "1"

    def route(self, query: str, hint_complexity: int | None = None) -> RoutePlan:
        """Determine model tier and effort from query.

        Args:
            query: Research topic or question
            hint_complexity: Optional pre-computed complexity (1-5) to use directly

        Returns:
            RoutePlan with recommended model, iterations, effort, complexity
        """
        complexity = hint_complexity if hint_complexity is not None else self._score(query)

        # Map complexity → model tier
        if complexity <= 1:
            model, effort = "fast", "low"
        elif complexity == 2:
            model, effort = "fast", "medium"
        elif complexity == 3:
            model, effort = "balanced", "medium"
        elif complexity == 4:
            model, effort = "powerful", "high"
        else:
            model, effort = "ultra", "high"

        # In cost-sensitive mode, downgrade by one tier
        if self.cost_sensitive and model != "fast":
            tier_order = ["fast", "balanced", "powerful", "ultra"]
            idx = tier_order.index(model) if model in tier_order else 1
            model = tier_order[max(0, idx - 1)]
            effort = "medium"

        # Iterations: 1 for trivial, scale up with complexity
        iterations = min(max(complexity, 1), 5)

        # Override: very specific known topics get fewer iterations
        q = query.lower()
        if any(k in q for k in self.SPECIFIC_MODEL_KEYWORDS):
            iterations = min(iterations, 3)

        return RoutePlan(
            model=MODEL_TIER.get(model, "gpt-4o-mini"),
            iterations=iterations,
            effort=effort,
            complexity=complexity,
            reasoning=self._explain(model, complexity, effort),
        )

    def _score(self, query: str) -> int:
        """Score query complexity 1-5."""
        q = query.lower()
        score = 1

        if any(k in q for k in self.CROSS_DOMAIN_KEYWORDS):
            score += 1
        if any(k in q for k in self.THEORY_KEYWORDS):
            score += 1
        if any(k in q for k in self.PRACTICAL_KEYWORDS):
            score = max(1, score - 1)

        # Well-scoped specific techniques get capped
        if any(k in q for k in self.SPECIFIC_MODEL_KEYWORDS):
            score = min(score, 2)

        return max(1, min(score, 5))

    def _explain(self, model: str, complexity: int, effort: str) -> str:
        return f"complexity={complexity}, tier={model}, effort={effort}"


# ─── Convenience CLI ───────────────────────────────────────────────────────────


def main():
    """Test auto-router from CLI."""
    router = AutoRouter()
    test_queries = [
        "RLHF alignment in large language models",
        "Comparison of transformer vs LSTM for time series",
        "Theory of convergence in gradient descent",
        "Implement a ResNet on CIFAR-10",
        "Understanding attention mechanism in transformers",
    ]
    print("Auto Router Test:\n")
    for q in test_queries:
        plan = router.route(q)
        print(f"  [{plan.complexity}] {plan.model:30s} iter={plan.iterations}  {q}")


if __name__ == "__main__":
    main()
