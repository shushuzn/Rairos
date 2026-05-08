"""Tests for cross_search, provenance, watch_papers."""

import pytest
from parsers.cross_search import search_papers_multi
from research_loop.provenance import PaperLocation, EquationSource, ClaimSource, AlgorithmSource
from core.watch_papers import Watcher, watch_and_rebuild


class TestCrossSearch:
    def test_signature(self):
        import inspect

        sig = inspect.signature(search_papers_multi)
        assert "query" in sig.parameters


class TestProvenance:
    def test_paper_location(self):
        loc = PaperLocation(section="Abstract", page=1, char_start=0, char_end=10)
        assert loc.section == "Abstract"
        assert loc.page == 1

    def test_equation_source(self):
        loc = PaperLocation(section="1", page=1, char_start=0, char_end=5)
        eq = EquationSource(index=1, equation="E=mc^2", location=loc)
        assert eq.equation == "E=mc^2"
        assert eq.location.section == "1"

    def test_claim_source(self):
        loc = PaperLocation(section="2", page=2, char_start=0, char_end=8)
        claim = ClaimSource(index=0, claim="This is novel", location=loc)
        assert claim.claim == "This is novel"

    def test_algorithm_source(self):
        loc = PaperLocation(section="3", page=3, char_start=0, char_end=6)
        alg = AlgorithmSource(index=0, description="Backprop", location=loc)
        assert alg.description == "Backprop"


class TestWatchPapers:
    def test_watcher_init(self):
        w = Watcher(path="D:/fake", interval=60, on_change=None)
        assert "fake" in str(w.path)
        assert w.interval == 60

    def test_watch_and_rebuild_signature(self):
        import inspect

        sig = inspect.signature(watch_and_rebuild)
        assert "papers_json" in sig.parameters
