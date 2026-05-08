"""Tests for research_loop/provenance.py dataclasses."""

import pytest
from research_loop.provenance import PaperLocation, EquationSource, ClaimSource, AlgorithmSource


class TestPaperLocation:
    def test_creation(self):
        loc = PaperLocation(section="3.2", page=5, char_start=1234, char_end=5678)
        assert loc.section == "3.2"
        assert loc.page == 5
        assert loc.char_start == 1234
        assert loc.char_end == 5678

    def test_short_ref_format(self):
        loc = PaperLocation(section="Abstract", page=1, char_start=0, char_end=99)
        assert loc.short_ref() == "§Abstractp1@0"

    def test_short_ref_with_algorithm_section(self):
        loc = PaperLocation(section="Algorithm 1", page=3, char_start=9999, char_end=12345)
        assert loc.short_ref() == "§Algorithm 1p3@9999"


class TestEquationSource:
    def test_creation(self):
        loc = PaperLocation(section="2.1", page=2, char_start=100, char_end=200)
        eq = EquationSource(index=0, equation=r"E = mc^2", location=loc)
        assert eq.index == 0
        assert eq.equation == r"E = mc^2"
        assert eq.location.section == "2.1"

    def test_tag(self):
        loc = PaperLocation(section="3", page=1, char_start=0, char_end=10)
        eq = EquationSource(index=5, equation=r"\alpha", location=loc)
        assert eq.tag() == "@eq[5]"

    def test_equation_with_complex_latex(self):
        loc = PaperLocation(section="A", page=1, char_start=0, char_end=1)
        eq = EquationSource(index=1, equation=r"\frac{\partial f}{\partial x} = \sum_{i=1}^n", location=loc)
        assert "@eq[1]" == eq.tag()
        assert r"\frac" in eq.equation


class TestClaimSource:
    def test_creation(self):
        loc = PaperLocation(section="1", page=1, char_start=50, char_end=150)
        claim = ClaimSource(index=3, claim="Attention mechanisms improve LLM performance.", location=loc)
        assert claim.index == 3
        assert "Attention" in claim.claim
        assert claim.location.page == 1

    def test_tag(self):
        loc = PaperLocation(section="Intro", page=1, char_start=0, char_end=50)
        claim = ClaimSource(index=7, claim="Test claim text.", location=loc)
        assert claim.tag() == "@claim[7]"


class TestAlgorithmSource:
    def test_creation(self):
        loc = PaperLocation(section="4", page=6, char_start=500, char_end=1200)
        algo = AlgorithmSource(index=2, description="Gradient descent optimization", location=loc)
        assert algo.index == 2
        assert algo.description == "Gradient descent optimization"
        assert algo.location.section == "4"

    def test_tag(self):
        loc = PaperLocation(section="5", page=10, char_start=0, char_end=100)
        algo = AlgorithmSource(index=12, description="Backpropagation through time", location=loc)
        assert algo.tag() == "@algo[12]"
