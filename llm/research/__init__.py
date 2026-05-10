"""LLM Research subpackage — gap analysis, hypothesis generation, paper analysis."""

from llm.research.gap_analyzer import GapAnalysisResultV2, ResearchGapV2
from llm.research.gap_detector import GapDetector, GapType, GapSeverity, ResearchGap
from llm.research.hypothesis_generator import (
    HypothesisGenerator,
    HypothesisResult,
    HypothesisType,
)
from llm.research.paper_analyzer import PaperAnalyzer, PaperAnalysisResult
from llm.research.paper_gap_extractor import (
    analyze_gap,
    analyze_embodied_planning,
    batch_analyze_embodied_planning,
    analyze_multi_paper_gaps,
    semantic_search_papers,
    gaps_to_research_questions,
)
from llm.research.research_path import (
    ResearchPathPlanner,
    ReadingPath,
    ReadingStep,
    PaperNode,
    ReadingLevel,
)
from llm.research.research_narrative_tracker import (
    ResearchNarrativeTracker,
    ResearchNarrativeService,
    ResearchThread,
    NarrativePhase,
    render_thread,
    render_dashboard,
)

__all__ = [
    # gap_analyzer
    "GapAnalysisResultV2",
    "ResearchGapV2",
    # gap_detector
    "GapDetector",
    "GapType",
    "GapSeverity",
    "ResearchGap",
    # hypothesis_generator
    "HypothesisGenerator",
    "HypothesisResult",
    "HypothesisType",
    # paper_analyzer
    "PaperAnalyzer",
    "PaperAnalysisResult",
    # paper_gap_extractor
    "analyze_gap",
    "analyze_embodied_planning",
    "batch_analyze_embodied_planning",
    "analyze_multi_paper_gaps",
    "semantic_search_papers",
    "gaps_to_research_questions",
    # research_path
    "ResearchPathPlanner",
    "ReadingPath",
    "ReadingStep",
    "PaperNode",
    "ReadingLevel",
    # research_narrative_tracker
    "ResearchNarrativeTracker",
    "ResearchNarrativeService",
    "ResearchThread",
    "NarrativePhase",
    "render_thread",
    "render_dashboard",
]
