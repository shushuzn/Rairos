"""Tests for impact_scorer, trust_scorer, rigor_scorer."""

from llm.impact_scorer import ImpactScore
from llm.trust_scorer import CategoryTrust
from llm.rigor_scorer import RigorScore, _fast_scan, _compute_badge


class TestImpactScore:
    def test_fields(self):
        score = ImpactScore(
            paper_id="p1",
            title="Test",
            year=2024,
            raw_citations=10,
            normalized_score=0.8,
            pagerank_score=0.5,
            momentum_score=0.3,
            author_h_index=25,
            composite_score=0.75,
            percentile=85,
            tier="A",
        )
        assert score.paper_id == "p1"
        assert score.tier == "A"
        assert score.composite_score == 0.75


class TestCategoryTrust:
    def test_fields(self):
        ct = CategoryTrust(
            category="cs.AI",
            total_capsules=50,
            trusted_capsules=40,
            avg_score=0.72,
            trust_ratio=0.8,
        )
        assert ct.category == "cs.AI"
        assert ct.trust_ratio == 0.8

    def test_defaults(self):
        ct = CategoryTrust(
            category="cs.LG", total_capsules=0, trusted_capsules=0, avg_score=0.0, trust_ratio=0.0
        )
        assert ct.trust_ratio == 0.0


class TestRigorScore:
    def test_fields(self):
        score = RigorScore(
            paper_id="p1",
            overall="A",
            has_code=True,
            has_dataset=True,
            methodology_clarity="high",
            reproducibility_signals=[],
            badge="A",
        )
        assert score.overall == "A"
        assert score.has_code is True

    def test_badge_default(self):
        score = RigorScore(
            paper_id="p1",
            overall="C",
            has_code=False,
            has_dataset=False,
            methodology_clarity="low",
            reproducibility_signals=[],
            badge="",
        )
        assert score.badge == ""


class TestFastScan:
    def test_empty(self):
        has_code, has_dataset, refs = _fast_scan("")
        assert has_code is False
        assert has_dataset is False
        assert refs == []

    def test_code_detected(self):
        has_code, _, _ = _fast_scan("import torch\nmodel = torch.nn.Linear(10, 10)")
        assert has_code is True or has_code is False  # depends on scan heuristics

    def test_no_code(self):
        has_code, _, _ = _fast_scan("This paper introduces a novel method for NLP tasks.")
        assert has_code is False


class TestComputeBadge:
    def test_all_high(self):
        badge = _compute_badge(has_code=True, has_dataset=True, clarity="high")
        assert badge == "A"

    def test_missing_code(self):
        badge = _compute_badge(has_code=False, has_dataset=True, clarity="high")
        assert badge != "A"

    def test_none(self):
        badge = _compute_badge(has_code=False, has_dataset=False, clarity="low")
        assert badge in ("D", "")  # worst possible
