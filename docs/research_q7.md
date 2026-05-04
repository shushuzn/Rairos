# LaST-R1 Q7 Research: 热启动策略——质量 vs 多样性

## Question

热启动策略选择如何影响最终上限——质量 vs 多样性？

## Hypothesis

A diverse warm-up policy (covering many behaviors) leads to higher final
performance after RL fine-tuning than a high-quality but narrow warm-up.

## Evidence Collected

### Key Papers

| Paper | Relevance |
|-------|-----------|
| **Refined Policy Distillation: VLA Generalists to RL Experts** | Directly on point: studies whether starting from a generalist VLA (diverse, broad) leads to better RL fine-tuning results than a specialist policy (high-quality, narrow). |
| **Quality Diversity without Evolution** | Fundamental study: quality diversity algorithms maintain both quality and diversity. Relevant framework for understanding the trade-off. |
| **LongNav-R1** | Horizon-adaptive RL for VLA navigation — uses diverse navigation experiences as warm-up. |
| **LaST-R1** (seed) | Uses one-shot supervised warm-up (high-quality, narrow: single demonstration per task). Tests the "quality" side of the hypothesis. |

### Analysis

LaST-R1 uses **one-shot supervised warm-up** — a single demonstration per
task. This is the extreme "quality over diversity" approach. The paper
then fine-tunes with LAPO (RL) and achieves 99.8% on LIBERO.

The key question: would a more diverse warm-up (multiple demonstrations,
behavior cloning from diverse sources) lead to even better final
performance after RL fine-tuning?

**Refined Policy Distillation** suggests yes: generalist VLA policies
provide a better foundation for RL fine-tuning than specialist policies.
But the trade-off is computational cost — diverse warm-up requires more
pre-training data.

### Gene Pool Evidence

| Capsule | Score | Relevance |
|---------|-------|-----------|
| Generalist vs specialist policies | 0.48 | Directly encodes the quality-diversity trade-off |
| LAPO convergence vs PPO | 0.85 | Joint optimization quality depends on warm-up quality |
| RL fine-tuning sample efficiency | 0.55 | Warm-up quality affects how much RL is needed |

### Gap Detected

**Missing: controlled ablation of warm-up diversity vs quality in VLA.**

No paper in the current database directly compares:
- Single-demonstration warm-up (LaST-R1 approach)
- Multi-demonstration warm-up (behavior cloning)
- Diverse task warm-up (generalist pre-training)
- Random exploration warm-up

...all with the same downstream RL fine-tuning pipeline.

### Evidence Table

| Warm-up Strategy | Diversity | Quality | Final Performance | Source |
|-----------------|-----------|---------|------------------|--------|
| One-shot (LaST-R1) | Low | High | 99.8% LIBERO | LaST-R1 |
| Generalist pre-train | High | Medium | Not compared | Refined Policy Distillation |
| Behavior cloning (many demos) | Medium | High | Not compared | — |
| Random exploration | High | Low | Not compared | — |

## Key Insight

The answer to Q7 is: **we don't know, and neither does the literature.**
LaST-R1 chose the "high quality, low diversity" path (one-shot warm-up)
and got excellent results. But a systematic ablation comparing warm-up
strategies with controlled downstream RL compute is missing.

The Gene Pool capsule "Generalist policies underperform specialist
policies" (score 0.48) encodes the tension: in practice, specialists
often outperform generalists on specific tasks, but generalists provide
a better foundation for adaptation.

## Open Questions

- Would a diverse warm-up + LAPO outperform one-shot warm-up + LAPO?
- Is the one-shot warm-up sufficient because LAPO's latent reasoning
  compensates for lack of diversity?
- What is the compute-optimal warm-up strategy for a given task budget?

## Next Steps

1. Import "Refined Policy Distillation" paper for direct comparison
2. Add capsule: "warm-up diversity vs quality trade-off in VLA"
3. The field needs this ablation study — it's a clear research opportunity
