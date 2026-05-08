"""Tests for research_loop/claim_graph.py."""

import pytest
from research_loop.claim_graph import (
    ClaimType,
    ComparisonOp,
    ClaimNode,
    ClaimEdge,
    Contradiction,
    BidirectionalContradiction,
    ClaimGraph,
)


class TestClaimType:
    def test_values(self):
        assert ClaimType.ACCURACY is not None
        assert ClaimType.SPEEDUP is not None
        assert ClaimType.REDUCTION is not None


class TestComparisonOp:
    def test_values(self):
        assert ComparisonOp.GTE is not None
        assert ComparisonOp.LTE is not None
        assert ComparisonOp.EQ is not None


class TestClaimNode:
    def test_fields(self):
        node = ClaimNode(
            claim_id="c1",
            paper_id="p1",
            claim_type=ClaimType.ACCURACY,
            value=95.0,
            comparison_op=ComparisonOp.GTE,
            source_text="accuracy is 95%",
        )
        assert node.claim_id == "c1"
        assert node.claim_type == ClaimType.ACCURACY
        assert node.value == 95.0


class TestClaimEdge:
    def test_fields(self):
        edge = ClaimEdge(
            from_paper="p1",
            to_paper="p2",
            claim_type=ClaimType.SPEEDUP,
            improvement_ratio=2.0,
            source_text="2x faster",
        )
        assert edge.from_paper == "p1"
        assert edge.improvement_ratio == 2.0


class TestContradiction:
    def test_fields(self):
        c = Contradiction(
            claim_a="c1",
            claim_b="c2",
            metric="accuracy",
            description="Disagree on baseline",
            severity="high",
        )
        assert c.metric == "accuracy"
        assert c.severity == "high"


class TestBidirectionalContradiction:
    def test_fields(self):
        edge1 = ClaimEdge("p1", "p2", ClaimType.ACCURACY, 1.0, "text1")
        edge2 = ClaimEdge("p2", "p1", ClaimType.ACCURACY, 0.9, "text2")
        bc = BidirectionalContradiction(
            paper_a="p1",
            paper_b="p2",
            edge_ab=edge1,
            edge_ba=edge2,
            severity="medium",
            description="Contradictory claims",
        )
        assert bc.paper_a == "p1"
        assert bc.severity == "medium"


class TestClaimGraph:
    def test_init(self):
        g = ClaimGraph()
        assert g is not None

    def test_add_claim(self):
        g = ClaimGraph()
        g.add_claim(
            paper_id="p1",
            claim_type=ClaimType.ACCURACY,
            value=95.0,
            comparison_op=ComparisonOp.GTE,
            source_text="source",
        )
        assert len(g.get_paper_claims("p1")) == 1

    def test_add_edge(self):
        g = ClaimGraph()
        g.add_edge(
            from_paper="p1",
            to_paper="p2",
            claim_type=ClaimType.SPEEDUP,
            improvement_ratio=2.0,
            source_text="faster",
        )
        assert len(g.edges) == 1
