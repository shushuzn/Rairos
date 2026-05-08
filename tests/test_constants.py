"""Tests for core/_constants.py — canonical constants."""

import pytest
from core._constants import (
    ARXIV_API,
    CROSSREF_WORKS,
    DOI_RESOLVER,
    RADAR_FILE,
    TIMELINE_FILE,
    DEFAULT_RESEARCH_DIRS,
)


class TestConstants:
    def test_arxiv_api_format(self):
        assert "arxiv.org" in ARXIV_API
        assert "{arxiv_id}" in ARXIV_API

    def test_crossref_works_format(self):
        assert "crossref.org" in CROSSREF_WORKS
        assert "{doi}" in CROSSREF_WORKS

    def test_doi_resolver(self):
        assert DOI_RESOLVER == "https://doi.org/"

    def test_radar_file(self):
        assert RADAR_FILE == "Radar.md"

    def test_timeline_file(self):
        assert TIMELINE_FILE == "Timeline.md"

    def test_default_research_dirs_count(self):
        assert len(DEFAULT_RESEARCH_DIRS) == 12

    def test_default_research_dirs_are_ordered(self):
        assert DEFAULT_RESEARCH_DIRS[0] == "00-Radar"
        assert DEFAULT_RESEARCH_DIRS[-1] == "11-Future-Directions"

    def test_default_research_dirs_have_numeric_prefix(self):
        for d in DEFAULT_RESEARCH_DIRS:
            assert d[:2].isdigit(), f"{d} missing numeric prefix"

    def test_default_research_dirs_unique(self):
        assert len(DEFAULT_RESEARCH_DIRS) == len(set(DEFAULT_RESEARCH_DIRS))
