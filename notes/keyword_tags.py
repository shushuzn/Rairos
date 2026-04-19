"""Keyword tag inference and predefined keyword tag patterns."""
import re
from typing import List

from core import Paper


KEYWORD_TAGS = [
    (r"\bagent(s)?\b|tool\s*use|function\s*calling", "Agent"),
    (r"\brag\b|retrieval-augmented|retrieval augmented", "RAG"),
    (r"\bmoe\b|mixture of experts", "MoE"),
    (r"\brlhf\b|preference optimization|dpo\b", "Alignment"),
    (r"\bevaluation\b|benchmark", "Evaluation"),
    (r"\bcompiler\b|kernel|cuda|inference", "Infrastructure"),
    (r"\bmultimodal\b|vision|audio", "Multimodal"),
    (r"\bcompression\b|quantization|distillation", "Optimization"),
    (r"\blong context\b|context length", "LongContext"),
    (r"\bsafety\b|jailbreak|red teaming", "Safety"),
]


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    text = f"{paper.title}\n{paper.abstract}".lower()
    out = []
    for pat, tg in KEYWORD_TAGS:
        if re.search(pat, text, flags=re.I):
            out.append(tg)
    return out if out else ["Unsorted"]
