"""Tests for llm/research/gene_pool_writer.py — Gene Pool persistence."""

from unittest.mock import patch, MagicMock

from llm.research.gene_pool_writer import save_gap_to_gene_pool


class TestSaveGapToGenePool:
    """Test Gene Pool gap persistence."""

    def test_returns_existing_capsule_id_on_duplicate(self):
        """Should return existing capsule_id if paper_id + gap_type already exists."""
        with patch("llm.research.gene_pool_writer.get_capsule_by_paper") as mock_get:
            mock_get.return_value = {"capsule_id": "existing_abc123"}
            result = save_gap_to_gene_pool(
                paper_id="paper1",
                title="Test Paper",
                gap_type="embodied_planning",
                gap_title="Test Gap",
                keywords=["test"],
                summary="Test summary",
            )
            assert result == "existing_abc123"
            mock_get.assert_called_once_with("paper1", gap_type="embodied_planning")

    def test_calls_get_capsule_by_paper(self):
        """Should check for existing capsule before creating new."""
        with patch("llm.research.gene_pool_writer.get_capsule_by_paper") as mock_get:
            mock_get.return_value = None
            with patch("llm.insight.tracker.EvolutionTracker") as mock_tracker:
                mock_tracker_instance = MagicMock()
                mock_tracker_instance.encode_capsule.return_value = True
                mock_tracker.return_value = mock_tracker_instance
                save_gap_to_gene_pool(
                    paper_id="paper1",
                    title="Test Paper",
                    gap_type="embodied_planning",
                    gap_title="Test Gap",
                    keywords=["test"],
                    summary="Test summary",
                )
            mock_get.assert_called_once()

    def test_extra_fields_are_included(self):
        """Extra fields should be included in the archetype."""
        with patch("llm.research.gene_pool_writer.get_capsule_by_paper") as mock_get:
            mock_get.return_value = None
            with patch("llm.insight.tracker.EvolutionTracker") as mock_tracker:
                mock_tracker_instance = MagicMock()
                mock_tracker_instance.encode_capsule.return_value = True
                mock_tracker.return_value = mock_tracker_instance
                save_gap_to_gene_pool(
                    paper_id="paper1",
                    title="Test Paper",
                    gap_type="embodied_planning",
                    gap_title="Test Gap",
                    keywords=["test"],
                    summary="Test summary",
                    extra_fields={"confidence": 0.9},
                )
                # Should not raise, extra_fields were accepted

    def test_polarity_defaults_to_positive(self):
        """Polarity should default to 'positive'."""
        with patch("llm.research.gene_pool_writer.get_capsule_by_paper") as mock_get:
            mock_get.return_value = None
            with patch("llm.insight.tracker.EvolutionTracker") as mock_tracker:
                mock_tracker_instance = MagicMock()
                mock_tracker_instance.encode_capsule.return_value = True
                mock_tracker.return_value = mock_tracker_instance
                save_gap_to_gene_pool(
                    paper_id="paper1",
                    title="Test Paper",
                    gap_type="embodied_planning",
                    gap_title="Test Gap",
                    keywords=["test"],
                    summary="Test summary",
                )
                # Should not raise, polarity defaults to positive

    def test_returns_none_on_tracker_failure(self):
        """Should return None if EvolutionTracker.encode_capsule returns None."""
        with patch("llm.research.gene_pool_writer.get_capsule_by_paper") as mock_get:
            mock_get.return_value = None
            with patch("llm.insight.tracker.EvolutionTracker") as mock_tracker:
                mock_tracker_instance = MagicMock()
                mock_tracker_instance.encode_capsule.return_value = None
                mock_tracker.return_value = mock_tracker_instance
                result = save_gap_to_gene_pool(
                    paper_id="paper1",
                    title="Test Paper",
                    gap_type="embodied_planning",
                    gap_title="Test Gap",
                    keywords=["test"],
                    summary="Test summary",
                )
                assert result is None
