"""Tests for research_loop/provenance.py."""

from research_loop.provenance import (
    PaperLocation,
    EquationSource,
    ClaimSource,
    AlgorithmSource,
)


class TestPaperLocation:
    def test_fields(self):
        loc = PaperLocation(section="3.2", page=5, char_start=1000, char_end=1050)
        assert loc.section == "3.2"
        assert loc.page == 5
        assert loc.char_start == 1000
        assert loc.char_end == 1050

    def test_short_ref(self):
        loc = PaperLocation(section="Abstract", page=1, char_start=0, char_end=100)
        assert loc.short_ref() == "§Abstractp1@0"

    def test_short_ref_unknown_page(self):
        loc = PaperLocation(section="Intro", page=0, char_start=50, char_end=200)
        assert loc.short_ref() == "§Introp0@50"


class TestEquationSource:
    def test_fields(self):
        loc = PaperLocation(section="2.1", page=3, char_start=200, char_end=350)
        eq = EquationSource(index=5, equation="E = mc^2", location=loc)
        assert eq.index == 5
        assert eq.equation == "E = mc^2"
        assert eq.location.section == "2.1"

    def test_tag(self):
        loc = PaperLocation(section="1", page=1, char_start=0, char_end=10)
        eq = EquationSource(index=3, equation="x = 1", location=loc)
        assert eq.tag() == "@eq[3]"


class TestClaimSource:
    def test_fields(self):
        loc = PaperLocation(section="Abstract", page=1, char_start=0, char_end=100)
        claim = ClaimSource(index=1, claim="Model achieves 95% accuracy", location=loc)
        assert claim.index == 1
        assert claim.claim == "Model achieves 95% accuracy"
        assert claim.location.page == 1

    def test_tag(self):
        loc = PaperLocation(section="4", page=10, char_start=500, char_end=600)
        claim = ClaimSource(index=7, claim="Speedup is 2x", location=loc)
        assert claim.tag() == "@claim[7]"


class TestAlgorithmSource:
    def test_fields(self):
        loc = PaperLocation(section="Algorithm 1", page=2, char_start=100, char_end=500)
        algo = AlgorithmSource(index=1, description="Gradient descent", location=loc)
        assert algo.index == 1
        assert algo.description == "Gradient descent"
        assert algo.location.section == "Algorithm 1"

    def test_tag(self):
        loc = PaperLocation(section="3", page=5, char_start=0, char_end=50)
        algo = AlgorithmSource(index=2, description="Backpropagation", location=loc)
        assert algo.tag() == "@algo[2]"
