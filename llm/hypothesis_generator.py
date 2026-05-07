"""Re-export from llm.research.hypothesis_generator for backward compatibility."""

from llm.research.hypothesis_generator import (
    HypothesisType,
    RiskLevel,
    ExperimentDesign,
    DifferentiationPoint,
    RiskAssessment,
    ResearchHypothesis,
    HypothesisResult,
    HypothesisGenerator,
    _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT,
    _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE,
)

__all__ = [
    "HypothesisType",
    "RiskLevel",
    "ExperimentDesign",
    "DifferentiationPoint",
    "RiskAssessment",
    "ResearchHypothesis",
    "HypothesisResult",
    "HypothesisGenerator",
    "_HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT",
    "_HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE",
]
