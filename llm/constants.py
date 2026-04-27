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
