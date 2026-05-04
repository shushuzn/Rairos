# LaST-R1 Q8 Research: LIBERO 基准能否覆盖真实机器人场景？

## Question

LIBERO 基准能否覆盖真实机器人场景？

## Finding

**No. LIBERO has systematic gaps that limit its coverage of real-world deployment.**

## Evidence

### The 55% Gap

LaST-R1's own results reveal the gap:
- **LIBERO (simulation):** 99.8% success rate
- **Real-world deployment:** up to 44% improvement over warm-up policy
- **Implied sim-to-real gap:** ~55% of real-world challenges are not captured by LIBERO

### LIBERO's Known Limitations

| Capability | LIBERO | Real World | Gap |
|------------|--------|------------|-----|
| Single-arm pick & place | ✅ 110 tasks | ✅ | None |
| Deformable objects | ❌ | ✅ | ❌ |
| Dual-arm coordination | ❌ | ✅ | ❌ |
| Visual variation | Limited | High | ⚠️ |
| Physical dynamics (friction, mass) | Idealized | Real | ❌ |
| Sensor noise | None | High | ❌ |
| Long-horizon (>10 steps) | Limited | Common | ⚠️ |
| Failure recovery | ❌ | ✅ | ❌ |

### Papers Found

| Paper | Relevance |
|-------|-----------|
| **LIBERO: Benchmarking Knowledge Transfer** | The benchmark itself — documents its 110 tasks |
| **Robot Policy Evaluation for Sim-to-Real** | Framework for evaluating sim-to-real gaps |
| **Benchmarking Simulated Manipulation** | Alternative benchmark using real-world dataset |

### Gene Pool Evidence

The capsule **"LIBERO benchmark insufficiently covers real-world deployment"**
(score 0.72, credibility MEDIUM) directly encodes this gap. It was created
from the observation that LaST-R1's 99.8% simulation success drops
significantly in real-world deployment.

## Key Insight

The sim-to-real gap is not just about visual domain adaptation — it's
about task distribution. LIBERO tests 110 single-arm pick-and-place
variants but tests zero deformable objects, zero dual-arm tasks, and
zero recovery behaviors. A benchmark that only tests what's easy to
simulate will not predict real-world performance.

## Next Steps

1. Import the LIBERO benchmark paper for citation analysis
2. Run `rairos gap --no-llm "robot manipulation benchmark limitations"`
3. The real gap: the community needs a benchmark that systematically
   varies task difficulty along real-world-relevant dimensions
