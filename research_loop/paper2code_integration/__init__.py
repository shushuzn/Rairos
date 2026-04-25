"""
paper2code Integration Module

Bridges ai-research-os with paper2code skill to create a complete pipeline:
  1. Fetch & parse paper (ai-research-os) → produces P-Note, C-Note, Radar
  2. Generate citation-anchored implementation (paper2code) → produces src/, configs/

Usage:
    from research_loop.paper2code_integration import PaperPipeline
    pipeline = PaperPipeline()
    result = pipeline.run("2106.09685", mode="minimal", framework="pytorch")
"""

import subprocess
import shutil
import os
from pathlib import Path
from typing import Optional


class PaperPipeline:
    """Complete paper-to-knowledge pipeline with code generation."""

    def __init__(self, work_dir: str = ".paper2code_work"):
        self.work_dir = Path(work_dir)
        self.paper2code_skill = Path.home() / ".claude" / "skills" / "paper2code"

    def run(self, arxiv_id: str, mode: str = "minimal", framework: str = "pytorch") -> dict:
        """
        Execute full pipeline: parse → analyze → implement

        Args:
            arxiv_id: e.g. "2106.09685" or full URL
            mode: minimal | full | educational
            framework: pytorch | jax | numpy

        Returns:
            dict with paths to generated artifacts
        """
        # Clean work directory
        self.work_dir.mkdir(exist_ok=True)
        paper_dir = self.work_dir / arxiv_id

        # Stage 1: Fetch and parse paper
        self._fetch_paper(arxiv_id, paper_dir)

        # Stage 2: Generate citation-anchored implementation
        self._generate_code(arxiv_id, paper_dir, mode, framework)

        return {
            "arxiv_id": arxiv_id,
            "paper_dir": str(paper_dir),
            "implementation_dir": str(paper_dir / "src"),
            "readme": str(paper_dir / "README.md"),
        }

    def _fetch_paper(self, arxiv_id: str, paper_dir: Path) -> None:
        """Fetch paper using ai-research-os parsers."""
        # Use existing core/fetch logic
        from core.fetch import fetch_paper
        fetch_paper(arxiv_id, str(paper_dir))

    def _generate_code(self, arxiv_id: str, paper_dir: Path, mode: str, framework: str) -> None:
        """Generate code using paper2code skill."""
        fetch_script = self.paper2code_skill / "scripts" / "fetch_paper.py"

        if not fetch_script.exists():
            raise FileNotFoundError(
                f"paper2code skill not found at {self.paper2code_skill}. "
                "Run: git clone https://github.com/PrathamLearnsToCode/paper2code.git ~/.claude/skills/"
            )

        # Run paper2code
        subprocess.run([
            "python", str(fetch_script),
            arxiv_id, str(paper_dir)
        ], check=True)

    def install_deps(self) -> None:
        """Install paper2code dependencies."""
        subprocess.run([
            "pip", "install", "pymupdf4llm", "pdfplumber", "requests", "pyyaml"
        ], check=True)
