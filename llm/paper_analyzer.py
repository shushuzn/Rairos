"""Re-export from llm.research.paper_analyzer for backward compatibility."""
import warnings
warnings.warn(
    f"Import from llm.paper_analyzer is deprecated, use llm.research.paper_analyzer instead",
    DeprecationWarning,
    stacklevel=2,
)


from llm.research.paper_analyzer import (
    PaperAnalysisResult,
    CitationClaim,
    PaperAnalyzer,
)

__all__ = [
    "PaperAnalysisResult",
    "CitationClaim",
    "PaperAnalyzer",
]
