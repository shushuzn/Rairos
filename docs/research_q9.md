# LaST-R1 Q9 Research: 迁移到非 VLA 架构

## Question

该框架能否迁移到非 VLA 架构（纯 action-conditioned）？

## Hypothesis

LAPO's latent-to-action optimization is VLA-agnostic and can be applied
to any action-conditioned policy (e.g., diffusion policies, ACT).

## Evidence Collected

### Key Papers

| Paper | Relevance |
|-------|-----------|
| **Diffusion Policy Policy Optimization** | Shows that policy optimization (the core of LAPO) CAN be applied to diffusion-based policies. Supports the hypothesis. |
| **Robotic VLA + Motion Image Diffusion** | Bridges VLA and diffusion approaches — shows the representations are compatible. |
| **Running VLAs at Real-time Speed** | Shows VLA architectures can be optimized for efficiency — implies architectural choices are not fundamental. |

### Analysis

The framework has two components that could transfer:

1. **Latent reasoning (CoT over physical dynamics):** Architecture-specific.
   Requires a latent space to reason in. Diffusion policies have a latent
   space (the denoising trajectory), so this could transfer. ACT
   (Action Chunking with Transformers) also has a latent transformer
   architecture that could support reasoning.

2. **LAPO (joint latent+action optimization):** Algorithm-agnostic.
   The core idea — jointly optimizing reasoning and action — can be
   applied to any differentiable policy. Diffusion Policy Policy
   Optimization shows this is feasible for diffusion models.

### Gene Pool Evidence

| Capsule | Score | Relevance |
|---------|-------|-----------|
| Diffusion vs token-based actions | 0.64 | Directly about representation choice — token-based (VLA) vs continuous (diffusion) |
| LAPO convergence vs PPO | 0.85 | Joint optimization — could apply to any architecture |

### Gap Detected

**Missing: empirical demonstration of LAPO on non-VLA architectures.**

The system found no paper that applies LAPO-style joint latent+action
optimization to diffusion policies or ACT. The theoretical basis exists
(Diffusion Policy Policy Optimization), but the specific framework has
not been transferred.

### Evidence Table

| Architecture | Latent Space | Supports LAPO? | Evidence |
|-------------|-------------|----------------|----------|
| VLA (LaST-R1) | Token latent | ✅ Native | LaST-R1 |
| Diffusion Policy | Denoising trajectory | ✅ Likely | DPPO |
| ACT | Transformer latent | ✅ Likely | Architecture similarity |
| Pure MLP policy | None | ❌ | No latent space |

## Key Insight

The answer is **likely yes, but unproven.** LAPO's core innovation (joint
optimization of reasoning and action) is algorithm-agnostic. Diffusion
Policy Policy Optimization proves that policy optimization works on
diffusion models. The missing link is implementing LAPO specifically
on a non-VLA architecture.

## Next Steps

1. Import "Diffusion Policy Policy Optimization" for direct comparison
2. The experimental test: implement LAPO-style reasoning in Diffusion Policy
3. This is a clear research opportunity: "LAPO-Diffusion: Latent Reasoning
   for Diffusion-Based Robot Policies"
