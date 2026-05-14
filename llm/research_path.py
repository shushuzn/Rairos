"""Re-export from llm.research.research_path for backward compatibility."""
import warnings
warnings.warn(
    "Import from llm.research_path is deprecated, use llm.research.research_path instead",
    DeprecationWarning,
    stacklevel=2,
)


from llm.research.research_path import (
    ReadingLevel,
    PaperNode,
    ReadingStep,
    ReadingPath,
    ResearchPathPlanner,
)

__all__ = [
    "ReadingLevel",
    "PaperNode",
    "ReadingStep",
    "ReadingPath",
    "ResearchPathPlanner",
]
