//! rairos-value-quantifier — Value Quantifier for AI Research OS.
//!
//! Ported from `core/value_quantifier.py` (175 LOC, pure stdlib).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub description: String,
}

pub struct ValueQuantifier {
    metrics: HashMap<String, f64>,
}

impl Default for ValueQuantifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueQuantifier {
    pub fn new() -> Self {
        let mut metrics = HashMap::new();
        metrics.insert("api_calls_saved".to_string(), 0.0);
        metrics.insert("papers_processed".to_string(), 0.0);
        metrics.insert("searches_performed".to_string(), 0.0);
        metrics.insert("hours_saved".to_string(), 0.0);
        metrics.insert("cost_saved_usd".to_string(), 0.0);
        metrics.insert("efficiency_gain_percent".to_string(), 0.0);
        Self { metrics }
    }

    pub fn update(&mut self, metric: &str, value: f64) {
        if self.metrics.contains_key(metric) {
            self.metrics.insert(metric.to_string(), value);
        }
    }

    pub fn calculate_value(&self) -> HashMap<String, ValueMetric> {
        let api_cost_per_call = 0.01;
        let research_hour_cost = 50.0;

        let api_calls_saved = self.metrics.get("api_calls_saved").copied().unwrap_or(0.0);
        let hours_saved = api_calls_saved * 0.1;
        let cost_saved = api_calls_saved * api_cost_per_call;
        let research_time_value = hours_saved * research_hour_cost;
        let papers_processed = self.metrics.get("papers_processed").copied().unwrap_or(0.0);

        let mut result = HashMap::new();
        result.insert(
            "api_calls_saved".to_string(),
            ValueMetric {
                name: "API调用节省".to_string(),
                value: api_calls_saved,
                unit: "次".to_string(),
                description: "通过缓存和智能重试节省".to_string(),
            },
        );
        result.insert(
            "hours_saved".to_string(),
            ValueMetric {
                name: "时间节省".to_string(),
                value: hours_saved,
                unit: "小时".to_string(),
                description: "自动化和优化带来的时间节省".to_string(),
            },
        );
        result.insert(
            "cost_saved".to_string(),
            ValueMetric {
                name: "成本节省".to_string(),
                value: cost_saved + research_time_value,
                unit: "美元".to_string(),
                description: "API成本和时间成本的总节省".to_string(),
            },
        );
        result.insert(
            "papers_processed".to_string(),
            ValueMetric {
                name: "论文处理".to_string(),
                value: papers_processed,
                unit: "篇".to_string(),
                description: "已处理的论文数量".to_string(),
            },
        );
        result
    }

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

        let total_value: f64 = values.values().map(|m| m.value).sum();
        if total_value > 0.0 {
            lines.push("-".repeat(60));
            lines.push(format!("💵 总价值: ${:.2}", total_value));
            lines.push("-".repeat(60));
            lines.push(String::new());
            lines.push("Volkswagen对比:".to_string());
            lines.push("  他们: 700-900欧元/年 ≈ 约770-990美元/年".to_string());
            lines.push(format!("  我们: ${:.2}（目前统计）", total_value));
            lines.push(String::new());
            lines.push("💡 提示: 持续使用，价值累积！".to_string());
        }

        lines.push("=".repeat(60));
        lines.join("\n")
    }

    pub fn get_vw_comparison(&self) -> String {
        let values = self.calculate_value();
        let our_value: f64 = values
            .values()
            .map(|m| m.value)
            .filter(|&v| v > 0.0)
            .sum::<f64>()
            * 12.0;
        let papers = values.get("papers_processed").map(|m| m.value).unwrap_or(0.0);
        let efficiency = papers * 10.0;

        format!(
            concat!(
                "\u{1f6a2} Volkswagen vs \u{1f680} AI Research OS\n\n",
                "Volkswagen V2G:\n",
                "  每年节省: 770-990 美元\n",
                "  系统级节省: 220亿欧元（2030年）\n",
                "  个人参与: 基础报酬 + 成本节省\n\n",
                "AI Research OS:\n",
                "  我们节省: ${:.2}（年化）\n",
                "  你的时间: 无价\n",
                "  研究效率: 提升{:.0}%（估算）\n\n",
                "\u{1f4a1} 两者都强调: 长期价值 > 短期成本"
            ),
            our_value, efficiency
        )
    }
}

// ─── Global quantifier ──────────────────────────────────────────────────────────

use std::sync::Mutex;
static QUANTIFIER: Mutex<Option<ValueQuantifier>> = Mutex::new(None);

pub fn get_quantifier() -> &'static Mutex<Option<ValueQuantifier>> {
    &QUANTIFIER
}

pub fn print_value_report() {
    if let Ok(mut g) = QUANTIFIER.lock() {
        if let Some(ref q) = *g {
            println!("{}", q.get_value_report());
        } else {
            *g = Some(ValueQuantifier::new());
            if let Some(ref q) = *g {
                println!("{}", q.get_value_report());
            }
        }
    }
}

pub fn print_vw_comparison() {
    if let Ok(mut g) = QUANTIFIER.lock() {
        if let Some(ref q) = *g {
            println!("{}", q.get_vw_comparison());
        } else {
            *g = Some(ValueQuantifier::new());
            if let Some(ref q) = *g {
                println!("{}", q.get_vw_comparison());
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_value_empty() {
        let q = ValueQuantifier::new();
        let values = q.calculate_value();
        assert_eq!(values.len(), 4);
        assert_eq!(values.get("api_calls_saved").unwrap().value, 0.0);
    }

    #[test]
    fn test_update_metric() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        let values = q.calculate_value();
        assert_eq!(values.get("api_calls_saved").unwrap().value, 100.0);
        assert_eq!(values.get("hours_saved").unwrap().value, 10.0); // 100 * 0.1
    }

    #[test]
    fn test_value_report_non_zero() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        q.update("papers_processed", 50.0);
        let report = q.get_value_report();
        assert!(report.contains("总价值"));
        assert!(report.contains("Volkswagen对比"));
    }

    #[test]
    fn test_vw_comparison() {
        let mut q = ValueQuantifier::new();
        q.update("api_calls_saved", 100.0);
        let comp = q.get_vw_comparison();
        assert!(comp.contains("Volkswagen"));
        assert!(comp.contains("AI Research OS"));
    }
}
