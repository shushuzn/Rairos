"""Tests for _notification_store in web.shared.

The notification store was moved to web/shared.py to break a circular import
that prevented web/app.py from loading. These tests verify the store works.
"""


class TestNotificationStore:
    """Test _notification_store behavior via web.shared (not web.app)."""

    def _get_store(self):
        from web.shared import _notification_store
        return _notification_store

    def test_store_starts_as_list(self):
        store = self._get_store()
        assert isinstance(store, list)

    def test_store_is_mutable_list(self):
        store = self._get_store()
        original_len = len(store)

        test_notification = {"uid": "test-001", "type": "alert", "message": "Test"}
        store.append(test_notification)
        assert len(store) == original_len + 1
        assert store[-1] == test_notification

        store.remove(test_notification)
        assert len(store) == original_len

    def test_notification_dict_structure(self):
        store = self._get_store()
        original_len = len(store)

        notification = {
            "uid": "test-002",
            "type": "contradiction",
            "message": "Method A contradicts Method B",
            "severity": "high",
        }
        store.append(notification)
        assert store[-1]["uid"] == "test-002"
        assert store[-1]["type"] == "contradiction"
        store.remove(notification)

        assert len(store) == original_len

    def test_clear_all_notifications(self):
        store = self._get_store()
        original_len = len(store)

        store.append({"uid": "test-clear-1", "type": "alert", "message": "A"})
        store.append({"uid": "test-clear-2", "type": "trend", "message": "B"})

        assert len(store) == original_len + 2

        store.clear()
        assert len(store) == 0

    def test_filter_notifications_by_uid(self):
        store = self._get_store()
        original_len = len(store)

        store.append({"uid": "filter-1", "type": "alert", "message": "X"})
        store.append({"uid": "filter-2", "type": "trend", "message": "Y"})
        store.append({"uid": "filter-3", "type": "alert", "message": "Z"})

        filtered = [n for n in store if n.get("uid", "").startswith("filter-")]
        assert len(filtered) == 3

        store[:] = [n for n in store if n.get("uid") != "filter-2"]
        remaining_uids = {n.get("uid") for n in store}
        assert "filter-2" not in remaining_uids
        assert "filter-1" in remaining_uids
        assert "filter-3" in remaining_uids

        store.clear()
        assert len(store) == original_len


class TestRenderGapAnalysisHtml:
    """Test _render_gap_analysis_html in web.shared."""

    def _render(self, result, papers):
        from web.shared import _render_gap_analysis_html
        return _render_gap_analysis_html(result, papers)

    def test_empty_result_shows_empty_message(self):
        html = self._render({}, [])
        assert "No gaps identified" in html

    def test_error_result_shows_error(self):
        html = self._render({"error": "Something went wrong"}, [])
        assert "Error: Something went wrong" in html

    def test_shared_themes_section(self):
        result = {
            "shared_themes": [
                {
                    "theme": "Scaling laws",
                    "papers": ["2201.00001", "2201.00002"],
                    "strength": "strong",
                    "description": "Both papers study scaling behavior",
                }
            ]
        }
        html = self._render(result, [
            {"id": "2201.00001", "title": "Paper A"},
            {"id": "2201.00002", "title": "Paper B"},
        ])
        assert "Scaling laws" in html
        assert "Shared Themes (1)" in html

    def test_frontier_gaps_section(self):
        result = {
            "frontier_gaps": [
                {
                    "gap_title": "Efficient fine-tuning",
                    "gap_type": "method_limitation",
                    "keywords": ["LoRA", "adapter"],
                    "summary": "Need better adapters",
                }
            ]
        }
        html = self._render(result, [])
        assert "Efficient fine-tuning" in html
        assert "Frontier Gaps (1)" in html
        assert "method_limitation" in html

    def test_contradictions_section(self):
        result = {
            "contradictions": [
                {
                    "gap_type": "contradiction",
                    "description": "Method A outperforms B on task X but not Y",
                }
            ]
        }
        html = self._render(result, [])
        assert "Contradictions (1)" in html
        assert "Method A outperforms B" in html

    def test_multiple_sections(self):
        result = {
            "shared_themes": [{"theme": "Theme A", "papers": [], "strength": "weak", "description": ""}],
            "frontier_gaps": [{"gap_title": "Gap A", "gap_type": "other", "keywords": [], "summary": ""}],
            "contradictions": [{"gap_type": "contradiction", "description": "C1"}],
        }
        html = self._render(result, [])
        assert "Theme A" in html
        assert "Gap A" in html
        assert "C1" in html
        assert "ga-section" in html


class TestRenderRQHtml:
    """Test _render_rq_html in web.shared."""

    def _render(self, result, frontier_gaps, paper_titles):
        from web.shared import _render_rq_html
        return _render_rq_html(result, frontier_gaps, paper_titles)

    def test_empty_questions_shows_empty_message(self):
        html = self._render({}, [], {})
        assert "No questions generated" in html

    def test_questions_rendered_with_difficulty(self):
        result = {
            "questions": [
                {
                    "question": "How does scaling affect robustness?",
                    "difficulty": "hard",
                    "gap_title": "Scaling gap",
                    "gap_type": "theoretical_gap",
                    "keywords": ["scaling", "robustness"],
                    "hypothesis": "Larger models are more robust",
                }
            ]
        }
        html = self._render(result, [], {})
        assert "How does scaling affect robustness?" in html
        assert "HARD" in html
        assert "Hypothesis:" in html

    def test_multiple_questions_numbered(self):
        result = {
            "questions": [
                {"question": "Q1", "difficulty": "easy", "gap_title": "", "gap_type": "", "keywords": [], "hypothesis": ""},
                {"question": "Q2", "difficulty": "medium", "gap_title": "", "gap_type": "", "keywords": [], "hypothesis": ""},
            ]
        }
        html = self._render(result, [], {})
        assert "Q1" in html
        assert "Q2" in html
        assert "rq-list" in html
