"""Re-export from llm.research.hypothesis_generator for backward compatibility."""
import warnings
warnings.warn(
    f"Import from llm.hypothesis_generator is deprecated, use llm.research.hypothesis_generator instead",
    DeprecationWarning,
    stacklevel=2,
)


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
