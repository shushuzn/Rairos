# LaST-R1 Q6 Research: 视觉 vs 物理 Latent 表征的 Attention 分布差异

## Question

视觉 vs 物理 latent 表征的 attention 分布差异？

## Hypothesis

Visual latent tokens attend more to object appearance and spatial layout,
while physical latent tokens attend to dynamics, forces, and temporal
dependencies.

## Evidence Collected

### Key Papers

| Paper | Relevance |
|-------|-----------|
| **When Vision Overrides Language** | Directly studies modality conflicts in VLA — finds vision dominates language in certain conditions. Framework applicable to visual vs physical latent analysis. |
| **VLA-Thinker** | Thinking-with-image — adds explicit reasoning tokens. The attention patterns between visual tokens and thinking tokens are analyzable. |
| **Robotic VLA + Motion Image Diffusion** | Joint visual-motion representation. Shows motion (physical) and appearance (visual) can be learned jointly. |
| **Running VLAs at Real-time Speed** | Efficiency analysis reveals that visual encoders dominate inference time — suggestive of attention imbalance. |

### Analysis

The system's Gene Pool provides relevant context:

- **Capsule: "Diffusion vs token-based action representations"** (score 0.64)
  — directly about representation choice. Token-based representations blend
  visual and physical information, while diffusion-based actions operate in
  a continuous space that may separate them.

- **Capsule: "LAPO convergence vs PPO"** (score 0.85) — joint optimization
  of latent reasoning + action. If visual and physical latents have
  different attention patterns, joint optimization may be harder than
  separate optimization.

### Gap Detected

**Missing: attention analysis of latent reasoning in VLA models.**

No paper in the current database specifically analyzes:

1. What visual features receive attention during latent reasoning steps
2. Whether physical dynamics tokens have distinct attention patterns
3. How the ratio of visual-to-physical attention changes with task type

### Evidence Table

| Representation | Attends To | Evidence |
|---------------|-----------|----------|
| Visual tokens | Object appearance, spatial layout | When Vision Overrides Language |
| Physical/dynamics | Motion, forces, temporal | Motion Image Diffusion |
| Latent reasoning (LaST-R1) | Unknown (not analyzed) | ❌ Gap |
| Explicit CoT (VLA-Thinker) | Task planning, object relations | Inference from architecture |

## Key Insight

The community lacks **interpretability tools for latent-space reasoning in
VLA.** While "When Vision Overrides Language" provides a framework for
understanding modality interactions, it studies explicit language-vision
conflicts, not latent-space dynamics. The gap is methodological: we need
probing techniques for continuous latent representations in robot policies.

## Open Questions

- Are visual and physical latents in LaST-R1's shared latent space or
  separated? The paper does not specify.
- Does LAPO's joint optimization encourage entanglement of visual and
  physical features?
- Can attention steering improve generalization (e.g., force more
  attention to physical dynamics during reasoning steps)?

## Next Steps

1. Import "When Vision Overrides Language" for interpretability framework
2. Run contradiction analysis: `rairos gap contradictions`
3. The answer to Q6 is: **we don't know yet** — the field needs
   latent-space probing methods for VLA models
