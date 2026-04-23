"""Tests for extable module — detector, parser, storage."""
from __future__ import annotations

import pytest

from extable.parser import ExperimentTableParser
from extable.storage import ExperimentDB


class TestExperimentTableParser:
    def test_regex_parse_empty_table(self):
        parser = ExperimentTableParser()
        result = parser.parse_table_to_struct([])
        assert result == {"tables": []}

    def test_regex_parse_single_row_table(self):
        parser = ExperimentTableParser()
        result = parser.parse_table_to_struct([["header"]])
        assert result == {"tables": []}

    def test_regex_parse_simple_metrics(self):
        parser = ExperimentTableParser()
        table = [
            ["model", "accuracy", "dataset"],
            ["baseline", "0.85", "squad"],
            ["ours", "0.92", "squad"],
        ]
        result = parser.parse_table_to_struct(table, context_title="Test")
        assert len(result["tables"]) == 1
        t = result["tables"][0]
        assert t["caption"] == "Test"
        assert any(m["name"] == "accuracy" for m in t["metrics"])
        assert "squad" in t["datasets"]
        assert "baseline" in t["models"]
        assert "ours" in t["models"]

    def test_regex_fallback_when_llm_none(self):
        parser = ExperimentTableParser(llm_client=None)
        table = [
            ["method", "f1"],
            ["a", "0.8"],
        ]
        result = parser.parse_table_to_struct(table)
        assert "tables" in result


class TestExperimentDB:
    def test_init_creates_db(self, tmp_path):
        db = ExperimentDB(db_path=str(tmp_path / "ext.db"))
        stats = db.stats()
        assert stats == {"papers": 0, "tables": 0}

    def test_add_paper_and_table(self, tmp_path):
        db = ExperimentDB(db_path=str(tmp_path / "ext.db"))
        db.add_paper("p1", "Title One")
        struct = {
            "caption": "Table 1",
            "metrics": [{"name": "acc", "value": 0.9}],
            "datasets": ["squad"],
            "models": ["ours"],
            "baselines": {},
            "ours_best": {"value": 0.9, "dataset": "squad", "metric": "acc"},
        }
        tid = db.add_table("p1", struct, raw_table=[["a", "b"]])
        assert isinstance(tid, str)
        assert len(tid) == 36

        tables = db.get_paper_tables("p1")
        assert len(tables) == 1
        assert tables[0]["paper_uid"] == "p1"
        assert tables[0]["metrics"][0]["value"] == 0.9

    def test_search_tables(self, tmp_path):
        db = ExperimentDB(db_path=str(tmp_path / "ext.db"))
        db.add_paper("p1", "Title")
        struct = {
            "caption": "T1",
            "metrics": [{"name": "accuracy", "value": 0.95}],
            "datasets": ["glue"],
            "models": ["m1"],
            "baselines": {},
            "ours_best": {},
        }
        db.add_table("p1", struct, raw_table=[["a"]])

        results = db.search_tables(metric="accuracy")
        assert len(results) == 1

        results = db.search_tables(metric="f1")
        assert len(results) == 0

    def test_export_csv(self, tmp_path):
        db = ExperimentDB(db_path=str(tmp_path / "ext.db"))
        db.add_paper("p1", "Title")
        struct = {
            "caption": "T1",
            "metrics": [],
            "datasets": ["d1"],
            "models": ["m1"],
            "baselines": {},
            "ours_best": {"value": 0.9, "dataset": "d1", "metric": "acc"},
        }
        db.add_table("p1", struct, raw_table=[["a"]])
        csv = db.export_to_csv()
        assert "paper_uid,table_id" in csv
        assert "p1" in csv


class TestTableDetector:
    def test_no_fitz_returns_empty(self):
        from extable.detector import TableDetector
        td = TableDetector()
        # If fitz is available this test still passes because we pass a bad path
        # but we mainly want to ensure the API works.
        # Force _has_fitz off to test fallback.
        td._has_fitz = False
        assert td.detect_tables(0, pdf_path=None) == []
        assert td.extract_all_tables("/nonexistent.pdf") == []
