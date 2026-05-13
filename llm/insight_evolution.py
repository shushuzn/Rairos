"""llm.insight_evolution — backwards-compatible shim.

All types have been moved to llm.insight sub-package:
- llm.insight.gene        -> CapsuleGene
- llm.insight.preferences -> ExplorationAction, PreferenceTag, EvolutionEvent
- llm.insight.profile     -> UserPreferenceProfile, GapExplorationState
- llm.insight.tracker    -> EvolutionTracker, get_evolution_tracker

This module re-exports everything for backwards compatibility.
"""

import warnings
warnings.warn(
    "Import from llm.insight_evolution is deprecated, use llm.insight instead",
    DeprecationWarning,
    stacklevel=2,
)

from llm.insight import (
    CapsuleGene,
    ExplorationAction,
    PreferenceTag,
    EvolutionEvent,
    UserPreferenceProfile,
    GapExplorationState,
    EvolutionTracker,
    get_evolution_tracker,
)

__all__ = [
    "CapsuleGene",
    "ExplorationAction",
    "PreferenceTag",
    "EvolutionEvent",
    "UserPreferenceProfile",
    "GapExplorationState",
    "EvolutionTracker",
    "get_evolution_tracker",
]
