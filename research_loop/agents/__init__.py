"""Research Squad — multi-agent coordination layer."""

from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus, MessageBus
from research_loop.agents.scout import ScoutAgent
from research_loop.agents.analyzer import AnalyzerAgent
from research_loop.agents.citation_hunter import CitationHunterAgent
from research_loop.agents.curator import CuratorAgent
from research_loop.agents.squad import SquadCoordinator

__all__ = [
    "BaseAgent",
    "AgentMessage",
    "AgentStatus",
    "MessageBus",
    "ScoutAgent",
    "AnalyzerAgent",
    "CitationHunterAgent",
    "CuratorAgent",
    "SquadCoordinator",
]
