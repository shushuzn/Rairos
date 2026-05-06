"""Re-export from llm.research.gap_detector for backward compatibility."""
from llm.research.gap_detector import (
    GapType,
    GapSeverity,
    ResearchGap,
    ResearchQuestion,
    GapAnalysisResult,
    GapDetector,
    _GAP_DETECTION_SYSTEM_PROMPT,
    _GAP_DETECTION_USER_PROMPT_TEMPLATE,
    _QUESTION_GENERATION_SYSTEM_PROMPT,
    _QUESTION_GENERATION_USER_PROMPT_TEMPLATE,
    _GAP_TYPE_PATTERNS,
    _GAP_QUESTION_TEMPLATES,
)

__all__ = [
    "GapType",
    "GapSeverity",
    "ResearchGap",
    "ResearchQuestion",
    "GapAnalysisResult",
    "GapDetector",
    "_GAP_DETECTION_SYSTEM_PROMPT",
    "_GAP_DETECTION_USER_PROMPT_TEMPLATE",
    "_QUESTION_GENERATION_SYSTEM_PROMPT",
    "_QUESTION_GENERATION_USER_PROMPT_TEMPLATE",
    "_GAP_TYPE_PATTERNS",
    "_GAP_QUESTION_TEMPLATES",
]
