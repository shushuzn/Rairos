"""Research backend — Python callbacks for the Rust DeepResearchAgent.

These functions bridge between the Rust agent's ResearchBackend trait
and the Python implementations (MCP tools, EvolutionTracker, etc.).
"""

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from research_loop.snapstate import PaperSnapshot, GapSnapshot
from research_loop.core import Paper


def cb_stream_plan(self, json_str: str) -> str:
    args = json.loads(json_str)
    if self.use_streaming_reasoning:
        result = self._stream_plan_search(args["iteration"])
    return result


def cb_search_papers(self, json_str: str) -> str:
    args = json.loads(json_str)
    papers = self._search_papers(args["query"], args["iteration"])
    return json.dumps([p.__dict__ for p in papers])


def cb_extract_paper(self, json_str: str) -> str:
    paper_dict = json.loads(json_str)
    paper = Paper(**paper_dict)
    snap = self._extract_papers([paper], 0)
    if snap:
        s = snap[0]
        return json.dumps({
            "arxiv_id": s.arxiv_id, "title": s.title, "abstract": s.abstract,
            "url": s.url,
            "extracted_text": s.extracted_text[:5000] if s.extracted_text else "",
        })
    return "{}"


def cb_analyze_gaps(self, json_str: str) -> str:
    args = json.loads(json_str)
    snaps = [PaperSnapshot(**s) for s in args.get("snapshots", [])]
    gaps = self._analyze_gaps(snaps, 0)
    return json.dumps([g.__dict__ for g in gaps])


def cb_get_search_guidance(self, json_str: str) -> str:
    args = json.loads(json_str)
    hint, confidence = self._get_search_guidance(
        args["topic"], args.get("gap_type", ""), args.get("gap_title", ""),
    )
    return json.dumps({"hint": hint, "confidence": confidence})


def cb_encode_accepted_gap(self, json_str: str) -> str:
    gap_dict = json.loads(json_str)
    self.tracker.record_gap_accept(
        topic=self.query,
        gap_type=gap_dict.get("gap_type", ""),
        gap_title=gap_dict.get("title", ""),
        gap_description=gap_dict.get("description", ""),
    )
    return ""


def cb_on_thought(self, json_str: str) -> str:
    thought = json.loads(json_str)
    self._record_thought(
        thought.get("role", "planner"),
        thought.get("content", ""),
        thought.get("iteration", 0),
    )
    return ""


def cb_find_skills(self, json_str: str) -> str:
    results = self._find_skills(json_str)
    return json.dumps([r.name if hasattr(r, 'name') else str(r) for r in results])


def cb_checkpoint(self, json_str: str) -> str:
    self._auto_checkpoint()
    return ""


def cb_new_session(self, json_str: str) -> str:
    args = json.loads(json_str)
    session = self.start()
    return json.dumps({"session_id": session.session_id})
