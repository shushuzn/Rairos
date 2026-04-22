"""Tests for trends/forecaster.py — TrendForecaster time-series analysis."""
from __future__ import annotations

import pytest

from kg.manager import KGManager
from trends.forecaster import TrendForecaster


@pytest.fixture
def tf(tmp_path):
    db = tmp_path / "test_kg.db"
    kg = KGManager(db_path=str(db))
    tf_obj = TrendForecaster(kg=kg)
    tf_obj._history_path = tmp_path / "radar_history.json"
    tf_obj._history = {}
    return tf_obj


class TestTrendForecasterTimeseries:
    def test_record_radar_snapshot(self, tf):
        radar = {"LLM": {"score": 75}, "RL": {"score": 50}}
        tf.record_radar_snapshot(radar)
        ts = tf.build_timeseries("LLM")
        assert len(ts) == 1
        assert ts[0][1] == 75

    def test_build_timeseries_empty_for_unknown_tag(self, tf):
        ts = tf.build_timeseries("GhostTag")
        assert ts == []

    def test_build_timeseries_respects_months_limit(self, tf):
        # Use unique tag name to avoid cross-test pollution
        for i in range(12):
            tf.record_radar_snapshot({"LimitTag": {"score": float(i)}})
        ts = tf.build_timeseries("LimitTag", months=6)
        assert len(ts) == 6


class TestTrendForecasterLinearSlope:
    def test_linear_slope_rising(self, tf):
        slope = tf._linear_slope([10.0, 20.0, 30.0])
        assert slope > 0

    def test_linear_slope_falling(self, tf):
        slope = tf._linear_slope([30.0, 20.0, 10.0])
        assert slope < 0

    def test_linear_slope_flat(self, tf):
        slope = tf._linear_slope([50.0, 50.0, 50.0])
        assert abs(slope) < 1e-10  # floating-point tolerance

    def test_linear_slope_insufficient_data(self, tf):
        slope = tf._linear_slope([50.0])
        assert slope == 0.0


class TestTrendForecasterDetectTrending:
    def test_detect_trending_no_tags(self, tf):
        result = tf.detect_trending()
        assert result == []

    def test_detect_trending_rising_tag(self, tf):
        # Use unique tag to avoid cross-test pollution
        tf.record_radar_snapshot({"RisingTag2": {"score": 10}})
        tf.record_radar_snapshot({"RisingTag2": {"score": 30}})
        tf.record_radar_snapshot({"RisingTag2": {"score": 50}})
        result = tf.detect_trending(threshold=0.5)
        tags = [r[0] for r in result]
        assert "RisingTag2" in tags

    def test_detect_trending_stable_tag_ignored(self, tf):
        for _ in range(6):
            tf.record_radar_snapshot({"FlatTag": {"score": 50}})
        result = tf.detect_trending(threshold=5.0)
        tags = [r[0] for r in result]
        assert "FlatTag" not in tags


class TestTrendForecasterPredict:
    def test_predict_next_insufficient_data(self, tf):
        # Fewer than 3 observations — use unique tag
        tf.record_radar_snapshot({"SparseTag": {"score": 50}})
        result = tf.predict_next("SparseTag")
        assert result["predicted"] is None
        assert result["reason"] == "insufficient data"

    def test_predict_next_returns_prediction(self, tf):
        for score in [20, 30, 40, 50, 60]:
            tf.record_radar_snapshot({"PredictTag": {"score": float(score)}})
        result = tf.predict_next("PredictTag")
        assert result["predicted"] is not None
        assert 0.0 <= result["predicted"] <= 200.0
        assert "confidence" in result
        assert result["trend"] in ("rising", "stable", "falling")

    def test_predict_next_confidence_bounded(self, tf):
        for score in [50, 50, 50, 50, 50]:
            tf.record_radar_snapshot({"SteadyTag": {"score": float(score)}})
        result = tf.predict_next("SteadyTag")
        assert 0.0 <= result["confidence"] <= 1.0


class TestTrendForecasterTopPredictions:
    def test_get_top_predictions(self, tf):
        for score in [10, 20, 30, 40, 50]:
            tf.record_radar_snapshot({"HotTag": {"score": float(score)}})
        preds = tf.get_top_predictions(top_k=3)
        assert len(preds) <= 3
        if len(preds) >= 2:
            assert preds[0]["combined"] >= preds[1]["combined"]


class TestTrendForecasterCompare:
    def test_compare_tags(self, tf):
        for score in [10, 20, 30]:
            tf.record_radar_snapshot({"TagA": {"score": float(score)}})
        for score in [50, 40, 30]:
            tf.record_radar_snapshot({"TagB": {"score": float(score)}})

        result = tf.compare_tags("TagA", "TagB")
        assert result["tag_a"] == "TagA"
        assert result["tag_b"] == "TagB"
        assert "slope_a" in result
        assert "slope_b" in result
        assert "trend_a" in result
        assert "trend_b" in result

    def test_compare_tags_unknown(self, tf):
        result = tf.compare_tags("GhostA", "GhostB")
        assert result["slope_a"] == 0.0
        assert result["slope_b"] == 0.0
        # Unknown tags have no trend — predict_next returns "unknown" trend
        assert result["trend_a"] == "unknown"
        assert result["trend_b"] == "unknown"
