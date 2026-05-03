# Roadmap

> Where is AI Research OS going?

**Last updated:** May 2026
**Maintainer:** @shushuzn

---

## v2.2 — Self-Evolution (Current)

Goal: Make the "self-evolving" part **real and visible**.

- [x] **Gene/Capsule system** — evolution mechanism live
  - [x] Gene Pool dual-store (gene_pool.jsonl + capsules.json)
  - [x] CapsuleGene lifecycle: active → consumed/archived
  - [x] consumed 闭环 (source_cap_id on suggestions)
  - [x] Capsule merge (Jaccard ≥ 0.80)
  - [x] Auto-archive (low_score_streak ≥ 3)
  - [ ] Visual dashboard showing how the system learns
  - [ ] Evolution log: what the system learned this week

- [ ] **Research gap detection** — surface what's missing
  - [x] Gap extraction from papers (LLM-based, paper_gap_extractor.py)
  - [ ] Generate research questions from gaps
  - [ ] Trend forecasting: where is the field going?

  **LaST-R1 Research Questions (arXiv:2604.28192)** — VLA + RL latent reasoning:
  - [ ] Q1: 物理 Latent Reasoning 最优表征：离散 vs 连续？ (gap_type: embodied_planning)
  - [ ] Q2: LAPO vs PPO：收敛速度与样本效率的量化 trade-off？ (gap_type: rl_efficiency)
  - [ ] Q3: 推理链长度与任务复杂度：成正比还是饱和效应？ (gap_type: reasoning_scaling)
  - [ ] Q4: Latent reasoning 能否 zero-shot 泛化到未见物体类别？ (gap_type: sim_to_real)
  - [ ] Q5: 多阶段任务中推理与动作交替频率如何最优调度？ (gap_type: planning_control)
  - [ ] Q6: 视觉 vs 物理 latent 表征的 attention 分布差异？ (gap_type: representation_learning)
  - [ ] Q7: 热启动策略选择如何影响最终上限——质量 vs 多样性？ (gap_type: rl_pretraining)
  - [ ] Q8: LIBERO 基准能否覆盖真实机器人场景？ (gap_type: benchmark_coverage)
  - [ ] Q9: 该框架能否迁移到非 VLA 架构（纯 action-conditioned）？ (gap_type: architecture_agnostic)
  - [ ] Q10: 推理可视化后人类能否直接干预修正 latent 路径？ (gap_type: human_ai_collaboration)

- [ ] **Weekly research digest**
  - Auto-generated summary of new papers
  - Changes in research landscape
  - Personalized recommendations

---

*This roadmap is a living document. Priorities shift based on user feedback and contributions.*
