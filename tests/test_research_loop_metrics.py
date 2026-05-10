"""Tests for research_loop/core.py Metrics class."""


class TestMetrics:
    """Test Metrics dataclass from research_loop/core.py."""

    def _metrics(self):
        from research_loop.core import Metrics
        return Metrics()

    def test_init_defaults(self):
        m = self._metrics()
        assert m.papers_processed == 0
        assert m.papers_failed == 0
        assert m.papers_skipped == 0
        assert m.llm_calls == 0
        assert m.llm_cost_usd == 0.0

    def test_snapshot_returns_dict(self):
        m = self._metrics()
        snap = m.snapshot()
        assert isinstance(snap, dict)
        assert snap["papers_processed"] == 0
        assert snap["papers_failed"] == 0
        assert snap["llm_calls"] == 0
        assert snap["llm_cost_usd"] == 0.0

    def test_increment_counters(self):
        m = self._metrics()
        m.papers_processed = 5
        m.papers_failed = 2
        m.papers_skipped = 1
        m.llm_calls = 10
        m.llm_cost_usd = 0.25

        snap = m.snapshot()
        assert snap["papers_processed"] == 5
        assert snap["papers_failed"] == 2
        assert snap["papers_skipped"] == 1
        assert snap["llm_calls"] == 10
        assert snap["llm_cost_usd"] == 0.25

    def test_snapshot_is_independent(self):
        m = self._metrics()
        snap1 = m.snapshot()

        m.papers_processed = 99
        snap2 = m.snapshot()

        # snap1 should not change (dict is by value)
        assert snap1["papers_processed"] == 0
        assert snap2["papers_processed"] == 99

    def test_snapshot_keys_complete(self):
        m = self._metrics()
        snap = m.snapshot()
        assert set(snap.keys()) == {
            "papers_processed",
            "papers_failed",
            "papers_skipped",
            "llm_calls",
            "llm_cost_usd",
        }
