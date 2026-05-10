"""Tests for llm/events.py — event processing pipeline."""

from unittest.mock import patch, MagicMock

from llm.events import (
    HIGH_IMPACT_KEYWORDS,
    process_event,
    _build_summary,
    _infer_gap_type,
    _fetch_event_news,
    _find_related_papers,
    _try_search_arxiv,
    _try_search_crossref,
    _try_search_semantic_scholar,
    render_event_report,
)


class TestHighImpactKeywords:
    """Test HIGH_IMPACT_KEYWORDS list."""

    def test_contains_military_keywords(self):
        """Should contain military conflict keywords."""
        for kw in ["导弹", "drone", "missile"]:
            assert kw in HIGH_IMPACT_KEYWORDS

    def test_contains_economic_keywords(self):
        """Should contain economic keywords."""
        for kw in ["利率", "inflation", "Fed"]:
            assert kw in HIGH_IMPACT_KEYWORDS

    def test_contains_energy_keywords(self):
        """Should contain energy keywords."""
        for kw in ["石油", "oil"]:
            assert kw in HIGH_IMPACT_KEYWORDS


class TestBuildSummary:
    """Test _build_summary()."""

    def test_extracts_keywords(self):
        """Should extract top keywords from news items."""
        news_items = [
            {"content": "伊朗发射导弹袭击沙特石油设施，导致油价飙升"},
            {"content": "伊朗导弹精准命中目标，市场担忧供应中断"},
        ]
        summary = _build_summary(news_items, "伊朗")
        assert "keywords" in summary
        assert "primary_keyword" in summary
        assert isinstance(summary["keywords"], list)

    def test_handles_empty_news(self):
        """Should handle empty news list with keyword as fallback."""
        summary = _build_summary([], "test")
        # When news is empty, keyword or "event" depending on expression structure
        assert "primary_keyword" in summary
        assert summary["capsule_title"] == "Event: test"

    def test_returns_timestamp(self):
        """Should include ISO timestamp."""
        summary = _build_summary([{"content": "test content"}], "test")
        assert "timestamp" in summary

    def test_capsule_title_from_first_item(self):
        """Capsule title should come from first item content."""
        news_items = [{"content": "Breaking: major development in Iran"}]
        summary = _build_summary(news_items, "")
        assert "Breaking" in summary["capsule_title"]

    def test_handles_non_dict_items(self):
        """Should handle non-dict items gracefully."""
        summary = _build_summary(["plain string item", 123], "test")
        assert "brief" in summary


class TestInferGapType:
    """Test _infer_gap_type()."""

    def test_military_returns_scalability_issue(self):
        """Military keywords should return scalability_issue."""
        summary = {"brief": "伊朗导弹袭击沙特石油设施，军事冲突升级"}
        gap = _infer_gap_type(summary)
        assert gap == "scalability_issue"

    def test_missile_keyword(self):
        """Missile keyword triggers scalability_issue."""
        summary = {"brief": "A drone missile strike occurred"}
        gap = _infer_gap_type(summary)
        assert gap == "scalability_issue"

    def test_oil_returns_evaluation_gap(self):
        """Oil/energy keywords should return evaluation_gap."""
        summary = {"brief": "石油供应担忧导致油价飙升"}
        gap = _infer_gap_type(summary)
        assert gap == "evaluation_gap"

    def test_energy_keyword(self):
        """Energy keyword triggers evaluation_gap."""
        summary = {"brief": "global oil energy crisis"}
        gap = _infer_gap_type(summary)
        assert gap == "evaluation_gap"

    def test_rate_returns_method_limitation(self):
        """Rate/inflation keywords should return method_limitation."""
        summary = {"brief": "美联储加息导致利率上升"}
        gap = _infer_gap_type(summary)
        assert gap == "method_limitation"

    def test_inflation_keyword(self):
        """Inflation keyword triggers method_limitation."""
        summary = {"brief": "inflation rate concerns Fed"}
        gap = _infer_gap_type(summary)
        assert gap == "method_limitation"

    def test_default_returns_unexplored_application(self):
        """Unknown content returns unexplored_application."""
        summary = {"brief": "something completely unrelated here"}
        gap = _infer_gap_type(summary)
        assert gap == "unexplored_application"


class TestFetchEventNews:
    """Test _fetch_event_news()."""

    def test_returns_empty_on_empty_keyword(self):
        """Empty keyword returns empty list."""
        with patch("llm.events.Jin10Client") as mock_client:
            client = mock_client.return_value
            client.search_flash.return_value = {"data": {"items": []}}
            result = _fetch_event_news(client, "", 5)
            assert result == []

    def test_searches_with_keyword(self):
        """Should call client.search_flash with keyword."""
        with patch("llm.events.Jin10Client") as mock_client:
            client = mock_client.return_value
            client.search_flash.return_value = {"data": {"items": [{"content": "test"}]}}
            _fetch_event_news(client, "伊朗", 5)
            client.search_flash.assert_called_once_with("伊朗")

    def test_limits_results(self):
        """Should limit to max_news items."""
        with patch("llm.events.Jin10Client") as mock_client:
            client = mock_client.return_value
            client.search_flash.return_value = {
                "data": {"items": [{"content": f"item{i}"} for i in range(10)]}
            }
            result = _fetch_event_news(client, "test", 3)
            assert len(result) == 3

    def test_handles_dict_response(self):
        """Should handle dict response with data.items."""
        with patch("llm.events.Jin10Client") as mock_client:
            client = mock_client.return_value
            client.search_flash.return_value = {"data": {"items": [{"content": "test"}]}}
            result = _fetch_event_news(client, "test", 5)
            assert len(result) == 1

    def test_handles_list_response(self):
        """Should handle list response directly."""
        with patch("llm.events.Jin10Client") as mock_client:
            client = mock_client.return_value
            client.search_flash.return_value = [{"content": "test1"}, {"content": "test2"}]
            result = _fetch_event_news(client, "test", 5)
            assert len(result) == 2


class TestTrySearchArxiv:
    """Test _try_search_arxiv()."""

    def test_returns_search_results(self):
        """Should return papers from search_arxiv."""
        mock_paper = MagicMock()
        mock_paper.uid = "2301.00001"
        mock_paper.title = "Test Paper"
        with patch("llm.events.search_arxiv", return_value=[mock_paper]):
            result = _try_search_arxiv("transformer", 3)
            assert len(result) == 1

    def test_returns_empty_on_error(self):
        """Should return empty list on exception."""
        with patch("llm.events.search_arxiv", side_effect=Exception("network error")):
            result = _try_search_arxiv("transformer", 3)
            assert result == []

    def test_retries_on_429(self):
        """Should retry with backoff on 429 rate limit."""
        mock_paper = MagicMock()
        mock_paper.uid = "2301.00001"
        import time as _time

        with (
            patch("llm.events.search_arxiv") as mock_search,
            patch.object(_time, "sleep") as mock_sleep,
        ):
            mock_search.side_effect = [
                Exception("Error 429"),
                Exception("Error 429"),
                [mock_paper],
            ]
            result = _try_search_arxiv("transformer", 3)
            assert len(result) == 1
            assert mock_sleep.call_count == 2


class TestTrySearchCrossref:
    """Test _try_search_crossref()."""

    def test_returns_empty_on_error(self):
        """Should return empty list when API fails."""
        with patch("urllib.request.urlopen", side_effect=Exception("network error")):
            result = _try_search_crossref("test", 3)
            assert result == []

    def test_handles_urlopen_error(self):
        """Should return empty list when URL open fails."""
        import urllib.error as urlerr

        with patch("urllib.request.urlopen", side_effect=urlerr.URLError("failed")):
            result = _try_search_crossref("test", 3)
            assert result == []


class TestTrySearchSemanticScholar:
    """Test _try_search_semantic_scholar()."""

    def test_returns_empty_on_error(self):
        """Should return empty list when API fails."""
        with patch("urllib.request.urlopen", side_effect=Exception("network error")):
            result = _try_search_semantic_scholar("test", 3)
            assert result == []

    def test_handles_urlopen_error(self):
        """Should return empty list when URL open fails."""
        import urllib.error as urlerr

        with patch("urllib.request.urlopen", side_effect=urlerr.URLError("failed")):
            result = _try_search_semantic_scholar("test", 3)
            assert result == []


class TestFindRelatedPapers:
    """Test _find_related_papers()."""

    def test_tries_arxiv_first(self):
        """Should try arXiv before other backends."""
        mock_paper = MagicMock()
        mock_paper.uid = "2301.00001"
        with patch("llm.events._try_search_arxiv", return_value=[mock_paper]) as mock_arxiv:
            result = _find_related_papers("transformer", 3)
            mock_arxiv.assert_called_once()
            assert len(result) == 1

    def test_falls_back_to_crossref(self):
        """Should try CrossRef when arXiv returns empty."""
        mock_paper = MagicMock()
        mock_paper.uid = "10.1234/test"
        with (
            patch("llm.events._try_search_arxiv", return_value=[]),
            patch("llm.events._try_search_crossref", return_value=[mock_paper]) as mock_cr,
        ):
            result = _find_related_papers("test", 3)
            mock_cr.assert_called_once()
            assert len(result) == 1

    def test_falls_back_to_semantic_scholar(self):
        """Should try Semantic Scholar when arXiv and CrossRef return empty."""
        mock_paper = MagicMock()
        mock_paper.uid = "S2-123"
        with (
            patch("llm.events._try_search_arxiv", return_value=[]),
            patch("llm.events._try_search_crossref", return_value=[]),
            patch("llm.events._try_search_semantic_scholar", return_value=[mock_paper]) as mock_ss,
        ):
            _find_related_papers("test", 3)
            mock_ss.assert_called_once()


class TestProcessEvent:
    """Test process_event() main function."""

    def test_returns_error_when_no_news(self):
        """Should return error dict when no news found."""
        mock_client = MagicMock()
        mock_client.search_flash.return_value = {"data": {"items": []}}
        with patch("llm.events.Jin10Client", return_value=mock_client):
            with patch("llm.events.EvolutionTracker"):
                result = process_event("nonexistent_keyword_xyz", max_news=5)
                assert "error" in result

    def test_returns_event_id_and_capsule_id(self):
        """Should return event_id and capsule_id."""
        mock_capsule = MagicMock()
        mock_capsule.capsule_id = "cap_abc123"
        mock_capsule.action_gap_title = "Test Gap"
        mock_capsule.encode_capsule = MagicMock(return_value=mock_capsule)
        mock_capsule.trigger_match = MagicMock(return_value=0.5)

        mock_paper = MagicMock(spec=["uid", "title"])
        mock_paper.uid = "2301.00001"
        mock_paper.title = "Test Paper"

        mock_client = MagicMock()
        mock_client.search_flash.return_value = {
            "data": {"items": [{"content": "伊朗导弹袭击沙特石油设施，导致油价飙升"}]}
        }
        with patch("llm.events.Jin10Client", return_value=mock_client):
            with patch("llm.events.EvolutionTracker", return_value=mock_capsule):
                with patch("llm.events._find_related_papers", return_value=[mock_paper]):
                    result = process_event("伊朗", max_news=5, max_papers=3)
                    assert "event_id" in result
                    assert "capsule_id" in result

    def test_includes_related_papers(self):
        """Should include related papers with relevance scores."""
        mock_capsule = MagicMock()
        mock_capsule.capsule_id = "cap_abc123"
        mock_capsule.action_gap_title = "Test Gap"
        mock_capsule.encode_capsule = MagicMock(return_value=mock_capsule)
        mock_capsule.trigger_match = MagicMock(return_value=0.5)

        mock_paper = MagicMock(spec=["uid", "title"])
        mock_paper.uid = "2301.00001"
        mock_paper.title = "Test Paper"

        mock_client = MagicMock()
        mock_client.search_flash.return_value = {
            "data": {"items": [{"content": "伊朗导弹袭击沙特"}]}
        }
        with patch("llm.events.Jin10Client", return_value=mock_client):
            with patch("llm.events.EvolutionTracker", return_value=mock_capsule):
                with patch("llm.events._find_related_papers", return_value=[mock_paper]):
                    result = process_event("伊朗")
                    assert "related_papers" in result
                    assert isinstance(result["related_papers"], list)

    def test_filters_low_relevance_papers(self):
        """Should filter out papers with relevance <= 0.1."""
        mock_capsule = MagicMock()
        mock_capsule.capsule_id = "cap_abc123"
        mock_capsule.action_gap_title = "Test Gap"
        mock_capsule.encode_capsule = MagicMock(return_value=mock_capsule)
        mock_capsule.trigger_match = MagicMock(return_value=0.05)

        mock_paper = MagicMock(spec=["uid", "title"])
        mock_paper.uid = "2301.00001"
        mock_paper.title = "Test Paper"

        mock_client = MagicMock()
        mock_client.search_flash.return_value = {"data": {"items": [{"content": "test"}]}}
        with patch("llm.events.Jin10Client", return_value=mock_client):
            with patch("llm.events.EvolutionTracker", return_value=mock_capsule):
                with patch("llm.events._find_related_papers", return_value=[mock_paper]):
                    result = process_event("test")
                    assert len(result["related_papers"]) == 0


class TestRenderEventReport:
    """Test render_event_report()."""

    def test_renders_error(self):
        """Should render error message."""
        result = {"error": "No news found"}
        rendered = render_event_report(result)
        assert "Error" in rendered
        assert "No news found" in rendered

    def test_renders_event_id(self):
        """Should display event ID."""
        result = {
            "event_id": "cap_123",
            "capsule_id": "cap_123",
            "timestamp": "2024-01-01T12:00:00",
            "keywords": ["伊朗", "导弹"],
            "capsule_title": "Test Capsule",
            "related_papers": [],
        }
        rendered = render_event_report(result)
        assert "cap_123" in rendered

    def test_renders_keywords(self):
        """Should display keywords."""
        result = {
            "event_id": "cap_123",
            "timestamp": "2024-01-01T12:00:00",
            "keywords": ["伊朗", "导弹", "石油"],
            "capsule_title": "Test",
            "related_papers": [],
        }
        rendered = render_event_report(result)
        assert "伊朗" in rendered

    def test_renders_related_papers(self):
        """Should list related papers."""
        result = {
            "event_id": "cap_123",
            "timestamp": "2024-01-01T12:00:00",
            "keywords": [],
            "capsule_title": "Test",
            "related_papers": [
                {"paper_id": "2301.00001", "title": "Test Paper", "relevance": 0.75}
            ],
        }
        rendered = render_event_report(result)
        assert "2301.00001" in rendered
        assert "Test Paper" in rendered
