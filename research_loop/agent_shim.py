"""Agent shim — wraps PyResearchAgent for orchestrator compatibility.

Replaces the old research_loop.deep_research module for orchestrator.py.
Provides start()/run() interface with session object compatibility.
"""

import json
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from db.database import Database
from rairos_research_py import PyResearchAgent


@dataclass
class _MockSession:
    """Mimics the old ResearchSession dataclass for pre-populating papers."""
    papers: List[Any] = field(default_factory=list)
    session_id: str = ""

    def __bool__(self):
        return True


@dataclass
class _MockResult:
    """Mimics the old DeepResearchResult dataclass."""
    session_id: str = ""
    query: str = ""
    iterations: int = 0
    papers: List[Any] = field(default_factory=list)
    gaps: List[Any] = field(default_factory=list)
    report: str = ""
    duration_seconds: float = 0.0

    def __bool__(self):
        return True


class AgentShim:
    """Compatibility wrapper — matches old DeepResearchAgent API."""

    def __init__(
        self,
        query: str,
        max_iterations: int = 3,
        max_papers_per_iteration: int = 5,
        verbose: bool = False,
        **_kwargs,
    ):
        self._query = query
        config = {
            "max_iterations": max_iterations,
            "max_papers_per_iteration": max_papers_per_iteration,
            "verbose": verbose,
            "use_streaming_reasoning": False,
            "auto_checkpoint": False,
            "checkpoint_every_n_steps": 1,
            "checkpoint_interval_seconds": 60,
        }
        self._agent = PyResearchAgent(
            query=query,
            config_json=json.dumps(config),
        )
        self._agent.db = Database()
        self._session = _MockSession()

    def start(self) -> _MockSession:
        """Start a new session (returns mock session for paper pre-population)."""
        s_id = self._agent.start()
        self._session = _MockSession(session_id=s_id)
        return self._session

    def run(self) -> _MockResult:
        """Run the research agent, merge pre-populated papers into result."""
        result_json = self._agent.run()
        result = json.loads(result_json)

        # Merge manually pre-populated papers (from orchestrator) with agent results
        all_papers = self._session.papers + result.get("papers", [])

        return _MockResult(
            session_id=result.get("session_id", self._session.session_id),
            query=result.get("query", self._query),
            iterations=result.get("iterations", 0),
            papers=all_papers,
            gaps=result.get("gaps", []),
            report=result.get("report", ""),
            duration_seconds=result.get("duration_seconds", 0.0),
        )

    @property
    def db(self):
        return self._agent.db
