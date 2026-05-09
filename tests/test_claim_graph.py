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
        assert ClaimType.PARAM_SIZE is not None
        assert ClaimType.MEMORY is not None
        assert ClaimType.OTHER is not None


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
        assert node.comparison_op == ComparisonOp.GTE

    def test_to_dict(self):
        node = ClaimNode(
            claim_id="c1",
            paper_id="p1",
            claim_type=ClaimType.ACCURACY,
            value=95.0,
            comparison_op=ComparisonOp.GTE,
            source_text="x" * 300,  # text > 200 chars
        )
        d = node.to_dict()
        assert d["claim_id"] == "c1"
        assert d["paper_id"] == "p1"
        assert d["claim_type"] == "accuracy"
        assert d["value"] == 95.0
        assert len(d["source_text"]) == 200  # truncated to 200

    def test_to_dict_short_text_not_truncated(self):
        """Short source_text is returned as-is."""
        node = ClaimNode(
            claim_id="c1",
            paper_id="p1",
            claim_type=ClaimType.ACCURACY,
            value=95.0,
            comparison_op=ComparisonOp.GTE,
            source_text="short text",
        )
        d = node.to_dict()
        assert d["source_text"] == "short text"


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
        assert edge.to_paper == "p2"
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
        assert bc.edge_ab.from_paper == "p1"
        assert bc.edge_ba.from_paper == "p2"


class TestClaimGraph:
    def test_init(self):
        g = ClaimGraph()
        assert g is not None
        assert len(g.nodes) == 0
        assert len(g.edges) == 0

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

    def test_get_all_claims_by_type(self):
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")
        g.add_claim("p2", ClaimType.SPEEDUP, 2.0, ComparisonOp.GTE, "src2")
        g.add_claim("p3", ClaimType.ACCURACY, 90.0, ComparisonOp.GTE, "src3")
        acc_claims = g.get_all_claims_by_type(ClaimType.ACCURACY)
        assert len(acc_claims) == 2
        spd_claims = g.get_all_claims_by_type(ClaimType.SPEEDUP)
        assert len(spd_claims) == 1

    def test_find_contradictions_none(self):
        """No contradictions when all claims agree direction."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")
        g.add_claim("p2", ClaimType.ACCURACY, 94.0, ComparisonOp.GTE, "src2")
        g.add_claim("p3", ClaimType.SPEEDUP, 2.0, ComparisonOp.GTE, "src3")
        contradictions = g.find_contradictions()
        assert len(contradictions) == 0

    def test_find_contradictions_high_severity(self):
        """Opposite ops with >5% gap → high severity."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")  # >= 95
        g.add_claim("p2", ClaimType.ACCURACY, 88.0, ComparisonOp.LTE, "src2")  # <= 88
        contradictions = g.find_contradictions()
        assert len(contradictions) == 1
        assert contradictions[0].severity == "high"
        assert contradictions[0].metric == "accuracy"
        assert contradictions[0].claim_a.paper_id == "p1"
        assert contradictions[0].claim_b.paper_id == "p2"

    def test_find_contradictions_medium_severity(self):
        """Opposite ops with ≤5% gap → medium severity."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")
        g.add_claim("p2", ClaimType.ACCURACY, 94.0, ComparisonOp.LTE, "src2")
        contradictions = g.find_contradictions()
        assert len(contradictions) == 1
        assert contradictions[0].severity == "medium"

    def test_find_contradictions_same_paper_skipped(self):
        """Same paper claims are not contradictions."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")
        g.add_claim("p1", ClaimType.ACCURACY, 88.0, ComparisonOp.LTE, "src2")
        contradictions = g.find_contradictions()
        assert len(contradictions) == 0

    def test_find_contradictions_non_monitored_types_skipped(self):
        """PARAM_SIZE and MEMORY claim types are not checked for contradictions."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.PARAM_SIZE, 1e9, ComparisonOp.LTE, "src1")
        g.add_claim("p2", ClaimType.PARAM_SIZE, 1e8, ComparisonOp.GTE, "src2")
        g.add_claim("p1", ClaimType.MEMORY, 16, ComparisonOp.GTE, "src3")
        g.add_claim("p2", ClaimType.MEMORY, 8, ComparisonOp.LTE, "src4")
        contradictions = g.find_contradictions()
        assert len(contradictions) == 0

    def test_find_contradictions_reduction_type(self):
        """REDUCTION type is monitored for contradictions."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.REDUCTION, 50.0, ComparisonOp.GTE, "src1")
        g.add_claim("p2", ClaimType.REDUCTION, 20.0, ComparisonOp.LTE, "src2")
        contradictions = g.find_contradictions()
        assert len(contradictions) == 1
        assert contradictions[0].metric == "reduction"

    def test_find_bidirectional_contradictions_none(self):
        """One-way edges don't create bidirectional contradictions."""
        g = ClaimGraph()
        g.add_edge("p1", "p2", ClaimType.ACCURACY, 1.5, "A is better than B")
        contradictions = g.find_bidirectional_contradictions()
        assert len(contradictions) == 0

    def test_find_bidirectional_contradictions_detected(self):
        """A→B and B→A edges create a bidirectional contradiction."""
        g = ClaimGraph()
        g.add_edge("p1", "p2", ClaimType.ACCURACY, 1.5, "p1 > p2")
        g.add_edge("p2", "p1", ClaimType.ACCURACY, 1.3, "p2 > p1")
        contradictions = g.find_bidirectional_contradictions()
        assert len(contradictions) == 1
        c = contradictions[0]
        assert c.paper_a in ("p1", "p2")
        assert c.paper_b in ("p1", "p2")
        assert c.paper_a != c.paper_b

    def test_find_bidirectional_multiple_pairs(self):
        """Multiple bidirectional pairs are all detected."""
        g = ClaimGraph()
        g.add_edge("p1", "p2", ClaimType.SPEEDUP, 2.0, "p1 > p2")
        g.add_edge("p2", "p1", ClaimType.SPEEDUP, 1.8, "p2 > p1")
        g.add_edge("p3", "p4", ClaimType.ACCURACY, 1.5, "p3 > p4")
        g.add_edge("p4", "p3", ClaimType.ACCURACY, 1.2, "p4 > p3")
        contradictions = g.find_bidirectional_contradictions()
        assert len(contradictions) == 2

    def test_to_dict_from_dict(self):
        """ClaimGraph roundtrips through dict."""
        g = ClaimGraph()
        g.add_claim("p1", ClaimType.ACCURACY, 95.0, ComparisonOp.GTE, "src1")
        g.add_edge("p1", "p2", ClaimType.SPEEDUP, 2.0, "faster")
        d = g.to_dict()
        g2 = ClaimGraph.from_dict(d)
        assert len(g2.nodes) == 1
        assert len(g2.edges) == 1
