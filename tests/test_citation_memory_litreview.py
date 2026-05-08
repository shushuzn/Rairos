"""Tests for citation_chain, research_memory, litreview_generator dataclasses."""

import pytest
from llm.citation_chain import CitationNode, CitationChain, ResearchFamily
from llm.research_memory import (
    StanceType,
    AnomalySeverity,
    ResearchStance,
    AnomalyAlert,
)
from llm.litreview_generator import LitReviewSection, LitReview, LitReviewResult


class TestCitationNode:
    def test_fields(self):
        node = CitationNode(paper_id="p1", title="Test", year=2024)
        assert node.paper_id == "p1"
        assert node.citation_count == 0
        assert node.cited_by == []

    def test_add_cited_by(self):
        node = CitationNode("p1", "T")
        node.cited_by.append("p2")
        assert "p2" in node.cited_by


class TestCitationChain:
    def test_empty(self):
        chain = CitationChain()
        assert chain.nodes == []
        assert chain.edges == []


class TestResearchFamily:
    def test_fields(self):
        family = ResearchFamily("f1", "a1", "Ancestor Paper")
        assert family.family_id == "f1"
        assert family.size == 0

    def test_with_papers(self):
        family = ResearchFamily("f1", "a1", "Title", papers=["p1", "p2"], size=2)
        assert family.size == 2


class TestEnums:
    def test_stance_values(self):
        assert StanceType.SUPPORTED is not None
        assert StanceType.REJECTED is not None
        assert StanceType.DEFERRED is not None

    def test_anomaly_values(self):
        assert AnomalySeverity.HIGH.value is not None
        assert AnomalySeverity.LOW.value is not None


class TestResearchStance:
    def test_fields(self):
        stance = ResearchStance(
            stance_id="s1",
            topic="RL",
            claim="PPO works",
            stance=StanceType.SUPPORTED,
            evidence_refs=["p1"],
            reasoning="Test reasoning",
            confidence=0.9,
        )
        assert stance.stance == StanceType.SUPPORTED
        assert stance.confidence == 0.9

    def test_to_dict(self):
        stance = ResearchStance("s1", "RL", "Claim", StanceType.REJECTED, ["p1"], "reason", 0.7)
        d = stance.to_dict()
        assert d["stance_id"] == "s1"


class TestAnomalyAlert:
    def test_fields(self):
        alert = AnomalyAlert(
            anomaly_id="a1",
            stance_id="s1",
            topic="RL",
            stance_claim="Test claim",
            paper_title="Test Paper",
            paper_arxiv_id="1234",
            anomaly_type="contradiction",
            severity=AnomalySeverity.HIGH,
            description="Test anomaly",
        )
        assert alert.severity == AnomalySeverity.HIGH

    def test_to_dict(self):
        alert = AnomalyAlert(
            "a1", "s1", "T", "C", "Paper", "1234", "type", AnomalySeverity.LOW, "desc"
        )
        d = alert.to_dict()
        assert d["anomaly_id"] == "a1"


class TestLitReviewSection:
    def test_fields(self):
        section = LitReviewSection("Title", "Content here")
        assert section.title == "Title"
        assert section.content == "Content here"

    def test_paper_refs(self):
        section = LitReviewSection("Methods", "Content", paper_refs=["p1", "p2"])
        assert len(section.paper_refs) == 2


class TestLitReview:
    def test_fields(self):
        review = LitReview(topic="RL", total_papers=5)
        assert review.topic == "RL"
        assert review.total_papers == 5

    def test_with_sections(self):
        section = LitReviewSection("H", "C")
        review = LitReview("AI", sections=[section], papers_used=["p1"])
        assert len(review.sections) == 1
        assert len(review.papers_used) == 1


class TestLitReviewResult:
    def test_fields(self):
        result = LitReviewResult(success=True, topic="RL")
        assert result.success is True

    def test_with_error(self):
        result = LitReviewResult(success=False, topic="RL", error="Not enough papers")
        assert result.error == "Not enough papers"
