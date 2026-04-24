"""Tests for core/basics.py."""
import json
import os
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.basics import (
    DEFAULT_RESEARCH_DIRS, get_research_dirs, get_default_concept_dir,
    get_default_radar_dir, slugify_title, safe_uid, read_text,
    write_text, ensure_research_tree,
)

class TestSlugifyTitle:
    def test_basic(self):
        assert slugify_title("Hello World") == "Hello-World"

    def test_preserves_case(self):
        assert slugify_title("PyTorch Basics") == "PyTorch-Basics"

    def test_strips_special_chars(self):
        assert slugify_title("Paper: #1 Review!") == "Paper-1-Review"

    def test_multiple_spaces(self):
        assert slugify_title("Hello    World") == "Hello-World"

    def test_leading_trailing_spaces(self):
        assert slugify_title("  Hello  ") == "Hello"

    def test_max_len_truncates(self):
        long_title = "A" * 100
        result = slugify_title(long_title, max_len=80)
        assert len(result) == 80
        assert not result.endswith("-")

    def test_empty_returns_paper(self):
        assert slugify_title("") == "Paper"

    def test_whitespace_only_returns_paper(self):
        assert slugify_title("   ") == "Paper"

    def test_only_special_chars(self):
        assert slugify_title("!!!") == "Paper"

    def test_dashes_collapsed(self):
        assert slugify_title("Hello--World---Test") == "Hello-World-Test"

    def test_strips_leading_trailing_dashes_underscores(self):
        assert slugify_title("--Hello--") == "Hello"
        assert slugify_title("__Hello__") == "Hello"


class TestSafeUid:
    def test_basic(self):
        assert safe_uid("hello-world") == "hello-world"

    def test_spaces_to_underscore(self):
        assert safe_uid("hello world") == "hello_world"

    def test_strips_special_chars(self):
        assert safe_uid("hello@world!") == "hello_world_"

    def test_preserves_dots_and_dashes(self):
        assert safe_uid("v1.0-beta") == "v1.0-beta"

    def test_empty(self):
        assert safe_uid("   ") == ""


class TestReadWriteText:
    def test_write_and_read(self, tmp_path):
        p = tmp_path / "test.txt"
        write_text(p, "hello world")
        assert read_text(p) == "hello world"

    def test_read_nonexistent_returns_empty(self, tmp_path):
        p = tmp_path / "nonexistent.txt"
        assert read_text(p) == ""

    def test_write_creates_parent_dirs(self, tmp_path):
        p = tmp_path / "subdir" / "nested" / "file.txt"
        write_text(p, "content")
        assert p.read_text(encoding="utf-8") == "content"


class TestEnsureResearchTree:
    def test_creates_root_and_subdirs(self, tmp_path):
        root = tmp_path / "research"
        ensure_research_tree(root)
        for d in DEFAULT_RESEARCH_DIRS:
            assert (root / d).is_dir()

    def test_idempotent(self, tmp_path):
        root = tmp_path / "research"
        ensure_research_tree(root)
        ensure_research_tree(root)
        for d in DEFAULT_RESEARCH_DIRS:
            assert (root / d).is_dir()


class TestResearchDirs:
    def test_default_research_dirs_has_12_entries(self):
        assert len(DEFAULT_RESEARCH_DIRS) == 12

    def test_default_research_dirs_starts_with_radar(self):
        assert DEFAULT_RESEARCH_DIRS[0] == "00-Radar"

    @patch('core.basics._get_config_path')
    def test_get_research_dirs_loads_from_file(self, mock_path, tmp_path):
        cfg = tmp_path / "categories.json"
        cfg.write_text(json.dumps(["Custom-Dir"]), encoding="utf-8")
        mock_path.return_value = cfg
        # Clear cache
        import core.basics
        core.basics.get_research_dirs.cache_clear()
        assert get_research_dirs() == ["Custom-Dir"]

    @patch('core.basics._get_config_path')
    def test_get_research_dirs_falls_back_to_default(self, mock_path, tmp_path):
        cfg = tmp_path / "categories.json"
        cfg.write_text("not a list", encoding="utf-8")
        mock_path.return_value = cfg
        import core.basics
        core.basics.get_research_dirs.cache_clear()
        assert get_research_dirs() == list(DEFAULT_RESEARCH_DIRS)

    def test_get_default_radar_dir(self):
        assert get_default_radar_dir() == "00-Radar"

    def test_get_default_concept_dir(self):
        assert get_default_concept_dir() == "01-Foundations"
