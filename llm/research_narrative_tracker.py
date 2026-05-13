"""Re-export from llm.research.research_narrative_tracker for backward compatibility."""
import warnings
warnings.warn(
    f"Import from llm.research_narrative_tracker is deprecated, use llm.research.research_narrative_tracker instead",
    DeprecationWarning,
    stacklevel=2,
)


from llm.research.research_narrative_tracker import (
    ResearchThread,
    NarrativePhase,
    ResearchNarrativeTracker,
    ResearchNarrativeService,
    _score_bar,
    _compute_verdict,
    render_thread,
    render_dashboard,
)

__all__ = [
    "ResearchThread",
    "NarrativePhase",
    "ResearchNarrativeTracker",
    "ResearchNarrativeService",
    "_score_bar",
    "_compute_verdict",
    "render_thread",
    "render_dashboard",
]
