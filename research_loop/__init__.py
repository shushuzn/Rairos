"""Research Loop Modules

Unified access to all research automation pipelines:
- research_loop.core: Autonomous research loop (search → extract → summarize)
- research_loop.paper2code: Citation-anchored code generation
- research_loop.evoskill: Benchmark-driven skill discovery
- research_loop.rag: Complete闭环 (paper → code → tests → skills)

Usage:
    from research_loop import PaperPipeline, EvoSkillPipeline, RagPipeline
    from research_loop.core import run_research
"""
from __future__ import annotations

# Re-export main classes for convenient imports
from research_loop.paper2code_integration import PaperPipeline
from research_loop.evoskill_integration import EvoSkillPipeline
from research_loop.rag_pipeline import RagPipeline

__all__ = [
    "PaperPipeline",
    "EvoSkillPipeline",
    "RagPipeline",
]
