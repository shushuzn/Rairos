"""Trend Forecasting using time-series analysis on Radar data.

Uses simple linear regression slope for trend detection
and exponential smoothing (Holt's method) for prediction.
Pure Python fallback when numpy unavailable.
"""

import json
import math
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

try:
    import numpy as np
    _HAS_NUMPY = True
except ImportError:
    _HAS_NUMPY = False

from kg.manager import KGManager


class TrendForecaster:
    """Analyze and forecast tag trends based on Radar heat history."""

    def __init__(self, kg: KGManager | None = None):
        self.kg = kg or KGManager()
        self._history_path = Path("data/radar_history.json")
        self._history_path.parent.mkdir(parents=True, exist_ok=True)
        self._history: dict[str, list[dict]] = self._load_history()

    def _load_history(self) -> dict:
        if self._history_path.exists():
            try:
                return json.loads(self._history_path.read_text(encoding="utf-8"))
            except Exception:
                pass
        return {}

    def save_history(self):
        self._history_path.write_text(json.dumps(self._history, ensure_ascii=False), encoding="utf-8")

    def record_radar_snapshot(self, radar_data: dict[str, dict]):
        """Record current radar scores as a timestamped snapshot."""
        ts = datetime.now(timezone.utc).isoformat()
        for tag, data in radar_data.items():
            if tag not in self._history:
                self._history[tag] = []
            self._history[tag].append({
                "timestamp": ts,
                "score": data.get("score", 0),
            })
        self.save_history()

    def build_timeseries(self, tag: str, months: int = 12) -> list[tuple[str, float]]:
        """Return [(month, score)] for a tag, covering last N months."""
        entries = self._history.get(tag, [])
        if not entries:
            return []
        return [(e["timestamp"][:7], e["score"]) for e in entries[-months:]]

    def detect_trending(self, threshold: float = 0.5) -> list[tuple[str, float]]:
        """Find tags with rising trend (positive slope above threshold)."""
        results = []
        for tag in self._history:
            ts = self.build_timeseries(tag, months=6)
            if len(ts) < 2:
                continue
            _, scores = zip(*ts)
            slope = self._linear_slope(list(scores))
            if slope > threshold:
                results.append((tag, round(slope, 4)))

        results.sort(key=lambda x: x[1], reverse=True)
        return results

    def predict_next(self, tag: str) -> dict:
        """Predict next month's heat using Holt's exponential smoothing."""
        ts = self.build_timeseries(tag, months=12)
        if len(ts) < 3:
            return {"predicted": None, "confidence": 0.0, "reason": "insufficient data", "trend": "unknown"}

        timestamps, scores = zip(*ts)
        scores = list(scores)

        # Holt's linear exponential smoothing
        alpha = 0.3
        beta = 0.1
        level = scores[0]
        trend = scores[1] - scores[0]
        for s in scores[1:]:
            new_level = alpha * s + (1 - alpha) * (level + trend)
            new_trend = beta * (new_level - level) + (1 - beta) * trend
            level = new_level
            trend = new_trend

        predicted = level + trend
        mean = sum(scores) / len(scores)
        variance = sum((s - mean) ** 2 for s in scores) / len(scores)
        std = math.sqrt(variance)
        confidence = 1.0 - (std / max(mean, 1)) if mean > 0 else 0.0
        confidence = max(0.0, min(1.0, confidence))

        return {
            "predicted": round(predicted, 2),
            "confidence": round(confidence, 3),
            "reason": f"based on {len(scores)} observations",
            "last_score": scores[-1],
            "trend": "rising" if trend > 0.1 else "stable" if abs(trend) <= 0.1 else "falling",
        }

    def get_top_predictions(self, top_k: int = 5) -> list[dict]:
        """Predict next-hot tags, sorted by predicted score * confidence."""
        predictions = []
        for tag in self._history:
            pred = self.predict_next(tag)
            if pred["predicted"] is not None:
                pred["tag"] = tag
                pred["combined"] = pred["predicted"] * pred["confidence"]
                predictions.append(pred)

        predictions.sort(key=lambda x: x["combined"], reverse=True)
        return predictions[:top_k]

    def compare_tags(self, tag_a: str, tag_b: str) -> dict:
        """Compare two tags on heat, trend, and prediction."""
        ts_a = self.build_timeseries(tag_a, months=6)
        ts_b = self.build_timeseries(tag_b, months=6)
        slope_a = self._linear_slope([s for _, s in ts_a]) if len(ts_a) >= 2 else 0.0
        slope_b = self._linear_slope([s for _, s in ts_b]) if len(ts_b) >= 2 else 0.0
        pred_a = self.predict_next(tag_a)
        pred_b = self.predict_next(tag_b)

        return {
            "tag_a": tag_a, "tag_b": tag_b,
            "slope_a": round(slope_a, 4), "slope_b": round(slope_b, 4),
            "trend_a": pred_a["trend"], "trend_b": pred_b["trend"],
            "predicted_a": pred_a["predicted"], "predicted_b": pred_b["predicted"],
            "confidence_a": pred_a["confidence"], "confidence_b": pred_b["confidence"],
            "scores_a": ts_a[-6:], "scores_b": ts_b[-6:],
        }

    def _linear_slope(self, values: list[float]) -> float:
        """Simple OLS linear regression slope."""
        if len(values) < 2:
            return 0.0
        if _HAS_NUMPY:
            x = np.arange(len(values), dtype=float)
            y = np.array(values, dtype=float)
            return float(np.polyfit(x, y, 1)[0])

        n = len(values)
        x = list(range(n))
        x_mean = (n - 1) / 2.0
        y_mean = sum(values) / n
        num = sum((x[i] - x_mean) * (values[i] - y_mean) for i in range(n))
        den = sum((x[i] - x_mean) ** 2 for i in range(n))
        if den == 0:
            return 0.0
        return num / den

    def record_current_radar(self):
        """Record a snapshot from current radar.json."""
        candidates = [Path("data/radar.json"), Path("radar.json")]
        for p in candidates:
            if p.exists():
                try:
                    data = json.loads(p.read_text(encoding="utf-8"))
                    self.record_radar_snapshot(data)
                    return
                except Exception:
                    pass
