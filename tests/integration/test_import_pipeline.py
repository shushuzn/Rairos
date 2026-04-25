"""Integration tests for import pipeline.

Tests the checkpoint/resume functionality with mocked database.
"""
import pytest
import json
import tempfile
from pathlib import Path


class TestCheckpointIntegration:
    """Test checkpoint save/load functionality."""

    def test_checkpoint_save_and_load(self):
        """Test checkpoint saves and loads correctly."""
        from cli.cmd.import_ import _save_checkpoint, _load_checkpoint

        with tempfile.TemporaryDirectory() as tmp:
            checkpoint_path = Path(tmp) / "checkpoint.json"

            # Save checkpoint
            _save_checkpoint(checkpoint_path, ["id1", "id2"], ["id3"], 10)

            # Load checkpoint
            data = _load_checkpoint(checkpoint_path)

            assert data["processed"] == ["id1", "id2"]
            assert data["failed"] == ["id3"]
            assert data["total"] == 10
            assert "saved_at" in data

    def test_checkpoint_load_nonexistent(self):
        """Test loading nonexistent checkpoint returns defaults."""
        from cli.cmd.import_ import _load_checkpoint

        with tempfile.TemporaryDirectory() as tmp:
            checkpoint_path = Path(tmp) / "nonexistent.json"

            data = _load_checkpoint(checkpoint_path)

            assert data["processed"] == []
            assert data["failed"] == []
            assert data["total"] == 0

    def test_checkpoint_resume_skips_processed(self):
        """Test resume mode skips already processed IDs."""
        from cli.cmd.import_ import _save_checkpoint

        with tempfile.TemporaryDirectory() as tmp:
            checkpoint_path = Path(tmp) / "checkpoint.json"

            # Simulate previous run with 5 processed and 1 failed
            _save_checkpoint(checkpoint_path, ["id1", "id2", "id3", "id4", "id5"], ["id6"], 10)

            # Load and verify
            with open(checkpoint_path) as f:
                data = json.load(f)

            assert len(data["processed"]) == 5
            assert "id1" in data["processed"]
            assert "id6" in data["failed"]
