"""
Value Quantifier - Inspired by Volkswagen's 700-900 euros annual savings.

Volkswagen promises:
- 700-900 euros annual savings
- Cost advantage for participating
- System cost savings of 22 billion euros by 2040

We quantify:
- Time saved
- API costs saved
- Research efficiency gains
"""
from typing import Dict, List
from dataclasses import dataclass


@dataclass
class ValueMetric:
    """Represents a value metric."""
    name: str
    value: float
    unit: str
    description: str


class ValueQuantifier:
    """
    Quantify value delivered to users.
    
    Inspired by Volkswagen's value proposition:
    - 700-900 euros savings
    - Time value metrics
    - Cost savings
    """
    
    def __init__(self):
        self.metrics = {
            "api_calls_saved": 0,
            "papers_processed": 0,
            "searches_performed": 0,
            "hours_saved": 0.0,
            "cost_saved_usd": 0.0,
            "efficiency_gain_percent": 0.0,
        }
    
    def update(self, metric: str, value: float):
        """Update a metric."""
        if metric in self.metrics:
            self.metrics[metric] = value
    
    def calculate_value(self) -> Dict[str, ValueMetric]:
        """Calculate total value delivered."""
        # VW-style value calculation
        api_cost_per_call = 0.01  # Assume $0.01 per API call
        research_hour_cost = 50  # Assume $50 per research hour
        
        # Calculate derived metrics
        hours_saved = self.metrics["api_calls_saved"] * 0.1  # 6 minutes per search
        cost_saved = self.metrics["api_calls_saved"] * api_cost_per_call
        research_time_value = hours_saved * research_hour_cost
        
        return {
            "api_calls_saved": ValueMetric(
                name="API调用节省",
                value=self.metrics["api_calls_saved"],
                unit="次",
                description="通过缓存和智能重试节省"
            ),
            "hours_saved": ValueMetric(
                name="时间节省",
                value=hours_saved,
                unit="小时",
                description="自动化和优化带来的时间节省"
            ),
            "cost_saved": ValueMetric(
                name="成本节省",
                value=cost_saved + research_time_value,
                unit="美元",
                description="API成本和时间成本的总节省"
            ),
            "papers_processed": ValueMetric(
                name="论文处理",
                value=self.metrics["papers_processed"],
                unit="篇",
                description="已处理的论文数量"
            ),
        }
    
    def get_value_report(self) -> str:
        """Generate value report (inspired by VW's 700-900 euros)."""
        values = self.calculate_value()
        
        lines = [
            "=" * 60,
            "💰 价值量化报告 (Volkswagen式收益计算)",
            "=" * 60,
            "",
            "Volkswagen承诺: 每年节省700-900欧元",
            "",
            "-" * 60,
        ]
        
        # VW-style value presentation
        for key, metric in values.items():
            if metric.value > 0:
                lines.append(
                    f"📊 {metric.name}: {metric.value:.1f} {metric.unit}"
                )
                lines.append(f"   {metric.description}")
                lines.append("")
        
        total_value = sum(m.value for m in values.values())
        if total_value > 0:
            lines.append("-" * 60)
            lines.append(f"💵 总价值: ${total_value:.2f}")
            lines.append("-" * 60)
            
            # VW-style comparison
            lines.append("")
            lines.append("Volkswagen对比:")
            lines.append("  他们: 700-900欧元/年 ≈ 约770-990美元/年")
            lines.append(f"  我们: ${total_value:.2f}（目前统计）")
            lines.append("")
            lines.append("💡 提示: 持续使用，价值累积！")
        
        lines.append("=" * 60)
        
        return "\n".join(lines)
    
    def get_vw_comparison(self) -> str:
        """Get VW-style comparison."""
        values = self.calculate_value()
        our_value = sum(m.value for m in values.values() if m.value > 0) * 12  # Annualize
        
        return f"""
🚗 Volkswagen vs 🚀 AI Research OS

Volkswagen V2G:
  每年节省: 770-990 美元
  系统级节省: 220亿欧元（2030年）
  个人参与: 基础报酬 + 成本节省

AI Research OS:
  我们节省: ${our_value:.2f}（年化）
  你的时间: 无价
  研究效率: 提升{values.get("papers_processed", 0) * 10}%（估算）

💡 两者都强调: 长期价值 > 短期成本
"""


# Global quantifier
_quantifier = None


def get_value_quantifier():
    """Get or create the global value quantifier."""
    global _quantifier
    if _quantifier is None:
        _quantifier = ValueQuantifier()
    return _quantifier


def print_value_report():
    """Print the value report."""
    quantifier = get_value_quantifier()
    print(quantifier.get_value_report())


def print_vw_comparison():
    """Print VW comparison."""
    quantifier = get_value_quantifier()
    print(quantifier.get_vw_comparison())
