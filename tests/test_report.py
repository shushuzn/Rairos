"""Tests for llm/report.py — research report generation."""

import pytest
from llm.report import _find, generate, save


class FakeCapsule:
    """Minimal fake for InsightCapsule-like objects."""
    def __init__(self, capsule_id, action_gap_title, trigger_topic, source_arxiv_category,
                 outcome_success_score=0.5, action_gap_type="method_limitation", credibility_badge="medium"):
        self.capsule_id = capsule_id
        self.action_gap_title = action_gap_title
        self.trigger_topic = trigger_topic
        self.source_arxiv_category = source_arxiv_category
        self.outcome_success_score = outcome_success_score
        self.action_gap_type = action_gap_type
        self.credibility_badge = credibility_badge


class TestFind:
    def test_empty_capsules(self):
        result = _find([], ["robot"])
        assert result == []

    def test_no_match(self):
        caps = [
            FakeCapsule("c1", "A neural method", "deep learning", "cs.LG"),
        ]
        result = _find(caps, ["robot", "embodied"])
        assert result == []

    def test_single_match_by_title(self):
        caps = [
            FakeCapsule("c1", "Robot navigation with VLA", "robotics", "cs.RO"),
        ]
        result = _find(caps, ["robot"])
        assert len(result) == 1
        assert result[0].capsule_id == "c1"

    def test_single_match_by_topic(self):
        caps = [
            FakeCapsule("c1", "Something about RL", "reinforcement learning", "cs.LG"),
        ]
        result = _find(caps, ["reinforcement"])
        assert len(result) == 1

    def test_case_insensitive(self):
        caps = [
            FakeCapsule("c1", "ROBOT navigation", "Embodied AI", "cs.RO"),
        ]
        result = _find(caps, ["robot", "embodied"])
        assert len(result) == 1

    def test_excludes_already_listed(self):
        caps = [
            FakeCapsule("c1", "Robot task", "robotics", "cs.RO"),
            FakeCapsule("c2", "Another robot task", "robotics", "cs.RO"),
        ]
        used = {"c1"}
        result = _find(caps, ["robot"], used)
        assert len(result) == 1
        assert result[0].capsule_id == "c2"

    def test_exclude_empty_set(self):
        caps = [FakeCapsule("c1", "Robot task", "robotics", "cs.RO")]
        result = _find(caps, ["robot"], set())
        assert len(result) == 1

    def test_exclude_none(self):
        caps = [FakeCapsule("c1", "Robot task", "robotics", "cs.RO")]
        result = _find(caps, ["robot"], None)
        assert len(result) == 1

    def test_multiple_matches(self):
        caps = [
            FakeCapsule("c1", "Robot task 1", "robotics", "cs.RO"),
            FakeCapsule("c2", "Robot task 2", "robotics", "cs.RO"),
        ]
        result = _find(caps, ["robot"])
        assert len(result) == 2


class TestGenerate:
    def test_generate_empty_tracker(self, monkeypatch):
        class FakeCapsule:
            capsule_id = "empty"
            action_gap_title = ""
            trigger_topic = ""
            source_arxiv_category = ""
            outcome_success_score = 0.0
            action_gap_type = "method_limitation"
            credibility_badge = "low"

        class FakeTracker:
            def _load_capsules(self):
                return [FakeCapsule()]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert isinstance(output, str)
        assert "RAIROS RESEARCH REPORT" in output
        assert "=" in output

    def test_generate_separates_research_from_events(self, monkeypatch):
        class FakeCapsule:
            def __init__(self, cid, cat):
                self.capsule_id = cid
                self.action_gap_title = f"Gap {cid}"
                self.trigger_topic = ""
                self.source_arxiv_category = cat
                self.outcome_success_score = 0.5
                self.action_gap_type = "method_limitation"
                self.credibility_badge = "medium"

        class FakeTracker:
            def _load_capsules(self):
                return [
                    FakeCapsule("r1", "cs.LG"),
                    FakeCapsule("r2", "cs.RO"),
                    FakeCapsule("e1", "cs.GL"),
                    FakeCapsule("e2", "cs.GL"),
                ]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert "Total: 4 capsules (2 research, 2 events)" in output

    def test_generate_vla_robotics_theme(self, monkeypatch):
        class FakeCapsule:
            def __init__(self, cid, title, topic, score):
                self.capsule_id = cid
                self.action_gap_title = title
                self.trigger_topic = topic
                self.source_arxiv_category = "cs.RO"
                self.outcome_success_score = score
                self.action_gap_type = "method_limitation"
                self.credibility_badge = "high"

        class FakeTracker:
            def _load_capsules(self):
                return [
                    FakeCapsule("v1", "VLA for manipulation", "robotics", 0.9),
                    FakeCapsule("v2", "Diffusion policy for robots", "embodied", 0.8),
                ]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert "VLA / ROBOTICS" in output
        assert "VLA for manipulation" in output
        assert "Diffusion policy" in output

    def test_generate_stats_section(self, monkeypatch):
        class FakeCapsule:
            def __init__(self, cid, gap_type, badge, score):
                self.capsule_id = cid
                self.action_gap_title = f"Gap {cid}"
                self.trigger_topic = ""
                self.source_arxiv_category = "cs.LG"
                self.action_gap_type = gap_type
                self.credibility_badge = badge
                self.outcome_success_score = score

        class FakeTracker:
            def _load_capsules(self):
                return [
                    FakeCapsule("c1", "method_limitation", "high", 0.9),
                    FakeCapsule("c2", "method_limitation", "low", 0.3),
                    FakeCapsule("c3", "evaluation_gap", "high", 0.85),
                ]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert "STATS" in output
        assert "Total: 3 capsules" in output
        assert "High credibility: 2" in output

    def test_generate_remaining_research_other_research_section(self, monkeypatch):
        """Capsules not matching any theme go to OTHER RESEARCH."""
        class FakeCapsule:
            def __init__(self, cid, title):
                self.capsule_id = cid
                self.action_gap_title = title
                self.trigger_topic = "misc"
                self.source_arxiv_category = "cs.LG"
                self.outcome_success_score = 0.5
                self.action_gap_type = "method_limitation"
                self.credibility_badge = "medium"

        class FakeTracker:
            def _load_capsules(self):
                return [FakeCapsule("x1", "Unmatched research topic")]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert "OTHER RESEARCH" in output
        assert "Unmatched research topic" in output

    def test_generate_events_deduplication(self, monkeypatch):
        """Events with same title (first 50 chars) should be deduplicated."""
        class FakeCapsule:
            def __init__(self, cid, title, cat):
                self.capsule_id = cid
                self.action_gap_title = title
                self.trigger_topic = ""
                self.source_arxiv_category = cat
                self.outcome_success_score = 0.5
                self.action_gap_type = "method_limitation"
                self.credibility_badge = "medium"

        class FakeTracker:
            def _load_capsules(self):
                return [
                    FakeCapsule("e1", "Breaking: Big AI announcement today!!!", "cs.GL"),
                    FakeCapsule("e2", "Breaking: Big AI announcement today!!!", "cs.GL"),
                ]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        assert output.count("Breaking: Big AI announcement") == 1

    def test_generate_sorts_by_score_descending(self, monkeypatch):
        class FakeCapsule:
            def __init__(self, cid, title, score):
                self.capsule_id = cid
                self.action_gap_title = title
                self.trigger_topic = "robot"
                self.source_arxiv_category = "cs.RO"
                self.outcome_success_score = score
                self.action_gap_type = "method_limitation"
                self.credibility_badge = "medium"

        class FakeTracker:
            def _load_capsules(self):
                return [
                    FakeCapsule("low", "Low score task", 0.2),
                    FakeCapsule("high", "High score task", 0.95),
                    FakeCapsule("mid", "Mid score task", 0.6),
                ]

        import llm.report as report_module
        monkeypatch.setattr(report_module, "EvolutionTracker", FakeTracker)

        output = generate()
        idx_high = output.index("High score task")
        idx_mid = output.index("Mid score task")
        idx_low = output.index("Low score task")
        assert idx_high < idx_mid < idx_low


class TestSave:
    def test_save_calls_generate(self, monkeypatch, tmp_path):
        import llm.report as report_module
        captured = {}

        def fake_generate():
            captured["called"] = True
            return "FAKE REPORT OUTPUT"

        monkeypatch.setattr(report_module, "generate", fake_generate)

        old_cwd = __import__("os").getcwd()
        try:
            __import__("os").chdir(tmp_path)
            result = save()
        finally:
            __import__("os").chdir(old_cwd)

        assert captured["called"] is True
        assert result == "SITUATION_REPORT.md"
        report_file = tmp_path / "SITUATION_REPORT.md"
        assert report_file.read_text(encoding="utf-8") == "FAKE REPORT OUTPUT"
