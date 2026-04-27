"""
Shared constants across the llm module.

集中管理所有模块级常量，避免重复定义和漂移。
"""

from __future__ import annotations

import os

# ── LLM defaults (used by all modules that call the LLM API) ─────────────────

LLM_BASE_URL = os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1")
LLM_MODEL = os.getenv("DEFAULT_LLM_MODEL", "gpt-4o-mini")

# ── AI/ML research keyword tracking ───────────────────────────────────────────

# Supersets of terms used for keyword extraction across trend_analyzer,
# research_session, and question_validator. Each consumer selects a subset
# appropriate to its needs.
AI_RESEARCH_KEYWORDS: set[str] = {
    # Core NLP/LLM
    "transformer", "attention", "bert", "gpt", "llm", "language model",
    "neural", "network", "embedding", "fine-tuning", "rlhf", "rag",
    "retrieval", "generative", "diffusion", "gan", "clip", "vit",
    # RL
    "reinforcement", "policy", "reward", "rl", "dpo", "ppo", "reward model",
    # Training
    "training", "optimization", "pre-training", "instruction", "alignment",
    # Multimodal
    "multimodal", "vision", "language", "speech", "audio",
    # Reasoning
    "constitutional", "reasoning", "chain-of-thought", "cot", "synthetic data",
    # Generic
    "model", "learning",
}

# ── Shared baseline keywords for SmartFollowUp topic classification ───────────
# All follow-up types share this baseline set of AI/ML terms.
# Individual types extend it with domain-specific keywords.
SMART_FOLLOWUP_BASE: set[str] = {
    # Core NLP/LLM
    "attention", "transformer", "bert", "gpt", "llm", "language model",
    "neural", "network", "embedding", "fine-tuning", "rlhf", "rag",
    "retrieval", "generative", "diffusion", "gan", "clip", "vit",
    "weight", "layer", "parameter", "gradient", "loss", "optimize",
    "softmax", "matrix", "dot", "product", "mechanism",
    # RL
    "reinforcement", "policy", "reward", "rl", "dpo", "ppo",
    # Training
    "training", "pre-training", "instruction", "alignment",
    # Multimodal
    "multimodal", "vision", "language", "speech", "audio",
    # Reasoning
    "constitutional", "reasoning", "chain-of-thought", "cot",
    # Implementation
    "implement", "code", "function", "class", "api", "library",
    "pytorch", "tensorflow", "module", "algorithm",
    # Comparison
    "vs", "versus", "better", "worse", "compare", "advantage", "disadvantage",
    # Evolution
    "based on", "follow", "extend", "improve", "build upon",
    "later", "previous", "next", "evolution", "derived", "succeed",
    # Application
    "apply", "use", "application", "industry", "practical",
    "deploy", "production", "real-world", "benchmark",
}
