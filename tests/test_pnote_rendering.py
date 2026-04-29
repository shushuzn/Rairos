"""
Tests for P-note rendering: radar chart and per-page heatmap.

Covers the F (radar chart) and G (page heatmap) features.
"""
from __future__ import annotations


from renderers.pnote import (
    render_radar_chart,
    _render_page_heatmap,
    _build_claims_section,
    _build_ai_block,
    _extract_rubric_scores,
)


# ── Radar chart ────────────────────────────────────────────────────────────────


class TestRenderRadarChart:
    """Smoke + boundary tests for the SVG radar chart."""

    def test_renders_valid_svg(self):
        scores = {
            "novelty": 4,
            "leverage": 3,
            "evidence": 5,
            "cost": 2,
            "moat": 3,
            "adoption": 2,
        }
        svg = render_radar_chart(scores)
        assert svg.startswith("<svg")
        assert "</svg>" in svg
        assert 'viewBox="0 0 280 280"' in svg
        assert "创新性" in svg
        assert "护城河" in svg

    def test_returns_empty_for_too_few_scores(self):
        assert render_radar_chart({}) == ""
        assert render_radar_chart({"novelty": 3, "leverage": 2}) == ""
        # 3 valid scores → renders (n=3 ≥ 3)

    def test_all_scores_out_of_range_returns_empty(self):
        assert render_radar_chart({"novelty": 0}) == ""
        assert render_radar_chart({"novelty": 6}) == ""
        assert render_radar_chart({"novelty": 99}) == ""

    def test_partial_valid_scores_renders(self):
        scores = {
            "novelty": 4,
            "leverage": 3,
            "evidence": 5,
        }
        svg = render_radar_chart(scores)
        assert "创新性" in svg
        assert "</svg>" in svg

    def test_renders_avg_badge(self):
        scores = {"novelty": 4, "leverage": 4, "evidence": 4, "cost": 4, "moat": 4, "adoption": 4}
        svg = render_radar_chart(scores)
        assert "avg" in svg
        assert '4.0' in svg

    def test_custom_size(self):
        svg = render_radar_chart({"novelty": 3, "leverage": 4, "evidence": 5}, size=400)
        assert 'viewBox="0 0 400 400"' in svg

    def test_does_not_raise_on_mixed_valid_invalid(self):
        scores = {
            "novelty": 3,
            "leverage": 0,   # out of range
            "evidence": 5,
            "cost": 99,      # out of range
            "moat": 3,
            "adoption": 4,
        }
        # Should only include novelty/evidence/moat/adoption
        svg = render_radar_chart(scores)
        assert "创新性" in svg
        assert "实验证据" in svg
        assert "</svg>" in svg


# ── Per-page heatmap ──────────────────────────────────────────────────────────


class TestRenderPageHeatmap:
    """Tests for the per-page confidence heatmap."""

    def test_all_safe_pages(self):
        claims = [
            {"page": 1, "evidence_score": 0.9, "claim_type": "numerical"},
            {"page": 1, "evidence_score": 0.8, "claim_type": "methodology"},
            {"page": 2, "evidence_score": 0.85, "claim_type": "descriptive"},
        ]
        svg_out = _render_page_heatmap(claims, [])
        assert "安全" in svg_out
        assert "100%" in svg_out
        assert "🟢" in svg_out

    def test_all_high_risk_pages(self):
        unverified = [
            {"page": 1, "evidence_score": 0.1, "claim_type": "numerical"},
            {"page": 2, "evidence_score": 0.0, "claim_type": "methodology"},
        ]
        out = _render_page_heatmap([], unverified)
        assert "高危" in out
        assert "🔴" in out
        assert "0%" in out

    def test_mixed_pages(self):
        claims = [
            {"page": 1, "evidence_score": 0.9, "claim_type": "numerical"},
            {"page": 2, "evidence_score": 0.6, "claim_type": "descriptive"},
        ]
        unverified = [
            {"page": 2, "evidence_score": 0.1, "claim_type": "numerical"},
            {"page": 3, "evidence_score": 0.2, "claim_type": "methodology"},
        ]
        out = _render_page_heatmap(claims, unverified)
        # Page 1: 1 verified, 0 unverified → 100% safe
        assert "100%" in out
        # Page 3: 0 verified, 1 unverified → 0% high risk
        assert "0%" in out

    def test_empty_returns_empty_string(self):
        assert _render_page_heatmap([], []) == ""
        assert _render_page_heatmap([], []) == ""

    def test_page_without_claims_not_shown(self):
        # Only pages with claims/unverified should appear
        out = _render_page_heatmap([], [])
        assert "1" not in out or out == ""

    def test_fills_gaps_between_min_and_max_pages(self):
        claims = [
            {"page": 1, "evidence_score": 0.9, "claim_type": "numerical"},
            {"page": 5, "evidence_score": 0.8, "claim_type": "methodology"},
        ]
        unverified = []
        out = _render_page_heatmap(claims, unverified)
        # Pages 1-5 should all appear in the table
        for pg in range(1, 6):
            assert f"| {pg} |" in out, f"Page {pg} missing from heatmap"

    def test_caution_range(self):
        claims = [{"page": 1, "evidence_score": 0.6, "claim_type": "descriptive"}]
        unverified = [{"page": 1, "evidence_score": 0.2, "claim_type": "numerical"}]
        out = _render_page_heatmap(claims, unverified)
        assert "存疑" in out
        assert "🟡" in out
        assert "50%" in out


# ── Claims section builder ────────────────────────────────────────────────────


class TestBuildClaimsSection:
    """Tests for _build_claims_section."""

    def test_empty_claims_data_returns_empty(self):
        assert _build_claims_section(None) == ""
        assert _build_claims_section({}) == ""
        assert _build_claims_section({"claims": [], "unverified_claims": []}) == ""

    def test_summary_table_present(self):
        claims_data = {
            "claims": [
                {"page": 1, "evidence_score": 0.9, "claim_type": "numerical",
                 "chunk_text": "准确率95%"},
            ],
            "unverified_claims": [
                {"page": 2, "evidence_score": 0.1, "claim_type": "numerical",
                 "chunk_text": "准确率99%", "verification_note": "no match"},
            ],
        }
        out = _build_claims_section(claims_data)
        assert "引用验证摘要" in out
        assert "✅ 已验证" in out
        assert "⚠️ 未验证" in out

    def test_heatmap_rendered_in_section(self):
        claims_data = {
            "claims": [
                {"page": 1, "evidence_score": 0.9, "claim_type": "numerical",
                 "chunk_text": "test"},
                {"page": 3, "evidence_score": 0.1, "claim_type": "numerical",
                 "chunk_text": "test"},
            ],
            "unverified_claims": [],
        }
        out = _build_claims_section(claims_data)
        assert "分页置信热图" in out
        assert "📍" in out

    def test_verified_claims_include_type_icon(self):
        claims_data = {
            "claims": [
                {"page": 1, "evidence_score": 0.9, "claim_type": "numerical",
                 "chunk_text": "准确率95%"},
            ],
            "unverified_claims": [],
        }
        out = _build_claims_section(claims_data)
        assert "📊" in out  # numerical icon

    def test_unverified_claims_include_verification_note(self):
        claims_data = {
            "claims": [],
            "unverified_claims": [
                {"page": 1, "evidence_score": 0.0, "claim_type": "methodology",
                 "chunk_text": "使用Transformer", "verification_note": "no match"},
            ],
        }
        out = _build_claims_section(claims_data)
        assert "未验证原因" in out


# ── AI block builder ──────────────────────────────────────────────────────────


class TestBuildAIBlock:
    """Tests for _build_ai_block."""

    def test_empty_returns_empty(self):
        assert _build_ai_block(None, "") == ""

    def test_raw_ai_draft_fallback(self):
        out = _build_ai_block(None, "This is an AI draft.")
        assert "AI Draft" in out
        assert "This is an AI draft." in out

    def test_parsed_ai_shows_raw_section(self):
        sections = {"__raw__": "Generated content from LLM."}
        rubric = {}
        out = _build_ai_block((sections, rubric), "")
        assert "Generated content from LLM." in out


# ── Rubric score extraction ────────────────────────────────────────────────────


class TestExtractRubricScores:
    """Tests for _extract_rubric_scores."""

    def test_extracts_valid_integer_scores(self):
        rubric = {
            "novelty": 4,
            "leverage": 3,
            "evidence": 5,
            "cost": 2,
            "moat": 3,
            "adoption": 2,
        }
        scores = _extract_rubric_scores(rubric)
        assert scores == {
            "novelty": 4, "leverage": 3, "evidence": 5,
            "cost": 2, "moat": 3, "adoption": 2,
        }

    def test_ignores_out_of_range(self):
        rubric = {
            "novelty": 4,
            "leverage": 99,
            "evidence": 0,
        }
        scores = _extract_rubric_scores(rubric)
        assert scores == {"novelty": 4}

    def test_ignores_non_integer(self):
        rubric = {
            "novelty": 4,
            "leverage": "high",
            "evidence": "five",     # not int
        }
        scores = _extract_rubric_scores(rubric)
        assert scores == {"novelty": 4}


# ── Cross-paper comparison ────────────────────────────────────────────────────


class TestRenderCrossPaperComparison:
    """Tests for _render_cross_paper_comparison."""

    def test_empty_dir_returns_empty(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        assert _render_cross_paper_comparison("test", None, analysis_dir=tmp_path) == ""

    def test_single_paper_returns_empty(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        (tmp_path / "paper_x").mkdir()
        (tmp_path / "paper_x" / "paper_analysis.json").write_text(
            '{"paper_id":"paper_x","claims":[],"unverified_claims":[]}',
            encoding="utf-8",
        )
        assert _render_cross_paper_comparison("paper_x", None, analysis_dir=tmp_path) == ""

    def test_two_papers_renders_table(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        # Paper A: 3 verified, 1 unverified → 75%
        (tmp_path / "paper_a").mkdir()
        (tmp_path / "paper_a" / "paper_analysis.json").write_text(
            '{"paper_id":"paper_a","claims":[{},{},{}],'
            '"unverified_claims":[{}],"rubric":{}}',
            encoding="utf-8",
        )
        # Paper B: 5 verified, 0 unverified → 100%
        (tmp_path / "paper_b").mkdir()
        (tmp_path / "paper_b" / "paper_analysis.json").write_text(
            '{"paper_id":"paper_b","claims":[{},{},{},{},{}],'
            '"unverified_claims":[],"rubric":{}}',
            encoding="utf-8",
        )
        out = _render_cross_paper_comparison("paper_a", None, analysis_dir=tmp_path)
        assert "跨论文引用验证对比" in out
        assert "paper_a" in out
        assert "paper_b" in out
        assert "100%" in out
        assert "75%" in out

    def test_current_paper_marked_bold(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        (tmp_path / "cur").mkdir()
        (tmp_path / "cur" / "paper_analysis.json").write_text(
            '{"paper_id":"cur","claims":[{}],'
            '"unverified_claims":[{}],"rubric":{}}',
            encoding="utf-8",
        )
        (tmp_path / "other").mkdir()
        (tmp_path / "other" / "paper_analysis.json").write_text(
            '{"paper_id":"other","claims":[{},{}],'
            '"unverified_claims":[],"rubric":{}}',
            encoding="utf-8",
        )
        out = _render_cross_paper_comparison("cur", None, analysis_dir=tmp_path)
        assert "**" in out  # current paper gets bold

    def test_colour_coding(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        # High rate → 🟢
        (tmp_path / "high").mkdir()
        (tmp_path / "high" / "paper_analysis.json").write_text(
            '{"paper_id":"high","claims":[{}],'
            '"unverified_claims":[],"rubric":{}}',
            encoding="utf-8",
        )
        # Low rate → 🔴
        (tmp_path / "low").mkdir()
        (tmp_path / "low" / "paper_analysis.json").write_text(
            '{"paper_id":"low","claims":[],'
            '"unverified_claims":[{},{},{}],"rubric":{}}',
            encoding="utf-8",
        )
        out = _render_cross_paper_comparison("high", None, analysis_dir=tmp_path)
        assert "🟢" in out
        assert "🔴" in out

    def test_uses_rubric_overall_as_label(self, tmp_path):
        from renderers.pnote import _render_cross_paper_comparison
        (tmp_path / "p1").mkdir()
        (tmp_path / "p1" / "paper_analysis.json").write_text(
            '{"paper_id":"p1","claims":[{}],'
            '"unverified_claims":[{}],'
            '"rubric":{"overall":"This paper proposes a solid framework"}}',
            encoding="utf-8",
        )
        (tmp_path / "p2").mkdir()
        (tmp_path / "p2" / "paper_analysis.json").write_text(
            '{"paper_id":"p2","claims":[{},{}],'
            '"unverified_claims":[],"rubric":{"overall":""}}',
            encoding="utf-8",
        )
        out = _render_cross_paper_comparison("p1", None, analysis_dir=tmp_path)
        assert "solid framework" in out
