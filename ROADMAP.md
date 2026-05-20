# Roadmap — v2.2

> Last updated: May 2026

## Research Plan: LaST-R1 Q2-Q10

Below is the systematic analysis of 9 open research questions derived from LaST-R1
(arXiv:2604.28192), with concrete plans for how Rairos tools can investigate each one.

### How to Run These Experiments

Each research plan follows the same pipeline:

```bash
# 1. Import seed papers
./rairos.sh import 2604.28192

# 2. Search for related work
./rairos.sh research "<topic>" --limit 10 --no-ai

# 3. Detect research gaps
./rairos.sh gap --no-llm "<topic>"

# 4. Run deep research agent (if LLM available)
./rairos.sh agent deep-research "<question>" --iterations 3

# 5. Set up subscription for continuous monitoring
./rairos.sh subscribe add "<topic>"

# 6. Start daemon for automatic evolution
./rairos.sh daemon start
```

---

### Q2: LAPO vs PPO — 收敛速度与样本效率的量化 trade-off？

**Hypothesis:** LAPO's joint optimization of latent reasoning + action space should converge faster than PPO's action-only optimization, but at higher per-step compute cost.

**Rairos pipeline:**
1. `./rairos.sh research "LAPO PPO comparison latent reasoning" --limit 15 --no-ai`
2. Import key comparison papers: PPO-based VLA methods, latent reasoning papers
3. `./rairos.sh gap extract <paper_id>` on each to extract claimed convergence rates
4. `./rairos.sh insight rate --card <id> --stars <N>` to rank findings

**Expected output:** A literature map showing convergence speed vs sample efficiency trade-off across methods, with Gene Pool capsules encoding the discovered patterns.

**Key papers to import:** LAPO paper sections, PPO-VLA baselines (e.g., RL-VLA, GR-2), latent action papers

---

### Q3: 推理链长度与任务复杂度：成正比还是饱和效应？

**Hypothesis:** Longer latent reasoning chains improve performance up to a saturation point, after which additional steps add noise without benefit. The saturation threshold depends on task complexity.

**Rairos pipeline:**
1. `./rairos.sh gap "adaptive latent chain length reasoning robotics" --no-llm`
2. Import LaST-R1's adaptive CoT mechanism paper + related work on chain length
3. Use `./rairos.sh compare` on papers that study reasoning depth vs performance
4. Track saturation thresholds as Gene Pool capsules with `outcome_success_score`

**Measurement framework:**
| Task Complexity | Optimal Chain Length | Saturation Point |
|----------------|---------------------|------------------|
| Single-step pick | ? | ? |
| Multi-stage (e.g. open drawer → pick object) | ? | ? |
| Long-horizon (e.g. make coffee) | ? | ? |

---

### Q4: Latent Reasoning 能否 Zero-Shot 泛化到未见物体类别？

**Hypothesis:** If latent reasoning learns physical dynamics rather than object-specific features, it should generalize to novel objects without fine-tuning.

**Rairos pipeline:**
1. `./rairos.sh research "zero-shot generalization robot manipulation latent reasoning" --no-ai`
2. Import LaST-R1 + all papers studying VLA generalization
3. Extract gap: `./rairos.sh gap extract <id>` on each generalization study
4. Build evidence table via `./rairos.sh insight`

**Evidence table schema:**
```
| Method | Seen Objs | Unseen Objs | Drop (%) |
|--------|-----------|-------------|----------|
| LaST-R1 | 99.8%     | ?           | ?        |
| Octo    | ?         | ?           | ?        |
| RT-2    | ?         | ?           | ?        |
```

---

### Q5: 多阶段任务中推理与动作交替频率如何最优调度？

**Hypothesis:** For multi-stage tasks, the optimal reasoning schedule is denser at stage boundaries (where planning is needed) and sparser during execution phases.

**Rairos pipeline:**
1. `./rairos.sh gap "reasoning action interleaving multi-step robot" --no-llm`
2. Import hierarchical planning + VLA papers
3. Use `./rairos.sh insight` to tag papers by their interleaving strategy
4. `./rairos.sh subscribe add "hierarchical VLA reasoning"` for continuous monitoring

---

### Q6: 视觉 vs 物理 Latent 表征的 Attention 分布差异？

**Hypothesis:** Visual latent tokens attend more to object appearance and spatial layout, while physical latent tokens attend to dynamics, forces, and temporal dependencies.

**Rairos pipeline:**
1. `./rairos.sh research "visual vs physical latent attention VLA" --no-ai`
2. Import LaST-R1 + interpretability papers
3. Extract gap: `./rairos.sh gap --no-llm "latent representation analysis VLA"`
4. Build contradiction map: `./rairos.sh gap contradictions`

---

### Q7: 热启动策略选择如何影响最终上限——质量 vs 多样性？

**Hypothesis:** A diverse warm-up policy (covering many behaviors) leads to higher final performance after RL fine-tuning than a high-quality but narrow warm-up.

**Rairos pipeline:**
1. `./rairos.sh research "warmup policy quality diversity VLA RL" --no-ai`
2. Import imitation learning + RL warm-up papers
3. Cross-reference: which warm-up strategy maps to which final performance
4. Encode as Gene Pool capsule: `trigger=(warmup_strategy) → action=(final_performance)`

---

### Q8: LIBERO 基准能否覆盖真实机器人场景？

**Hypothesis:** LIBERO's 110 tasks have systematic gaps: (a) no deformable objects, (b) no dual-arm coordination, (c) limited visual variation.

**Rairos pipeline:**
1. `./rairos.sh gap extract 2604.28192` — extract LaST-R1's own claims about LIBERO
2. `./rairos.sh research "LIBERO benchmark limitations robotics" --no-ai`
3. Build benchmark coverage matrix:
```
| Capability          | LIBERO | Real World |
|--------------------|--------|------------|
| Single-arm pick    | ✅     | ✅         |
| Deformable objects | ❌     | ✅         |
| Dual-arm           | ❌     | ✅         |
| Visual variation   | Limited| High       |
```

---

### Q9: 该框架能否迁移到非 VLA 架构（纯 Action-Conditioned）？

**Hypothesis:** LAPO's latent-to-action optimization is VLA-agnostic and can be applied to any action-conditioned policy (e.g., diffusion policies, ACT).

**Rairos pipeline:**
1. `./rairos.sh research "latent reasoning action conditioned policy robot" --no-ai`
2. Import non-VLA architectures: Diffusion Policy, ACT, π0
3. `./rairos.sh insight rate --card <id> --stars <N>` — tag by architecture type
4. `./rairos.sh gap --no-llm "LAPO transfer non-VLA"` to detect missing evidence

---

### Q10: 推理可视化后人类能否直接干预修正 Latent 路径？

**Hypothesis:** If latent reasoning is interpretable, humans can identify errors and correct latent tokens, improving policy performance without retraining.

**Rairos pipeline:**
1. `./rairos.sh gap extract <id>` on LaST-R1 + interpretability papers
2. `./rairos.sh research "human intervention latent reasoning corrigibility" --no-ai`
3. Import papers on: mechanistic interpretability, human-in-the-loop robotics
4. `./rairos.sh gap contradictions` — find papers that argue for vs against human intervention

---

## Progress Tracking

- [x] Q1: 物理 Latent Reasoning 最优表征：离散 vs 连续？ (already answered in paper)
- [x] Q2: LAPO vs PPO trade-off — **executed**. ArXiv: found LAPO paper. Gene Pool: 6 capsules, gap: "no systematic comparison study exists". Key finding: ablation with matched compute budgets is missing.
- [x] Q3: 推理链长度与任务复杂度 — **executed**. ArXiv: found LongNav-R1 (horizon-adaptive), VLA-Thinker, Re-FORC. Key insight: LaST-R1's adaptive mechanism implies saturation — if longer chains were always better, adaptation would be unnecessary. Report: docs/research_q3.md
- [ ] Q3: 推理链长度与任务复杂度 — pipeline: `./rairos.sh gap "adaptive latent chain length" --no-llm`
- [ ] Q4: Zero-shot 泛化 — pipeline: `./rairos.sh research "zero-shot VLA generalization" --no-ai`
- [ ] Q5: 推理与动作交替调度 — pipeline: `./rairos.sh subscribe add "hierarchical VLA reasoning"`
- [ ] Q6: Attention 分布差异 — pipeline: `./rairos.sh gap contradictions`
- [ ] Q7: 热启动策略 — pipeline: `./rairos.sh research "warmup VLA RL" --no-ai`
- [ ] Q8: LIBERO 覆盖分析 — pipeline: `./rairos.sh research "LIBERO limitations" --no-ai`
- [ ] Q9: 非 VLA 迁移 — pipeline: `./rairos.sh insight rate`
- [ ] Q10: 人类干预 — pipeline: `./rairos.sh gap contradictions`

## Implementation Priority

| Question | Impact | Feasibility | Priority | Dependencies |
|----------|--------|-------------|----------|-------------|
| Q2 | 🔥🔥🔥 | High | P0 | More papers on PPO-VLA |
| Q8 | 🔥🔥🔥 | High | P0 | LIBERO benchmark papers |
| Q4 | 🔥🔥 | Medium | P1 | Generalization benchmarks |
| Q9 | 🔥🔥 | Medium | P1 | Non-VLA architecture papers |
| Q3 | 🔥🔥 | Medium | P1 | Chain length ablation studies |
| Q5 | 🔥 | Low | P2 | Multi-stage task datasets |
| Q6 | 🔥 | Low | P2 | Attention analysis tools |
| Q7 | 🔥 | Low | P2 | Warm-up ablation studies |
| Q10 | 🔥 | Low | P2 | Interpretability tools |
