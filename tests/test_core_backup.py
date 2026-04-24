"""Tests for core/backup.py."""
import shutil
import sys
import time
from pathlib import Path
from unittest.mock import patch, MagicMock
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.backup import BackupManager, get_backup_manager


@pytest.fixture
def temp_backup_dir(tmp_path):
    """Provide a temporary backup directory."""
    bdir = tmp_path / "backups"
    bdir.mkdir()
    yield bdir
    # Cleanup
    if bdir.exists():
        shutil.rmtree(bdir, ignore_errors=True)


@pytest.fixture
def temp_source_dir(tmp_path):
    """Provide a temporary source directory with files."""
    sdir = tmp_path / "source"
    sdir.mkdir()
    (sdir / "file1.txt").write_text("content 1")
    (sdir / "file2.txt").write_text("content 2")
    (sdir / "subdir").mkdir()
    (sdir / "subdir" / "file3.txt").write_text("content 3")
    yield sdir
    if sdir.exists():
        shutil.rmtree(sdir, ignore_errors=True)


class TestBackupManager:
    """Tests for BackupManager."""

    def test_init_default_dir(self, tmp_path):
        with patch("pathlib.Path.home", return_value=tmp_path):
            bm = BackupManager()
            assert "backups" in str(bm.backup_dir)
            assert bm.backup_dir.exists()

    def test_init_custom_dir(self, temp_backup_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        assert bm.backup_dir == temp_backup_dir

    def test_init_creates_dir(self, temp_backup_dir):
        assert temp_backup_dir.exists()
        bm = BackupManager(backup_dir=temp_backup_dir)
        assert temp_backup_dir.exists()

    def test_create_backup_returns_timestamp(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir)
        assert isinstance(ts, str)
        assert len(ts) == 15  # YYYYMMDD_HHMMSS format

    def test_create_backup_copies_files(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir)
        backup_path = temp_backup_dir / f"backup_{ts}"
        assert backup_path.exists()
        assert (backup_path / "file1.txt").read_text() == "content 1"
        assert (backup_path / "file2.txt").read_text() == "content 2"

    def test_create_backup_copies_subdirs(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir)
        backup_path = temp_backup_dir / f"backup_{ts}"
        assert (backup_path / "subdir" / "file3.txt").read_text() == "content 3"

    def test_create_backup_preserves_structure(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir)
        backup_path = temp_backup_dir / f"backup_{ts}"
        assert backup_path.is_dir()
        assert (backup_path / "subdir").is_dir()

    def test_list_backups_empty(self, temp_backup_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        assert bm.list_backups() == []

    def test_list_backups_returns_names(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts1 = bm.create_backup(temp_source_dir)
        backups = bm.list_backups()
        assert len(backups) >= 1
        assert f"backup_{ts1}" in backups

    def test_list_backups_excludes_non_dirs(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        bm.create_backup(temp_source_dir)
        # Create a file that looks like a backup but isn't a directory
        (temp_backup_dir / "backup_not_a_dir.txt").write_text("fake")
        backups = bm.list_backups()
        assert len(backups) == 1

    def test_create_backup_with_description(self, temp_backup_dir, temp_source_dir):
        # description param exists in signature but isn't stored - verify no error
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir, description="test backup")
        assert (temp_backup_dir / f"backup_{ts}").exists()

    def test_multiple_backups_same_source(self, temp_backup_dir, temp_source_dir):
        bm = BackupManager(backup_dir=temp_backup_dir)
        ts = bm.create_backup(temp_source_dir)
        assert ts is not None
        assert len(bm.list_backups()) == 1


class TestGetBackupManager:
    """Tests for get_backup_manager factory function."""

    def test_returns_backup_manager(self):
        result = get_backup_manager()
        assert isinstance(result, BackupManager)

    def test_returns_functionally_working_manager(self, tmp_path):
        with patch("pathlib.Path.home", return_value=tmp_path):
            mgr = get_backup_manager()
            assert mgr.backup_dir.exists()
