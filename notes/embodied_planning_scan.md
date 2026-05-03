# Embodied Planning Papers Scan — cs.CL/cs.CV/cs.RO

## Ingested Papers

### 1. Lifting Embodied World Models for Planning and Control
- **arXiv ID**: 2604.26182
- **Authors**: Alex N. Wang, Trevor Darrell, Pavel Izmailov, Yutong Bai, Amir Bar
- **Filed**: cs.CV (2026-04-28)
- **Why Relevant**: Directly addresses embodied world models — the core of embodied planning. Proposes lifting 2D visual representations into 3D spatial representations for planning and control of physical agents. Most directly relevant to embodied planning topic.

### 2. Learning Long-Context Diffusion Policies via Past-Token Prediction
- **arXiv ID**: 2505.09561v2
- **Authors**: Marcel Torne, Andy Tang, Yuejiang Liu, Chelsea Finn
- **Filed**: cs.RO (2025-05-14)
- **Why Relevant**: Combines long-context reasoning with diffusion policies for robot control — targets multi-step manipulation tasks in long-horizon settings. Relevant to embodied planning as it addresses temporal planning for physical agents.

### 3. Robo360: A 3D Omnispective Multi-Material Robotic Manipulation Dataset
- **arXiv ID**: 2312.06686
- **Authors**: Litian Liang, Liuyu Bian, Caiwei Xiao, et al.
- **Filed**: cs.CV (2023-12-09)
- **Why Relevant**: Large-scale robotic manipulation dataset covering diverse materials — foundational data infrastructure for embodied planning and agent training. Provides the 3D perception substrate needed for physical world modeling.

## Key Findings — Embodied Planning Landscape

1. **World Models are the bottleneck**: Current research converges on the idea that planning requires lifted 3D world representations — raw 2D vision insufficient for downstream control decisions.

2. **Long-context + physical reasoning gap**: Diffusion policies are strong at motion-level control but struggle with long-horizon task planning. Long-context LLMs provide reasoning but lack grounded physical understanding.

3. **3D-centric perception is resurgent**: Robo360-style omnispective 3D datasets are emerging as the preferred substrate for training embodied agents, replacing 2D RGB-only approaches.

4. **LLM-as-planner vs learned planners**: Two dominant camps — (a) LLMs for high-level task decomposition, (b) end-to-end learned world models for low-level control. No strong unified approach yet.

## Recommendations for Gene Pool Additions

| Gene | Rationale |
|------|-----------|
| **3D World Model Lifting** — propose lifting 2D→3D representations as a core capability for embodied planning systems; captures the key architectural insight from Wang et al. | Embodied planning requires spatial reasoning beyond text; 3D world models are the missing substrate |
| **Hierarchical Planning with LLM High-Level Decomposition** — decompose tasks with LLMs, execute with learned low-level policies | Most promising approach — LLM handles task graph, diffusion/RL handles motion control |
| **Omnispective 3D Perception Fusion** — fuse multi-view RGB+D for material-aware manipulation understanding | Robo360's approach is the emerging standard for embodied agent training data |

## Search Notes

- cs.CL search yielded few embodied planning papers directly; the relevant papers appear primarily under **cs.CV** and **cs.RO**
- The arXiv CLI search for "embodied planning" and "robotics LLM planning" returned zero results; broader queries were needed
- All 3 selected papers ingested successfully via `python -m cli ingest {arxiv_id} --skip-embed`
