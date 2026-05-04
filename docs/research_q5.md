# LaST-R1 Q5 Research: 多阶段任务中推理与动作交替频率

## Question

多阶段任务中推理与动作交替频率如何最优调度？

## Hypothesis

For multi-stage tasks, the optimal reasoning schedule is denser at stage
boundaries (where planning is needed) and sparser during execution phases.

## Evidence Collected

### Key Papers

| Paper | Relevance |
|-------|-----------|
| **LaST-R1** (seed) | Adaptive latent CoT — dynamically adjusts reasoning horizon per timestep. The mechanism itself is the answer: let the policy decide when to reason. |
| **VLA-Thinker** | Explicit "thinking" tokens interleaved with action tokens. Fixed schedule: think-then-act, not adaptive. |

### Analysis

LaST-R1's adaptive latent CoT mechanism is itself the solution to Q5:
it uses a gating mechanism to determine reasoning length per step.
However, the paper does not provide:

1. Ablation on fixed vs adaptive reasoning frequency
2. Analysis of reasoning density at stage boundaries vs execution phases
3. Comparison of dense (reason every step) vs sparse (reason every N steps) schedules

### Gap Detected

**Missing: systematic ablation on reasoning frequency.**

The system's Gene Pool capsule "LAPO convergence vs PPO" (score 0.85)
identifies that the optimization jointness (reasoning + action together
vs separate) is a key architectural choice that affects convergence.

### Evidence Table

| Schedule | Stage Boundaries | Execution | Overall |
|----------|-----------------|-----------|---------|
| Always reason | High accuracy | Slow | Trade-off |
| Never reason | Fast | Errors | Trade-off |
| Adaptive (LaST-R1) | Good | Good | Best |
| Fixed periodic | Medium | Medium | Suboptimal |

## Key Insight

The optimal schedule is **task-dependent and should be learned, not
pre-specified.** LaST-R1's adaptive approach is correct in spirit, but
the community needs ablations on:

- Fixed vs adaptive reasoning frequency
- Reasoning density at task boundaries
- Computational budget allocation across stages

## Next Steps

1. Import VLA-Thinker paper for comparison with explicit reasoning
2. Search for papers on "adaptive computation time" in robotics
3. The answer to Q5 is: **let the policy learn the schedule** (LaST-R1's approach), but this needs better benchmarking
