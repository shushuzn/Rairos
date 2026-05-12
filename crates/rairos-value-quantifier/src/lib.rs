//! rairos-value-quantifier — VW-style value quantification.
//!
//! Inspired by Volkswagen's 700-900 euros annual savings model.
//! Quantifies: time saved, API costs saved, research efficiency gains.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

/// A single quantified value metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub description: String,
}

/// Value quantifier inspired by Volkswagen's value proposition.
#[derive(Debug, Clone)]
pub struct ValueQuantifier {
    api_calls_saved: f64,
    papers_processed: f64,
    searches_performed: f64,
    hours_saved: f64,
    cost_saved_usd: f64,
    efficiency_gain_percent: f64,
}

impl Default for ValueQuantifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueQuantifier {
    pub fn new() -> Self {
        Self {
            api_calls_saved: 0.0,
            papers_processed: 0.0,
            searches_performed: 0.0,
            hours_saved: 0.0,
            cost_saved_usd: 0.0,
            efficiency_gain_percent: 0.0,
        }
    }

    /// Update a specific metric by name.
    pub fn update(&mut self, metric: &str, value: f64) {
        match metric {
            "api_calls_saved" => self.api_calls_saved = value,
            "papers_processed" => self.papers_processed = value,
            "searches_performed" => self.searches_performed = value,
            "hours_saved" => self.hours_saved = value,
            "cost_saved_usd" => self.cost_saved_usd = value,
            "efficiency_gain_percent" => self.efficiency_gain_percent = value,
            _ => {}
        }
    }

    /// Calculate derived metrics and return all value metrics.
    pub fn calculate_value(&self) -> HashMap<String, ValueMetric> {
        let api_cost_per_call = 0.01;
        let research_hour_cost = 50.0;

        // 6 minutes per search
        let hours_from_api = self.api_calls_saved * 0.1;
        let cost_from_api = self.api_calls_saved * api_cost_per_call;
        let research_time_value = hours_from_api * research_hour_cost;

        let mut map = HashMap::new();
        map.insert(
            "api_calls_saved".to_string(),
            ValueMetric {
                name: "API调用节省".to_string(),
                value: self.api_calls_saved,
                unit: "次".to_string(),
                description: "通过缓存和智能重试节省".to_string(),
            },
        );
        map.insert(
            "hours_saved".to_string(),
            ValueMetric {
                name: "时间节省".to_string(),
                value: hours_from_api + self.hours_saved,
                unit: "小时".to_string(),
                description: "自动化和优化带来的时间节省".to_string(),
            },
        );
        map.insert(
            "cost_saved".to_string(),
            ValueMetric {
                name: "成本节省".to_string(),
                value: cost_from_api + research_time_value + self.cost_saved_usd,
                unit: "美元".to_string(),
                description: "API成本和时间成本的总节省".to_string(),
            },
        );
        map.insert(
            "papers_processed".to_string(),
            ValueMetric {
                name: "论文处理".to_string(),
                value: self.papers_processed,
                unit: "篇".to_string(),
                description: "已处理的论文数量".to_string(),
            },
        );
        map
    }

    /// Get total value (sum of all positive metrics).
    fn total_value(&self) -> f64 {
        self.calculate_value()
            .values()
            .filter(|m| m.value > 0.0)
            .map(|m| m.value)
            .sum()
    }

    /// Generate a formatted value report.
    pub fn get_value_report(&self) -> String {
        let values = self.calculate_value();
        let mut lines = Vec::new();

        lines.push("=".repeat(60));
        lines.push("💰 价值量化报告 (Volkswagen式收益计算)".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());
        lines.push("Volkswagen承诺: 每年节省700-900欧元".to_string());
        lines.push(String::new());
        lines.push("-".repeat(60));

        for (_key, metric) in &values {
            if metric.value > 0.0 {
                lines.push(format!(
                    "📊 {}: {:.1} {}",
                    metric.name, metric.value, metric.unit
                ));
                lines.push(format!("   {}", metric.description));
                lines.push(String::new());
            }
        }

        let total = self.total_value();
        if total > 0.0 {
            lines.push("-".repeat(60));
            lines.push(format!("💵 总价值: ${:.2}", total));
            lines.push("-".repeat(60));
            lines.push(String::new());
            lines.push("Volkswagen对比:".to_string());
            lines.push("  他们: 700-900欧元/年 ≈ 约770-990美元/年".to_string());
            lines.push(format!("  我们: ${:.2}（目前统计）", total));
            lines.push(String::new());
            lines.push("💡 提示: 持续使用，价值累积！".to_string());
        }

        lines.push("=".repeat(60));
        lines.join("\n")
    }

    /// Get Volkswagen-style comparison string.
    pub fn get_vw_comparison(&self) -> String {
        let values = self.calculate_value();
        let our_value = self.total_value() * 12.0; // Annualize
        let efficiency = values
            .get("papers_processed")
            .map(|m| (m.value as i32) * 10)
            .unwrap_or(0);

        format!(
            r#"🚗 Volkswagen vs 🚀 AI Research OS

Volkswagen V2G:
  每年节省: 770-990 美元
  系统级节省: 220亿欧元（2030年）
  个人参与: 基础报酬 + 成本节省

AI Research OS:
  我们节省: ${:.2}（年化）
  你的时间: 无价
  研究效率: 提升{}%（估算）

💡 两者都强调: 长期价值 > 短期成本"#,
            our_value, efficiency
        )
    }

    /// Add to api_calls_saved counter.
    pub fn add_api_calls_saved(&mut self, n: f64) {
        self.api_calls_saved += n;
    }

    /// Add to papers_processed counter.
    pub fn add_papers_processed(&mut self, n: f64) {
        self.papers_processed += n;
    }

    /// Get current api_calls_saved.
    pub fn api_calls_saved(&self) -> f64 {
        self.api_calls_saved
    }

    /// Get current papers_processed.
    pub fn papers_processed(&self) -> f64 {
        self.papers_processed
    }
}

// ─── Global system ──────────────────────────────────────────────────────────────

static GLOBAL_QUANTIFIER: LazyLock<Mutex<ValueQuantifier>> =
    LazyLock::new(|| Mutex::new(ValueQuantifier::new()));

/// Get the global value quantifier.
pub fn get_value_quantifier() -> std::sync::MutexGuard<'static, ValueQuantifier> {
    GLOBAL_QUANTIFIER.lock().unwrap()
}

/// Print the value report to stdout.
pub fn print_value_report() {
    let q = get_value_quantifier();
    println!("{}", q.get_value_report());
}

/// Print VW comparison to stdout.
pub fn print_vw_comparison() {
    let q = get_value_quantifier();
    println!("{}", q.get_vw_comparison());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_quantifier_zeros() {
        let q = ValueQuantifier::new();
        assert_eq!(q.api_calls_saved, 0.0);
        assert_eq!(q.papers_processed, 0.0);
    }

    #[test]
    fn test_update_metric() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        assert_eq!(q.api_calls_saved, 100.0);
        q.update("papers_processed", 50.0);
        assert_eq!(q.papers_processed, 50.0);
    }

    #[test]
    fn test_calculate_value() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        q.update("papers_processed", 10.0);

        let values = q.calculate_value();
        assert_eq!(values.get("api_calls_saved").unwrap().value, 100.0);
        // hours_saved = 100 * 0.1 = 10
        assert_eq!(values.get("hours_saved").unwrap().value, 10.0);
        // cost_saved = 100*0.01 + 10*50 = 1 + 500 = 501
        assert_eq!(values.get("cost_saved").unwrap().value, 501.0);
    }

    #[test]
    fn test_get_value_report_contains_total() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        let report = q.get_value_report();
        assert!(report.contains("总价值"));
        assert!(report.contains("Volkswagen对比"));
    }

    #[test]
    fn test_get_vw_comparison() {
        let q = ValueQuantifier::new();
        let comp = q.get_vw_comparison();
        assert!(comp.contains("Volkswagen"));
        assert!(comp.contains("AI Research OS"));
    }

    #[test]
    fn test_add_api_calls() {
        let mut q = ValueQuantifier::new();
        q.add_api_calls_saved(50.0);
        q.add_api_calls_saved(50.0);
        assert_eq!(q.api_calls_saved, 100.0);
    }

    #[test]
    fn test_empty_metrics_total_zero() {
        let q = ValueQuantifier::new();
        assert_eq!(q.total_value(), 0.0);
    }

    #[test]
    fn test_positive_metrics_sum() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        // hours = 10, cost = 501, papers = 0
        assert!(q.total_value() > 0.0);
    }
}
