"""Tests for web/shared.py — ProgressStore."""

from __future__ import annotations

import sys
from pathlib import Path
from threading import Thread

sys.path.insert(0, str(Path(__file__).parent.parent))

from web.shared import ProgressStore


class TestProgressStore:
    def test_create(self):
        store = ProgressStore()
        store.create("job-1")
        job = store.get("job-1")
        assert job is not None
        assert job["status"] == "pending"
        assert job["progress_pct"] == 0

    def test_update_status(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", status="running")
        job = store.get("job-1")
        assert job["status"] == "running"

    def test_update_stage(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", stage="parsing")
        job = store.get("job-1")
        assert job["stage"] == "parsing"

    def test_update_message(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", message="Fetching paper metadata")
        job = store.get("job-1")
        assert job["message"] == "Fetching paper metadata"

    def test_update_progress_pct(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", progress_pct=50)
        job = store.get("job-1")
        assert job["progress_pct"] == 50

    def test_update_multiple_fields(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", status="running", stage="extracting", message="Extracting text", progress_pct=25)
        job = store.get("job-1")
        assert job["status"] == "running"
        assert job["stage"] == "extracting"
        assert job["message"] == "Extracting text"
        assert job["progress_pct"] == 25

    def test_update_negative_progress_does_not_change(self):
        store = ProgressStore()
        store.create("job-1")
        store.update("job-1", progress_pct=50)
        store.update("job-1", progress_pct=-1)
        assert store.get("job-1")["progress_pct"] == 50

    def test_update_missing_job_is_noop(self):
        store = ProgressStore()
        store.update("nonexistent", status="running", stage="x")
        assert store.get("nonexistent") is None

    def test_get_missing_job_returns_none(self):
        store = ProgressStore()
        assert store.get("nonexistent") is None

    def test_create_idempotent(self):
        store = ProgressStore()
        store.create("job-1")
        store.create("job-1")
        assert store.get("job-1")["status"] == "pending"

    def test_multiple_jobs(self):
        store = ProgressStore()
        store.create("job-1")
        store.create("job-2")
        store.create("job-3")
        store.update("job-2", status="running")
        assert store.get("job-1")["status"] == "pending"
        assert store.get("job-2")["status"] == "running"
        assert store.get("job-3")["status"] == "pending"

    def test_cleanup_clears_all(self):
        store = ProgressStore()
        store.create("job-1")
        store.create("job-2")
        store.cleanup()
        assert store.get("job-1") is None
        assert store.get("job-2") is None


class TestProgressStoreThreadSafety:
    def test_concurrent_updates(self):
        """Multiple threads can update the same job safely."""
        store = ProgressStore()
        store.create("job-1")
        errors = []

        def worker(pct_start: int, count: int):
            try:
                for i in range(count):
                    store.update("job-1", progress_pct=pct_start + i)
            except Exception as e:
                errors.append(e)

        t1 = Thread(target=worker, args=(0, 50))
        t2 = Thread(target=worker, args=(50, 50))
        t1.start()
        t2.start()
        t1.join()
        t2.join()

        assert not errors
        job = store.get("job-1")
        assert job["progress_pct"] >= 0

    def test_concurrent_create_and_get(self):
        """Creating and getting jobs from multiple threads does not crash."""
        store = ProgressStore()
        errors = []

        def worker(job_id: str):
            try:
                store.create(job_id)
                for _ in range(10):
                    _ = store.get(job_id)
            except Exception as e:
                errors.append(e)

        threads = [Thread(target=worker, args=(f"job-{i}",)) for i in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert not errors
