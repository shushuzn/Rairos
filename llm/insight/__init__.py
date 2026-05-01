"""llm.insight — Insight Evolution sub-package.

Re-exports all public types from the split modules.
Backwards-compatible with the original llm.insight_evolution module.
"""

from llm.insight.gene import CapsuleGene
from llm.insight.preferences import ExplorationAction, PreferenceTag, EvolutionEvent
from llm.insight.profile import UserPreferenceProfile, GapExplorationState
from llm.insight.tracker import EvolutionTracker, get_evolution_tracker

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
