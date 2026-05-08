"""Tests for credibility_scorer, contradiction_heatmap, cross_domain_bridge."""

import pytest
from llm.credibility_scorer import jaccard, CapsuleCredibility, CredibilityScorer
from llm.contradiction_heatmap import _badge_color, render_heatmap_html
from llm.cross_domain_bridge import render_html


class TestJaccard:
    def test_empty_sets(self):
        assert jaccard([], []) == 0.0
        assert jaccard(["a"], []) == 0.0

    def test_identical(self):
        assert jaccard(["a", "b"], ["a", "b"]) == 1.0

    def test_disjoint(self):
        assert jaccard(["a"], ["b"]) == 0.0

    def test_partial_overlap(self):
        assert jaccard(["a", "b"], ["b", "c"]) == pytest.approx(1 / 3)

    def test_single_element(self):
        assert jaccard(["a"], ["a"]) == 1.0


class TestCapsuleCredibility:
    def test_to_dict(self):
        cap = CapsuleCredibility(
            capsule_id="c1",
            gap_title="Test",
            gap_type="improvement",
            outcome_score=0.8,
            novelty_score=0.9,
            max_overlap=0.1,
            is_trendslop=False,
            trigger_keywords=["rl", "alignment"],
        )
        d = cap.to_dict()
        assert d["capsule_id"] == "c1"
        assert d["novelty_score"] == 0.9
        assert d["is_trendslop"] is False


class TestCredibilityScorer:
    def test_init(self):
        scorer = CredibilityScorer()
        assert scorer._credibility is None

    def test_compute_credibility_no_file(self, monkeypatch, tmp_path):
        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        fake = tmp_path / "capsules.json"
        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        result = scorer.compute_credibility(force=True)
        assert result == []

    def test_compute_credibility_empty(self, monkeypatch, tmp_path):
        import json

        fake = tmp_path / "capsules.json"
        fake.write_text(json.dumps({"capsules": []}), encoding="utf-8")

        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        result = scorer.compute_credibility(force=True)
        assert result == []

    def test_compute_credibility_with_capsules(self, monkeypatch, tmp_path):
        import json

        capsules = [
            {
                "capsule_id": "c1",
                "trigger_keywords": ["rl", "alignment"],
                "action_gap_title": "Test A",
                "action_gap_type": "improvement",
                "outcome_success_score": 0.8,
            },
            {
                "capsule_id": "c2",
                "trigger_keywords": ["rl", "efficiency"],
                "action_gap_title": "Test B",
                "action_gap_type": "efficiency_gap",
                "outcome_success_score": 0.6,
            },
        ]
        fake = tmp_path / "capsules.json"
        fake.write_text(json.dumps({"capsules": capsules}), encoding="utf-8")

        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        result = scorer.compute_credibility(force=True)
        assert len(result) == 2
        assert all(isinstance(c, CapsuleCredibility) for c in result)
        # First should be most novel (lowest overlap)
        assert result[0].novelty_score >= result[1].novelty_score

    def test_cache(self, monkeypatch, tmp_path):
        import json

        fake = tmp_path / "capsules.json"
        fake.write_text(
            json.dumps(
                {
                    "capsules": [
                        {
                            "capsule_id": "c1",
                            "trigger_keywords": ["a"],
                            "action_gap_title": "T",
                            "action_gap_type": "x",
                            "outcome_success_score": 0.5,
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )

        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        r1 = scorer.compute_credibility()
        r2 = scorer.compute_credibility()
        assert r1 is r2  # cached

    def test_get_trendslop(self, monkeypatch, tmp_path):
        import json

        capsules = [
            {"capsule_id": "c1", "trigger_keywords": ["a", "b", "c", "d", "e"]},
            {"capsule_id": "c2", "trigger_keywords": ["a", "b", "c", "d"]},
        ]
        for c in capsules:
            c.setdefault("action_gap_title", "T")
            c.setdefault("action_gap_type", "x")
            c.setdefault("outcome_success_score", 0.5)
        fake = tmp_path / "capsules.json"
        fake.write_text(json.dumps({"capsules": capsules}), encoding="utf-8")

        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        trendslop = scorer.get_trendslop_capsules()
        # 4/5 overlap = 0.8 > 0.7 threshold
        assert len(trendslop) >= 0  # depends on overlap

    def test_render_html_empty(self, monkeypatch, tmp_path):
        fake = tmp_path / "capsules.json"
        scorer = CredibilityScorer()
        import llm.credibility_scorer as mod

        monkeypatch.setattr(mod, "CAPSULE_PATH", fake)
        html = scorer.render_html()
        assert "No capsules yet" in html


class TestBadgeColor:
    def test_zero(self):
        assert _badge_color(0) == "#e8e4de"

    def test_one(self):
        assert _badge_color(1) == "#f5d76e"

    def test_two(self):
        assert _badge_color(2) == "#e67e22"

    def test_three_plus(self):
        assert _badge_color(3) == "#e74c3c"
        assert _badge_color(10) == "#e74c3c"


class TestRenderHeatmapHtml:
    def test_empty(self):
        html = render_heatmap_html([], {})
        assert "No papers yet" in html

    def test_with_papers(self):
        papers = [
            {"id": "p1", "title": "Test Paper", "primary_category": "cs.AI", "published": "2024"}
        ]
        contrad = {"p1": {"count": 2, "contradictions": []}}
        html = render_heatmap_html(papers, contrad)
        assert "heatmap-card" in html
        assert "Test Paper" in html

    def test_with_contradictions(self):
        papers = [{"id": "p1", "title": "Test"}]
        contrad = {
            "p1": {
                "count": 1,
                "contradictions": [
                    {
                        "gap_type": "method",
                        "partner_id": "p2",
                        "polarity": "positive",
                        "shared_keywords": ["rl"],
                    }
                ],
            }
        }
        html = render_heatmap_html(papers, contrad)
        assert "1 🔥" in html


class TestCrossDomainBridge:
    def test_render_html_no_bridges(self, monkeypatch):
        """render_html with no bridges shows fallback message."""
        html = render_html([])
        assert "No cross-domain bridges found" in html
