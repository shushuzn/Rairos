//! rairos-trends — Trend Forecasting using time-series analysis on Radar data
//!
//! Uses simple linear regression slope for trend detection
//! and exponential smoothing (Holt's method) for prediction.
//!
//! Ported from `trends/forecaster.py`.

#![allow(clippy::unnecessary_unwrap)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum TrendsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, TrendsError>;

// ============================================================================
// Types
// ============================================================================

/// A single timestamped radar score entry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarEntry {
    pub timestamp: String,
    pub score: f64,
}

/// Prediction result from Holt's exponential smoothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub predicted: Option<f64>,
    pub confidence: f64,
    pub reason: String,
    pub last_score: Option<f64>,
    pub trend: String,
}

/// Comparison between two tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagComparison {
    pub tag_a: String,
    pub tag_b: String,
    pub slope_a: f64,
    pub slope_b: f64,
    pub trend_a: String,
    pub trend_b: String,
    pub predicted_a: Option<f64>,
    pub predicted_b: Option<f64>,
    pub confidence_a: f64,
    pub confidence_b: f64,
    pub scores_a: Vec<(String, f64)>,
    pub scores_b: Vec<(String, f64)>,
}

/// The main forecaster struct.
#[derive(Debug, Clone, Default)]
pub struct TrendForecaster {
    history: HashMap<String, Vec<RadarEntry>>,
    history_path: String,
}

// ============================================================================
// Implementation
// ============================================================================

impl TrendForecaster {
    /// Create a new TrendForecaster, optionally with a custom history path.
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            history_path: "data/radar_history.json".to_string(),
        }
    }

    /// Create with a specific history file path.
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        let history_path = path.as_ref().to_string_lossy().to_string();
        let history = if path.as_ref().exists() {
            Self::load_history_from_path(path.as_ref()).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            history,
            history_path,
        }
    }

    fn load_history_from_path(path: &Path) -> Result<HashMap<String, Vec<RadarEntry>>> {
        let contents = fs::read_to_string(path)?;
        let map: HashMap<String, Vec<RadarEntry>> = serde_json::from_str(&contents)?;
        Ok(map)
    }

    /// Record current radar scores as a timestamped snapshot.
    pub fn record_radar_snapshot(&mut self, radar_data: &HashMap<String, RadarScoreData>) {
        let ts = chrono_now_iso();
        for (tag, data) in radar_data {
            let entries = self.history.entry(tag.clone()).or_default();
            entries.push(RadarEntry {
                timestamp: ts.clone(),
                score: data.score,
            });
        }
        self.save_history();
    }

    /// Build a timeseries [(month, score)] for a tag, covering last N months.
    pub fn build_timeseries(&self, tag: &str, months: usize) -> Vec<(String, f64)> {
        let entries = match self.history.get(tag) {
            Some(e) => e,
            None => return Vec::new(),
        };
        entries
            .iter()
            .rev()
            .take(months)
            .map(|e| (e.timestamp[..7].to_string(), e.score))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Find tags with rising trend (positive slope above threshold).
    pub fn detect_trending(&self, threshold: f64) -> Vec<(String, f64)> {
        let mut results = Vec::new();
        for tag in self.history.keys() {
            let ts = self.build_timeseries(tag, 6);
            if ts.len() < 2 {
                continue;
            }
            let scores: Vec<f64> = ts.iter().map(|(_, s)| *s).collect();
            let slope = linear_slope(&scores);
            if slope > threshold {
                results.push((tag.clone(), round(slope, 4)));
            }
        }
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Predict next month's heat using Holt's exponential smoothing.
    pub fn predict_next(&self, tag: &str) -> Prediction {
        let ts = self.build_timeseries(tag, 12);
        if ts.len() < 3 {
            return Prediction {
                predicted: None,
                confidence: 0.0,
                reason: "insufficient data".to_string(),
                last_score: None,
                trend: "unknown".to_string(),
            };
        }

        let scores: Vec<f64> = ts.iter().map(|(_, s)| *s).collect();
        let last_score = scores.last().copied();

        // Holt's linear exponential smoothing
        let alpha = 0.3;
        let beta = 0.1;
        let mut level = scores[0];
        let mut trend = scores[1] - scores[0];
        for s in &scores[1..] {
            let new_level = alpha * s + (1.0 - alpha) * (level + trend);
            let new_trend = beta * (new_level - level) + (1.0 - beta) * trend;
            level = new_level;
            trend = new_trend;
        }

        let predicted = level + trend;
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        let std = variance.sqrt();
        let confidence = if mean > 0.0 {
            1.0 - (std / mean.max(1.0))
        } else {
            0.0
        };
        let confidence = confidence.clamp(0.0, 1.0);

        let trend_str = if trend > 0.1 {
            "rising"
        } else if trend.abs() <= 0.1 {
            "stable"
        } else {
            "falling"
        };

        Prediction {
            predicted: Some(round(predicted, 2)),
            confidence: round(confidence, 3),
            reason: format!("based on {} observations", scores.len()),
            last_score,
            trend: trend_str.to_string(),
        }
    }

    /// Predict next-hot tags, sorted by predicted score * confidence.
    pub fn get_top_predictions(&self, top_k: usize) -> Vec<TagPrediction> {
        let mut predictions: Vec<TagPrediction> = Vec::new();
        for tag in self.history.keys() {
            let pred = self.predict_next(tag);
            if pred.predicted.is_some() {
                let combined = pred.predicted.unwrap() * pred.confidence;
                predictions.push(TagPrediction {
                    tag: tag.clone(),
                    predicted: pred.predicted,
                    confidence: pred.confidence,
                    reason: pred.reason,
                    last_score: pred.last_score,
                    trend: pred.trend,
                    combined: round(combined, 4),
                });
            }
        }
        predictions.sort_by(|a, b| {
            b.combined
                .partial_cmp(&a.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        predictions.truncate(top_k);
        predictions
    }

    /// Compare two tags on heat, trend, and prediction.
    pub fn compare_tags(&self, tag_a: &str, tag_b: &str) -> TagComparison {
        let ts_a = self.build_timeseries(tag_a, 6);
        let ts_b = self.build_timeseries(tag_b, 6);

        let scores_a: Vec<f64> = ts_a.iter().map(|(_, s)| *s).collect();
        let scores_b: Vec<f64> = ts_b.iter().map(|(_, s)| *s).collect();

        let slope_a = if scores_a.len() >= 2 {
            linear_slope(&scores_a)
        } else {
            0.0
        };
        let slope_b = if scores_b.len() >= 2 {
            linear_slope(&scores_b)
        } else {
            0.0
        };

        let pred_a = self.predict_next(tag_a);
        let pred_b = self.predict_next(tag_b);

        TagComparison {
            tag_a: tag_a.to_string(),
            tag_b: tag_b.to_string(),
            slope_a: round(slope_a, 4),
            slope_b: round(slope_b, 4),
            trend_a: pred_a.trend.clone(),
            trend_b: pred_b.trend.clone(),
            predicted_a: pred_a.predicted,
            predicted_b: pred_b.predicted,
            confidence_a: pred_a.confidence,
            confidence_b: pred_b.confidence,
            scores_a: ts_a,
            scores_b: ts_b,
        }
    }

    /// Save history to disk.
    pub fn save_history(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.history) {
            if let Some(parent) = Path::new(&self.history_path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&self.history_path, json);
        }
    }

    /// Record a snapshot from current radar.json.
    pub fn record_current_radar(&mut self) -> bool {
        let candidates = ["data/radar.json", "radar.json"];
        for p in &candidates {
            let path = Path::new(p);
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(path) {
                    if let Ok(data) =
                        serde_json::from_str::<HashMap<String, RadarScoreData>>(&contents)
                    {
                        self.record_radar_snapshot(&data);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Access raw history.
    pub fn history(&self) -> &HashMap<String, Vec<RadarEntry>> {
        &self.history
    }
}

/// Extra data needed when recording radar snapshots (score field).
#[derive(Debug, Clone)]
pub struct RadarScoreData {
    pub score: f64,
}

impl<'de> Deserialize<'de> for RadarScoreData {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            WithScore { score: f64 },
            JustScore(f64),
        }
        let raw = Raw::deserialize(deserializer)?;
        match raw {
            Raw::WithScore { score } => Ok(RadarScoreData { score }),
            Raw::JustScore(score) => Ok(RadarScoreData { score }),
        }
    }
}

/// A prediction with its tag attached, for top-k results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagPrediction {
    pub tag: String,
    pub predicted: Option<f64>,
    pub confidence: f64,
    pub reason: String,
    pub last_score: Option<f64>,
    pub trend: String,
    pub combined: f64,
}

// ============================================================================
// Pure-Rust linear slope (OLS)
// ============================================================================

fn linear_slope(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean = values.iter().sum::<f64>() / n as f64;
    let num: f64 = values
        .iter()
        .enumerate()
        .map(|(i, &y)| (i as f64 - x_mean) * (y - y_mean))
        .sum();
    let den: f64 = values
        .iter()
        .enumerate()
        .map(|(i, _)| (i as f64 - x_mean).powi(2))
        .sum();
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn round(v: f64, decimals: u32) -> f64 {
    let m = 10_f64.powi(decimals as i32);
    (v * m).round() / m
}

fn chrono_now_iso() -> String {
    // Avoid depending on the chrono crate — use std::time::Utc and format manually.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Format as ISO 8601 UTC: YYYY-MM-DDTHH:MM:SS.sssZ
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    // Simple conversion: no full chrono, but good enough for timestamp prefix
    let days_since_epoch = secs / 86400;
    let secs_of_day = secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;
    // Julian day offset for Unix epoch (1970-01-01) = 2440588
    let julian_day = 2440588 + days_since_epoch as i64;
    // Convert Julian day to Y-M-D (Gregorian calendar)
    let (year, month, day) = julian_to_ymd(julian_day);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, millis
    )
}

/// Convert a Julian day number to Gregorian year, month, day.
fn julian_to_ymd(jd: i64) -> (i64, u32, u32) {
    // Adapted from Numerical Recipes
    let j = jd + 32044;
    let g = j / 146097;
    let dg = j % 146097;
    let c = (dg / 36524 + 1) * 3 / 4;
    let dc = dg - c * 36524;
    let b = dc / 1461;
    let db = dc % 1461;
    let a = (db / 365 + 1) * 3 / 4;
    let da = db - a * 365;
    let y = g * 400 + c * 100 + b * 4 + a;
    let m = (da * 5 + 308) / 153 - 2;
    let d = da - (m + 4) * 153 / 5 + 122;
    let year = y - 4800 + (m + 2) / 12;
    let month = ((m + 2) % 12 + 1) as u32;
    let day = d as u32 + 1;
    (year, month, day)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_linear_slope_rising() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let slope = linear_slope(&values);
        assert!((slope - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_linear_slope_falling() {
        let values = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let slope = linear_slope(&values);
        assert!((slope - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_linear_slope_flat() {
        let values = vec![3.0, 3.0, 3.0, 3.0];
        let slope = linear_slope(&values);
        assert!((slope - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_detect_trending() {
        let mut fc = TrendForecaster::new();
        fc.history.insert(
            "LLM".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 2.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 3.0,
                },
                RadarEntry {
                    timestamp: "2024-04-01T00:00:00Z".to_string(),
                    score: 4.0,
                },
            ],
        );
        fc.history.insert(
            "XML".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 5.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 4.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 3.0,
                },
                RadarEntry {
                    timestamp: "2024-04-01T00:00:00Z".to_string(),
                    score: 2.0,
                },
            ],
        );

        let trending = fc.detect_trending(0.0);
        assert_eq!(trending.len(), 1);
        assert_eq!(trending[0].0, "LLM");
        assert!(trending[0].1 > 0.0);
    }

    #[test]
    fn test_predict_next_insufficient_data() {
        let fc = TrendForecaster::new();
        let pred = fc.predict_next("nonexistent");
        assert!(pred.predicted.is_none());
        assert_eq!(pred.reason, "insufficient data");
    }

    #[test]
    fn test_predict_next_holt_smoothing() {
        let mut fc = TrendForecaster::new();
        fc.history.insert(
            "test".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 2.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 3.0,
                },
            ],
        );
        let pred = fc.predict_next("test");
        assert!(pred.predicted.is_some());
        assert!(pred.confidence >= 0.0 && pred.confidence <= 1.0);
    }

    #[test]
    fn test_build_timeseries() {
        let mut fc = TrendForecaster::new();
        fc.history.insert(
            "AI".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-15T10:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-02-20T10:00:00Z".to_string(),
                    score: 2.0,
                },
                RadarEntry {
                    timestamp: "2024-03-10T10:00:00Z".to_string(),
                    score: 3.0,
                },
            ],
        );
        let ts = fc.build_timeseries("AI", 12);
        assert_eq!(ts.len(), 3);
        // Should be grouped by month prefix
        assert_eq!(ts[0].0, "2024-01");
        assert_eq!(ts[1].0, "2024-02");
        assert_eq!(ts[2].0, "2024-03");
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("history.json");
        {
            let mut fc = TrendForecaster::with_path(&path);
            fc.history.insert(
                "tag1".to_string(),
                vec![RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 5.0,
                }],
            );
            fc.save_history();
        }
        let fc2 = TrendForecaster::with_path(&path);
        assert!(fc2.history.contains_key("tag1"));
    }

    #[test]
    fn test_compare_tags() {
        let mut fc = TrendForecaster::new();
        fc.history.insert(
            "A".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 2.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 3.0,
                },
            ],
        );
        fc.history.insert(
            "B".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 10.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 9.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 8.0,
                },
            ],
        );
        let comp = fc.compare_tags("A", "B");
        assert_eq!(comp.tag_a, "A");
        assert_eq!(comp.tag_b, "B");
        assert!(comp.slope_a > 0.0);
        assert!(comp.slope_b < 0.0);
        assert_eq!(comp.trend_a, "rising");
        assert_eq!(comp.trend_b, "falling");
    }

    #[test]
    fn test_get_top_predictions() {
        let mut fc = TrendForecaster::new();
        fc.history.insert(
            "hot".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 10.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 20.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 30.0,
                },
            ],
        );
        fc.history.insert(
            "cold".to_string(),
            vec![
                RadarEntry {
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-02-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
                RadarEntry {
                    timestamp: "2024-03-01T00:00:00Z".to_string(),
                    score: 1.0,
                },
            ],
        );
        let tops = fc.get_top_predictions(2);
        assert_eq!(tops.len(), 2);
        // "hot" should be ranked higher due to higher combined score
        assert_eq!(tops[0].tag, "hot");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_round_utility() {
        assert!((round(3.14159, 2) - 3.14).abs() < 0.0001);
        assert!((round(1.5, 0) - 2.0).abs() < 0.0001);
    }
}
