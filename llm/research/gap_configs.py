"""GAP ANALYZER CONFIGS — Q1-Q10预配置模板."""

from typing import Dict

GAP_ANALYZER_CONFIGS: Dict[str, dict] = {
    "embodied_planning": {
        "gap_type": "embodied_planning",
        "result_fields": ["representation_type", "confidence", "evidence", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in robotics and embodied AI.
Given a paper's title and abstract, determine how the paper represents physical reasoning (latent reasoning over physical dynamics).

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Answer these questions about the paper's latent representation approach:

1. Does the paper use DISCRETE latent representations?
   (e.g., discrete tokens, symbolic, quantized, language-like discrete states)
   Look for: "discrete", "symbolic", "token", "quantized", "categorical", "language"

2. Does the paper use CONTINUOUS latent representations?
   (e.g., continuous vectors, real-valued embeddings, diffusion, Gaussian, continuous distributions)
   Look for: "continuous", "diffusion", "Gaussian", "real-valued", "embedding", "vector"

3. Is it HYBRID (both)?
   (e.g., discrete reasoning + continuous execution, world model tokens + action distributions)

4. What is the KEY EVIDENCE from the abstract?

5. What OPEN QUESTION remains about this representation choice?

Provide your analysis in this format:
representation_type: [discrete|continuous|hybrid]
confidence: [0.0-1.0]
evidence: [key phrases from abstract]
gap_title: [specific research question about this representation choice]
summary: [2-sentence analysis]""",
        "keywords": ["embodied", "latent", "reasoning", "VLA", "robotics"],
    },
    "rl_efficiency": {
        "gap_type": "rl_efficiency",
        "result_fields": [
            "algorithm",
            "convergence_speed",
            "sample_efficiency",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in reinforcement learning.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the RL algorithm and its efficiency characteristics:

1. Which RL algorithm does the paper focus on?
   (e.g., PPO, LAPO, SAC, TD3, DDPG, Q-learning variants)

2. How fast does it converge compared to baselines?
   Look for: "converges in X steps", "sample efficiency", "faster than", "reduced samples"

3. What is the sample efficiency ranking?

4. What OPEN QUESTION remains about efficiency tradeoffs?

Provide your analysis in this format:
algorithm: [algorithm name]
convergence_speed: [fast|medium|slow|unknown]
sample_efficiency: [high|medium|low|unknown]
gap_title: [specific research question about RL efficiency]
summary: [2-sentence analysis]""",
        "keywords": ["RL", "reinforcement learning", "efficiency", "convergence", "PPO", "LAPO"],
    },
    "reasoning_scaling": {
        "gap_type": "reasoning_scaling",
        "result_fields": [
            "chain_length",
            "task_complexity",
            "scaling_behavior",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in reasoning systems.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's approach to reasoning chain length and task complexity:

1. What is the inference chain length discussed?
   Look for: "chain-of-thought", "reasoning steps", "N steps", "depth"

2. How does chain length scale with task complexity?

3. What is the relationship between reasoning depth and performance?

4. What OPEN QUESTION remains about scaling reasoning?

Provide your analysis in this format:
chain_length: [short|medium|long|variable|unknown]
task_complexity: [simple|moderate|high|unknown]
scaling_behavior: [linear|sublinear|superlinear|decreasing|unknown]
gap_title: [specific research question about reasoning scaling]
summary: [2-sentence analysis]""",
        "keywords": ["reasoning", "chain-of-thought", "scaling", "inference", "complexity"],
    },
    "sim_to_real": {
        "gap_type": "sim_to_real",
        "result_fields": [
            "generalization_level",
            "domain_gap",
            "transfer_quality",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in sim-to-real transfer.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's sim-to-real generalization capability:

1. Does the paper achieve zero-shot generalization?
   Look for: "zero-shot", "domain randomization", "unseen", "out-of-distribution"

2. How large is the domain gap between simulation and real?

3. What is the quality of transfer?

4. What OPEN QUESTION remains about generalization bounds?

Provide your analysis in this format:
generalization_level: [zero-shot|few-shot|full-transfer|none|unknown]
domain_gap: [small|medium|large|unknown]
transfer_quality: [high|medium|low|unknown]
gap_title: [specific research question about sim-to-real]
summary: [2-sentence analysis]""",
        "keywords": [
            "sim-to-real",
            "zero-shot",
            "generalization",
            "domain randomization",
            "transfer",
        ],
    },
    "planning_control": {
        "gap_type": "planning_control",
        "result_fields": [
            "alternation_freq",
            "planning_depth",
            "control_type",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in planning and control systems.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's planning and control architecture:

1. How often does reasoning alternate with action execution?
   Look for: "replan", "online planning", "action frequency", "control loop"

2. What is the planning depth (how many steps ahead)?

3. Is it hierarchical or flat control?

4. What OPEN QUESTION remains about planning frequency?

Provide your analysis in this format:
alternation_freq: [high|medium|low|adaptive|unknown]
planning_depth: [shallow|medium|deep|variable|unknown]
control_type: [hierarchical|flat|hybrid|unknown]
gap_title: [specific research question about planning/control]
summary: [2-sentence analysis]""",
        "keywords": ["planning", "control", "replanning", "hierarchical", "action"],
    },
    "representation_learning": {
        "gap_type": "representation_learning",
        "result_fields": [
            "attention_type",
            "modality_focus",
            "latent_structure",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in representation learning.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's representation learning approach:

1. Where is attention focused — visual features or physical dynamics?
   Look for: "visual", "physical", "latent", "attention", "feature"

2. What modality does the latent representation encode?

3. How is the latent space structured?

4. What OPEN QUESTION remains about representation quality?

Provide your analysis in this format:
attention_type: [visual|physical|both|unknown]
modality_focus: [vision|physics|multimodal|unknown]
latent_structure: [discrete|continuous|structured|unknown]
gap_title: [specific research question about representation learning]
summary: [2-sentence analysis]""",
        "keywords": ["representation", "attention", "visual", "latent", "features"],
    },
    "rl_pretraining": {
        "gap_type": "rl_pretraining",
        "result_fields": [
            "pretrain_strategy",
            "quality_diversity",
            "transfer_gain",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in RL pretraining.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's RL pretraining approach:

1. What is the pretraining strategy?
   Look for: "pretrained", "warm-start", "imitation learning", "offline", "online"

2. Does it prioritize quality or diversity of pretraining data?

3. How much does pretraining help downstream tasks?

4. What OPEN QUESTION remains about pretraining tradeoffs?

Provide your analysis in this format:
pretrain_strategy: [imitation|offline|online|multi-task|mixed|unknown]
quality_diversity: [quality-focused|diversity-focused|balanced|unknown]
transfer_gain: [high|medium|low|none|unknown]
gap_title: [specific research question about RL pretraining]
summary: [2-sentence analysis]""",
        "keywords": ["pretraining", "warm-start", "imitation learning", "offline RL", "transfer"],
    },
    "benchmark_coverage": {
        "gap_type": "benchmark_coverage",
        "result_fields": [
            "benchmark_used",
            "real_robot_eval",
            "coverage_gap",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in robot learning benchmarks.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's evaluation approach:

1. Which benchmark does the paper use?
   Look for: "LIBERO", "RLBench", "MetaWorld", "real robot", "simulation benchmark"

2. Does it evaluate on real robots or only simulation?

3. What aspects of real-world deployment are NOT covered?

4. What OPEN QUESTION remains about benchmark validity?

Provide your analysis in this format:
benchmark_used: [LIBERO|RLBench|MetaWorld|other|none|unknown]
real_robot_eval: [yes|no|partial|unknown]
coverage_gap: [small|medium|large|unknown]
gap_title: [specific research question about benchmark coverage]
summary: [2-sentence analysis]""",
        "keywords": ["benchmark", "LIBERO", "RLBench", "evaluation", "real robot"],
    },
    "architecture_agnostic": {
        "gap_type": "architecture_agnostic",
        "result_fields": [
            "architecture_type",
            "transfer_scope",
            "model_agnostic",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in robot learning architectures.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's architecture and transfer properties:

1. What architecture does the paper use?
   Look for: "VLA", "CNN", "Transformer", "diffusion", "GPT", "language model"

2. How well does it transfer to different architectures?

3. Is the approach architecture-agnostic?

4. What OPEN QUESTION remains about architecture transfer?

Provide your analysis in this format:
architecture_type: [VLA|Transformer|CNN|diffusion|hybrid|other|unknown]
transfer_scope: [narrow|medium|broad|architecture-agnostic|unknown]
model_agnostic: [yes|partial|no|unknown]
gap_title: [specific research question about architecture transfer]
summary: [2-sentence analysis]""",
        "keywords": ["VLA", "architecture", "transfer", "Transformer", "CNN", "diffusion"],
    },
    "human_ai_collaboration": {
        "gap_type": "human_ai_collaboration",
        "result_fields": [
            "intervention_type",
            "latent_correction",
            "collaboration_mode",
            "gap_title",
            "summary",
        ],
        "prompt_template": """You are a research analyst specializing in human-AI collaboration.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's human-AI collaboration approach:

1. How does human intervention occur?
   Look for: "human in the loop", "intervention", "correction", "feedback", "teleoperation"

2. Does human correction modify latent representations or only actions?

3. What is the collaboration mode?

4. What OPEN QUESTION remains about human-AI teaming?

Provide your analysis in this format:
intervention_type: [latent|action|both|unknown]
latent_correction: [yes|no|partial|unknown]
collaboration_mode: [teleop|correction|feedback|shared-control|unknown]
gap_title: [specific research question about human-AI collaboration]
summary: [2-sentence analysis]""",
        "keywords": ["human-AI", "collaboration", "intervention", "teleoperation", "correction"],
    },
}
