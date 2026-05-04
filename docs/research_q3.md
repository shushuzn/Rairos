# LaST-R1 Q3 Research: 推理链长度与任务复杂度

## Question

推理链长度与任务复杂度：成正比还是饱和效应？

## Hypothesis

Longer latent reasoning chains improve performance up to a saturation point,
after which additional steps add noise without benefit. The saturation
threshold depends on task complexity.

## Evidence Collected by Rairos

### Papers Found (ArXiv Search)

| Paper | Relevance |
|-------|-----------|
| LaST-R1 (2604.28192) | Adaptive latent CoT — dynamically adjusts reasoning horizon |
| LongNav-R1 | Horizon-adaptive multi-turn RL for long-horizon VLA navigation |
| VLA-Thinker | Boosting VLA through thinking-with-image |
| Re-FORC | Adaptive reward prediction for efficient CoT reasoning |

### Gene Pool Capsules Used

| Capsule | Score | Relevance |
|---------|-------|-----------|
| LAPO convergence vs PPO | 0.85 | Joint latent + action optimization |
| RL fine-tuning sample efficiency | 0.55 | Training cost of long reasoning chains |
| Diffusion vs token-based actions | 0.64 | Representation choice affects chain length |

### Gap Analysis

The system found: **No paper in the current database systematically studies
the relationship between reasoning chain length and task complexity for
latent reasoning in VLA.** LaST-R1 claims adaptive CoT works, but does not
provide ablation data on chain length vs performance.

## Key Insight

The saturation effect hypothesis is supported by two converging signals:

1. **LaST-R1's adaptive mechanism**: If longer chains were always better,
   there would be no need for adaptation. The fact that LaST-R1 dynamically
   adjusts horizon implies diminishing returns.

2. **LongNav-R1's approach**: Horizon-adaptive RL for navigation separately
   confirms that different tasks need different reasoning horizons.

## Open Questions

- What is the optimal chain length for single-step vs multi-step tasks?
- Does the saturation point depend on the action representation
  (continuous latent vs discrete token)?
- Can we predict optimal chain length from task features alone?

## Next Steps

1. Run `rairos research "reasoning chain length VLA ablation" --no-ai`
2. Import papers that study reasoning depth vs performance
3. Build quantitative evidence table with chain length benchmarks
