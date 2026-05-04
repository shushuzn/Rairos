"""UserPreferenceProfile and GapExplorationState."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional


@dataclass
class UserPreferenceProfile:
    """Learned user research preferences."""

    total_sessions: int = 0

    total_events: int = 0

    # Action counts

    views: int = 0

    accepts: int = 0

    rejects: int = 0

    expands: int = 0

    hypothesizes: int = 0

    # Gap type preferences (which types user engages with)

    gap_type_preferences: Dict[str, float] = field(default_factory=dict)

    # Keyword preferences (learned from gap titles user engages with)

    keyword_preferences: Dict[str, float] = field(default_factory=dict)

    # Topics explored

    topics_explored: List[str] = field(default_factory=list)

    topic_frequency: Dict[str, int] = field(default_factory=dict)

    # Preference tags (computed) — tag name -> confidence [0, 1]

    # Confidence > 0.6 = stable preference, 0.3-0.6 = emerging, < 0.3 = tentative

    preference_tags: Dict[str, float] = field(default_factory=dict)

    # Recent topics (last 10)

    recent_topics: List[str] = field(default_factory=list)

    # Last updated

    last_updated: str = ""


@dataclass
class GapExplorationState:
    """Current state of a gap exploration session."""

    topic: str

    session_id: str

    started_at: str

    events: List[EvolutionEvent] = field(default_factory=list)

    gaps_explored: List[str] = field(default_factory=list)  # gap titles

    gaps_accepted: List[str] = field(default_factory=list)  # accepted gap titles

    gaps_rejected: List[str] = field(default_factory=list)  # rejected gap titles

    hypotheses_generated: int = 0
