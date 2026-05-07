"""Tests for llm/gene_pool_io.py — Gene Pool I/O operations."""

import pytest
from unittest.mock import patch, MagicMock
import json
import tempfile
from pathlib import Path

from llm.gene_pool_io import (
    get_capsule_by_paper,
    paper_exists_in_pool,
    export_pool,
    import_pool,
    _backup_name,
    _list_backups,
    get_backup_info,
    render_backup_html,
)


class TestGetCapsuleByPaper:
    """Test get_capsule_by_paper lookup function."""

    def test_returns_none_when_no_capsules(self):
        """Should return None when no capsules exist."""
        with patch("llm.gene_pool_io.load_capsules") as mock_load:
            mock_load.return_value = []
            result = get_capsule_by_paper("paper123", gap_type="embodied_planning")
            assert result is None

    def test_finds_existing_capsule_by_paper_id(self):
        """Should find capsule matching paper_id."""
        capsules = [
            {
                "capsule_id": "cap1",
                "created_at": "2026-01-01T00:00:00",
                "archetype": {"source_paper_id": "paper1"},
                "trigger_gap_type": "embodied_planning",
            },
        ]
        with patch("llm.gene_pool_io.load_capsules") as mock_load:
            mock_load.return_value = capsules
            result = get_capsule_by_paper("paper1", gap_type="embodied_planning")
            assert result is not None
            assert result["capsule_id"] == "cap1"

    def test_returns_none_for_missing_paper(self):
        """Should return None when paper_id not found."""
        capsules = [
            {
                "capsule_id": "cap1",
                "created_at": "2026-01-01T00:00:00",
                "archetype": {"source_paper_id": "paper1"},
                "trigger_gap_type": "embodied_planning",
            },
        ]
        with patch("llm.gene_pool_io.load_capsules") as mock_load:
            mock_load.return_value = capsules
            result = get_capsule_by_paper("nonexistent")
            assert result is None


class TestPaperExistsInPool:
    """Test paper_exists_in_pool function."""

    def test_returns_false_when_no_match(self):
        """Should return False when no matching capsule exists."""
        with patch("llm.gene_pool_io.load_capsules") as mock_load:
            mock_load.return_value = []
            result = paper_exists_in_pool("paper123")
            assert result is False

    def test_returns_true_when_match_exists(self):
        """Should return True when capsule with matching paper_id exists."""
        capsules = [
            {
                "capsule_id": "cap1",
                "created_at": "2026-01-01T00:00:00",
                "archetype": {"source_paper_id": "paper1"},
            },
        ]
        with patch("llm.gene_pool_io.load_capsules") as mock_load:
            mock_load.return_value = capsules
            result = paper_exists_in_pool("paper1")
            assert result is True


class TestBackupName:
    """Test backup naming function."""

    def test_backup_name_format(self):
        """Backup name should use gene_pool_{stamp}.tar.gz format."""
        name = _backup_name("20260101_120000")
        assert name == "gene_pool_20260101_120000.tar.gz"

    def test_different_stamps_produce_different_names(self):
        """Different timestamps should produce different names."""
        name1 = _backup_name("20260101_120000")
        name2 = _backup_name("20260101_130000")
        assert name1 != name2


class TestListBackups:
    """Test backup listing function."""

    def test_returns_list(self):
        """Should return a list of backup names."""
        with patch("llm.gene_pool_io.Path") as mock_path:
            mock_instance = MagicMock()
            mock_instance.iterdir.return_value = []
            mock_path.return_value = mock_instance
            result = _list_backups()
            assert isinstance(result, list)


class TestGetBackupInfo:
    """Test backup information retrieval."""

    def test_returns_dict(self):
        """Should return a dictionary with backup info."""
        with patch("llm.gene_pool_io._list_backups") as mock_list:
            mock_list.return_value = ["gene_pool_20260101_120000.tar.gz"]
            with patch("llm.gene_pool_io.Path") as mock_path:
                mock_instance = MagicMock()
                mock_instance.stat.return_value.st_size = 1024
                mock_instance.exists.return_value = True
                mock_path.return_value = mock_instance
                result = get_backup_info()
                assert isinstance(result, dict)

    def test_empty_when_no_backups(self):
        """Should return empty dict when no backups exist."""
        with patch("llm.gene_pool_io._list_backups") as mock_list:
            mock_list.return_value = []
            result = get_backup_info()
            assert isinstance(result, dict)


class TestRenderBackupHtml:
    """Test backup HTML rendering."""

    def test_with_empty_info(self):
        """Should render HTML when info dict is properly structured."""
        info = {
            "available": 0,
            "total_size_mb": "0",
            "backups": [],
            "max_backups": 10,
            "stamps": [],
        }
        result = render_backup_html(info)
        assert isinstance(result, str)
        assert len(result) > 0

    def test_with_backup_data(self):
        """Should render HTML when backup data provided."""
        info = {
            "available": 1,
            "total_size_mb": "1.0",
            "backups": ["gene_pool_20260101.tar.gz"],
            "max_backups": 10,
            "stamps": ["20260101"],
        }
        result = render_backup_html(info)
        assert isinstance(result, str)
        assert len(result) > 0
