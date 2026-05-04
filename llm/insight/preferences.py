"""Preference tracking: ExplorationAction, PreferenceTag, EvolutionEvent."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional


class ExplorationAction(Enum):
    """User actions during research exploration."""

    VIEWED = "viewed"  # 查看详情

    ACCEPTED = "accepted"  # 采纳/喜欢

    REJECTED = "rejected"  # 忽略/跳过

    EXPANDED = "expanded"  # 展开子问题

    HYPOTHESIZED = "hypothesized"  # 生成了假说

    VALIDATED = "validated"  # 验证了问题

    NARRATED = "narrated"  # 编织了故事

    INSIGHT_RATED = "insight_rated"  # 用户对 insight 打分

    # paper2code pipeline events
    IMPLEMENTATION_PASS = "implementation_pass"  # paper→code→test 全部通过
    IMPLEMENTATION_FAIL = "implementation_fail"  # paper→code→test 有失败


class PreferenceTag(Enum):
    """Research preference tags learned from behavior."""

    METHOD_FOCUSED = "method_focused"  # 方法论导向

    APPLICATION_FOCUSED = "app_focused"  # 应用导向

    THEORY_FOCUSED = "theory_focused"  # 理论导向

    HIGH_RISK_TOLERANT = "high_risk"  # 高风险容忍

    LOW_RISK_TOLERANT = "low_risk"  # 低风险容忍

    EXPLORATORY = "exploratory"  # 探索型

    CONFIRMATORY = "confirmatory"  # 验证型

    CROSS_DOMAIN = "cross_domain"  # 跨领域兴趣


@dataclass
class EvolutionEvent:
    """A single exploration event."""

    timestamp: str

    topic: str

    action: ExplorationAction

    gap_type: str = ""  # GapType enum value

    gap_title: str = ""

    gap_description: str = ""

    hypothesis_id: str = ""

    question_id: str = ""

    paper_ids: List[str] = field(default_factory=list)

    duration_seconds: int = 0  # Time spent on this item

    notes: str = ""  # User's optional notes

    insight_card_id: str = ""  # Insight card ID for insight_rated events


# ─── Gene/Capsule Storage ──────────────────────────────────────────────────────
