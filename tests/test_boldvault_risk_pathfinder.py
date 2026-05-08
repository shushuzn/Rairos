"""Tests for bold_vault, at_risk_scanner, citation_pathfinder_web."""
import json
import pytest
from llm.bold_vault import _jaccard, BoldCapsule, get_bold_capsules, render_html as bold_html
from llm.at_risk_scanner import AtRiskCapsule, get_at_risk_capsules, keep_active, pin_to_ttl, render_html as risk_html
from llm.citation_pathfinder_web import build_citation_graph, render_citation_graph_svg, render_citation_chain_html


# ── bold_vault ──────────────────────────────────────────────────────────────

class TestJaccardVault:
    def test_identical(self):
        assert _jaccard(["a", "b"], ["a", "b"]) == 1.0

    def test_disjoint(self):
        assert _jaccard(["a"], ["b"]) == 0.0

    def test_empty(self):
        assert _jaccard([], ["a"]) == 0.0


class TestBoldCapsule:
    def test_fields(self):
        b = BoldCapsule(
            capsule_id="c1", gap_title="Test", gap_type="theoretical_gap",
            polarity="positive", outcome_score=0.8, novelty_score=0.9,
            trigger_keywords=["rl"], reason="theoretical"
        )
        assert b.capsule_id == "c1"
        assert b.outcome_score == 0.8


class TestGetBoldCapsules:
    def test_no_file(self, monkeypatch, tmp_path):
        import llm.bold_vault as mod
        monkeypatch.setattr(mod, "CAPSULE_PATH", tmp_path / "nonexistent.json")
        result = get_bold_capsules()
        assert result == []

    def test_empty_capsules(self, monkeypatch, tmp_path):
        import llm.bold_vault as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": []}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_bold_capsules()
        assert result == []

    def test_theoretical_gap(self, monkeypatch, tmp_path):
        import llm.bold_vault as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "action_gap_type": "theoretical_gap", "trigger_keywords": ["quantum"],
             "action_gap_title": "Quantum RL", "outcome_success_score": 0.9, "status": "active"}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_bold_capsules()
        assert len(result) == 1
        assert "theoretical" in result[0].reason

    def test_negative_polarity(self, monkeypatch, tmp_path):
        import llm.bold_vault as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "action_gap_type": "improvement", "trigger_keywords": ["rl"],
             "polarity": "negative", "action_gap_title": "RL Fails", "outcome_success_score": 0.2, "status": "active"}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_bold_capsules()
        assert len(result) == 1
        assert "negative" in result[0].reason

    def test_not_active_skipped(self, monkeypatch, tmp_path):
        import llm.bold_vault as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "action_gap_type": "theoretical_gap", "trigger_keywords": ["a"],
             "status": "archived", "action_gap_title": "T", "outcome_success_score": 0.5}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_bold_capsules()
        assert result == []


class TestBoldRenderHtml:
    def test_empty(self):
        html = bold_html([])
        assert "No bold hypotheses" in html

    def test_with_capsules(self):
        caps = [BoldCapsule("c1", "Test Gap", "theoretical_gap", "positive", 0.8, 0.9, ["rl"], "theoretical")]
        html = bold_html(caps)
        assert "bold-card" in html
        assert "Test Gap" in html


# ── at_risk_scanner ─────────────────────────────────────────────────────────

class TestAtRiskCapsule:
    def test_fields(self):
        cap = AtRiskCapsule("c1", "Test", "type", 0.5, low_score_streak=3, status="active")
        assert cap.low_score_streak == 3


class TestGetAtRiskCapsules:
    def test_nonexistent_file(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        monkeypatch.setattr(mod, "CAPSULE_PATH", tmp_path / "nope.json")
        result = get_at_risk_capsules()
        assert result == []

    def test_below_threshold(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "low_score_streak": 1, "status": "active",
             "action_gap_title": "T", "action_gap_type": "x", "outcome_success_score": 0.5}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_at_risk_capsules()
        assert result == []

    def test_at_risk(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "low_score_streak": 3, "status": "active",
             "action_gap_title": "Bad Gap", "action_gap_type": "method", "outcome_success_score": 0.1}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_at_risk_capsules()
        assert len(result) == 1
        assert result[0].low_score_streak == 3

    def test_archived_skipped(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "low_score_streak": 5, "status": "archived",
             "action_gap_title": "T", "action_gap_type": "x", "outcome_success_score": 0.1}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        result = get_at_risk_capsules()
        assert result == []


class TestKeepActiveAndPin:
    def test_keep_active_not_found(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": []}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        assert keep_active("nope") is False

    def test_keep_active_resets(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "low_score_streak": 5, "pinned_ttl": 10}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        assert keep_active("c1") is True
        data = json.loads(p.read_text(encoding="utf-8"))
        assert data["capsules"][0]["low_score_streak"] == 0
        assert data["capsules"][0]["pinned_ttl"] == 0

    def test_pin_to_ttl(self, monkeypatch, tmp_path):
        import llm.at_risk_scanner as mod
        p = tmp_path / "capsules.json"
        p.write_text(json.dumps({"capsules": [
            {"capsule_id": "c1", "low_score_streak": 3, "pinned_ttl": 0}
        ]}), encoding="utf-8")
        monkeypatch.setattr(mod, "CAPSULE_PATH", p)
        assert pin_to_ttl("c1", ttl=5) is True
        data = json.loads(p.read_text(encoding="utf-8"))
        assert data["capsules"][0]["pinned_ttl"] == 5
        assert data["capsules"][0]["low_score_streak"] == 0


class TestAtRiskRenderHtml:
    def test_empty(self):
        html = risk_html([])
        assert "No at-risk capsules" in html

    def test_with_capsules(self):
        caps = [AtRiskCapsule("c1", "Risky Gap", "method", 0.1, 3, "active")]
        html = risk_html(caps)
        assert "at-risk" in html
        assert "Risky Gap" in html
        assert "3" in html


# ── citation_pathfinder_web ──────────────────────────────────────────────────

class TestBuildCitationGraph:
    def test_structure(self):
        graph = build_citation_graph("p1", "Test Paper", ["p2"], ["cap1"])
        assert "nodes" in graph
        assert "edges" in graph
        assert len(graph["nodes"]) >= 2
        assert len(graph["edges"]) >= 1

    def test_source_node(self):
        graph = build_citation_graph("arxiv:1234", "My Paper", [], [])
        nodes = {n["id"]: n for n in graph["nodes"]}
        assert nodes["arxiv:1234"]["type"] == "source_paper"


class TestRenderCitationGraphSvg:
    def test_empty(self):
        svg = render_citation_graph_svg({"nodes": [], "edges": []})
        assert "<svg" in svg
        assert "</svg>" in svg

    def test_with_data(self):
        graph = build_citation_graph("p1", "Paper A", ["p2", "p3"], ["cap1"])
        svg = render_citation_graph_svg(graph)
        assert "<svg" in svg
        assert "source_paper" not in svg  # SVG uses text, not type attribute
        assert "📄" in svg

    def test_with_params(self):
        svg = render_citation_graph_svg(paper_id="p1", paper_title="T", cited_paper_ids=["p2"], cited_capsule_ids=[])
        assert "<svg" in svg
        assert "📄" in svg


class TestRenderCitationChainHtml:
    def test_no_file(self, monkeypatch, tmp_path):
        import llm.citation_pathfinder_web as mod
        monkeypatch.setattr(mod, "CAPSULES_PATH", tmp_path / "nonexistent.json")
        html = render_citation_chain_html("p1", "Paper", ["p2"], [])
        assert "<svg" in html
        assert "Citation Pathfinder" in html
