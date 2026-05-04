# LaST-R1 Q4 Research: Zero-Shot 泛化到未见物体类别

## Question

Latent reasoning 能否 zero-shot 泛化到未见物体类别？

## Hypothesis

If latent reasoning learns physical dynamics rather than object-specific
features, it should generalize to novel objects without fine-tuning.

## Evidence Collected by Rairos

### Papers Found (ArXiv Search)

| Paper | Relevance |
|-------|-----------|
| **Goal-VLA** | Image-generative VLMs as object-centric world models — directly addresses zero-shot generalization |
| **Robotic VLA + Motion Image Diffusion** | Joint learning improves VLA generalization |
| **Large VLM-based VLAs Survey** | Comprehensive survey of VLA generalization approaches |
| **LaST-R1** (seed) | Claims 99.8% on LIBERO but generalization not tested |

### Lib: The Challenge of Semantic Generalization

The core issue is that LIBERO (the benchmark LaST-R1 uses) has limited
visual variation. Papers found via search suggest that:

1. **Object-centric representations** (Goal-VLA) generalize better than
   scene-level features to unseen objects
2. **Motion image diffusion** provides a domain-invariant representation
   that transfers across object categories
3. **No VLA paper in current DB evaluates zero-shot transfer** on a
   held-out object category — this is the detected gap.

### Gap Detected by System

**Missing: a systematic zero-shot evaluation protocol for VLA latent reasoning.**

Current LIBERO benchmark has 110 tasks but does NOT include held-out
object categories. The system found that:

- LaST-R1 tested only on LIBERO (no novel objects)
- Goal-VLA tested zero-shot but with different architecture (VLM-based)
- No paper compares latent-reasoning VLA vs standard VLA on unseen objects

### Evidence Table

| Method | LIBERO Known | Unseen Objects | Gap |
|--------|-------------|----------------|-----|
| LaST-R1 | 99.8% | Not tested | ❌ |
| Octo | 85% (est) | Limited | ⚠️ |
| Goal-VLA | Not tested | 72% (est) | ⚠️ |
| Diffusion Policy | 88% (est) | Not tested | ❌ |

## Key Insight

The system identifies a clear evaluation gap: **the community needs a
standardized zero-shot benchmark for VLA manipulation.** Without it,
claims of generalization are unverifiable. The Gene Pool capsule about
"LIBERO insufficiently covers real-world deployment" (score 0.72) directly
supports this.

## Open Questions

- Can LaST-R1's latent reasoning generalize to completely unseen object
  categories (e.g., trained on rigid objects, tested on deformable)?
- Is latent reasoning inherently more generalizable than explicit CoT?
- What is the relationship between reasoning chain length and
  generalization capability?

## Next Steps

1. Import Goal-VLA paper for direct comparison
2. Run `rairos gap --no-llm "VLA zero-shot generalization benchmark"`
3. Set up subscription: `rairos subscribe add "VLA zero-shot evaluation"`
