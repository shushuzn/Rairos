"""Re-export from llm.research.gap_analyzer for backward compatibility."""

from llm.research.gap_analyzer import (
    ResearchGapV2,
    GapAnalysisResultV2,
    GapAnalyzerV2,
    render_gap_report,
    render_combined_report,
    _GAP_TYPE_NAMES,
)
from llm.gap_detector import GapType

__all__ = [
    "ResearchGapV2",
    "GapAnalysisResultV2",
    "GapAnalyzerV2",
    "render_gap_report",
    "render_combined_report",
    "_GAP_TYPE_NAMES",
    "GapType",
]
