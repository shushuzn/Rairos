"""Provenance dataclasses — track paper location for extracted content."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class PaperLocation:
    """Absolute position of a content item within the concatenated paper text."""

    section: str  # e.g. "3.2", "Abstract", "Algorithm"
    page: int  # 1-based page number (0 = unknown)
    char_start: int  # character offset in concatenated full-text
    char_end: int  # inclusive end offset

    def short_ref(self) -> str:
        return f"§{self.section}p{self.page}@{self.char_start}"


@dataclass
class EquationSource:
    """An equation with its provenance."""

    index: int
    equation: str  # raw LaTeX string
    location: PaperLocation

    def tag(self) -> str:
        return f"@eq[{self.index}]"


@dataclass
class ClaimSource:
    """A claim with its provenance."""

    index: int
    claim: str
    location: PaperLocation

    def tag(self) -> str:
        return f"@claim[{self.index}]"


@dataclass
class AlgorithmSource:
    """An algorithm description with its provenance."""

    index: int
    description: str
    location: PaperLocation

    def tag(self) -> str:
        return f"@algo[{self.index}]"
